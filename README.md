# Reverb Protocol

Reverb Protocol is the substrate library for dispute-mediated commerce on Arc. Extracted from project-reverb on 2026-05-23. `RefundProtocolFixed` is a fork of `circlefin/refund-protocol@b506b17` with four classes of correctness fix (CEI on `_executeRefund`, cumulative over-withdraw guard on `earlyWithdrawByArbiter`, debt-settle-before-early-withdraw, zero-recipient guard); see CHANGELOG for bug-by-bug walkthrough. Apache-2.0 attribution preserved on the vendored upstream and the fix. The interface (`IRefundProtocol`) is the standard; this repository ships one vanilla implementation; other parties may deploy their own conformant implementations.

## Contract surface

| Contract | Purpose |
|---|---|
| `src/IRefundProtocol.sol` | External interface. Standard signature surface for consumers. |
| `src/RefundProtocolFixed.sol` | Vanilla implementation. Apache-2.0 fork of `circlefin/refund-protocol@b506b17` with four classes of fix. |
| `script/Deploy.s.sol` | Foundry deployment script for the vanilla implementation. |
| `test/RefundProtocolFixed.t.sol` | Test suite for the fixed implementation. |
| `test/vendor/RefundProtocolUpstream.sol` | Vendored upstream contract used in differential tests. |

## Substrate primitives

Five substrate-level interfaces with reference implementations under `src/reference/`. Each interface is the cross-consumer standard; the reference implementations are one valid impl. Consumer products may ship their own conformant implementations with product-specific economics.

| Interface | Reference implementation | Purpose |
|---|---|---|
| `src/IBountyAccrual.sol` | `src/reference/BountyAccrualVanilla.sol` | Accrue and claim bounty notional. Funder credits a recipient; recipient claims later. |
| `src/IReputationRegistry.sol` | `src/reference/ReputationRegistryVanilla.sol` | Cumulative reputation scoring. Designated recorders log uphold or reject outcomes. |
| `src/ICCTPReceiver.sol` | `src/CCTPReceiverMixin.sol` (abstract) | Receive CCTP v2 messages on Arc, mint USDC, dispatch a consumer payload. |
| `src/IBondYieldVault.sol` | `src/reference/USYCBondVault.sol` | Deposit principal in a yield-bearing tokenized treasury wrapper; withdraw principal plus accrued yield. |
| `src/IStableFXSwap.sol` | `src/reference/FxEscrowAdapter.sol` | Atomic same-block stablecoin FX swap. EURC <-> USDC via StableFX FxEscrow. |
| `src/IAttributable.sol` | (marker) | The `bytes32 builder` attribution convention for third-party UI surfaces. |

The reference implementations are wired to Arc-testnet pre-deployed contracts:

- `CCTPReceiverMixin` targets `MessageTransmitterV2` at `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275`.
- `USYCBondVault` targets the USYC Teller at `0x9fdF14c5B14173D74C08Af27AebFf39240dC105A`.
- `FxEscrowAdapter` targets StableFX FxEscrow at `0x867650F5eAe8df91445971f14d89fd84F0C9a9f8`.

Consumer products that adopt the substrate:

- `reverbprotocol/markets`: `Operator.sol` adopts the `IAttributable` convention on every fill entry point.
- `damanfi/copy-bond`: declares conformance to `IBountyAccrual`, `IReputationRegistry`, and the `IAttributable` convention.

### HumdRegistry: immutable trust anchor + sidecar pattern

HumdRegistry is the immutable trust anchor for any hum subnet built on Reverb Protocol. The registry's identity layer never changes; extensions compose sideways. Daman's `ReputationRegistry` and `BountyAccrual` (in `damanfi/copy-bond`) are sidecars keyed by the same address space as HumdRegistry but storing their own state, separately upgradeable. The split is intentional: the integrity-critical write path (HumdRegistry's advertise + ownership rule) stays physically untouchable; the application-specific state (reputation, bounty, future annotations) lives in upgradeable sibling contracts. Pattern documented in https://github.com/adiled/hum/issues/39.

## Security posture

The deployed configuration on Arc testnet (`.deployments/arc-testnet.json`):

- **UUPS upgradeable.** `RefundProtocolFixed` is an `Initializable` + `UUPSUpgradeable` contract behind an ERC1967 proxy. Implementation deployed at the addresses in the deployments file; upgrade authority gated by `_authorizeUpgrade` on the owner.
- **TimelockController owns upgrade authority.** The proxy's `owner()` is a TimelockController with a 24-hour minimum delay. Every upgrade is `schedule()`'d, visible on-chain during the delay window, and `execute()`'d only after the delay elapses. Cancellable during the window.
- **Safe multisig fronts the TimelockController.** A 3-of-5 Safe is the sole proposer + executor on the TimelockController. The Safe address holds the testnet-posture signer roster; production rotation to independent signers is a documented mainnet pre-requisite.
- **Pausable critical paths.** `pay`, `refundByRecipient`, `refundByArbiter`, `withdraw`, `earlyWithdrawByArbiter` are gated by `whenNotPaused`. `pause()` is callable by the `pauser` address (Safe directly, no Timelock delay; emergency stop). `unpause()` is `onlyOwner` (Timelock-gated, 24h). `settleDebt`, `depositArbiterFunds`, `withdrawArbiterFunds`, `setLockupSeconds`, `updateRefundTo` remain live during pause so cleanup and admin paths are not halted.
- **Reentrancy.** `ReentrancyGuardTransient` (EIP-1153 transient storage; Arc Cancun config) on every state-mutating external function. CEI ordering verified across all fix paths.
- **Selector + event freeze tests.** `test/SelectorFreezeRefundProtocolFixed.t.sol` locks 12 external function selectors and 6 event topic hashes. Any change to a signature or event shape fails the freeze test before merge.
- **Stateful fuzz invariants.** `test/RefundProtocolInvariant.t.sol` runs 256 fuzz runs (128,000 calls per invariant) asserting the FIX-2 cumulative-withdraw bound and aggregate balance integrity. 0 reverts, 0 invariant failures.
- **Slither static analysis.** `.github/workflows/security.yml` runs Slither on every PR with `fail-on=high`. Mythril runs nightly via cron and uploads a markdown report as an artifact.
- **Production-deploy guidance** on each reference implementation in `src/reference/` so consumer products can replicate the same discipline.

## Fix summary

Four classes of correctness fix applied to the upstream contract, each marked inline with `FIX-{N}`:

- `FIX-1`: Checks-Effects-Interactions ordering on `_executeRefund`. Upstream transferred tokens before marking the payment refunded, exposing a reentrancy surface against any non-standard ERC-20 with a transfer hook.
- `FIX-2`: Cumulative over-withdraw guard on `earlyWithdrawByArbiter`. Upstream checked `withdrawalAmount > payment.amount`, which permitted multiple distinct-salt sessions to each withdraw up to the full amount, collectively draining past 100% of the original payment as long as the recipient's aggregate balance covered it.
- `FIX-3`: Debt-settle-before-early-withdraw. Upstream `withdraw()` settles outstanding debts before withdrawal; upstream `earlyWithdrawByArbiter` did not, so a recipient with outstanding debt could route around settlement by signing an early-withdraw request.
- `FIX-4`: Zero-recipient guard on `earlyWithdrawByArbiter`. Upstream allowed an arbiter-crafted call where `ecrecover`-degenerate signatures could collide with `address(0)`.

## Build

```bash
forge install foundry-rs/forge-std --no-commit
forge install OpenZeppelin/openzeppelin-contracts --no-commit
forge build
forge test -vv
```

## Consumers

- `reverbprotocol/markets` consumes this library as a Foundry dependency for prediction-market settlement and dispute resolution.
- `damanfi/copy-bond` consumes this library as a Foundry dependency for slash-bond dispute routing.

Other parties may deploy their own conformant implementations of `IRefundProtocol` with different arbiter selection, fee schedules, or domain-separator strings.

## License

Apache-2.0. Upstream copyright Circle Internet Group, Inc. preserved on the vendored contract and the fix.
