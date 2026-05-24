//! `ToolCallObserver`: surface every tool-call flowing through a persona's sid for
//! dashboards + debugging. The persona itself does not introspect tool calls (those are the
//! worker's decisions); the observer is an out-of-band reader.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedToolCall {
    pub sid: String,
    pub call_id: String,
    pub tool_name: String,
    pub args: Value,
    pub observed_at_unix_secs: u64,
}

#[derive(Debug, Default)]
pub struct ToolCallObserver {
    seen: Vec<ObservedToolCall>,
}

impl ToolCallObserver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, obs: ObservedToolCall) {
        self.seen.push(obs);
    }

    /// Total observed tool-calls across all sids.
    pub fn count(&self) -> usize {
        self.seen.len()
    }

    /// Calls matching a tool name (e.g. count `markets_resolve_market` across all personas).
    pub fn count_by_tool(&self, tool: &str) -> usize {
        self.seen.iter().filter(|c| c.tool_name == tool).count()
    }

    /// All observed calls; for dashboards that want the full transcript.
    pub fn drain(&mut self) -> Vec<ObservedToolCall> {
        std::mem::take(&mut self.seen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_aggregate_across_sids_and_tools() {
        let mut o = ToolCallObserver::new();
        o.record(ObservedToolCall {
            sid: "s-1".into(),
            call_id: "c-1".into(),
            tool_name: "markets_create_market".into(),
            args: Value::Null,
            observed_at_unix_secs: 1,
        });
        o.record(ObservedToolCall {
            sid: "s-1".into(),
            call_id: "c-2".into(),
            tool_name: "markets_resolve_market".into(),
            args: Value::Null,
            observed_at_unix_secs: 2,
        });
        o.record(ObservedToolCall {
            sid: "s-2".into(),
            call_id: "c-3".into(),
            tool_name: "markets_resolve_market".into(),
            args: Value::Null,
            observed_at_unix_secs: 3,
        });
        assert_eq!(o.count(), 3);
        assert_eq!(o.count_by_tool("markets_resolve_market"), 2);
        assert_eq!(o.count_by_tool("markets_create_market"), 1);
        assert_eq!(o.count_by_tool("daman_post_bond"), 0);
    }
}
