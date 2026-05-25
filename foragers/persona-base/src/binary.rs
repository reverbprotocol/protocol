//! `PersonaBinaryBuilder`: the canonical "one persona = one process" composition root.
//!
//! Pulls together a forager (via `reverb-arc-fs`-style `PersonaForagerBuilder`) and an
//! asker loop (via this crate's `AskerLoop`) into a single binary that:
//!
//! - holds one EOA private key (the forager owns it; the process IS the bee)
//! - exposes one namespaced tool surface
//! - subscribes to its gossip topics + chain events
//! - opens one sid into the local worker bee on every persona-event
//!
//! Consumer-product persona binaries call `PersonaBinaryBuilder::new(...).run(...)` and that's it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForagerConfig {
    pub allowed_contracts: Vec<String>,
    pub rpc_url: String,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskerConfig {
    /// Gossip topics this persona subscribes to.
    pub topics: Vec<String>,
    /// Worker bee name to prompt into.
    pub worker_bee: String,
}

/// Specification for a persona binary. `bee_name`, `private_key`, `system_prompt`, `tools`,
/// `forager_config`, `asker_config` are the load-bearing fields; the runtime wires this spec
/// to humd via a transport injected at `run` time.
#[derive(Debug, Clone)]
pub struct PersonaBinarySpec {
    pub bee_name: String,
    pub namespace: String,
    pub system_prompt: String,
    pub forager_config: ForagerConfig,
    pub asker_config: AskerConfig,
}

impl PersonaBinarySpec {
    pub fn new(
        bee_name: impl Into<String>,
        namespace: impl Into<String>,
        system_prompt: impl Into<String>,
        forager_config: ForagerConfig,
        asker_config: AskerConfig,
    ) -> Self {
        Self {
            bee_name: bee_name.into(),
            namespace: namespace.into(),
            system_prompt: system_prompt.into(),
            forager_config,
            asker_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_carries_required_fields() {
        let spec = PersonaBinarySpec::new(
            "markets-auto-create",
            "mkac",
            "You identify newly-released macro data and decide whether to create a forward market.",
            ForagerConfig {
                allowed_contracts: vec!["0x344b472b7b1ad0a35e11718bc063fd46f4282db2".into()],
                rpc_url: "https://rpc.testnet.arc.network".into(),
                source_url: None,
            },
            AskerConfig {
                topics: vec!["reverb-markets/releases/macro".into()],
                worker_bee: "claude-cli".into(),
            },
        );
        assert_eq!(spec.bee_name, "markets-auto-create");
        assert_eq!(spec.namespace, "mkac");
        assert_eq!(spec.asker_config.topics.len(), 1);
        assert_eq!(spec.forager_config.allowed_contracts.len(), 1);
    }
}
