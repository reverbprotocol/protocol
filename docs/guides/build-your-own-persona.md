# Build your own persona

A walkthrough for shipping a persona bee that imports `persona-base`. The target outcome is
a Cargo crate exposing a `PersonaBee` implementation with subscriptions, a role-overlay
system prompt, and an `on_event` handler that returns `Decision::Prompt` or `Decision::Skip`.

## Decide your role first

A persona has one role. Multiple roles mean multiple personas (and multiple sids in the worker bee).

For each role, answer:

- **What event triggers a decision?** Gossip topic, chain event, or both.
- **What system prompt scopes the role?** Keep it tight. The persona's job is to be honest
  about the role; the worker's job is to decide what to do.
- **What's the variant axis?** Different prompt overlays for the same role (e.g.
  `aggressive` vs `conservative` watchdog variants).

Naming convention: `{product}-{role}-{variant}`. Examples: `markets-arbiter-conservative`,
`daman-watchdog-aggressive`, `my-product-watcher-fast`.

## Crate scaffold

```toml
# my-product-personas/Cargo.toml
[package]
name = "my-product-personas"
version = "0.1.0"
edition = "2021"
description = "Persona bees for My Product."

[dependencies]
persona-base = { git = "https://github.com/reverbprotocol/protocol" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## A minimal persona

```rust
use async_trait::async_trait;
use persona_base::{Decision, Event, EventFilter, PersonaBee};

pub const WATCHER_SYSTEM_PROMPT: &str = "\
You are a watcher for My Product. On every event you observe, decide whether the event \
warrants an on-chain action (e.g. file a slash claim, post a bond, ratify a result). If \
yes, emit a tool call from the my_product_* surface. If no, log and skip. Be conservative: \
prefer skip over action when the evidence is ambiguous.\
";

pub struct MyProductWatcher {
    pub bee_name: String,
}

impl MyProductWatcher {
    pub fn new(variant: impl Into<String>) -> Self {
        Self {
            bee_name: format!("my-product-watcher-{}", variant.into()),
        }
    }
}

#[async_trait]
impl PersonaBee for MyProductWatcher {
    fn bee_name(&self) -> &str {
        &self.bee_name
    }

    fn subscribe_topics(&self) -> Vec<String> {
        vec!["my-product/events/observability".into()]
    }

    fn subscribe_chain_events(&self) -> Vec<EventFilter> {
        vec![EventFilter {
            contract: "0x...your-contract-proxy...".into(),
            event_signature: "MyEvent(uint256,address,uint256)".into(),
            from_block: None,
        }]
    }

    fn persona_system_prompt(&self) -> Option<String> {
        Some(WATCHER_SYSTEM_PROMPT.into())
    }

    async fn on_event(&self, event: Event) -> Decision {
        let user_prompt = serde_json::to_string_pretty(&event).unwrap_or_default();
        Decision::Prompt {
            sid: format!("{}/{}", self.bee_name, sid_suffix()),
            system_prompt: WATCHER_SYSTEM_PROMPT.into(),
            user_prompt,
        }
    }
}

fn sid_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}
```

That's it. The persona is now ready to be loaded into a runtime.

## Subscription patterns

**Single topic, single chain event** — most personas. Simple.

**Multiple topics across products** — cross-product subscriptions. Same persona ruling on
multiple products. See [cross-product arbiter scenario](/scenarios/cross-product-arbiter).

```rust
fn subscribe_topics(&self) -> Vec<String> {
    vec![
        "my-product/events/observability".into(),
        "other-product/disputes/observability".into(),
    ]
}
```

**Multiple chain events on the same contract** — common for personas that watch a contract
for multiple state transitions.

```rust
fn subscribe_chain_events(&self) -> Vec<EventFilter> {
    vec![
        EventFilter { contract: "0x...".into(), event_signature: "MarketCreated(uint256,address,uint256,bytes32)".into(), from_block: None },
        EventFilter { contract: "0x...".into(), event_signature: "MarketResolved(uint256,uint8)".into(), from_block: None },
    ]
}
```

**Cross-contract subscriptions** — same persona watches multiple contracts. Useful for
arbiter personas that observe both a product contract and the shared
`RefundProtocolFixed @ 0xc8bF99c5...`.

## Prompt-overlay discipline

The role-overlay system prompt is the entire role description. Five guidelines:

1. **Tight scope.** "You are X. You observe Y. You decide Z." Three sentences is often
   enough. The Reverb Markets `markets-auto-create-macro` overlay is five sentences and
   that's already considered verbose.

2. **No hardcoded decisions in the persona.** The persona does not decide; it assembles
   state and prompts. If you find yourself writing `if/then` logic in the persona, the
   logic belongs in the contract layer or the safety pipeline, not the persona.

3. **No tool-call examples in the prompt.** The worker bee knows the tool surface from the
   forager's hello manifest. Telling the worker the exact tool to call is L1-shaped
   automation, not L5-shaped autonomy.

4. **Conservative bias for ambiguous cases.** When the evidence is unclear, the persona
   should prefer to skip rather than to prompt. False positives waste the worker's
   resources; false negatives are recoverable (the next event triggers a fresh sid).

5. **Variants over decision trees.** Different prompt overlays for `aggressive` vs
   `conservative` variants. Run multiple variants in parallel; the mesh aggregates the
   diversity. Hardcoding a "mode" switch in one persona's prompt is L1; running N personas
   with N variants is L5.

## Decision::Skip vs Decision::Prompt

- `Decision::Prompt` triggers a bloom: the AskerLoop emits `chi:"prompt"` to the worker.
- `Decision::Skip` no-ops: the AskerLoop logs the reason and returns; no worker resource
  is spent.

When in doubt, return `Decision::Skip`. The persona is cheap; the worker is expensive.
Skip-by-default is the right discipline.

## Bust-detection state machine

For personas representing economic actors (a follower bee whose principal may drain), the
substrate ships a `BustDetector` you can use directly:

```rust
use persona_base::BustDetector;

let mut detector = BustDetector::new(/* warning */ 100, /* bust */ 20);
let state = detector.observe(75);  // returns BustState::Warning
let state = detector.observe(15);  // BustState::Busted
detector.mark_loan_requested();    // BustState::LoanRequested
detector.mark_loan_granted();      // BustState::LoanGranted
detector.mark_recovered();         // BustState::Recovered
```

Useful in personas that participate in peer-credit gossip patterns. See
[Operating model](/OPERATING_MODEL#cooperative-equilibrium-examples) for the broader pattern.

## Tool-call observation

For dashboards and debugging, the substrate ships a `ToolCallObserver`:

```rust
use persona_base::ToolCallObserver;
use persona_base::observer::ObservedToolCall;

let mut observer = ToolCallObserver::new();
// runtime feeds observed tool-calls into observer.record(...);
println!("total tool calls: {}", observer.count());
println!("markets_resolve_market: {}", observer.count_by_tool("markets_resolve_market"));
```

The observer is out-of-band: it doesn't gate any persona action; it just surfaces what the
worker emitted for human review.

## Testing

The substrate's `persona-base` includes tests for the trait shape; your tests assert your
specific subscriptions + decision behavior:

```rust
#[tokio::test]
async fn on_gossip_returns_prompt_with_persona_overlay() {
    let p = MyProductWatcher::new("fast");
    let d = p.on_event(Event::Gossip {
        topic: "my-product/events/observability".into(),
        body: serde_json::json!({"event": "X"}),
    }).await;
    match d {
        Decision::Prompt { system_prompt, sid, .. } => {
            assert!(system_prompt.contains("watcher for My Product"));
            assert!(sid.starts_with("my-product-watcher-fast/"));
        }
        Decision::Skip { .. } => panic!("expected prompt"),
    }
}
```

## Running multiple variants

Three variants of the same persona, all on the same humd:

```rust
let personas = vec![
    MyProductWatcher::new("aggressive"),
    MyProductWatcher::new("conservative"),
    MyProductWatcher::new("fast"),
];

for p in personas {
    runtime.spawn_persona(p);
}
```

Each variant has a distinct `bee_name` and a distinct keyring entry; the mesh sees them as
three distinct bees. The variants compete on the bounty surface (winner-takes-the-bounty
discourages duplicate-effort waste) and accumulate reputation independently.

## Reference implementations

- `reverbprotocol/markets/agents/reverb-markets-personas` — four personas for the prediction-market product (auto-create, auto-resolve, auto-dispute, arbiter)
- `damanfi/copy-bond/agents/daman-personas` — Daman's persona set (watchdog, arbiter, recruiter, ...)

## What's next

- [Operating model](/OPERATING_MODEL#the-persona-bee-contract) for the formal contract.
- [Autonomy spectrum](/AUTONOMY_SPECTRUM) to decide which autonomy level your venture
  operates at; the persona-bee pattern is the L5 shape.
- [Dispute via hum](/scenarios/dispute-via-hum) for an end-to-end scenario exercising a
  watchdog persona end-to-end.
