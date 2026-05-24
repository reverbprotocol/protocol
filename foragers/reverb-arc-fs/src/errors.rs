//! Structured error types returned via `chi:"tool-result"`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "PascalCase")]
pub enum ForagerError {
    #[error("invalid key format: expected 0x-prefixed 64-char hex")]
    InvalidKeyFormat,

    #[error("keyring i/o: {0}")]
    KeyringIo(String),

    #[error("keyring file has insecure permissions (mode {mode:o}); require 0600")]
    KeyringPermissions { mode: u32 },

    #[error("no key registered for bee `{bee}`")]
    KeyringMiss { bee: String },

    #[error("config i/o: {0}")]
    ConfigIo(String),

    #[error("invalid config: {0}")]
    ConfigInvalid(String),

    #[error("write tools require an `as_bee` argument")]
    AuthMissing,

    #[error("auth mismatch: chi.from `{from}` != as_bee `{as_bee}`; refused to impersonate")]
    AuthMismatch { from: String, as_bee: String },

    #[error("ABI validation failed: {reason}")]
    AbiValidation { reason: String },

    #[error("simulation would revert: {reason}")]
    SimulationRevert { reason: String },

    #[error("rate limit hit for bee `{bee}`: {limit} tx/min")]
    RateLimitBee { bee: String, limit: u32 },

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
}
