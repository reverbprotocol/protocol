# UUPS upgrade lifecycle

The full Safe + Timelock + UUPS upgrade path end-to-end. The scenario validates the
queued-upgrade visibility property holds throughout: from proposal to execution, any party
monitoring the Timelock can inspect the queued implementation bytecode before it lands.

## Setup

**Trust tier**: T1 (operator devices) for the demo; production posture has independent signers.

**Actors**:

- Five Safe signers (operator-controlled on testnet; documented mainnet pre-requisite is three minimum independent parties).
- The deployed substrate proxy at `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F`.
- The Safe multisig at `0x70a34ca4964a16a934432871a593acba5dd63cf1`.
- The TimelockController at `0xa22510860289751C092e67B15b827020CE09DAbf` with 86400 second (24h) delay.
- An arbitrary observer monitoring the Timelock's `CallScheduled` event.

**Pre-state**:

- `proxy.owner() == TimelockController`
- `proxy.pauser() == Safe`
- A new implementation `RefundProtocolFixedV2` has been deployed at a fresh address.

## Happy path

**Step 0: implementation deployed**
The deployer EOA deploys `RefundProtocolFixedV2`. The deployment is observable via the
explorer; the new implementation is not yet the active implementation behind the proxy.

**Step 1: Safe signers compose the upgrade**
Three of five Safe signers (3-of-5 threshold) sign a Safe transaction that calls
`TimelockController.schedule(target, value, data, predecessor, salt, delay)` where:

- `target = proxy address`
- `value = 0`
- `data = upgradeToAndCall(newImpl, "")` encoded selector + args
- `predecessor = bytes32(0)`
- `salt = chosen value`
- `delay = 86400` (must meet or exceed Timelock's minimum delay)

The Safe execution emits `TimelockController.CallScheduled` with the computed `operationId`.

**Step 2: queued-upgrade visibility window opens**
For the next 24 hours, the upgrade is publicly queued. Any observer can:

- Query `TimelockController.getTimestamp(operationId)` for the earliest execution time.
- Decode the queued `data` to recover the target implementation address.
- Read the implementation's bytecode + abi from on-chain.

If the queued upgrade is malicious or wrong, observers have 24 hours to react: they can exit positions, file disputes, or coordinate a `TimelockController.cancel(operationId)` call from the Safe.

**Step 3: cancellation, if needed**
If the operator decides the upgrade is wrong (post-review), three of five Safe signers
compose a `TimelockController.cancel(operationId)` call. The operation enters the cancelled state; no execution path is reachable. Observable as `TimelockController.Cancelled` event.

**Step 4: execution after delay**
After 24h elapses, three of five Safe signers compose a `TimelockController.execute(...)` call with the same parameters. The Timelock validates the operation has matured and calls `proxy.upgradeToAndCall(newImpl, "")`. The proxy's `_authorizeUpgrade` checks
`msg.sender == owner` (the Timelock) and accepts.

**Step 5: post-upgrade verification**
The proxy's implementation slot now points at `RefundProtocolFixedV2`. The proxy's storage layout is preserved (the V2 contract's storage matches V1's plus appended additions plus the
`__gap[49]` reservation). External callers continue to interact with the same proxy address; the new logic is live.

**Step 6: storage-layout CI gate (post-deploy verification)**
A follow-up CI step runs `forge inspect RefundProtocolFixedV2 storageLayout` against the
prior `forge inspect RefundProtocolFixed storageLayout` snapshot. If the layout drifted in
an incompatible way (reordered slots, type changes, missing variables), the gate fails the build retroactively. The upgrade still landed on-chain; the CI gate is the operator's safety net for catching mistakes.

**Step 7: selector + event freeze tests**
The `test/SelectorFreezeRefundProtocolFixed.t.sol` suite runs against the new
implementation. If the V2 changed any external function signature or event topic, the freeze test fails and the operator pulls the upgrade-author into a review cycle (post-deploy mitigation, since the test runs in CI not at deploy time).

## Failure modes

**Insufficient signers.** The Safe transaction requires 3-of-5 signers. If only 2 sign, the Safe transaction does not execute. No on-chain side effect on the substrate.

**Delay underflow.** If the proposer specifies `delay < Timelock minimum delay`, the
`TimelockController.schedule` reverts with `TimelockController: insufficient delay`. The upgrade is not queued.

**Cancellation race.** If the Safe attempts to cancel after the operation has been executed, the cancel reverts. The window for cancellation is exactly the delay period.

**Implementation bytecode tampered.** If a signer-side compromise swaps the implementation address in the queued `data` payload, the queued `data` is on-chain and observable. Honest observers (other Safe signers, independent dispute-monitoring agents, watchdog personas configured to alert on Timelock activity) can flag the swap during the 24h window.

**Storage layout drift.** A V2 that reorders storage variables would corrupt state on
upgrade. The storage-layout CI gate catches this post-deploy; until the gate is in place,
the discipline is verbal review at code-review time. Substrate ships with `__gap[49]`
reservation so additive changes are safe; reorderings remain a manual-review concern.

**Pauser-vs-upgrade race.** If the Safe pauses the contract during the Timelock delay (emergency stop), the upgrade still executes when the delay matures. Pause and upgrade are independent authority paths; the operator may pause during upgrade for additional safety.

## Success criteria

- The `CallScheduled` event fires with the correct operationId.
- During the 24h window, any caller can read the queued target + data via
  `TimelockController.getTimestamp(operationId)` + the event payload.
- The `execute` call only succeeds at or after `scheduledTime + delay`.
- The proxy's `implementation()` (queried via `ERC1967Utils.getImplementation()`) returns the new address post-execute.
- The proxy's `owner()` is unchanged (still the Timelock).
- Post-upgrade, the full test suite passes against the new implementation.

## Validation scope

The scenario exercises:

- The Timelock-fronted-by-Safe ownership chain established at deploy ([Security posture](/security-posture))
- The UUPS pattern's `_authorizeUpgrade` gating
- The queued-upgrade visibility property that makes the 24h delay meaningful
- The cancellation path
- The storage-layout discipline (`__gap` reservation as future-additive insurance)
- The selector + event freeze tests as the post-upgrade ABI-regression check
- The pauser-vs-upgrade authority separation (Section 4 of the production controls)
