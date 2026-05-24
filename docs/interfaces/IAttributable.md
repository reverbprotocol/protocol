# IAttributable

Marker interface for the `bytes32 builder` attribution convention.

## Source

[`src/IAttributable.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IAttributable.sol)

## The convention

1. Every external entry point on a consumer contract that may be invoked through a third-party UI or signing surface carries an explicit `bytes32 builder` argument.
2. Every related event emitted from such an entry point includes a `bytes32 indexed builder` field so the attribution is queryable from any indexer.

The interface itself declares no methods; conformance is asserted by declaring `is IAttributable` on the consumer contract and adopting the argument-and-event convention across its external surface. This marker provides a stable on-chain handle for tooling that wants to filter for attribution-aware contracts.

## Lineage

- The `bytes32` attribution shape is documented in the Polymarket trading reference at [`docs.polymarket.com/trading/orders/attribution`](https://docs.polymarket.com/trading/orders/attribution).
- The rationale ("structured outputs travel with the order, so the identity attributable to a fill is verifiable on chain rather than asserted off chain") is articulated in Canteen's 2026-05-01 essay [Unbundling the Prediction Market Stack](https://thecanteenapp.com/analysis/2026/05/01/unbundling-the-prediction-market-stack.html).

## Adopters

| Repo | Surface |
|---|---|
| [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets) | `Operator.sol` carries `bytes32 builder` on every fill entry point and emits attribution on every `OrderFilled` and `BuilderFeeAccrued` event. |
| [`damanfi/copy-bond`](https://github.com/damanfi/copy-bond) | Declares conformance and adopts the convention on its slash-claim and bond-post surfaces. |

Consumer products in other repositories may declare conformance and adopt the convention on their own external surfaces.
