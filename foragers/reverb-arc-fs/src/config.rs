//! Forager scoping configuration loaded from `~/.config/hum/{forager-name}/config.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::ForagerError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RateLimit {
    pub per_bee_tx_per_minute: u32,
    pub per_tool_tx_per_minute: u32,
    pub global_tx_per_minute: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            per_bee_tx_per_minute: 12,
            per_tool_tx_per_minute: 60,
            global_tx_per_minute: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub explorer_api: String,
    pub chain_id: u64,
    /// Hard boundary on which contracts the forager will write to. Analog of `humfs`'s `fs.roots`.
    pub allowed_contracts: Vec<String>,
    pub rate_limit: RateLimit,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ForagerError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ForagerError::ConfigIo(e.to_string()))?;
        let cfg: Config = serde_json::from_str(&content)
            .map_err(|e| ForagerError::ConfigIo(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ForagerError> {
        if self.rpc_url.is_empty() {
            return Err(ForagerError::ConfigInvalid("rpc_url is empty".into()));
        }
        if self.chain_id == 0 {
            return Err(ForagerError::ConfigInvalid("chain_id is zero".into()));
        }
        if self.allowed_contracts.is_empty() {
            return Err(ForagerError::ConfigInvalid(
                "allowed_contracts must list at least one contract; the forager refuses unrestricted-write configs".into(),
            ));
        }
        for c in &self.allowed_contracts {
            if !c.starts_with("0x") || c.len() != 42 {
                return Err(ForagerError::ConfigInvalid(format!(
                    "invalid contract address: {c}"
                )));
            }
        }
        Ok(())
    }

    /// Whether this contract is in the allowed-writes list. Address comparison is case-insensitive.
    pub fn is_allowed_contract(&self, contract: &str) -> bool {
        let target = contract.to_ascii_lowercase();
        self.allowed_contracts
            .iter()
            .any(|c| c.to_ascii_lowercase() == target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Config {
        Config {
            rpc_url: "https://rpc.testnet.arc.network".into(),
            explorer_api: "https://testnet.arcscan.app/api/v2".into(),
            chain_id: 5042002,
            allowed_contracts: vec![
                "0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F".into(),
                "0x344b472b7b1ad0a35e11718bc063fd46f4282db2".into(),
            ],
            rate_limit: RateLimit::default(),
        }
    }

    #[test]
    fn rate_limit_defaults_match_spec() {
        let r = RateLimit::default();
        assert_eq!(r.per_bee_tx_per_minute, 12);
        assert_eq!(r.per_tool_tx_per_minute, 60);
        assert_eq!(r.global_tx_per_minute, 200);
    }

    #[test]
    fn validates_clean_config() {
        assert!(fixture().validate().is_ok());
    }

    #[test]
    fn rejects_empty_allowed_contracts() {
        let mut c = fixture();
        c.allowed_contracts.clear();
        let err = c.validate().unwrap_err();
        assert!(matches!(err, ForagerError::ConfigInvalid(_)));
    }

    #[test]
    fn rejects_malformed_address() {
        let mut c = fixture();
        c.allowed_contracts.push("not-an-address".into());
        assert!(c.validate().is_err());
    }

    #[test]
    fn allowed_contract_check_is_case_insensitive() {
        let c = fixture();
        // Lowercase variant of the substrate proxy
        assert!(c.is_allowed_contract("0xc8bf99c55703bc682a3efd5c8a728eaeda3e121f"));
        assert!(c.is_allowed_contract("0xC8BF99C55703BC682A3EFD5C8A728EAEDA3E121F"));
        assert!(!c.is_allowed_contract("0x1111111111111111111111111111111111111111"));
    }
}
