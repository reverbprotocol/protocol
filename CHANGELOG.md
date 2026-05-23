# Changelog

## 2026-05-23 — Extracted

Extracted as a standalone substrate library from `project-reverb/apps/dispute-escrow/`. The Operator contract and the prediction-market consumer surface moved to `reverbprotocol/markets`. This repository holds the dispute primitive only.

Interface `IRefundProtocol` introduced; `RefundProtocolFixed` declares `is IRefundProtocol`. No semantic changes to the fixed contract.

## 0.1.0 — RefundProtocolFixed initial fork

Forked from `circlefin/refund-protocol@b506b17` (`src/RefundProtocol.sol`). Four classes of correctness fix; constructor and storage layout preserved so a deployment can stand in for the upstream where calling code expects upstream's interface.

### Fixes

**FIX-1: Checks-Effects-Interactions on `_executeRefund`.** Upstream transferred tokens to `refundTo` before marking `payments[id].refunded = true`. A non-standard ERC-20 with a transfer hook (or a refundTo contract receiving via fallback in a future variant) could re-enter `refundByRecipient` or `refundByArbiter` and observe `refunded == false`, allowing a second `_executeRefund` against the same payment. The fix moves the state assignment ahead of the `safeTransfer`. `nonReentrant` modifier added on every external state-changing entry point as a defense-in-depth backstop.

**FIX-2: Cumulative over-withdraw guard on `earlyWithdrawByArbiter`.** Upstream's per-iteration check was `withdrawalAmount > payment.amount`, comparing against the original payment notional rather than the remaining un-withdrawn balance. With two distinct-salt EIP-712 sessions a recipient could sign two early-withdrawals of 90 each against a 100-amount payment, and as long as the recipient's aggregate `balances` covered the 180 outflow (e.g. they had a second payment from another sender), the contract paid out 180 against a 100 payment. The fix compares against `payment.amount - payment.withdrawnAmount`. The vendored proof (`test_FIX2_upstream_drainsPastFullPaymentAmount`) demonstrates the upstream drain; the fixed contract reverts `InvalidWithdrawalAmount` on the second session.

**FIX-3: Debt settlement before `earlyWithdrawByArbiter`.** Upstream's `withdraw()` opens with `_settleDebt(msg.sender)`; `earlyWithdrawByArbiter` did not. A recipient with outstanding `debts[recipient]` could route around the settlement path by signing an early-withdraw request, draining a fresh inbound payment to themselves while the debt sat untouched. The fix calls `_settleDebt(recipient)` after signature/replay checks and before the per-payment loop. The vendored proof (`test_FIX3_upstream_earlyWithdrawBypassesDebtSettlement`) shows the upstream contract leaving `debts[receiver]` at 100 after a 100 early-withdraw; the fixed contract drains debt first, then early-withdraws against the post-settlement balance.

**FIX-4: Zero-recipient guard on `earlyWithdrawByArbiter`.** Upstream did not reject `recipient == address(0)`. Combined with degenerate signature parameters that can ecrecover to `address(0)` (e.g. malformed `v`/`r`/`s`), an arbiter could pass the signature check by setting `recipient = address(0)` and crash funds into the zero address with no recipient consent. The fix reverts `RecipientIsZeroAddress` before any other work. Constructor also rejects zero `_arbiter` and `_fiatToken`, and `pay()` rejects zero `to`.

### Other changes

- `SafeERC20` wrapping on every `IERC20` call. Upstream's bare `transfer` and `transferFrom` ignore non-conforming ERC-20 return values; the wrapping reverts on `false` returns and on non-empty unexpected returndata.
- `arbiter` and `fiatToken` are `immutable`. Upstream stored them in regular storage even though they were never mutated.
- `withdrawalHashes[h] = true` is set before the external transfer in `earlyWithdrawByArbiter` (CEI; sibling to FIX-1).
- `_settleDebt` early-returns when there is no debt or no balance to settle, saving a write per call. Emits `DebtSettled` so off-chain agents can attribute settlement events without re-deriving from balance deltas.
- `pay()` increments `nonce` in an `unchecked` block. Saves ~30 gas per payment; safe because a 256-bit counter cannot overflow under any deployment-realistic call rate.

### Compatibility

External function signatures, event topics, and `EARLY_WITHDRAWAL_TYPEHASH` are unchanged. A caller built against upstream interacts identically with the fixed contract, with the additional behaviors that:

- Calls that would have over-withdrawn under FIX-2 now revert.
- Calls that would have bypassed debt under FIX-3 now settle debt first; if remaining balance is then insufficient, the early-withdraw reverts.
- `address(0)` parameters revert at the entry point rather than silently propagating.

The `DebtSettled` and `WithdrawalFeePaid` events are additive; downstream indexers built against the upstream event set continue to work without modification.
