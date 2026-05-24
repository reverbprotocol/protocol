# Quickstart

From clone to "your first persona bee sending a transaction on Arc testnet" in roughly
fifteen minutes. Assumes you already have `forge`, `cargo`, `node`, and `gh` installed.

## 0. Prerequisites

```bash
forge --version    # >= 1.7
cargo --version    # >= 1.95
node --version     # >= 20
gh auth status     # authenticated
```

You'll also need an Arc-testnet wallet with a small USDC balance (the chain's native gas).
Faucet at [`faucet.circle.com`](https://faucet.circle.com).

## 1. Clone the substrate

```bash
git clone https://github.com/reverbprotocol/protocol
cd protocol
```

Install Foundry deps and run the test suite:

```bash
forge install foundry-rs/forge-std OpenZeppelin/openzeppelin-contracts OpenZeppelin/openzeppelin-contracts-upgradeable --no-git
forge test
```

You should see 53 tests pass. The 18 selector + event freeze tests, the 13 RefundProtocolFixed tests, the 20 substrate-primitive tests, and the 2 stateful fuzz invariants should all be green.

Build the Cargo workspace (forager + persona scaffolding):

```bash
cargo test --workspace
```

You should see 21 Rust tests pass across `reverb-arc-fs` and `persona-base`.

## 2. Inspect the live deployment

Live Arc-testnet addresses are committed at `.deployments/arc-testnet.json`. Read the
substrate's deployed configuration:

```bash
cat .deployments/arc-testnet.json | jq .contracts.RefundProtocolFixed
```

Verify on-chain ownership at the Timelock:

```bash
cast call 0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F \
  "owner()(address)" \
  --rpc-url https://rpc.testnet.arc.network
# expected: 0xa22510860289751C092e67B15b827020CE09DAbf
```

## 3. Build a consumer-product forager

A consumer product extends `reverb-arc-fs` by importing it as a Cargo dependency and adding
product-specific tools. Minimal extension:

```toml
# my-product-arc-fs/Cargo.toml
[package]
name = "my-product-arc-fs"
version = "0.1.0"
edition = "2021"

[dependencies]
reverb-arc-fs = { git = "https://github.com/reverbprotocol/protocol" }
async-trait = "0.1"
serde_json = "1"
```

```rust
// my-product-arc-fs/src/lib.rs
use async_trait::async_trait;
use reverb_arc_fs::manifest::Hello;
use reverb_arc_fs::tools::{Idempotency, Tool, ToolCall, ToolResult};
use serde_json::json;

pub fn manifest() -> Hello {
    Hello::base("my-product-arc-fs", "0.1.0")
        .with_wire("my-product/arc-fs")
        .extend(
            ["my-event".to_string()],
            ["my_product_do_thing".to_string()],
        )
}

pub struct DoThing;

#[async_trait]
impl Tool for DoThing {
    fn name(&self) -> &'static str { "my_product_do_thing" }
    fn idempotency(&self) -> Idempotency { Idempotency::NotIdempotent }
    async fn invoke(&self, call: ToolCall) -> ToolResult {
        // Wire the safety pipeline + concrete chain send here.
        ToolResult::ok(call.call_id, json!({"status": "scaffold"}))
    }
}
```

The forager-hive contract is documented at [Operating model](/OPERATING_MODEL#the-forager-hive-contract). Read it before shipping a forager to production.

## 4. Build a consumer-product persona

```toml
# my-product-personas/Cargo.toml
[dependencies]
persona-base = { git = "https://github.com/reverbprotocol/protocol" }
async-trait = "0.1"
serde_json = "1"
```

```rust
use async_trait::async_trait;
use persona_base::{Decision, Event, EventFilter, PersonaBee};

pub struct MyProductWatcher;

#[async_trait]
impl PersonaBee for MyProductWatcher {
    fn bee_name(&self) -> &str { "my-product-watcher-default" }

    fn subscribe_topics(&self) -> Vec<String> {
        vec!["my-product/events/observability".into()]
    }

    fn subscribe_chain_events(&self) -> Vec<EventFilter> {
        vec![]
    }

    fn persona_system_prompt(&self) -> Option<String> {
        Some(
            "You are a watcher for my-product. On every event you observe, decide whether to \
             emit a my_product_do_thing tool call. Be conservative."
                .into(),
        )
    }

    async fn on_event(&self, event: Event) -> Decision {
        Decision::Prompt {
            sid: format!("{}/some-suffix", self.bee_name()),
            system_prompt: self.persona_system_prompt().unwrap(),
            user_prompt: serde_json::to_string_pretty(&event).unwrap_or_default(),
        }
    }
}
```

The persona-bee contract is documented at [Operating model](/OPERATING_MODEL#the-persona-bee-contract). The role allowlist is enforced at the forager layer, not the persona layer.

## 5. Configure the forager

The forager's config at `~/.config/hum/{forager-name}/config.json`:

```json
{
  "rpc_url": "https://rpc.testnet.arc.network",
  "explorer_api": "https://testnet.arcscan.app/api/v2",
  "chain_id": 5042002,
  "allowed_contracts": [
    "0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F"
  ],
  "rate_limit": {
    "per_bee_tx_per_minute": 12,
    "per_tool_tx_per_minute": 60,
    "global_tx_per_minute": 200
  }
}
```

And the keyring at `~/.config/hum/{forager-name}/keyring.json` (chmod 0600):

```json
{
  "my-product-watcher-default": "0x...64-char-hex..."
}
```

The forager refuses to load a keyring with looser permissions than 0600 on unix platforms.

## 6. Wire to humd

The substrate publishes the forager + persona scaffolding; the actual humd attachment is the runtime's job. The Reverb Markets demo runtime lives in [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets); the Daman demo runtime lives in [`damanfi/copy-bond`](https://github.com/damanfi/copy-bond). Either is a reasonable starting point for a new consumer-product runtime.

The minimum runtime responsibilities:

1. Start a local humd (`humd --subnet your-product`)
2. Boot your forager process; emit its hello on humd-attach
3. Boot your persona bee process; have it subscribe to the right topics + chain events
4. Wire a worker bee (`claude-cli`, `vercel-ai`, etc.) that the persona prompts into
5. Observe the bloom transcript via the persona's logs

## 7. Verify

A persona observing an event, emitting a prompt, the worker emitting a tool call, the forager routing through the safety pipeline, the transaction landing on Arc, and the receipt cache holding the tx hash for downstream reads.

If all six steps work in sequence, you have an L5 deployment on the substrate.

## What's next

- Read [Operating model](/OPERATING_MODEL) for the full spec.
- Read [Autonomy spectrum](/AUTONOMY_SPECTRUM) to choose the autonomy level that fits your venture.
- See [Build your own forager](/guides/build-your-own-forager) for a deeper walkthrough.
- See [Build your own persona](/guides/build-your-own-persona) for a deeper walkthrough.
- Scenarios in `docs/scenarios/` document end-to-end usage patterns with the five-section shape inherited from hum.
