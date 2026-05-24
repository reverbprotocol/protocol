# CHANGELOG

Canonical changelog at [`CHANGELOG.md`](https://github.com/reverbprotocol/protocol/blob/main/CHANGELOG.md) in the repository root.

## 2026-05-24 — Arc-testnet deploy + Security posture

UUPS upgradeable refactor + atomic Arc-testnet deploy. The contract surface (selectors + events) is unchanged; the deployment architecture is new.

### RefundProtocolFixed refactor

`Initializable` + `UUPSUpgradeable` + `OwnableUpgradeable` + `PausableUpgradeable` + `EIP712Upgradeable`. `ReentrancyGuardTransient` for nonReentrant on every state-mutating external function (EIP-1153 transient storage; Arc Cancun config).

- Constructor replaced by `initialize(arbiter, fiatToken, eip712Name, eip712Version, owner, pauser)`; gated by the `initializer` modifier; constructor calls `_disableInitializers()`.
- `arbiter` and `fiatToken` converted from `immutable` to regular storage for proxy compatibility; storage layout preserved with `pauser` appended and `__gap[49]` reserving 49 slots for future additions.
- Owner (TimelockController in production) gates `_authorizeUpgrade` and `unpause()`. Pauser (Safe multisig) gates `pause()` with no delay.
- `whenNotPaused` on `pay`, `refundByRecipient`, `refundByArbiter`, `withdraw`, `earlyWithdrawByArbiter`. `settleDebt`, `depositArbiterFunds`, `withdrawArbiterFunds`, `setLockupSeconds`, `updateRefundTo` remain live during pause.
- `setPauser(newPauser)` onlyOwner for pauser rotation through the Timelock.

### CCTPReceiverMixin refactor

Upgrade-safe abstract reference. Storage namespaced per ERC-7201 at slot `0x009c3710a3eb8a7e5b03d4342e58f75d7df000a9a29f59f713993589837e6200` so the mixin's slots never collide with the inheriting contract's storage layout. `messageTransmitter` and `usdc` moved from immutables to namespaced storage with public view accessors. Constructor replaced by internal `__CCTPReceiver_init(mt, usdc)`.

### Deploy

Atomic single-broadcast script (`script/Deploy.s.sol`) deploys the implementation, deploys an ERC1967Proxy, calls `initialize` with the Safe + TimelockController addresses, asserts on-chain that `proxy.owner()` equals the TimelockController, and exits.

Live on Arc testnet:
- Proxy: `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F`
- Implementation: `0xc4d76141CEA6b8D4b1bBF467e92d6a87F46C53Ed`
- Owner (Timelock): `0xa22510860289751C092e67B15b827020CE09DAbf`
- Pauser (Safe 3-of-5): `0x70a34ca4964a16a934432871a593acba5dd63cf1`

### CI

`.github/workflows/security.yml` runs Slither on every PR with `fail-on=high`. Mythril runs nightly via cron at 03:17 UTC and uploads a markdown report as an artifact.

### Tests

Two new stateful fuzz invariants (`invariant_noOverWithdraw`, `invariant_aggregateBalanceBoundedByTokenBalance`) over 256 fuzz runs × 128,000 calls per invariant. 18 new selector + event freeze tests. Suite: 53 of 53 green.

## 2026-05-24 — Substrate primitives

Six new interfaces with reference implementations under `src/reference/`. Each interface defines a cross-consumer standard; the reference impls are one valid implementation each, suitable for direct use or as a starting point for consumer-specific variants. See [Substrate primitives](/interfaces/).

## 2026-05-23 — Extracted

Extracted as a standalone substrate library from project-reverb. The Operator contract and the prediction-market consumer surface moved to [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets). This repository holds the dispute primitive only.

Interface `IRefundProtocol` introduced; `RefundProtocolFixed` declares `is IRefundProtocol`. No semantic changes to the fixed contract at extraction.

## 0.1.0 — RefundProtocolFixed initial fork

Forked from `circlefin/refund-protocol@b506b17`. Four classes of correctness fix per the [IRefundProtocol page](/interfaces/IRefundProtocol#four-classes-of-fix).
