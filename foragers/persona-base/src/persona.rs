//! The `PersonaBee` trait. Consumer products implement this for each role they need.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A chain-event filter the persona subscribes to. Concrete encoding is left to the runtime
/// (alloy/ethers `Filter`, raw RPC params, etc.); the persona only declares intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub contract: String,
    pub event_signature: String,
    pub from_block: Option<u64>,
}

/// An event delivered to the persona's `on_event` handler. Either a gossip tone observed on
/// a subscribed topic, or a chain event observed on a subscribed filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    Gossip { topic: String, body: Value },
    ChainEvent { contract: String, event: String, data: Value },
}

/// The persona's decision after assembling world state. The persona itself decides only
/// whether to prompt the worker (and with what context); the worker's LLM decides what to
/// do via tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    /// Prompt the worker on a new sid; pair the role-overlay system prompt with the assembled
    /// world state.
    Prompt { sid: String, system_prompt: String, user_prompt: String },
    /// No-op for this observation. The persona logged and moved on.
    Skip { reason: String },
}

/// Implemented by every persona role.
#[async_trait]
pub trait PersonaBee: Send + Sync {
    /// The bee name (e.g. `daman-watchdog-aggressive`, `markets-arbiter-conservative`).
    fn bee_name(&self) -> &str;

    /// Gossip topics this persona subscribes to.
    fn subscribe_topics(&self) -> Vec<String>;

    /// Chain event filters this persona subscribes to.
    fn subscribe_chain_events(&self) -> Vec<EventFilter>;

    /// The role-overlay system prompt this persona pairs with every prompt. For L5 personas,
    /// this is the persona's entire role description. For L1 personas, returns `None` and
    /// `on_event` is deterministic.
    fn persona_system_prompt(&self) -> Option<String>;

    /// The persona's loop: assemble world state from `event`, decide whether to prompt the
    /// worker. The runtime (`AskerLoop`) handles emitting the prompt + observing tool calls.
    async fn on_event(&self, event: Event) -> Decision;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubPersona;

    #[async_trait]
    impl PersonaBee for StubPersona {
        fn bee_name(&self) -> &str { "test-persona-noop" }
        fn subscribe_topics(&self) -> Vec<String> {
            vec!["test/topic".into()]
        }
        fn subscribe_chain_events(&self) -> Vec<EventFilter> { vec![] }
        fn persona_system_prompt(&self) -> Option<String> {
            Some("You are a test persona.".into())
        }
        async fn on_event(&self, _event: Event) -> Decision {
            Decision::Skip { reason: "stub".into() }
        }
    }

    #[tokio::test]
    async fn stub_persona_returns_skip() {
        let p = StubPersona;
        let d = p
            .on_event(Event::Gossip {
                topic: "test/topic".into(),
                body: serde_json::json!({}),
            })
            .await;
        match d {
            Decision::Skip { reason } => assert_eq!(reason, "stub"),
            _ => panic!("expected skip"),
        }
    }

    #[test]
    fn event_serializes_with_kind_tag() {
        let e = Event::Gossip {
            topic: "daman/slash/observability".into(),
            body: serde_json::json!({"claim_id": 42}),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"kind\":\"Gossip\""));
        assert!(json.contains("\"topic\":\"daman/slash/observability\""));
    }
}
