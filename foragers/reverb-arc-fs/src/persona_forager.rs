//! `PersonaForager` + `PersonaForagerBuilder`. The forager-as-library composition root.
//!
//! Each persona binary builds exactly one `PersonaForager` at startup: one bee_name, one EOA
//! private key, one namespaced tool surface, one humd connection. The forager IS the bee
//! whose key it holds. There is no shared signer, no `as_bee` per-call routing, no
//! multi-tenant keyring; the process boundary is the identity boundary.
//!
//! Consumer-product crates (`reverb-markets-arc-fs`, `daman-arc-fs`, future) expose
//! `<product>_tools(namespace: &str) -> Vec<Tool>` factories. Persona binaries pass them
//! into `PersonaForagerBuilder::with_tools` alongside any base `arc_*` tools they want
//! enabled.

use crate::config::{Config, RateLimit};
use crate::errors::ForagerError;
use crate::identity::BeeIdentity;
use crate::keyring::PrivateKey;
use crate::manifest::Hello;
use crate::safety::RateLimiter;
use crate::tools::{Tool, ToolRegistry};

/// One bee, one process, one set of tools, one EOA. The forager dispatches incoming
/// `chi:"tool-call"` tones to the right tool by name, runs each through the safety pipeline,
/// and emits `chi:"tool-result"`.
///
/// The thrum/humd wiring is injected by the runtime that spawns the persona binary; this
/// struct holds only the substrate-shipped pieces (registry, identity, key, allowlist,
/// rate limiter, sender, simulation gate).
pub struct PersonaForager {
    pub bee_name: String,
    pub identity: BeeIdentity,
    pub hello: Hello,
    pub tools: ToolRegistry,
    pub private_key: PrivateKey,
    pub allowed_contracts: Vec<String>,
    pub rate_limiter: RateLimiter,
}

impl std::fmt::Debug for PersonaForager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersonaForager")
            .field("bee_name", &self.bee_name)
            .field("hid", &self.identity.hid_string())
            .field("tool_count", &self.tools.len())
            .field("allowed_contracts", &self.allowed_contracts)
            .finish()
    }
}

impl PersonaForager {
    pub fn builder() -> PersonaForagerBuilder {
        PersonaForagerBuilder::default()
    }
}

/// Composable builder. Required: `bee_name`, `identity`, `private_key`. Optional:
/// `with_tools`, `allowed_contracts`, `rate_limit`, `provides`, `bee_role`, `hive`.
#[derive(Default)]
pub struct PersonaForagerBuilder {
    bee_name: Option<String>,
    namespace: Option<String>,
    identity: Option<BeeIdentity>,
    private_key: Option<PrivateKey>,
    tools: Vec<Tool>,
    allowed_contracts: Vec<String>,
    rate_limit: Option<RateLimit>,
    source_url: Option<String>,
    wire: Option<String>,
    provides: Option<Vec<String>>,
    bee_role: Option<Vec<String>>,
    hive: Option<String>,
}

impl PersonaForagerBuilder {
    pub fn bee_name(mut self, n: impl Into<String>) -> Self {
        self.bee_name = Some(n.into());
        self
    }

    /// Namespace prefix for tool names. Convention: short alias of the bee_name.
    pub fn namespace(mut self, n: impl Into<String>) -> Self {
        self.namespace = Some(n.into());
        self
    }

    /// Stable ed25519 identity. Required. Derive via
    /// `BeeIdentity::load_or_mint(<bee_name>)`.
    pub fn identity(mut self, id: BeeIdentity) -> Self {
        self.identity = Some(id);
        self
    }

    pub fn private_key(mut self, key: PrivateKey) -> Self {
        self.private_key = Some(key);
        self
    }

    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn allowed_contracts(mut self, contracts: impl IntoIterator<Item = String>) -> Self {
        self.allowed_contracts.extend(contracts);
        self
    }

    pub fn rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn wire(mut self, wire: impl Into<String>) -> Self {
        self.wire = Some(wire.into());
        self
    }

    pub fn source(mut self, url: impl Into<String>) -> Self {
        self.source_url = Some(url.into());
        self
    }

    /// Override the default capability tags. Default is `["session"]`.
    pub fn provides(mut self, caps: impl IntoIterator<Item = String>) -> Self {
        self.provides = Some(caps.into_iter().collect());
        self
    }

    /// Override the default role array. Default is `["forager"]`.
    pub fn bee_role(mut self, roles: impl IntoIterator<Item = String>) -> Self {
        self.bee_role = Some(roles.into_iter().collect());
        self
    }

    /// Override the hive (catalogue key) name. Default matches `bee_name`.
    pub fn hive(mut self, hive: impl Into<String>) -> Self {
        self.hive = Some(hive.into());
        self
    }

    /// Build the forager. Returns `BuilderIncomplete` if any required field is missing.
    pub fn build(self) -> Result<PersonaForager, ForagerError> {
        let bee_name = self
            .bee_name
            .ok_or_else(|| ForagerError::BuilderIncomplete("bee_name is required".into()))?;
        let identity = self
            .identity
            .ok_or_else(|| ForagerError::BuilderIncomplete("identity is required".into()))?;
        let private_key = self
            .private_key
            .ok_or_else(|| ForagerError::BuilderIncomplete("private_key is required".into()))?;

        let hive_name = self.hive.unwrap_or_else(|| bee_name.clone());
        let mut hello = Hello::base(hive_name, "0.1.0").with_hid(identity.hid_string());
        if let Some(wire) = self.wire {
            hello = hello.with_wire(wire);
        }
        if let Some(src) = self.source_url {
            hello = hello.with_source(src);
        }
        if let Some(roles) = self.bee_role {
            hello = hello.with_bee_role(roles);
        }
        if let Some(caps) = self.provides {
            hello = hello.with_provides(caps);
        }

        // Replace the base tool list with the persona's namespaced tool defs.
        // Each Tool serializes as {name, description, inputSchema} via its
        // builder-attached description + schema; humd's hello parser reads
        // those objects and humd injects them into every chi:"prompt" the
        // worker sees, so claude reliably discovers the tool surface.
        let tools = ToolRegistry::new().with_tools(self.tools);
        hello = hello.with_tool_defs(tools.tool_defs());

        let rate_limiter = RateLimiter::new(self.rate_limit.unwrap_or_default());

        Ok(PersonaForager {
            bee_name,
            identity,
            hello,
            tools,
            private_key,
            allowed_contracts: self.allowed_contracts,
            rate_limiter,
        })
    }

    /// Convenience: build from a loaded `Config` (allowed_contracts + rate_limit).
    pub fn with_config(mut self, config: Config) -> Self {
        self.allowed_contracts.extend(config.allowed_contracts);
        self.rate_limit = Some(config.rate_limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::BeeRole;
    use crate::tools::{Idempotency, Tool, ToolResult};
    use serde_json::json;

    fn stub_tool(name: &str) -> Tool {
        Tool::new(name.to_string(), Idempotency::Idempotent, |call| async move {
            ToolResult::ok(call.call_id, json!({"stub": true}))
        })
    }

    fn pk() -> PrivateKey {
        PrivateKey::new("0x".to_string() + &"a".repeat(64)).unwrap()
    }

    fn id(kind: &str) -> BeeIdentity {
        BeeIdentity::from_seed(kind, BeeRole::Forager, [0x11u8; 32])
    }

    #[test]
    fn builder_requires_bee_name() {
        let err = PersonaForager::builder()
            .identity(id("x"))
            .private_key(pk())
            .build()
            .unwrap_err();
        assert!(matches!(err, ForagerError::BuilderIncomplete(_)));
    }

    #[test]
    fn builder_requires_identity() {
        let err = PersonaForager::builder()
            .bee_name("markets-auto-create")
            .private_key(pk())
            .build()
            .unwrap_err();
        assert!(matches!(err, ForagerError::BuilderIncomplete(_)));
    }

    #[test]
    fn builder_requires_private_key() {
        let err = PersonaForager::builder()
            .bee_name("markets-auto-create")
            .identity(id("markets-auto-create"))
            .build()
            .unwrap_err();
        assert!(matches!(err, ForagerError::BuilderIncomplete(_)));
    }

    #[test]
    fn builder_composes_minimal_forager_with_valid_hid() {
        let f = PersonaForager::builder()
            .bee_name("markets-auto-create")
            .identity(id("markets-auto-create"))
            .private_key(pk())
            .build()
            .unwrap();
        assert_eq!(f.bee_name, "markets-auto-create");
        assert_eq!(f.tools.len(), 0);
        assert!(f.hello.hid.starts_with("fbee_"), "got {}", f.hello.hid);
        assert_eq!(f.hello.hid.len(), 69);
    }

    #[test]
    fn builder_includes_namespaced_tools() {
        let f = PersonaForager::builder()
            .bee_name("markets-auto-create")
            .identity(id("markets-auto-create"))
            .private_key(pk())
            .with_tools([
                stub_tool("mkac_create_market"),
                stub_tool("mkac_read_market_state"),
            ])
            .build()
            .unwrap();
        assert_eq!(f.tools.len(), 2);
        assert!(f.tools.lookup("mkac_create_market").is_some());
        let names: Vec<&str> = f
            .hello
            .tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"mkac_create_market"));
    }

    #[test]
    fn hello_defaults_to_forager_role_and_uses_bee_name_as_hive() {
        let f = PersonaForager::builder()
            .bee_name("daman-leader-alpha")
            .identity(id("daman-leader-alpha"))
            .private_key(pk())
            .build()
            .unwrap();
        assert_eq!(f.hello.bee, vec!["forager".to_string()]);
        assert_eq!(f.hello.hive, "daman-leader-alpha");
        assert_eq!(f.hello.provides, vec!["session".to_string()]);
    }

    #[test]
    fn hive_override_replaces_default() {
        let f = PersonaForager::builder()
            .bee_name("daman-leader-alpha")
            .hive("daman-persona-shared")
            .identity(id("daman-leader-alpha"))
            .private_key(pk())
            .build()
            .unwrap();
        assert_eq!(f.hello.hive, "daman-persona-shared");
    }

    #[test]
    fn builder_respects_allowed_contracts() {
        let f = PersonaForager::builder()
            .bee_name("markets-arbiter")
            .identity(id("markets-arbiter"))
            .private_key(pk())
            .allowed_contracts(["0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F".to_string()])
            .build()
            .unwrap();
        assert_eq!(f.allowed_contracts.len(), 1);
    }
}
