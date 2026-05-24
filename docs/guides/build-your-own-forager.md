# Build your own forager

A walkthrough for shipping a consumer-product forager that extends `reverb-arc-fs`. The
target outcome is a Cargo crate that emits its own hello manifest, declares its product-specific tools, and reuses the substrate's wallet keyring + safety pipeline.

## Decide your tool surface first

Before any code, list the product-specific tools you need. Each tool answers a question:
"what's the smallest action a worker bee would take through this surface?"

For Daman (slash-bonded copy-trading), the answer was:

- `daman_register_leader` (admit a new leader against the consumer-product contract)
- `daman_post_bond` (the leader posts their bond)
- `daman_subscribe` (a follower subscribes to a leader)
- `daman_file_slash_claim` (a watchdog files against a degrading leader)
- `daman_rule_slash_claim` (arbiter rules)
- `daman_unsubscribe` (follower exits)
- `daman_read_leader_state` (anyone reads current state)

For Reverb Markets the answer was a different seven tools ([see the markets repo](https://github.com/reverbprotocol/markets/tree/main/foragers/reverb-markets-arc-fs)).

For your product, write the list. Each tool has:

- A namespaced snake_case name (`my_product_*`)
- A typed input schema
- A typed output schema
- An idempotency declaration (`Idempotent` for reads, `NotIdempotent` for writes)

## Crate scaffold

```toml
# my-product-arc-fs/Cargo.toml
[package]
name = "my-product-arc-fs"
version = "0.1.0"
edition = "2021"
description = "My Product forager hive: extends reverb-arc-fs with product-specific tools."

[dependencies]
reverb-arc-fs = { git = "https://github.com/reverbprotocol/protocol" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Hello manifest

The substrate's `Hello::base` builds the base manifest; `extend` adds your product surface:

```rust
// my-product-arc-fs/src/hello.rs
use reverb_arc_fs::manifest::Hello;

pub const MY_PRODUCT_CHIS: &[&str] = &[
    "my-product-action-completed",
    "my-product-event-observed",
];

pub const MY_PRODUCT_TOOLS: &[&str] = &[
    "my_product_do_action",
    "my_product_read_state",
];

pub fn manifest() -> Hello {
    Hello::base("my-product-arc-fs", "0.1.0")
        .with_wire("my-product/arc-fs")
        .with_source("https://github.com/your-org/my-product")
        .extend(
            MY_PRODUCT_CHIS.iter().map(|s| s.to_string()),
            MY_PRODUCT_TOOLS.iter().map(|s| s.to_string()),
        )
}
```

The hello manifest's `tools` field is the routing table humd uses. Every tool you declare here must have a corresponding `Tool` impl in your crate (humd will route `chi:"tool-call"` to your forager by toolName; if no impl handles the call, the forager errors with `ForagerError::UnknownTool`).

## Tool implementations

Each tool implements `Tool` from `reverb-arc-fs`:

```rust
// my-product-arc-fs/src/tools.rs
use async_trait::async_trait;
use reverb_arc_fs::tools::{Idempotency, Tool, ToolCall, ToolResult};
use serde_json::json;

pub struct DoAction;

#[async_trait]
impl Tool for DoAction {
    fn name(&self) -> &'static str { "my_product_do_action" }
    fn idempotency(&self) -> Idempotency { Idempotency::NotIdempotent }
    async fn invoke(&self, call: ToolCall) -> ToolResult {
        // 1. Pull product-specific args from call.args
        // 2. Encode the consumer-product calldata
        // 3. Defer to reverb-arc-fs::safety pipeline (auth + abi + sim + rate + send)
        // 4. Return ToolResult with tx hash on success or structured error on failure
        ToolResult::ok(call.call_id, json!({"status": "scaffold"}))
    }
}
```

## Safety pipeline integration

Each write tool **must** run through the six-stage pipeline. The substrate's `reverb-arc-fs::safety` module provides:

- `check_auth(&call)` — stage 1
- `check_abi(&args, schema_check_fn)` — stage 2
- `check_simulation(&gate, to, data)` — stage 3 (takes a `SimulationGate` trait impl)
- `RateLimiter::check_and_record(bee, tool)` — stage 4
- `send(&sender, &keyring, bee, to, data)` — stages 5+6 (takes a `Sender` trait impl)

The `SimulationGate` and `Sender` traits are runtime-injected. Your crate doesn't depend on alloy/ethers directly; the runtime that spawns the forager process wires the concrete chain transport.

Pattern inside your tool's `invoke`:

```rust
async fn invoke(&self, call: ToolCall) -> ToolResult {
    // Stage 1: auth
    if let Err(e) = reverb_arc_fs::safety::check_auth(&call) {
        return ToolResult::fail(call.call_id, e);
    }
    // Stage 2: ABI
    if let Err(e) = reverb_arc_fs::safety::check_abi(&call.args, |args| {
        // Validate args against your contract's input shape
        if args.get("marketId").is_none() { return Err("missing marketId".into()); }
        Ok(())
    }) {
        return ToolResult::fail(call.call_id, e);
    }
    // Stages 3-6: runtime injects gate + sender + rate limiter
    // ... (see your crate's runtime/wire module for the concrete handler)
    ToolResult::ok(call.call_id, json!({"status": "scaffold"}))
}
```

For production wiring, see how `reverb-markets-arc-fs` composes the pipeline against the
deployed Operator proxy.

## Scoping config

Your forager loads its config from `~/.config/hum/my-product-arc-fs/config.json`:

```json
{
  "rpc_url": "https://rpc.testnet.arc.network",
  "explorer_api": "https://testnet.arcscan.app/api/v2",
  "chain_id": 5042002,
  "allowed_contracts": [
    "0x...your-product-contract-1...",
    "0x...your-product-contract-2..."
  ],
  "rate_limit": {
    "per_bee_tx_per_minute": 12,
    "per_tool_tx_per_minute": 60,
    "global_tx_per_minute": 200
  }
}
```

`allowed_contracts` is your forager's hard write boundary. Any tool call that targets a contract not in this list is rejected by the safety pipeline regardless of bee authorization. This is the "fs.roots" analog from `humfs` and is the substrate's protection against a misbehaving consumer-side persona that issues tool calls outside its product surface.

## Keyring

The keyring at `~/.config/hum/my-product-arc-fs/keyring.json`:

```json
{
  "my-product-watcher-default": "0xabc...64-char-hex...",
  "my-product-arbiter-strict": "0xdef...64-char-hex..."
}
```

File permissions must be 0600 on unix; the substrate refuses to load otherwise. Each persona's keyring entry maps `bee_name → EOA private key`. The forager's auth check verifies the incoming tool call's `from` matches `as_bee` before looking up the key.

## Testing

Unit tests for the hello manifest:

```rust
#[test]
fn manifest_includes_both_base_and_product_surface() {
    let h = manifest();
    assert!(h.tools.contains(&"arc_send_tx".to_string()));        // base
    assert!(h.tools.contains(&"my_product_do_action".to_string())); // product
}
```

Tests for each tool's idempotency declaration + scaffold behavior:

```rust
#[test]
fn write_tools_are_not_idempotent() {
    assert_eq!(DoAction.idempotency(), Idempotency::NotIdempotent);
}
```

Integration tests that spin a local anvil + your forager + a fake persona + a fake worker bee:

```rust
#[tokio::test]
async fn end_to_end_tool_call_routes_through_safety_pipeline() {
    // 1. Deploy your contract to anvil
    // 2. Boot the forager with a test keyring + test config
    // 3. Construct a ToolCall the way a persona would
    // 4. Assert the resulting tx hash + receipt match expectation
}
```

Reference implementations: `reverbprotocol/markets/foragers/reverb-markets-arc-fs` for the
extension-with-7-tools pattern; `damanfi/copy-bond/foragers/daman-arc-fs` for the
extension-with-7-tools-and-bust-detection pattern.

## Frame coherence

The forager name, the tool names, the chi vocabulary additions, the config file names — all
follow universal language. No religious or regional signal. The forager is infrastructure;
infrastructure does not carry positioning. The substrate's frame-coherence audit pattern
documented at [Security posture](/security-posture) applies to every consumer-side surface.

## What's next

- See [Build your own persona](/guides/build-your-own-persona) for the asker side.
- See [Operating model](/OPERATING_MODEL#cooperative-equilibrium-examples) for how your
  forager composes with other consumer products' foragers on a shared humd ensemble.
- See [UUPS upgrade lifecycle](/scenarios/uups-upgrade) if your consumer-product contracts
  are also UUPS-upgradeable (recommended).
