//! Tool surface. After the forager-as-library refactor, a Tool is a struct holding a name,
//! an idempotency declaration, and an async handler closure. The forager that owns the
//! private key composes its tool list at construction and dispatches by name.
//!
//! Process boundary IS identity boundary: every forager process holds exactly one EOA, so
//! every tool call routed to this forager is implicitly authorized as that EOA. There is no
//! per-call `as_bee` arg, no `check_auth` stage; the chi vocabulary keeps the `as_bee` field
//! for backward-compat with the prior shape but consumers ignore it.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::ForagerError;

/// Whether the tool may be retried without side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Idempotency {
    Idempotent,
    NotIdempotent,
}

/// A tool-call as the consumer-product persona's worker bee emits it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Echoed back in the `tool-result` so the consumer can correlate concurrent calls.
    pub call_id: String,
    /// The chi-level `from` identity of the asking persona. Informational only after the
    /// forager-as-library refactor; the forager-process boundary is the identity boundary.
    pub from: String,
    /// Backward-compat field; no longer consumed by the safety pipeline.
    pub as_bee: Option<String>,
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
        Self { call_id: call_id.into(), ok: true, value: Some(value), error: None }
    }
    pub fn fail(call_id: impl Into<String>, error: ForagerError) -> Self {
        Self { call_id: call_id.into(), ok: false, value: None, error: Some(error) }
    }
}

/// Boxed async handler type.
type Handler = Arc<dyn Fn(ToolCall) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> + Send + Sync>;

/// A tool. Owned by exactly one forager process; named with the persona's namespace prefix.
#[derive(Clone)]
pub struct Tool {
    name: String,
    idempotency: Idempotency,
    handler: Handler,
}

impl Tool {
    pub fn new<F, Fut>(name: impl Into<String>, idem: Idempotency, handler: F) -> Self
    where
        F: Fn(ToolCall) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ToolResult> + Send + 'static,
    {
        Self {
            name: name.into(),
            idempotency: idem,
            handler: Arc::new(move |call| Box::pin(handler(call))),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn idempotency(&self) -> Idempotency {
        self.idempotency
    }

    pub async fn invoke(&self, call: ToolCall) -> ToolResult {
        (self.handler)(call).await
    }
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("idempotency", &self.idempotency)
            .finish()
    }
}

/// The forager's tool registry. Built at construction from the base `arc_*` tools plus
/// consumer-product extensions. Lookup is by exact tool name.
#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn register(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    pub fn lookup(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name() == name)
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<_> = self.tools.iter().map(|t| t.name().to_string()).collect();
        v.sort();
        v
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stub_tool(name: &str) -> Tool {
        Tool::new(name.to_string(), Idempotency::Idempotent, |call| async move {
            ToolResult::ok(call.call_id, json!({"stub": true}))
        })
    }

    #[test]
    fn registry_lookup_by_name() {
        let mut r = ToolRegistry::new();
        r.register(stub_tool("arc_read_balance"));
        assert!(r.lookup("arc_read_balance").is_some());
        assert!(r.lookup("nonexistent").is_none());
        assert_eq!(r.len(), 1);
    }

    #[tokio::test]
    async fn tool_invoke_runs_handler_closure() {
        let t = stub_tool("arc_read_state");
        let call = ToolCall {
            call_id: "c-1".into(),
            from: "test-persona".into(),
            as_bee: None,
            tool_name: "arc_read_state".into(),
            args: json!({}),
        };
        let r = t.invoke(call).await;
        assert!(r.ok);
        assert_eq!(r.call_id, "c-1");
    }

    #[test]
    fn names_are_sorted() {
        let r = ToolRegistry::new()
            .with_tools([stub_tool("z_last"), stub_tool("a_first"), stub_tool("m_middle")]);
        assert_eq!(r.names(), vec!["a_first", "m_middle", "z_last"]);
    }
}
