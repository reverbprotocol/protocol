# Dispute via hum

A watchdog persona on one humd observes a degradation event in a consumer product's contract
state, gossips a structured slash-claim across the ensemble, and a forager on a different humd
routes the claim through `RefundProtocolFixed` on Arc. The scenario validates that the off-chain
operating model (hum gossip + persona scaffolding + forager safety pipeline) composes
end-to-end with the on-chain dispute primitive without any operator-side glue.

## Setup

**Trust tier**: T1 (operator devices) for the demo; the scenario also works at T4 with the same wire pattern.

**Actors**:

- `humd-watch` — hosts `daman-watchdog-aggressive` persona bee
- `humd-fs` — hosts `daman-arc-fs` forager (extends `reverb-arc-fs`); holds the watchdog's keyring entry
- `humd-claude` — hosts `claude-cli` worker bee
- A real `DamanCopyBond` consumer-product contract deployed against the substrate's
  `RefundProtocolFixed @ 0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F`

The three humds form an ensemble; each humd knows the others via shared operator pubkey.

**Wire vocabulary used**:

- `chi:"hello"` (initial ensemble handshake)
- `chi:"gossip-publish"` with topic `daman/slash/observability`
- `chi:"prompt"` (persona to worker)
- `chi:"tool-call"` with toolName `daman_file_slash_claim`
- `chi:"tool-result"` carrying the on-chain tx hash + receipt
- `chi:"finish"` (bloom terminator with usage)

## Happy path

**Step 1: ensemble handshake**
At boot, each humd advertises its capabilities via `chi:"hello"`. `humd-watch` advertises chis `[..., gossip-publish, prompt]`. `humd-fs` advertises tools `[arc_send_tx, daman_file_slash_claim, ...]`. `humd-claude` advertises chis `[..., tool-call]` and the worker bee `claude-cli`.

**Step 2: degradation observed**
A leader on Daman triggers the configured loss-streak threshold (seven consecutive losing settlements). The watchdog's chain-event subscription (`SettlementCompleted` on the local `DamanCopyBond` proxy) fires. `AskerLoop` calls `persona.on_event(Event::ChainEvent {...})`.

**Step 3: persona decides to prompt**
The persona returns `Decision::Prompt` with a fresh sid, the watchdog's role-overlay system prompt, and the user prompt containing the seven settlement events. `AskerLoop` emits `chi:"prompt"` to `humd-claude` via the ensemble.

**Step 4: worker decides to file**
`claude-cli` runs the model, decides the loss streak warrants a slash-claim, and emits `chi:"tool-call"` with `toolName: "daman_file_slash_claim"`, `callId: "c-7"`, `args: {leader: "0x…", evidence_cid: "bafk…", filed_at_block: 8421337}`.

**Step 5: humd routes by toolName**
humd-claude inspects the tool-call, matches `daman_file_slash_claim` against the hello manifest of `daman-arc-fs` on humd-fs, and routes the tone via the ensemble. humd-claude stashes `callId → claude-cli client_id` in its `tool_routes` registry so the result returns to the right place.

**Step 6: forager runs safety pipeline**
`daman-arc-fs` on humd-fs receives the tool-call. Six stages execute in order:

1. **Auth check**: `chi.from` (`daman-watchdog-aggressive`) equals `args.as_bee`. Pass.
2. **ABI validation**: `leader` is a valid address; `evidence_cid` is a non-empty CID; `filed_at_block` is a `u64`. Pass.
3. **Simulation gate**: `eth_call` against `DamanCopyBond.fileSlashClaim` with the watchdog's EOA. Pass (the simulation confirms the leader is registered and the bond is sufficient).
4. **Rate limit**: per-bee + per-tool + global counters under threshold. Pass.
5. **Send**: forager signs with the watchdog's EOA from the keyring, submits via JSON-RPC.
6. **Receipt cache**: tx hash + receipt stashed.

**Step 7: on-chain settlement**
`DamanCopyBond.fileSlashClaim` runs. Internally it calls `RefundProtocolFixed.pay` to escrow the slash bond against `RefundProtocolFixed`'s arbiter address. The substrate's dispute primitive holds the bond pending the challenge window.

**Step 8: tool-result returns**
`daman-arc-fs` emits `chi:"tool-result"` with `callId: "c-7"`, `ok: true`, `value: {tx_hash, receipt}`. humd-fs routes back through the ensemble to humd-claude via `tool_routes`, which delivers to the waiting `claude-cli` instance.

**Step 9: bloom finishes**
`claude-cli` emits `chi:"finish"` with the usage payload. `AskerLoop` on humd-watch collects the transcript and idles until the next subscription event fires.

## Failure modes

**Auth mismatch.** If `chi.from` does not equal `args.as_bee`, the forager surfaces `ForagerError::AuthMismatch` in the tool-result. The persona logs and moves on; no on-chain side effect.

**Simulation revert.** If the leader is not registered, the simulation reverts with the contract's error. The forager surfaces `ForagerError::SimulationRevert { reason }` without spending gas. The persona logs.

**Rate-limit hit.** If the watchdog has already filed at the per-bee threshold within the
minute, the forager surfaces `ForagerError::RateLimitBee` without simulating. The persona's
runtime should retry after the cooldown or escalate (e.g. emit a `chi:"breath"` to surface
the rate-limit hit to the operator's dashboard).

**Chain revert despite simulation pass.** Race condition: another watchdog filed the same
claim between simulation and send. The forager logs the race + surfaces `ForagerError::SendFailed { reason }`.

**Forager unreachable mid-call.** humd-fs disappears between the tool-call route and the
tool-result. humd-claude's `tool_routes` entry times out and surfaces `chi:"error"` to the worker bee. The persona's bloom transcript shows the error; the next subscription event triggers a fresh sid.

## Success criteria

- Every chi delivery hops within RTT + 50ms across the ensemble.
- The tool-call's `callId` round-trips with the matching tool-result.
- The slash bond appears in `RefundProtocolFixed.balances[arbiter]` after the tx confirms.
- The persona's bloom transcript ends with `chi:"finish"`.
- The receipt cache on humd-fs holds the tx hash for at least the rate-limit window so concurrent personas can query without re-spending RPC quota.

## Validation scope

The scenario exercises:

- `IRefundProtocol.pay` from a consumer-product contract
- The forager-hive contract end-to-end (hello, tool routing, six-stage safety pipeline, keyring lookup, receipt cache)
- The persona-bee contract end-to-end (subscription, AskerLoop, role-overlay prompt, sid management)
- Cross-humd routing via the ensemble (humd-watch → humd-claude → humd-fs → humd-claude → humd-watch)
- The `IAttributable` convention via the consumer product's `bytes32 builder` field on the slash-claim event
