//! Five-stage safety pipeline applied to every write tool call (after the forager-as-library
//! refactor; the prior `check_auth` stage is removed since the forager process boundary IS
//! the identity boundary).
//!
//! 1. ABI validation: args match on-chain function input schema
//! 2. Allowed-contracts boundary: target contract is in the forager's config
//! 3. Simulation gate: `eth_call` against current state; revert surfaces error
//! 4. Rate limit: per-tool + global throttles
//! 5. Send: sign + submit with the forager's single EOA; on chain revert despite simulation
//!    pass, log race + return error
//! 6. Receipt cache: tx hash + receipt cached for downstream reads
//!
//! Each stage is a checkable invariant. A forager that skips any stage is non-conformant.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::config::RateLimit;
use crate::errors::ForagerError;
use crate::keyring::PrivateKey;

/// Stage 1: ABI validation. Implementations plug in their concrete ABI shape via a closure.
pub fn check_abi<F>(args: &Value, schema_check: F) -> Result<(), ForagerError>
where
    F: FnOnce(&Value) -> Result<(), String>,
{
    schema_check(args).map_err(|reason| ForagerError::AbiValidation { reason })
}

/// Stage 2: allowed-contracts boundary. Hard write boundary on which contracts the forager
/// will send transactions to.
pub fn check_allowed(allowed: &[String], contract: &str) -> Result<(), ForagerError> {
    let target = contract.to_ascii_lowercase();
    let ok = allowed.iter().any(|c| c.to_ascii_lowercase() == target);
    if ok {
        Ok(())
    } else {
        Err(ForagerError::ContractNotAllowed { contract: contract.to_string() })
    }
}

/// Stage 3: simulation gate.
pub trait SimulationGate: Send + Sync {
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

/// Stage 4: rate limit. Sliding-window counter per tool + global. Per-bee counters are no
/// longer meaningful (each forager process is one bee; per-bee is per-process; that's the
/// global counter for this process).
pub struct RateLimiter {
    config: RateLimit,
    per_tool: Mutex<HashMap<String, Vec<Instant>>>,
    global: Mutex<Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(config: RateLimit) -> Self {
        Self {
            config,
            per_tool: Mutex::new(HashMap::new()),
            global: Mutex::new(Vec::new()),
        }
    }

    pub fn check_and_record(&self, tool: &str) -> Result<(), ForagerError> {
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

        global.push(now);
        tool_window.push(now);
        Ok(())
    }
}

/// Stage 5+6 result.
#[derive(Debug, Clone)]
pub struct SendOutcome {
    pub tx_hash: String,
    pub receipt: Value,
}

/// Stage 5: send. Concrete sign-and-submit lives behind this trait.
pub trait Sender: Send + Sync {
    fn sign_and_send(&self, key: &PrivateKey, to: &str, data: &[u8])
        -> Result<SendOutcome, String>;
}

pub fn send(
    sender: &dyn Sender,
    key: &PrivateKey,
    to: &str,
    data: &[u8],
) -> Result<SendOutcome, ForagerError> {
    sender
        .sign_and_send(key, to, data)
        .map_err(|reason| ForagerError::SendFailed { reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_boundary_blocks_unknown_contract() {
        let allowed = vec!["0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F".to_string()];
        assert!(check_allowed(&allowed, "0xc8bf99c55703bc682a3efd5c8a728eaeda3e121f").is_ok());
        let err = check_allowed(&allowed, "0x1111111111111111111111111111111111111111").unwrap_err();
        assert!(matches!(err, ForagerError::ContractNotAllowed { .. }));
    }

    #[test]
    fn rate_limit_blocks_at_per_tool_cap() {
        let rl = RateLimiter::new(RateLimit {
            per_bee_tx_per_minute: 1000,
            per_tool_tx_per_minute: 2,
            global_tx_per_minute: 100,
        });
        assert!(rl.check_and_record("markets_resolve_market").is_ok());
        assert!(rl.check_and_record("markets_resolve_market").is_ok());
        let err = rl.check_and_record("markets_resolve_market").unwrap_err();
        assert!(matches!(err, ForagerError::RateLimitTool { .. }));
    }

    #[test]
    fn rate_limit_blocks_at_global_cap() {
        let rl = RateLimiter::new(RateLimit {
            per_bee_tx_per_minute: 1000,
            per_tool_tx_per_minute: 100,
            global_tx_per_minute: 1,
        });
        rl.check_and_record("markets_create_market").unwrap();
        let err = rl.check_and_record("markets_resolve_market").unwrap_err();
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
