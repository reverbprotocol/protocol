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
