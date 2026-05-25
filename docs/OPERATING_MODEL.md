# Operating model

Reverb Protocol publishes two layers of standard:

1. **On-chain interfaces** (covered in [Substrate primitives](/interfaces/)) define what contracts a consumer product deploys.
2. **Off-chain operating model** (this page) defines how a consumer product's agents and personas attach to a humd ensemble, route tool calls, manage wallets, and gossip across products.

A consumer product that follows both layers gets architectural compatibility with every other consumer product on the substrate for free. Two reference deployments demonstrate this concretely today: Daman (slash-bonded copy-trading) and Reverb Markets (third-party prediction-market operator), both running on a shared humd ensemble.

## Two-layer published-as-standard claim

The on-chain layer is enforced by Solidity types: a contract that declares `is IBountyAccrual` must match the interface or compilation fails. The off-chain layer is enforced by spec: a forager that declares itself a `reverb-arc-fs` extension must conform to the hello-manifest shape, the tool-routing convention, and the safety-pipeline requirements documented below. Both layers are published; both layers are versioned; both layers are consumer-product-extensible.

Future consumer products inherit the operating model by importing the substrate's `reverb-arc-fs` and `persona-base` crates and extending them with product-specific tool surfaces and persona role definitions.

## The forager hive contract

A forager is a thrum-attached process that owns a resource and exposes operations as `chi:"tool-call"`-addressable tools routed by humd. The substrate ships `reverb-arc-fs` as a Rust library that consumer-product persona binaries import to compose their own per-persona forager. There is no standalone shared-forager process; the forager IS each persona binary.

This mirrors `humfs`'s per-instance `fs.roots` scoping from hum's hives catalogue: process boundary is identity boundary. A persona's tool surface, EOA private key, and ed25519 stable identity all share the process they run in. A second persona is a second process with a second key and a second hid.

### Two keys per persona binary

Every persona binary holds two keys with separate lifecycles and separate storage paths:

| Key | Type | Storage path | Purpose |
|---|---|---|---|
| `BeeIdentity` | ed25519 (32-byte seed) | `$XDG_STATE_HOME/hum/bees/<bee_name>.key` (0600) | Stable identity to humd. Hashed via `sha256(pubkey)` into the mandatory `hid` field on the hello as `fbee_<hex>`. Minted on first boot, reloaded every subsequent boot. |
| `PrivateKey` | secp256k1 (hex EOA) | `$XDG_CONFIG_HOME/hum/<bee_name>/key.hex` (0600) | Arc transaction signer. Funded with USDC on Arc testnet for gas. Minted out-of-band via the persona's mint script. |

The `hid` is mandatory. Without it, humd cannot deduplicate the bee across reconnects and every reconnect leaks a fresh manifest, multiplying the bee's tool count until humd restarts. `PersonaForagerBuilder` enforces both keys at construction; a builder missing either field fails with `BuilderIncomplete`.

### Hello manifest

Every forager announces itself with a `chi:"hello"` tone carrying a manifest:

```json
{
  "chi": "hello",
  "bee": "markets-auto-create-default",
  "hid": "fbee_12437df7ceba2c3be95b928503132d5a133cb6d013fc286a577681515fbed9da",
  "version": "0.1.0",
  "protoVersion": "0.7.0",
  "propensity": {
    "statefulness": "stateful",
    "richness": "rich",
    "wire": "reverb-markets/arc-fs"
  },
  "chis": [
    "hello", "echo", "log", "perf-mark",
    "tool-call", "tool-result", "tool-meta",
    "gossip-publish"
  ],
  "tools": [
    "mkac_create_market",
    "mkac_read_market_state",
    "mkac_read_settlement_history",
    "mkac_subscribe_release_feed"
  ],
  "source": "https://github.com/reverbprotocol/markets/tree/main/agents/reverb-markets-personas"
}
```

The manifest's `tools` field is the routing table. humd inspects every incoming `chi:"tool-call"` and routes by tool name to the bee whose hello declares it. Each persona's tool names carry a short namespace prefix (`mkac_`, `mkar_`, `mkad_`, `mkarb_` for the four Reverb Markets personas) so that multiple personas on the same humd can coexist without name collisions.

### Tool registry expectations

Each tool has:

- A tool name following `<namespace>_<action>`, where namespace is a short alias of the persona's bee name (e.g. `mkac` for `markets-auto-create-*`).
- A typed input schema (validated against the on-chain function's input shape before any chain call).
- A typed output schema (returned via `chi:"tool-result"` with the original `callId`).
- An idempotency declaration (read tools are idempotent; write tools are not).

Role enforcement is structural: a persona's `PersonaForagerBuilder::with_tools(...)` call registers only the tools that persona is authorized to invoke. A tool not registered cannot be called through that persona's forager process. The `markets-auto-create` binary registers `mkac_create_market` but never `mkac_rule_dispute`; the LLM running inside that persona's worker session cannot reach disputes even if it hallucinates a call.

There is no per-call `as_bee` auth check, because the process IS the bee. A tool call arriving on the persona's thrum socket is by definition addressed to that persona's EOA.

### Safety pipeline requirements

Every write tool runs through a five-stage pipeline. Each stage may surface a structured error to the consumer via `chi:"tool-result"`.

1. **ABI validation.** Args match the on-chain function's input schema.
2. **Allowed-contracts gate.** The target contract is in the persona's `allowed_contracts` list, the analog of `humfs`'s `fs.roots`.
3. **Simulation gate.** `eth_call` against current state; revert surfaces in tool-result with the reason.
4. **Rate limit.** Per-tool + global throttles.
5. **Send.** Sign with the persona's `PrivateKey` + submit. On chain revert despite simulation pass, log race and return error.

A forager that skips any stage is non-conformant. Tests in the substrate's `reverb-arc-fs` crate exercise each stage as an independent invariant.

### Scoping configuration

The persona binary's per-process config lives at `$XDG_CONFIG_HOME/hum/<bee_name>/config.json`:

```json
{
  "rpc_url": "https://rpc.testnet.arc.network",
  "explorer_api": "https://testnet.arcscan.app/api/v2",
  "chain_id": 5042002,
  "allowed_contracts": [
    "0x344b472b7b1ad0a35e11718bc063fd46f4282db2"
  ],
  "rate_limit": {
    "per_tool_tx_per_minute": 60,
    "global_tx_per_minute": 200
  }
}
```

`allowed_contracts` is the persona's write boundary. The `markets-auto-create` persona only writes to the Reverb Markets `Operator`; the `markets-arbiter` persona writes to both the `Operator` and the substrate's `RefundProtocolFixed` dispute primitive. Setting the list correctly per persona is part of role definition.

## The persona bee contract

A persona bee is a thin asker. It assembles world state from gossip + chain events, opens a sid into a local LLM worker (e.g. `claude-cli`), prompts the worker with a tight role-overlay system prompt, observes the worker's tool calls flow through the local forager, and logs.

The substrate ships `persona-base` as the canonical scaffolding. Consumer products implement the `PersonaBee` trait for each role they need.

### The thin-asker pattern

The persona itself contains no decision logic. The decision is the LLM's, made inside the worker bee's sid. The persona's job is to:

1. Subscribe to relevant gossip topics + chain event filters
2. On observation, assemble the current world state (recent events, current contract state, current bee balance)
3. Construct a prompt that pairs the role-overlay system prompt with the observed world state
4. Open a sid into the local LLM worker; emit `chi:"prompt"`
5. Observe `chi:"tool-call"`, `chi:"tool-result"`, `chi:"chunk"`, and `chi:"finish"` on the sid
6. Log the bloom for audit

The role allowlist (which tool calls a persona is permitted to make) is enforced **structurally at composition time**, not via per-call auth: each persona binary registers only the tools it is authorized to call via `PersonaForagerBuilder::with_tools(...)`. A tool the persona did not register cannot be invoked through that persona's process. Because the process is the bee, a tool call landing on the persona's thrum socket is by definition addressed to that persona's EOA.

### Sid management

Each persona owns a sid namespace (`{product}-{role}-{variant}/...`). New events spawn fresh sids; long-running observations may reuse a sid across multiple tool calls. The persona is responsible for sid hygiene (closing finished sids, not leaking sid state across role boundaries).

### Gossip + event subscriptions

Personas declare their subscriptions in the `subscribe_topics() -> Vec<String>` and `subscribe_chain_events() -> Vec<EventFilter>` methods of the `PersonaBee` trait. Subscriptions become humd-side routing rules; the persona's `on_event()` handler is called with each matched delivery.

## Cross-product mesh conventions

Two consumer products running on the same humd ensemble share gossip topics, share the chi vocabulary base, share the forager safety pipeline, and share the chain identity space (via HumdRegistry on the shared subnet).

### Gossip topic naming

Topics follow `{product}/{domain}/{purpose}`:

- `daman/credit/p2p` — Daman's peer-to-peer credit gossip
- `daman/slash/observability` — Daman's slash-claim observability stream
- `reverb-markets/disputes/observability` — Reverb Markets's dispute observability stream
- `reverb-markets/releases/macro` — Reverb Markets's macro-release feed

A persona on one product MAY subscribe to topics on the other product. The cross-product mesh is the substrate's promise that this works without any new wiring.

### Chi vocabulary

The base chi vocabulary (`hello`, `prompt`, `chunk`, `tool-call`, `tool-result`, `finish`, `error`, `attach`, `cancel`, `breath`, `drone`, `gossip-publish`) is inherited from hum's wire spec. Each consumer product adds product-specific chis to its forager manifest (e.g. Reverb Markets's `market-created`, `dispute-filed`, `dispute-ruled`). Cross-product personas understand the base chis universally and the extension chis via the relevant product's documentation.

### Bee-name namespacing

Bee names follow `{product}-{role}-{variant}`:

- `daman-watchdog-aggressive`
- `daman-arbiter-strict`
- `markets-auto-create-macro`
- `markets-arbiter-conservative`

The variant suffix allows N personas of the same role with different prompt overlays. humd treats them as distinct bees; the forager's keyring scopes by bee name.

## Cooperative equilibrium examples

The shared operating model enables emergent cross-product behavior without any operator-side glue.

### A Daman watchdog observing Reverb Markets disputes

`daman-watchdog-aggressive` subscribes to `reverb-markets/disputes/observability`. When Reverb Markets's auto-dispute persona files a dispute, the daman watchdog observes the dispute payload via the gossip topic. The watchdog's LLM may decide that the same dispute pattern indicates a Daman leader degradation (e.g. the same address that authored the bad market resolution is also a leader on Daman). The watchdog emits `daman_file_slash_claim` via the Daman forager. Both products' on-chain economics (`IBountyAccrual`, `IReputationRegistry`) are shared; the cross-product observation produces correlated reputation signals across both products.

### An arbiter persona ruling across both products

A neutral arbiter persona can be defined with subscriptions to both `daman/disputes/observability` and `reverb-markets/disputes/observability`. The same role definition, the same prompt overlay, two product surfaces. The arbiter's rulings flow through whichever forager the dispute lives on; the on-chain dispute primitive (`RefundProtocolFixed` at `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F`) is shared by both products.

### Cross-product reputation accumulation

Currently per-subnet (per `HumdRegistry` deployment). Cross-subnet reputation portability is documented as an open research question in [What Reverb Protocol does not do](/AUTONOMY_SPECTRUM#what-reverb-protocol-does-not-do).

## Autonomy spectrum applied to the forager layer

The forager layer has its own autonomy progression, parallel to the persona layer's spectrum:

- **L0 deterministic forager** (current `reverb-arc-fs` shape): hardcoded tool dispatch, deterministic safety pipeline, deterministic ABI validation. The forager is operator-trusted infrastructure that the personas trust to be predictable.
- **L5 sovereign forager** (documented future scope): the forager itself wraps an LLM session, interpreting ambiguous tool calls and refusing or rewriting them. Useful for a hardened forager that wants to add semantic safety on top of the cryptographic safety pipeline.

The pilot operates at L0 on the forager layer (deterministic) and L5 on the persona layer (sovereign LLM-driven decisions). L5 on the forager layer is not in pilot scope.

## Reference implementations

- **`foragers/reverb-arc-fs/`** (Rust library): the canonical Arc-state + wallet substrate. Exposes `BeeIdentity::load_or_mint`, `PrivateKey::load`, `PersonaForager`, `PersonaForagerBuilder`, the safety pipeline traits, and the base `arc_*` tool definitions. Imported as a Cargo dependency by every consumer-product persona binary. Source at [`reverbprotocol/protocol`](https://github.com/reverbprotocol/protocol/tree/main/foragers/reverb-arc-fs).
- **`foragers/persona-base/`** (Rust library): persona-side scaffolding. `PersonaBee` trait + `AskerLoop` + `ToolCallObserver` + `BustDetector` + `PersonaBinarySpec`. Source at [`reverbprotocol/protocol`](https://github.com/reverbprotocol/protocol/tree/main/foragers/persona-base).
- **`reverbprotocol/markets/foragers/reverb-markets-arc-fs/`** (Rust library): exposes `markets_tools(namespace: &str) -> Vec<Tool>` and per-action factory functions (`create_market`, `resolve_market`, `file_dispute`, `rule_dispute`, `read_market_state`, `read_settlement_history`, `subscribe_release_feed`). Each persona binary picks its own subset.
- **`reverbprotocol/markets/agents/reverb-markets-personas/`** (four Rust binaries): `markets-auto-create`, `markets-auto-resolve`, `markets-auto-dispute`, `markets-arbiter`. Each composes `PersonaForagerBuilder` + role-scoped subset of `markets_tools` + role-scoped `allowed_contracts` + the matching `PersonaBee` impl from the library half of the crate.
- **`damanfi/copy-bond/`** (in progress at time of writing): same composition pattern, separate persona set.

A consumer product that wants to operate on the substrate at any autonomy level above L0 imports `persona-base` and `reverb-arc-fs`, defines per-action tool factories in a product-specific library crate, and ships one binary per persona role. The persona's role is defined by three composition choices: which tools it registers, which contracts it lists in `allowed_contracts`, and the role-overlay system prompt its `PersonaBee` impl returns.
