//! Structured error types returned via `chi:"tool-result"`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "PascalCase")]
pub enum ForagerError {
    #[error("invalid key format: expected 0x-prefixed 64-char hex")]
    InvalidKeyFormat,

    #[error("key i/o: {0}")]
    KeyIo(String),

    #[error("key file has insecure permissions (mode {mode:o}); require 0600")]
    KeyPermissions { mode: u32 },

    #[error("config i/o: {0}")]
    ConfigIo(String),

    #[error("invalid config: {0}")]
    ConfigInvalid(String),

    #[error("ABI validation failed: {reason}")]
    AbiValidation { reason: String },

    #[error("simulation would revert: {reason}")]
    SimulationRevert { reason: String },

    #[error("rate limit hit for tool `{tool}`: {limit} tx/min")]
    RateLimitTool { tool: String, limit: u32 },

    #[error("global rate limit hit: {limit} tx/min")]
    RateLimitGlobal { limit: u32 },

    #[error("send failed: {reason}")]
    SendFailed { reason: String },

    #[error("contract `{contract}` is not in the allowed-writes list")]
    ContractNotAllowed { contract: String },

    #[error("unknown tool `{tool}` (not in hello manifest)")]
    UnknownTool { tool: String },

    #[error("forager configuration error: {0}")]
    BuilderIncomplete(String),
}
