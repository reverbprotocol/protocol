//! Tool registry + dispatch trait.
//!
//! A tool is a named callable the forager exposes to humd via its hello manifest. Each tool
//! has a typed input schema, a typed output schema, and an idempotency declaration. Read
//! tools are idempotent; write tools are not.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::ForagerError;

/// Whether the tool may be retried without side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Idempotency {
    /// Repeated calls with the same args produce the same result; no chain state change.
    Idempotent,
    /// Calling twice may produce two on-chain transactions; not safely retryable.
    NotIdempotent,
}

/// A tool-call as the consumer-product persona emits it (inside a `chi:"tool-call"` tone).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Echoed back in the `tool-result` so the consumer can correlate concurrent calls.
    pub call_id: String,
    /// The chi-level `from` identity of the asking persona.
    pub from: String,
    /// The bee whose EOA should sign (for write tools). MUST equal `from` or auth fails.
    pub as_bee: Option<String>,
    /// snake_case tool name; matches an entry from the hello manifest's `tools` field.
    pub tool_name: String,
    pub args: Value,
}

/// The result a tool returns (inside a `chi:"tool-result"` tone).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub ok: bool,
    pub value: Option<Value>,
    pub error: Option<ForagerError>,
}

impl ToolResult {
    pub fn ok(call_id: impl Into<String>, value: Value) -> Self {
        Self {
            call_id: call_id.into(),
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn fail(call_id: impl Into<String>, error: ForagerError) -> Self {
        Self {
            call_id: call_id.into(),
            ok: false,
            value: None,
            error: Some(error),
        }
    }
}

/// Implemented by every tool. The forager's dispatch loop locates a `Tool` by `name()` and
/// calls `invoke()` with the validated args.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn idempotency(&self) -> Idempotency;
    async fn invoke(&self, call: ToolCall) -> ToolResult;
}

/// The forager's runtime tool registry. Built at boot from the hello manifest plus extension
/// crates.
pub struct ToolRegistry {
    by_name: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name();
        self.by_name.insert(name, tool);
    }

    pub fn lookup(&self, name: &str) -> Option<&dyn Tool> {
        self.by_name.get(name).map(|t| t.as_ref())
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.by_name.keys().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTool;

    #[async_trait]
    impl Tool for StubTool {
        fn name(&self) -> &'static str { "arc_read_balance" }
        fn idempotency(&self) -> Idempotency { Idempotency::Idempotent }
        async fn invoke(&self, call: ToolCall) -> ToolResult {
            ToolResult::ok(call.call_id, serde_json::json!({"balance": "1000"}))
        }
    }

    #[test]
    fn registry_lookup_by_name() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(StubTool));
        assert!(r.lookup("arc_read_balance").is_some());
        assert!(r.lookup("nonexistent").is_none());
        assert_eq!(r.len(), 1);
    }

    #[tokio::test]
    async fn stub_tool_returns_ok_with_call_id() {
        let t = StubTool;
        let call = ToolCall {
            call_id: "c-42".into(),
            from: "daman-watchdog-aggressive".into(),
            as_bee: Some("daman-watchdog-aggressive".into()),
            tool_name: "arc_read_balance".into(),
            args: serde_json::json!({"address": "0xabc"}),
        };
        let r = t.invoke(call).await;
        assert_eq!(r.call_id, "c-42");
        assert!(r.ok);
    }
}
