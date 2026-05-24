//! `AskerLoop`: boilerplate for the prompt-and-observe loop.
//!
//! The runtime that hosts a persona implements its own concrete asker loop wired against
//! humd's chi transport. This module provides the shape: how a persona's `Decision::Prompt`
//! turns into an emit-prompt + observe-tool-calls + await-finish sequence on the worker bee's
//! sid.
//!
//! The boilerplate is independent of any specific humd binding so persona implementations
//! stay testable. A runtime crate (e.g. `daman-runtime` or `reverb-markets-runtime`) plugs in
//! the concrete transport.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::persona::{Decision, Event, PersonaBee};

/// One observation from the worker bee's sid. Mirrors the chi vocabulary at the protocol
/// layer: `chunk`, `tool-call`, `tool-result`, `finish`, `error`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "chi")]
pub enum Observation {
    #[serde(rename = "chunk")]
    Chunk { sid: String, text: String },
    #[serde(rename = "tool-call")]
    ToolCall { sid: String, call_id: String, tool_name: String, args: Value },
    #[serde(rename = "tool-result")]
    ToolResult { sid: String, call_id: String, ok: bool, value: Option<Value> },
    #[serde(rename = "finish")]
    Finish { sid: String, usage: Option<Value> },
    #[serde(rename = "error")]
    Error { sid: String, qualifier: String, detail: Option<String> },
}

/// Transport injected by the runtime. Concrete impls wire against humd.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Emit `chi:"prompt"` on a sid into the worker bee. The transport handles the chi
    /// envelope construction and routing to the local humd.
    async fn prompt(&self, sid: &str, system_prompt: &str, user_prompt: &str)
        -> Result<(), TransportError>;

    /// Receive the next observation from the sid (chunk, tool-call, tool-result, finish,
    /// error). Blocks until one arrives or the transport closes.
    async fn next_observation(&self, sid: &str) -> Option<Observation>;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    #[error("sid `{sid}` is closed")]
    SidClosed { sid: String },
    #[error("transport i/o: {0}")]
    Io(String),
}

/// The boilerplate asker loop. `run_one` processes a single event end-to-end: ask persona
/// for a decision, optionally emit prompt, optionally observe until finish.
pub struct AskerLoop<P: PersonaBee, T: Transport> {
    pub persona: P,
    pub transport: T,
}

impl<P: PersonaBee, T: Transport> AskerLoop<P, T> {
    pub fn new(persona: P, transport: T) -> Self {
        Self { persona, transport }
    }

    /// Process one event. Returns the bloom transcript if the persona prompted; an empty
    /// transcript if the persona skipped.
    pub async fn run_one(&self, event: Event) -> Result<Vec<Observation>, TransportError> {
        let decision = self.persona.on_event(event).await;
        let (sid, system, user) = match decision {
            Decision::Skip { reason } => {
                tracing::debug!(persona = self.persona.bee_name(), reason, "persona skipped");
                return Ok(vec![]);
            }
            Decision::Prompt { sid, system_prompt, user_prompt } => (sid, system_prompt, user_prompt),
        };
        self.transport.prompt(&sid, &system, &user).await?;
        let mut transcript = Vec::new();
        while let Some(obs) = self.transport.next_observation(&sid).await {
            let is_terminal = matches!(obs, Observation::Finish { .. } | Observation::Error { .. });
            transcript.push(obs);
            if is_terminal {
                break;
            }
        }
        Ok(transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persona::{EventFilter, PersonaBee};
    use std::sync::Mutex;

    struct PromptOncePersona;

    #[async_trait]
    impl PersonaBee for PromptOncePersona {
        fn bee_name(&self) -> &str { "test-prompter" }
        fn subscribe_topics(&self) -> Vec<String> { vec![] }
        fn subscribe_chain_events(&self) -> Vec<EventFilter> { vec![] }
        fn persona_system_prompt(&self) -> Option<String> { Some("S".into()) }
        async fn on_event(&self, _event: Event) -> Decision {
            Decision::Prompt {
                sid: "s-1".into(),
                system_prompt: "S".into(),
                user_prompt: "U".into(),
            }
        }
    }

    struct MockTransport {
        prompted: Mutex<Vec<(String, String, String)>>,
        observations: Mutex<Vec<Observation>>,
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn prompt(&self, sid: &str, system: &str, user: &str)
            -> Result<(), TransportError>
        {
            self.prompted.lock().unwrap().push((sid.into(), system.into(), user.into()));
            Ok(())
        }
        async fn next_observation(&self, _sid: &str) -> Option<Observation> {
            let mut q = self.observations.lock().unwrap();
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        }
    }

    #[tokio::test]
    async fn asker_loop_runs_prompt_and_collects_until_finish() {
        let transport = MockTransport {
            prompted: Mutex::new(vec![]),
            observations: Mutex::new(vec![
                Observation::Chunk { sid: "s-1".into(), text: "ok".into() },
                Observation::Finish { sid: "s-1".into(), usage: None },
            ]),
        };
        let loop_ = AskerLoop::new(PromptOncePersona, transport);
        let transcript = loop_
            .run_one(Event::Gossip {
                topic: "x".into(),
                body: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert_eq!(transcript.len(), 2);
        assert_eq!(loop_.transport.prompted.lock().unwrap().len(), 1);
        match &transcript[1] {
            Observation::Finish { sid, .. } => assert_eq!(sid, "s-1"),
            _ => panic!("expected finish at end"),
        }
    }
}
