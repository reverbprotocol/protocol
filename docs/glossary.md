# Glossary

Substrate-specific vocabulary, the operating-model layer's terms inherited from hum, and the
production-deploy terms inherited from the on-chain layer. Definitions are tight on purpose:
each term is one sentence with a cross-link to where it shows up.

## Substrate layer

**ABI freeze test**
A unit test that asserts a contract's selectors and event-topic hashes match precomputed
constants. Locks the on-chain ABI as a regression baseline. See `test/SelectorFreezeRefundProtocolFixed.t.sol` in the substrate repo.

**arbiter**
The address authorized to call `refundByArbiter` and `earlyWithdrawByArbiter` on `RefundProtocolFixed`. On Arc testnet today this is the deployer EOA; production rotates to a dedicated multisig before mainnet.

**bytes32 builder**
The attribution tag carried on every external entry point that can be invoked through a
third-party UI. Convention documented by [IAttributable](/interfaces/IAttributable); travels through events for indexer queries.

**cumulative withdrawn**
Per-payment running total of all amounts withdrawn through `earlyWithdrawByArbiter`. Bounded by `payment.amount` per the FIX-2 invariant. Locked under stateful fuzz as `invariant_noOverWithdraw`.

**dispute primitive**
`RefundProtocolFixed`, the substrate's on-chain dispute-mediated escrow. Live at proxy
`0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F` on Arc testnet. Forked from `circlefin/refund-protocol@b506b17` with four classes of correctness fix.

**hardened arbiter**
A future deployment where the arbiter address is a Safe multisig with on-chain dispute review SLA. The substrate supports this today; the testnet posture has the deployer as arbiter for demo velocity.

**HumdRegistry sidecar pattern**
The architectural split that keeps the immutable identity layer separate from the
upgradeable application-state layer. Pattern documented at [HumdRegistry sidecar pattern](/humd-registry-sidecar).

**immutable upstream**
The vendored `RefundProtocolUpstream.sol` in the substrate repo's `test/vendor/` directory. Reproduces the upstream contract verbatim so the differential tests in `RefundProtocolFixed.t.sol` can prove that the four classes of fix change behavior without breaking compatibility for upstream-targeted callers.

**operating model**
The off-chain published standard: forager hive contract + persona bee contract + cross-product mesh conventions. Documented at [Operating model](/OPERATING_MODEL).

**proxy / implementation**
The ERC1967 proxy at the public-facing address; the implementation contract behind it (rotated by `_authorizeUpgrade` on the Timelock-owned `upgradeToAndCall`). The two addresses appear together in [`.deployments/arc-testnet.json`](https://github.com/reverbprotocol/protocol/blob/main/.deployments/arc-testnet.json).

**reference implementation**
A concrete contract under `src/reference/` that implements one of the substrate interfaces. Shipped for direct fork-and-adapt; production-deploy controls are the consumer's responsibility.

**reputation sidecar**
A separate upgradeable contract that stores per-bee score keyed by HumdRegistry-identity addresses. Implements `IReputationRegistry`. Distinct from HumdRegistry itself, which is immutable and identity-only.

**Safe multisig**
The 3-of-5 Gnosis Safe at `0x70a34ca4964a16a934432871a593acba5dd63cf1`. Proposer + executor on the Timelock; direct pauser on the substrate contracts; testnet posture has all 5 signers operator-controlled.

**TimelockController**
The OpenZeppelin contract at `0xa22510860289751C092e67B15b827020CE09DAbf` with a 24-hour minimum delay. Owns every UUPS proxy in the substrate deployment. Sole proposer + executor is the Safe.

**UUPS proxy**
Universal Upgradeable Proxy Standard. The substrate's deployment pattern: ERC1967 proxy points at the implementation; the proxy is the public-facing address; upgrade authority is gated by `_authorizeUpgrade(address)` requiring owner authority.

## Operating-model layer (inherited from hum's wire spec)

**asker**
A persona bee. The asker assembles world state and prompts the worker bee; it does not decide. The worker (claude-cli, vercel-ai, ollama, etc.) decides via tool calls.

**bee**
An instance of a hive. A running participant of one type in the hum ensemble. Examples: `daman-watchdog-aggressive`, `markets-auto-create-macro`, `reverb-arc-fs`.

**bloom**
One turn of conversation. Opened by a `prompt` chi and closed by a `finish` chi.

**chi**
A message type that travels on a thrum tone. Base vocabulary: `hello`, `prompt`, `chunk`,
`tool-call`, `tool-result`, `finish`, `error`, `attach`, `cancel`, `breath`, `drone`,
`gossip-publish`. Consumer products extend with product-specific chis (e.g. `dispute-filed`, `slash-claim`, `bond-posted`).

**ensemble**
The mesh of cooperating humds where bees gossip across hosts. A single bloom can draw
inference from one humd, filesystem operations from another, settlement from a third.

**forager hive**
A hive whose responsibility is to translate an outside wire to thrum. The substrate's
canonical forager hive is `reverb-arc-fs` (translates Arc chain state + wallet operations to
thrum). hum's reference foragers include `humfs`, `twilio-sms`, `paid-oracle`.

**hello manifest**
The JSON payload emitted by a bee on `chi:"hello"`. Declares the bee's name, version, propensity, supported chis, supported tools, and source URL.

**hive**
The kind. A typology a bee conforms to. Multiple bees may instantiate the same hive (e.g.
five `daman-watchdog` bees on one humd).

**humd**
The hum daemon. The local process that hosts bees, routes chi, and bridges to other humds in
the ensemble.

**HumdRegistry**
The on-chain registry contract where bees advertise their public keys. Immutable; one address space per subnet. Source at `adiled/hum/contracts/`.

**nest**
The local registry of nestled bees inside a humd. Where bees gather once they've completed handshake.

**nestler**
A bee mid-handshake. Awaiting the breath that accepts its hello.

**nestled**
A nestler once registered into the nest.

**petal**
One unit of content. Text, image, a tool call, or its result.

**propensity**
A bee's hello-declared self-classification: statefulness (stateful / stateless), richness
(rich / thin), wire (the protocol family it speaks).

**sid**
A sigil identifier scoped to a persona's bloom on a worker bee. Naming convention: `{bee-name}/{unique-suffix}`.

**sigil**
Bloom identifier. Persists across the bloom's lifetime; routes responses back to the correct
caller.

**thrum**
The hum-native vibration protocol. Carries tones across a range of chi. Reference clients in
Rust, Go, TypeScript, Python.

**tone**
One transmission on the thrum protocol. Carries one chi plus metadata (sender, recipient, sigil, callId where applicable).

**tool**
A named callable a forager exposes via its hello manifest. snake_case; namespaced by
product (`arc_*`, `markets_*`, `daman_*`). Idempotency declared per tool.

**wane**
The cursor tracking which petals each humd has consumed. Enables partition+heal reconciliation via diff comparison; never time-window resync.

**worker bee**
A bee that runs an LLM (claude-cli, vercel-ai, openai-server, ollama-server). Receives prompts on a sid and emits chunks, tool calls, and tool results.

## Production-deploy layer

**circuit breaker**
The Safe's pause path on `RefundProtocolFixed` and `Operator`. Gated by the `pauser` role (Safe directly, no delay). Distinct from `unpause`, which is owner-gated through the Timelock.

**ERC-7201**
EIP for namespaced storage in upgradeable contracts. Used by `CCTPReceiverMixin` to isolate its slots from inheriting contract layouts at slot
`0x009c3710a3eb8a7e5b03d4342e58f75d7df000a9a29f59f713993589837e6200`.

**FIX-1 / FIX-2 / FIX-3 / FIX-4**
The four classes of correctness fix applied to the upstream `circlefin/refund-protocol@b506b17`. CEI ordering, cumulative over-withdraw guard, debt-settle-before-early-withdraw, zero-recipient guard. See [IRefundProtocol](/interfaces/IRefundProtocol#four-classes-of-fix).

**Idempotency**
Property of a tool: `Idempotent` (read tools; safe to retry) or `NotIdempotent` (write tools; retry may produce duplicate transactions). Declared per tool via the `Tool::idempotency` trait method.

**Initializer-only-once invariant**
Asserts that the `initialize` function on each UUPS proxy cannot be called a second time after deploy. Locked under stateful fuzz via `invariant_initializerOnlyOnce`.

**ReentrancyGuardTransient**
EIP-1153 transient-storage variant of OpenZeppelin's `ReentrancyGuard`. Used by the substrate's upgradeable contracts because transient storage is proxy-safe and zeros out per transaction.
