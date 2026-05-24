//! # reverb-arc-fs
//!
//! Canonical resource forager for Arc chain state + wallet operations under the Reverb
//! Protocol operating-model standard.
//!
//! A forager is a thrum-attached process that owns a resource and exposes operations as
//! `chi:"tool-call"`-addressable tools routed by humd. This crate provides the substrate's
//! reference implementation of the forager-hive contract documented at
//! <https://reverbprotocol.github.io/protocol/OPERATING_MODEL>.
//!
//! Consumer products extend this crate by importing it as a dependency and adding their own
//! product-specific tools that internally compose `arc_read_*` and `arc_send_tx`.
//!
//! ## Scope
//!
//! This crate ships the off-chain operating-model surface:
//!
//! - the hello-manifest builder ([`manifest`])
//! - the wallet keyring ([`keyring`]) with per-bee scoping and 0600 file permissions
//! - the scoping config loader ([`config`]) with `allowed_contracts` and rate limits
//! - the tool registry and dispatch trait ([`tools`])
//! - the six-stage safety pipeline ([`safety`]): auth, ABI validation, simulation gate, rate
//!   limit, send, receipt cache. Each stage is a checkable invariant.
//!
//! The thrum/humd attachment layer and the actual chain I/O are abstracted behind traits so
//! the crate is unit-testable and can be wired against any concrete RPC transport (alloy,
//! ethers, custom) by the operator's process layer.

pub mod manifest;
pub mod keyring;
pub mod config;
pub mod tools;
pub mod safety;
pub mod errors;

pub use errors::ForagerError;
