# Architecture

A conceptual overview of where each substrate piece lives, how the layers compose, and what a request looks like end-to-end.

## Three layers

Reverb Protocol composes from three architectural layers. Each layer is independently
extensible by consumer products; each layer publishes its standard.

```
┌──────────────────────────────────────────────────────────────────────┐
│ Persona layer (off-chain)                                            │
│   PersonaBee implementations                                         │
│   tight role-overlay system prompts; one sid per bloom               │
│   subscriptions to gossip topics + chain events                      │
│                                                                       │
│   examples: markets-auto-create-macro, daman-watchdog-aggressive     │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ chi:"prompt", chi:"attach"
┌──────────────────────────────────────────────────────────────────────┐
│ Worker layer (off-chain)                                             │
│   LLM-shaped bees: claude-cli, vercel-ai, ollama-server, etc.        │
│   emits chi:"tool-call" with structured args                         │
│   emits chi:"chunk" / chi:"finish" / chi:"error"                     │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ chi:"tool-call" routed by toolName
┌──────────────────────────────────────────────────────────────────────┐
│ Forager layer (off-chain; published as standard)                     │
│   reverb-arc-fs (base)                                               │
│     ↓ extended by                                                    │
│   reverb-markets-arc-fs, daman-arc-fs, future-product-arc-fs         │
│                                                                       │
│   owns: wallet keyring (per-bee), safety pipeline (6 stages),        │
│         scoping config (allowed_contracts), receipt cache            │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ signed transaction on Arc
┌──────────────────────────────────────────────────────────────────────┐
│ Substrate layer (on-chain; published as standard)                    │
│   RefundProtocolFixed (UUPS proxy at 0xc8bF99c5…)                   │
│     ← shared by all consumer products as the dispute primitive       │
│   IBountyAccrual, IReputationRegistry, IAttributable, ICCTPReceiver, │
│   IBondYieldVault, IStableFXSwap, IRefundProtocol                   │
│     ← interfaces; consumer products ship conformant implementations  │
│                                                                       │
│   governance: TimelockController (24h) ← Safe multisig (3-of-5)     │
└──────────────────────────────────────────────────────────────────────┘
```

The persona layer is the consumer product's. The worker layer is whatever LLM the consumer picks. The forager layer is the substrate's reference (`reverb-arc-fs`) plus the consumer's extensions. The substrate layer is the on-chain Solidity surface that every consumer shares.

## Request lifecycle

A representative request: a `markets-auto-resolve-strict` persona settles a market on Arc against a freshly-released NFP print.

```
0. release-feed forager pushes chi:"gossip-publish" with topic
   "reverb-markets/releases/macro" and body {release: "NFP", value: 184_000, ...}

1. markets-auto-resolve-strict observes the gossip via its subscribe_topics().
   AskerLoop calls persona.on_event(Event::Gossip{...}).

2. persona.on_event returns Decision::Prompt with:
     sid = "markets-auto-resolve-strict/{nanos}"
     system_prompt = role overlay (the persona's persona_system_prompt())
     user_prompt = serialized release event

3. AskerLoop emits chi:"prompt" {sid, system_prompt, user_prompt}
   to the local worker bee (claude-cli).

4. claude-cli receives the prompt, runs the model, decides to settle
   market id 42 with outcome YES. Emits chi:"tool-call" {
     callId: "c-1", toolName: "markets_resolve_market",
     args: { marketId: 42, outcome: "YES", evidence_cid: "bafk…" }
   }

5. humd inspects the chi:"tool-call". Looks up "markets_resolve_market"
   in the tool routing table → reverb-markets-arc-fs.
   Routes the tone to reverb-markets-arc-fs.

6. reverb-markets-arc-fs runs the six-stage safety pipeline:
     ✓ auth check: chi.from == args.as_bee → "markets-auto-resolve-strict"
     ✓ ABI validation: marketId is uint256, outcome is uint8 in {0,1}
     ✓ simulation gate: eth_call against Operator → would succeed
     ✓ rate limit: per-bee/per-tool/global counters under threshold
     ✓ send: sign + submit via wallet keyring entry for
              markets-auto-resolve-strict
     ✓ receipt cache: stash tx hash + receipt

7. reverb-markets-arc-fs emits chi:"tool-result" {
     callId: "c-1", ok: true,
     value: { tx_hash: "0x…", block: 8421442 }
   }

8. humd routes the tool-result back to claude-cli via the
   "callId → worker client_id" mapping.

9. claude-cli emits chi:"finish" {sid, usage}.

10. AskerLoop collects the transcript and returns; persona is idle until the
    next subscription event fires.
```

Every step is observable. Step 0 is humd gossip; steps 4-6 emit on-chain events the substrate emits (`ResolutionProposed`); step 7's receipt cache means concurrent personas observing the same market can query without re-querying RPC.

## Multi-humd ensemble

A consumer-product deployment with one humd is fine for the demo. A real ensemble has multiple humds gossiping. The wire-level pattern (per [hum's scenarios](https://adiled.github.io/hum/scenarios/)):

- **humd-A**: hosts the personas (`markets-auto-resolve-strict`, etc.)
- **humd-B**: hosts the worker bee (claude-cli)
- **humd-C**: hosts `reverb-markets-arc-fs` (owns the wallet keyring)
- **humd-D**: hosts the release-feed forager

A `chi:"tool-call"` emitted by claude-cli on humd-B for `markets_resolve_market` routes
through the ensemble to humd-C, executes against Arc, and the `chi:"tool-result"` routes back to humd-B via the same path. Operator can walk away (eggs-on-the-hum pattern); the bloom finishes honestly because every link is symmetric.

The substrate operating model does not prescribe humd topology; it prescribes the chi vocabulary and the safety contract. A deployment can run one humd or ten; the personas see the same surface.

## Trust tiers

The substrate inherits hum's trust tier model:

| Tier | Setup | Use case |
|---|---|---|
| **T1** | Sibling devices via operator pubkey | One operator, multiple machines (laptop + server + phone) |
| **T2** | Known-circle via out-of-band key exchange | Operator + invited contributors |
| **T3** | Federated via cert chain (`root → humd`) | Two orgs jointly run a subnet |
| **T4** | Open peer-to-peer | Permissionless mesh; sybil-resistance via reputation economics |

The substrate's economic primitives (`IBountyAccrual`, `IReputationRegistry`) are designed for T4 (or T3-with-T4-bees). At T1/T2 the economic incentives still apply but the trust assumptions reduce the threat model.

## Where each repo lives

| Repo | Layer | Role |
|---|---|---|
| [`reverbprotocol/protocol`](https://github.com/reverbprotocol/protocol) | substrate + forager + persona-base | Publishes the standards: on-chain interfaces + off-chain operating model |
| [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets) | consumer product | Reverb Markets storefront, contracts, forager extension, persona bees |
| [`damanfi/copy-bond`](https://github.com/damanfi/copy-bond) | consumer product | Daman contracts |
| [`damanfi/protocol`](https://github.com/damanfi/protocol) | consumer-product protocol | The open standard Daman is the first deployment of |
| [`adiled/hum`](https://github.com/adiled/hum) | wire substrate | The humd binary, thrum protocol, HumdRegistry contract |

The substrate stands alone. Consumer products plug in by extending the forager + persona templates with their own tool surface and role definitions.
