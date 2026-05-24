# Cross-product arbiter

One arbiter persona rules on disputes from two consumer products on a shared humd ensemble.
The scenario validates that the substrate's cooperative-equilibrium claim at the operating-model layer is operational, not theoretical: a single persona definition can rule across products without any product-specific glue, because both products consume the same `IRefundProtocol` and the same gossip-topic convention.

## Setup

**Trust tier**: T1 (operator devices) for the demo.

**Actors**:

- `markets-arbiter-neutral` — the persona under test. Subscribes to disputes from both products.
- `humd-arbiter` — hosts the arbiter persona
- `humd-claude` — hosts the worker bee
- `humd-markets-fs` — hosts `reverb-markets-arc-fs` forager (Reverb Markets product surface)
- `humd-daman-fs` — hosts `daman-arc-fs` forager (Daman product surface)

The arbiter's keyring entry lives on whichever humd it submits transactions through. For
simplicity in the demo, both `humd-markets-fs` and `humd-daman-fs` hold the same arbiter EOA;
in production each product has its own arbiter address with its own dedicated bee variant.

**Wire vocabulary used**:

- `chi:"gossip-publish"` with topics `reverb-markets/disputes/observability` and `daman/disputes/observability`
- `chi:"prompt"` / `chi:"tool-call"` / `chi:"tool-result"` / `chi:"finish"` on the arbiter's sid
- `toolName` values `markets_rule_dispute` and `daman_rule_dispute`

## Happy path

**Step 1: subscriptions registered**
`markets-arbiter-neutral.subscribe_topics()` returns
`["reverb-markets/disputes/observability", "daman/disputes/observability"]`. humd-arbiter
records the persona's interest in both topics. The persona's `subscribe_chain_events()`
returns filters on both products' `RefundProtocolFixed`-derived events (the shared substrate
proxy is `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F`; both products write into it).

**Step 2: dispute observed on Reverb Markets**
Reverb Markets's `markets-auto-dispute-aggressive` files a dispute via `markets_file_dispute`.
`reverb-markets-arc-fs` emits `chi:"gossip-publish"` on topic
`reverb-markets/disputes/observability` with the dispute payload.

**Step 3: arbiter assembles world state**
`AskerLoop` on humd-arbiter calls `persona.on_event(Event::Gossip {...})`. The persona returns
`Decision::Prompt` with a fresh sid namespaced
`markets-arbiter-neutral/{nanos}`, the arbiter's role-overlay system prompt, and the user
prompt containing the dispute payload + the original resolution + the release data.

**Step 4: worker rules**
`claude-cli` runs the model. Reads the dispute, the original resolution, the release data. Decides the original resolution was correct (the dispute's claim that the release print contradicts the resolution does not hold up against the actual print value). Emits `chi:"tool-call"` with `toolName: "markets_rule_dispute"` and `args: {dispute_id: 9, ruling: "rejected", rationale_cid: "bafk…"}`.

**Step 5: forager routes**
humd-claude inspects the tool-call, matches `markets_rule_dispute` against humd-markets-fs's
hello manifest, and routes via the ensemble. The six-stage safety pipeline runs on the
forager side; the transaction lands on Arc; the receipt returns.

**Step 6: same persona handles a Daman dispute next**
Independently, Daman's `daman-arbiter-strict` persona is busy with its own caseload. Meanwhile, an external watchdog files a slash-claim on Daman. `daman-arc-fs` emits `chi:"gossip-publish"` on `daman/disputes/observability` with the slash payload.

**Step 7: cross-product observation fires**
The same `markets-arbiter-neutral` persona on humd-arbiter receives the gossip via its second subscription. `AskerLoop` calls `persona.on_event(Event::Gossip {...})` with the Daman dispute. The persona returns `Decision::Prompt` with a fresh sid (same persona, different bloom).

**Step 8: worker rules on Daman**
`claude-cli` runs the model again, this time with the Daman dispute payload. Decides the slash-claim is well-grounded. Emits `chi:"tool-call"` with `toolName: "daman_rule_dispute"` (a different tool, exposed by the Daman forager). humd-claude routes to humd-daman-fs. The six-stage safety pipeline runs; the slash dispatches; the bond moves.

**Step 9: shared dispute primitive resolves**
Both rulings (the rejection on Reverb Markets, the upholding on Daman) ultimately settle
against `RefundProtocolFixed @ 0xc8bF99c5...`. The substrate's dispute primitive is shared by both products; the same arbiter address can act on both because the substrate's `IRefundProtocol.refundByArbiter` (and the consumer-product wrappers around it) take the caller as the arbiter authority.

## Failure modes

**Cross-product authentication mismatch.** If the arbiter persona's keyring entry on humd-markets-fs uses a different EOA than the one on humd-daman-fs, the auth check passes (both EOAs are valid arbiters for their respective products) but the persona effectively operates as two distinct on-chain identities. The reputation registries on each product accumulate separately. This is by design at T4: cross-product reputation portability is documented as an open research question in [What Reverb Protocol does not do](/AUTONOMY_SPECTRUM#what-reverb-protocol-does-not-do).

**Topic-confusion attack.** A malicious forager publishes on `daman/disputes/observability` with a fake payload. The arbiter persona prompts the worker; the worker decides to rule; the forager's safety pipeline catches the issue at the simulation gate (the dispute_id doesn't exist on the Daman contract; eth_call reverts). No on-chain side effect. The persona logs; the reputation registry of the fake-forager doesn't accrue.

**Persona prompt prompt injection.** The dispute payload contains prompt-injection text aimed at the worker. The persona's role-overlay system prompt is short and tightly-scoped; the worker should reject the injection. If the worker ships a tool-call that's outside its role allowlist, the forager's auth check passes (the persona is the persona) but the contract's authorization layer fails (the arbiter can't, say, post a bond on the leader's behalf because the contract method requires the leader's signature). On-chain authorization is the last-resort defense.

**Both forgers unreachable.** If both humd-markets-fs and humd-daman-fs disappear, the
arbiter persona prompts the worker, the worker emits a tool-call, humd-claude can't route, the persona's `AskerLoop` sees `chi:"error"` after timeout. The persona logs and idles until subscriptions recover.

## Success criteria

- Both gossip topics deliver to the arbiter persona within RTT + 50ms.
- Each ruling produces an on-chain transaction within the configured rate-limit window.
- The shared `RefundProtocolFixed` proxy emits the expected events for both products' rulings.
- The arbiter persona's bloom transcript shows two distinct sids, one per dispute.
- The receipt cache on each product's forager holds the tx hash for downstream reads.

## Validation scope

The scenario exercises:

- A single `PersonaBee` implementation operating across two products via subscription multiplexing
- Two separate consumer-product foragers, each with their own tool surface, both routed by humd from the same worker bee
- The shared `RefundProtocolFixed` proxy handling rulings from both products
- The cooperative-equilibrium examples documented in [Operating model](/OPERATING_MODEL#cooperative-equilibrium-examples)
- The `{product}/{domain}/{purpose}` gossip-topic convention being load-bearing for cross-product mesh patterns
