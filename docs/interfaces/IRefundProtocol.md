# IRefundProtocol

External interface for the refund-protocol primitive. The vanilla implementation is `RefundProtocolFixed`, a forked-and-fixed version of `circlefin/refund-protocol@b506b17`.

## Purpose

A dispute-mediated escrow for stablecoin commerce. A payer escrows funds toward a recipient; the recipient may withdraw after a lockup window; an arbiter may refund or early-withdraw against signed authorizations.

## Source

[`src/IRefundProtocol.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IRefundProtocol.sol)

## External functions

```solidity
function DOMAIN_SEPARATOR() external view returns (bytes32);

function pay(address to, uint256 amount, address refundTo) external;
function refundByRecipient(uint256 paymentID) external;
function refundByArbiter(uint256 paymentID) external;
function settleDebt(address recipient) external;
function depositArbiterFunds(uint256 amount) external;
function withdrawArbiterFunds(uint256 amount) external;
function setLockupSeconds(address recipient, uint256 recipientLockupSeconds) external;
function withdraw(uint256[] calldata paymentIDs) external;
function earlyWithdrawByArbiter(
    uint256[] calldata paymentIDs,
    uint256[] calldata withdrawalAmounts,
    uint256 feeAmount,
    uint256 expiry,
    uint256 salt,
    address recipient,
    uint8 v,
    bytes32 r,
    bytes32 s
) external;

function updateRefundTo(uint256 paymentID, address newRefundTo) external;
function hashEarlyWithdrawalInfo(
    uint256[] calldata paymentIDs,
    uint256[] calldata withdrawalAmounts,
    uint256 feeAmount,
    uint256 expiry,
    uint256 salt
) external view returns (bytes32);
```

## Reference implementation

`RefundProtocolFixed` ([`src/RefundProtocolFixed.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/RefundProtocolFixed.sol)) is the deployed vanilla impl. UUPS upgradeable, Pausable, Ownable; production controls per the [Security posture](/security-posture).

## Four classes of fix

The fix dossier against upstream `circlefin/refund-protocol@b506b17`:

- **FIX-1: CEI on `_executeRefund`.** Upstream transferred tokens before marking the payment refunded, exposing a reentrancy surface against any non-standard ERC-20 with a transfer hook. The fix moves the state assignment ahead of `safeTransfer`.
- **FIX-2: Cumulative over-withdraw guard on `earlyWithdrawByArbiter`.** Upstream's per-iteration check compared against the original payment notional rather than the remaining un-withdrawn balance. With two distinct-salt EIP-712 sessions, a recipient could drain past 100% of the original payment. The fix compares against `payment.amount - payment.withdrawnAmount`. Locked under stateful fuzz as `invariant_noOverWithdraw`.
- **FIX-3: Debt settle before early-withdraw.** Upstream's `withdraw()` settles outstanding debts before withdrawal; upstream `earlyWithdrawByArbiter` did not. The fix calls `_settleDebt(recipient)` after signature checks and before the per-payment loop.
- **FIX-4: Zero-recipient guard on `earlyWithdrawByArbiter`.** Upstream did not reject `recipient == address(0)`. Degenerate signatures could collide with `address(0)`. The fix reverts `RecipientIsZeroAddress` before any other work.

See [CHANGELOG](/changelog) for the bug-by-bug walkthrough.

## Conformance

Any contract that exposes the function signatures listed above MAY declare `is IRefundProtocol` and serve as a drop-in for `RefundProtocolFixed`. Consumers should treat the interface as the stable surface and the concrete implementation address as a runtime-configured detail.
