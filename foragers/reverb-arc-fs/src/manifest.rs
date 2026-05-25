//! Hello manifest emitted by `reverb-arc-fs` on attach.

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub chi: String,
    pub bee: String,
    /// Stable bee identity per the hum hives contract: `fbee_<hex>` for foragers,
    /// `wbee_<hex>` for workers, where hex is `sha256(ed25519 pubkey)`. Mandatory; humd
    /// deduplicates a bee across reconnects via this field and warns with
    /// `bee.hid.missing` or `bee.hid.invalid` otherwise.
    pub hid: String,
    pub version: String,
    #[serde(rename = "protoVersion")]
    pub proto_version: String,
    pub propensity: Propensity,
    pub chis: Vec<String>,
    pub tools: Vec<String>,
    pub source: String,
}

impl Hello {
    /// Builder for a base `reverb-arc-fs` hello. The hid defaults to empty; callers must
    /// supply a valid hid via [`Hello::with_hid`] before sending. `PersonaForagerBuilder`
    /// enforces this; if you construct a Hello by hand, you are responsible.
    pub fn base(bee: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            chi: "hello".into(),
            bee: bee.into(),
            hid: String::new(),
            version: version.into(),
            proto_version: PROTO_VERSION.into(),
            propensity: Propensity {
                statefulness: "stateful".into(),
                richness: "rich".into(),
                wire: "reverb/arc-fs".into(),
            },
            chis: BASE_CHIS.iter().map(|s| s.to_string()).collect(),
            tools: BASE_TOOLS.iter().map(|s| s.to_string()).collect(),
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

    /// Extend a base hello with product-specific chis and tools. The result preserves base
    /// tools and chis and appends the additions.
    pub fn extend(
        mut self,
        added_chis: impl IntoIterator<Item = String>,
        added_tools: impl IntoIterator<Item = String>,
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

    #[test]
    fn base_hello_includes_all_base_tools_and_chis() {
        let hello = Hello::base("reverb-arc-fs", "0.1.0");
        assert_eq!(hello.bee, "reverb-arc-fs");
        assert_eq!(hello.proto_version, PROTO_VERSION);
        assert_eq!(hello.hid, "", "base hello has empty hid; caller must set via with_hid");
        for tool in BASE_TOOLS {
            assert!(hello.tools.contains(&tool.to_string()), "missing {tool}");
        }
        for chi in BASE_CHIS {
            assert!(hello.chis.contains(&chi.to_string()), "missing {chi}");
        }
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
        let hello = Hello::base("daman-arc-fs", "0.1.0")
            .with_wire("daman/arc-fs")
            .with_source("https://github.com/damanfi/copy-bond/tree/main/foragers/daman-arc-fs")
            .extend(
                ["bond-posted".to_string(), "slash-claim".to_string()],
                ["daman_register_leader".to_string(), "daman_post_bond".to_string()],
            );
        assert_eq!(hello.propensity.wire, "daman/arc-fs");
        assert!(hello.chis.contains(&"hello".to_string()));
        assert!(hello.chis.contains(&"bond-posted".to_string()));
        assert!(hello.tools.contains(&"arc_send_tx".to_string()));
        assert!(hello.tools.contains(&"daman_post_bond".to_string()));
    }

    #[test]
    fn hello_serializes_camel_case_and_includes_hid() {
        let hello = Hello::base("reverb-arc-fs", "0.1.0")
            .with_hid("fbee_deadbeef");
        let json = serde_json::to_string(&hello).unwrap();
        // protoVersion uses camelCase per the operating-model spec
        assert!(json.contains("\"protoVersion\":\"0.7.0\""));
        assert!(json.contains("\"chi\":\"hello\""));
        assert!(json.contains("\"hid\":\"fbee_deadbeef\""));
    }
}
