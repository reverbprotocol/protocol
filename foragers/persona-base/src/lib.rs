//! # persona-base
//!
//! Persona bee scaffolding under the Reverb Protocol operating-model standard. A persona is
//! a thin asker: it assembles world state from gossip + chain events, opens a sid into a
//! local LLM worker (e.g. `claude-cli`), prompts the worker with a tight role-overlay system
//! prompt, observes the worker's tool calls flow through the local forager, and logs.
//!
//! The persona itself contains no decision logic. The decision is the LLM's, made inside
//! the worker bee's sid. The persona's job is to be honest about its role and to keep its
//! prompts tight.
//!
//! Spec: <https://reverbprotocol.github.io/protocol/OPERATING_MODEL#the-persona-bee-contract>

pub mod persona;
pub mod asker;
pub mod observer;
pub mod bust;
pub mod binary;

pub use persona::{Decision, Event, EventFilter, PersonaBee};
pub use asker::AskerLoop;
pub use observer::ToolCallObserver;
pub use bust::BustDetector;
pub use binary::{AskerConfig, ForagerConfig, PersonaBinarySpec};
