//! # reverb-arc-fs
//!
//! Reusable Rust crate that consumer-product persona binaries import to compose their own
//! per-persona forager. The forager-as-library pattern: each persona binary is its own
//! forager process holding one EOA private key, one namespaced tool surface, one humd
//! connection, and one stable ed25519 hid.
//!
//! Process boundary IS identity boundary, matching the `humfs` per-instance `fs.roots`
//! pattern from hum's hives catalogue. Consumer products extend the base tool set via
//! sibling crates (e.g. `reverb-markets-arc-fs`) that expose
//! `<product>_tools(namespace: &str) -> Vec<Tool>` factories.
//!
//! Each persona binary carries two keys with separate lifecycles:
//! - [`BeeIdentity`] — ed25519 seed at `$XDG_STATE_HOME/hum/bees/<kind>.key`, hashed to
//!   `fbee_<hex>` for the mandatory `hid` field on the hello. humd uses this to dedupe
//!   the bee across reconnects; without it, every reconnect leaks a fresh manifest.
//! - [`PrivateKey`] — secp256k1 EOA at `~/.config/hum/<bee>/key`, used to sign Arc
//!   transactions through the forager's safety pipeline.
//!
//! Spec: <https://reverbprotocol.github.io/protocol/OPERATING_MODEL>
//! Hum hives contract: <https://adiled.github.io/hum/hives/>

pub mod manifest;
pub mod keyring;
pub mod identity;
pub mod config;
pub mod tools;
pub mod safety;
pub mod errors;
pub mod persona_forager;

pub use errors::ForagerError;
pub use identity::{BeeIdentity, BeeRole};
pub use keyring::PrivateKey;
pub use manifest::Hello;
pub use persona_forager::{PersonaForager, PersonaForagerBuilder};
pub use tools::{Idempotency, Tool, ToolCall, ToolRegistry, ToolResult};
