//! Hello manifest emitted by `reverb-arc-fs` on attach.
//!
//! Shape aligns with the hum hives contract (see
//! <https://adiled.github.io/hum/hives/>): `bee` is a role array
//! (`["forager"]` or `["worker"]`), `hive` carries the bee-instance name,
//! `tools` is an array of `{name, description, inputSchema}` objects so
//! humd's hello parser registers them and the prompt-forward path injects
//! them into every chi:"prompt" `foragerTools` field for the worker.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The substrate's protocol version. Bumps when the forager-hive contract changes.
pub const PROTO_VERSION: &str = "0.7.0";

/// Default base tool surface for `reverb-arc-fs`. Consumer-product extensions add their own
/// tools to this list.
pub const BASE_TOOLS: &[&str] = &[
    "arc_read_balance",
    "arc_read_event",
    "arc_read_state",
    "arc_subscribe_events",
    "arc_sign_typed_data",
    "arc_send_tx",
];

/// Default base chi vocabulary the forager handles. Extensions may add product-specific chis.
pub const BASE_CHIS: &[&str] = &[
    "hello",
    "echo",
    "log",
    "perf-mark",
    "tool-call",
    "tool-result",
    "tool-meta",
    "gossip-publish",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Propensity {
    pub statefulness: String,
    pub richness: String,
    pub wire: String,
}

/// The hum-canonical hello shape. `bee` is the ROLE array humd routes by
/// (`["forager"]` / `["worker"]` / both); `hive` is the bee-instance name
/// that humd uses as the catalogue key. `tools` is an array of object defs
/// `{name, description, inputSchema}` so humd's parser can register them
/// and the prompt-forward path can inject them into chi:"prompt"
/// `foragerTools` for the worker cell to see.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub chi: String,
    /// Role array, e.g. `["forager"]`. humd routes by membership: a tool-call
    /// for a given tool name lands at whichever bee in the forager role
    /// declares that tool in its `tools` array.
    pub bee: Vec<String>,
    /// Bee-instance name. humd uses this as the catalogue key (the hum hive
    /// kind). Each persona-forager binary instance carries a distinct hive
    /// name (e.g. `daman-persona-daman-leader-alpha`).
    pub hive: String,
    /// Stable bee identity per the hum hives contract: `fbee_<hex>` for foragers,
    /// `wbee_<hex>` for workers, where hex is `sha256(ed25519 pubkey)`. Mandatory;
    /// humd deduplicates a bee across reconnects via this field and warns with
    /// `bee.hid.missing` or `bee.hid.invalid` otherwise.
    pub hid: String,
    pub version: String,
    #[serde(rename = "protoVersion")]
    pub proto_version: String,
    pub propensity: Propensity,
    pub chis: Vec<String>,
    /// Capability tags humd uses to auto-disallow conflicting built-in tools
    /// at the worker (see `mcp::capability::capability_tools`). Forager-as-
    /// session bees should advertise `["session"]`.
    pub provides: Vec<String>,
    /// Tool definitions in the hum-canonical shape:
    /// `[{ "name": "...", "description": "...", "inputSchema": { ... } }, ...]`.
    /// humd's hello parser reads each entry as `{name, description, inputSchema}`
    /// and drops entries lacking a `name` string.
    pub tools: Vec<Value>,
    pub source: String,
}

impl Hello {
    /// Builder for a base `reverb-arc-fs` hello. Defaults `bee` to
    /// `["forager"]` and `hive` to the supplied instance name so humd
    /// routes the bee correctly without further configuration. The hid
    /// defaults to empty; callers must supply a valid hid via
    /// [`Hello::with_hid`] before sending. `PersonaForagerBuilder` enforces
    /// this; if you construct a Hello by hand, you are responsible.
    pub fn base(hive_name: impl Into<String>, version: impl Into<String>) -> Self {
        let hive_name = hive_name.into();
        Self {
            chi: "hello".into(),
            bee: vec!["forager".into()],
            hive: hive_name,
            hid: String::new(),
            version: version.into(),
            proto_version: PROTO_VERSION.into(),
            propensity: Propensity {
                statefulness: "stateful".into(),
                richness: "rich".into(),
                wire: "reverb/arc-fs".into(),
            },
            chis: BASE_CHIS.iter().map(|s| s.to_string()).collect(),
            provides: vec!["session".into()],
            tools: BASE_TOOLS
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s,
                        "description": "",
                        "inputSchema": { "type": "object", "properties": {} }
                    })
                })
                .collect(),
            source: "https://github.com/reverbprotocol/protocol/tree/main/foragers/reverb-arc-fs"
                .into(),
        }
    }

    /// Set the stable hid on the hello. Required before send. Format must be
    /// `fbee_<64hex>` or `wbee_<64hex>` per the hum hives contract.
    pub fn with_hid(mut self, hid: impl Into<String>) -> Self {
        self.hid = hid.into();
        self
    }

    /// Set the role array. Default is `["forager"]`. A bee that produces compute
    /// in addition to translating wires can advertise `["forager", "worker"]`.
    pub fn with_bee_role(mut self, roles: impl IntoIterator<Item = String>) -> Self {
        self.bee = roles.into_iter().collect();
        self
    }

    /// Set the hive (bee-instance) name humd uses as the catalogue key.
    pub fn with_hive(mut self, hive: impl Into<String>) -> Self {
        self.hive = hive.into();
        self
    }

    /// Replace the provides list. humd auto-disallows built-in tool names
    /// that overlap any declared capability.
    pub fn with_provides(mut self, provides: impl IntoIterator<Item = String>) -> Self {
        self.provides = provides.into_iter().collect();
        self
    }

    /// Replace the tools array with the persona's own tool defs. Each value
    /// must serialize as `{name, description, inputSchema}`; build them via
    /// `Tool::to_tool_def()` or `ToolRegistry::tool_defs()`.
    pub fn with_tool_defs(mut self, tools: impl IntoIterator<Item = Value>) -> Self {
        self.tools = tools.into_iter().collect();
        self
    }

    /// Extend a base hello with product-specific chis and tool defs.
    /// Preserves base chis + tools and appends the additions.
    pub fn extend(
        mut self,
        added_chis: impl IntoIterator<Item = String>,
        added_tools: impl IntoIterator<Item = Value>,
    ) -> Self {
        self.chis.extend(added_chis);
        self.tools.extend(added_tools);
        self
    }

    /// Set the wire string on the propensity. Extensions override the base wire to namespace
    /// themselves (e.g. `reverb-markets/arc-fs`, `daman/arc-fs`).
    pub fn with_wire(mut self, wire: impl Into<String>) -> Self {
        self.propensity.wire = wire.into();
        self
    }

    /// Set the source URL on the manifest. Extensions point at their own repo path.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn base_hello_includes_all_base_tools_and_chis() {
        let hello = Hello::base("reverb-arc-fs", "0.1.0");
        assert_eq!(hello.bee, vec!["forager".to_string()]);
        assert_eq!(hello.hive, "reverb-arc-fs");
        assert_eq!(hello.proto_version, PROTO_VERSION);
        assert_eq!(hello.hid, "", "base hello has empty hid; caller must set via with_hid");
        let names: Vec<&str> = hello
            .tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        for tool in BASE_TOOLS {
            assert!(names.contains(tool), "missing {tool}");
        }
        for chi in BASE_CHIS {
            assert!(hello.chis.contains(&chi.to_string()), "missing {chi}");
        }
        assert_eq!(hello.provides, vec!["session".to_string()]);
    }

    #[test]
    fn with_hid_populates_field() {
        let hello = Hello::base("reverb-arc-fs", "0.1.0")
            .with_hid("fbee_".to_string() + &"0".repeat(64));
        assert!(hello.hid.starts_with("fbee_"));
        assert_eq!(hello.hid.len(), 69);
    }

    #[test]
    fn extend_appends_product_specific_surface() {
        let added_tool = json!({
            "name": "daman_register_leader",
            "description": "register a copy-trading leader",
            "inputSchema": { "type": "object", "properties": {} }
        });
        let hello = Hello::base("daman-arc-fs", "0.1.0")
            .with_wire("daman/arc-fs")
            .with_source("https://github.com/damanfi/copy-bond/tree/main/foragers/daman-arc-fs")
            .extend(
                ["bond-posted".to_string(), "slash-claim".to_string()],
                [added_tool.clone()],
            );
        assert_eq!(hello.propensity.wire, "daman/arc-fs");
        assert!(hello.chis.contains(&"hello".to_string()));
        assert!(hello.chis.contains(&"bond-posted".to_string()));
        let names: Vec<&str> = hello
            .tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"arc_send_tx"));
        assert!(names.contains(&"daman_register_leader"));
    }

    #[test]
    fn hello_serializes_camel_case_and_includes_hid_and_hive() {
        let hello = Hello::base("daman-leader-alpha", "0.1.0")
            .with_hid("fbee_deadbeef");
        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.contains("\"protoVersion\":\"0.7.0\""));
        assert!(json.contains("\"chi\":\"hello\""));
        assert!(json.contains("\"hid\":\"fbee_deadbeef\""));
        assert!(json.contains("\"hive\":\"daman-leader-alpha\""));
        assert!(json.contains("\"bee\":[\"forager\"]"));
        assert!(json.contains("\"provides\":[\"session\"]"));
    }

    #[test]
    fn with_bee_role_replaces_default() {
        let h = Hello::base("h", "0.1.0").with_bee_role(["worker".to_string()]);
        assert_eq!(h.bee, vec!["worker".to_string()]);
    }

    #[test]
    fn with_provides_replaces_default() {
        let h = Hello::base("h", "0.1.0").with_provides(["fs".to_string()]);
        assert_eq!(h.provides, vec!["fs".to_string()]);
    }

    #[test]
    fn with_tool_defs_replaces_tools_array() {
        let h = Hello::base("h", "0.1.0").with_tool_defs([json!({
            "name": "mytool",
            "description": "d",
            "inputSchema": { "type": "object", "properties": {} }
        })]);
        assert_eq!(h.tools.len(), 1);
        assert_eq!(h.tools[0].get("name").and_then(|n| n.as_str()), Some("mytool"));
    }
}
