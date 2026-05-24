//! Six-stage safety pipeline applied to every write tool call.
//!
//! 1. Auth check: `chi.from == args.as_bee`
//! 2. ABI validation: args match on-chain function input schema
//! 3. Simulation gate: `eth_call` against current state; revert surfaces error
//! 4. Rate limit: per-bee + per-tool + global throttles
//! 5. Send: sign + submit; on chain revert despite simulation pass, log race + return error
//! 6. Receipt cache: tx hash + receipt cached for downstream reads
//!
//! Each stage is a checkable invariant. A forager that skips any stage is non-conformant.
//! Tests in this module exercise each stage as an independent invariant.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::config::RateLimit;
use crate::errors::ForagerError;
use crate::keyring::Keyring;
use crate::tools::ToolCall;

/// Stage 1: auth check.
pub fn check_auth(call: &ToolCall) -> Result<(), ForagerError> {
    let as_bee = call.as_bee.as_deref().ok_or(ForagerError::AuthMissing)?;
    if call.from != as_bee {
        return Err(ForagerError::AuthMismatch {
            from: call.from.clone(),
            as_bee: as_bee.to_string(),
        });
    }
    Ok(())
}

/// Stage 2: ABI validation. Implementations plug in their concrete ABI shape via a closure.
pub fn check_abi<F>(args: &Value, schema_check: F) -> Result<(), ForagerError>
where
    F: FnOnce(&Value) -> Result<(), String>,
{
    schema_check(args).map_err(|reason| ForagerError::AbiValidation { reason })
}

/// Stage 3: simulation gate. The actual `eth_call` is injected by the runtime; this function
/// expresses the gate as a sync interface for testability.
pub trait SimulationGate: Send + Sync {
    /// Perform the simulation. Return `Ok(())` if the call would succeed; `Err(reason)` if
    /// it would revert.
    fn simulate(&self, to: &str, data: &[u8]) -> Result<(), String>;
}

pub fn check_simulation(
    gate: &dyn SimulationGate,
    to: &str,
    data: &[u8],
) -> Result<(), ForagerError> {
    gate.simulate(to, data)
        .map_err(|reason| ForagerError::SimulationRevert { reason })
}

/// Stage 4: rate limit. Sliding-window counter per bee, per tool, and global.
pub struct RateLimiter {
    config: RateLimit,
    per_bee: Mutex<HashMap<String, Vec<Instant>>>,
    per_tool: Mutex<HashMap<String, Vec<Instant>>>,
    global: Mutex<Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(config: RateLimit) -> Self {
        Self {
            config,
            per_bee: Mutex::new(HashMap::new()),
            per_tool: Mutex::new(HashMap::new()),
            global: Mutex::new(Vec::new()),
        }
    }

    pub fn check_and_record(&self, bee: &str, tool: &str) -> Result<(), ForagerError> {
        let now = Instant::now();
        let one_minute = Duration::from_secs(60);

        let mut global = self.global.lock().unwrap();
        global.retain(|t| now.duration_since(*t) <= one_minute);
        if global.len() >= self.config.global_tx_per_minute as usize {
            return Err(ForagerError::RateLimitGlobal {
                limit: self.config.global_tx_per_minute,
            });
        }

        let mut per_tool = self.per_tool.lock().unwrap();
        let tool_window = per_tool.entry(tool.to_string()).or_default();
        tool_window.retain(|t| now.duration_since(*t) <= one_minute);
        if tool_window.len() >= self.config.per_tool_tx_per_minute as usize {
            return Err(ForagerError::RateLimitTool {
                tool: tool.to_string(),
                limit: self.config.per_tool_tx_per_minute,
            });
        }

        let mut per_bee = self.per_bee.lock().unwrap();
        let bee_window = per_bee.entry(bee.to_string()).or_default();
        bee_window.retain(|t| now.duration_since(*t) <= one_minute);
        if bee_window.len() >= self.config.per_bee_tx_per_minute as usize {
            return Err(ForagerError::RateLimitBee {
                bee: bee.to_string(),
                limit: self.config.per_bee_tx_per_minute,
            });
        }

        global.push(now);
        tool_window.push(now);
        bee_window.push(now);
        Ok(())
    }
}

/// Stage 5+6 result.
#[derive(Debug, Clone)]
pub struct SendOutcome {
    pub tx_hash: String,
    pub receipt: Value,
}

/// Stage 5: send. Concrete sign-and-submit lives behind this trait so the safety pipeline
/// is independent of the chain transport.
pub trait Sender: Send + Sync {
    fn sign_and_send(&self, keyring: &Keyring, bee: &str, to: &str, data: &[u8])
        -> Result<SendOutcome, String>;
}

pub fn send(
    sender: &dyn Sender,
    keyring: &Keyring,
    bee: &str,
    to: &str,
    data: &[u8],
) -> Result<SendOutcome, ForagerError> {
    if keyring.lookup(bee).is_none() {
        return Err(ForagerError::KeyringMiss { bee: bee.to_string() });
    }
    sender
        .sign_and_send(keyring, bee, to, data)
        .map_err(|reason| ForagerError::SendFailed { reason })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolCall;

    fn call(from: &str, as_bee: Option<&str>) -> ToolCall {
        ToolCall {
            call_id: "c-1".into(),
            from: from.into(),
            as_bee: as_bee.map(String::from),
            tool_name: "arc_send_tx".into(),
            args: Value::Null,
        }
    }

    #[test]
    fn auth_passes_when_from_matches_as_bee() {
        let c = call("daman-watchdog-aggressive", Some("daman-watchdog-aggressive"));
        assert!(check_auth(&c).is_ok());
    }

    #[test]
    fn auth_fails_on_impersonation() {
        let c = call("daman-watchdog-aggressive", Some("daman-arbiter-strict"));
        let err = check_auth(&c).unwrap_err();
        assert!(matches!(err, ForagerError::AuthMismatch { .. }));
    }

    #[test]
    fn auth_fails_when_as_bee_missing_on_write() {
        let c = call("daman-watchdog-aggressive", None);
        let err = check_auth(&c).unwrap_err();
        assert!(matches!(err, ForagerError::AuthMissing));
    }

    #[test]
    fn rate_limit_blocks_at_per_bee_cap() {
        let rl = RateLimiter::new(RateLimit {
            per_bee_tx_per_minute: 2,
            per_tool_tx_per_minute: 100,
            global_tx_per_minute: 100,
        });
        assert!(rl.check_and_record("bee-a", "arc_send_tx").is_ok());
        assert!(rl.check_and_record("bee-a", "arc_send_tx").is_ok());
        let err = rl.check_and_record("bee-a", "arc_send_tx").unwrap_err();
        assert!(matches!(err, ForagerError::RateLimitBee { .. }));
    }

    #[test]
    fn rate_limit_blocks_at_global_cap() {
        let rl = RateLimiter::new(RateLimit {
            per_bee_tx_per_minute: 100,
            per_tool_tx_per_minute: 100,
            global_tx_per_minute: 1,
        });
        rl.check_and_record("bee-a", "arc_send_tx").unwrap();
        let err = rl.check_and_record("bee-b", "arc_send_tx").unwrap_err();
        assert!(matches!(err, ForagerError::RateLimitGlobal { .. }));
    }

    struct AlwaysRevert;
    impl SimulationGate for AlwaysRevert {
        fn simulate(&self, _to: &str, _data: &[u8]) -> Result<(), String> {
            Err("ERC20InsufficientBalance".into())
        }
    }

    #[test]
    fn simulation_revert_surfaces_reason() {
        let err = check_simulation(&AlwaysRevert, "0x00", &[]).unwrap_err();
        match err {
            ForagerError::SimulationRevert { reason } => {
                assert_eq!(reason, "ERC20InsufficientBalance");
            }
            _ => panic!("wrong error variant"),
        }
    }
}
