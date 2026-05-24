# Security posture

The deployed configuration on Arc testnet ([`.deployments/arc-testnet.json`](https://github.com/reverbprotocol/protocol/blob/main/.deployments/arc-testnet.json)):

## UUPS upgradeable

`RefundProtocolFixed` is an `Initializable` + `UUPSUpgradeable` contract behind an ERC1967 proxy. Implementation deployed at the address in the deployments file; upgrade authority gated by `_authorizeUpgrade` on the owner.

## TimelockController owns upgrade authority

The proxy's `owner()` is a TimelockController with a 24-hour minimum delay. Every upgrade is `schedule()`'d, visible on-chain during the delay window, and `execute()`'d only after the delay elapses. Cancellable during the window via the Safe's `cancel(operationId)`.

- Timelock address: `0xa22510860289751C092e67B15b827020CE09DAbf`
- Delay: 86400 seconds (24 hours)
- Proposer + executor + admin: the Safe multisig

## Safe multisig fronts the TimelockController

A 3-of-5 Safe is the sole proposer + executor on the TimelockController. The Safe is also the `pauser` on the contract; it can pause without Timelock delay (emergency stop). Unpause flows through `onlyOwner` (Timelock-gated, 24h).

- Safe address: `0x70a34ca4964a16a934432871a593acba5dd63cf1`
- Threshold: 3 of 5
- Singleton version: Safe v1.4.1
- Testnet posture: all 5 signers are operator-controlled. Mainnet pre-requisite is independent signer distribution (three minimum independent parties holding hardware-wallet or HSM-managed keys).

## Pausable critical paths

`pay`, `refundByRecipient`, `refundByArbiter`, `withdraw`, `earlyWithdrawByArbiter` are gated by `whenNotPaused`. The dispute-record state stays readable; new disputes and settlements halt.

`settleDebt`, `depositArbiterFunds`, `withdrawArbiterFunds`, `setLockupSeconds`, `updateRefundTo` remain live during pause so cleanup and admin paths are not halted.

## Reentrancy

`ReentrancyGuardTransient` (EIP-1153 transient storage; Arc Cancun config) on every state-mutating external function. CEI ordering verified across all fix paths.

## Selector + event freeze tests

[`test/SelectorFreezeRefundProtocolFixed.t.sol`](https://github.com/reverbprotocol/protocol/blob/main/test/SelectorFreezeRefundProtocolFixed.t.sol) locks 12 external function selectors and 6 event topic hashes. Any change to a signature or event shape fails the matching freeze test before merge.

## Stateful fuzz invariants

[`test/RefundProtocolInvariant.t.sol`](https://github.com/reverbprotocol/protocol/blob/main/test/RefundProtocolInvariant.t.sol) runs 256 fuzz runs (128,000 calls per invariant) asserting:

- `invariant_noOverWithdraw`: cumulative withdrawnAmount on any payment is bounded by its original amount. Locks the FIX-2 property under stateful fuzz.
- `invariant_aggregateBalanceBoundedByTokenBalance`: the sum of per-account balances tracked in the contract never exceeds the contract's actual token holdings.

0 reverts, 0 invariant failures on the most recent run.

## Static analysis

[`.github/workflows/security.yml`](https://github.com/reverbprotocol/protocol/blob/main/.github/workflows/security.yml) runs Slither on every PR with `fail-on=high`. Mythril runs nightly via cron at 03:17 UTC and uploads a markdown report as an artifact.

## Production-deploy guidance for consumers

Every reference implementation in `src/reference/` carries a NatSpec block clarifying it is shipped for direct fork-and-adapt; the production-deploy controls (UUPS proxy, Pausable on state mutators, Timelock + Safe ownership, slither/mythril CI) are the consumer's responsibility. The reference is the starting point; the deployed posture is the consumer's call.
