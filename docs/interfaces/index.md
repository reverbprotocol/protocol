# Substrate primitives

Six interfaces ship with reference implementations under [`src/reference/`](https://github.com/reverbprotocol/protocol/tree/main/src/reference). Each interface is the cross-consumer standard; the reference implementations are one valid impl. Consumer products may ship their own conformant implementations with product-specific economics.

## Interfaces

| Interface | Reference | Purpose |
|---|---|---|
| [IRefundProtocol](/interfaces/IRefundProtocol) | `RefundProtocolFixed` | Dispute-mediated escrow for stablecoin commerce. |
| [IBountyAccrual](/interfaces/IBountyAccrual) | `BountyAccrualVanilla` | Funder accrues a claim; recipient claims later. |
| [IReputationRegistry](/interfaces/IReputationRegistry) | `ReputationRegistryVanilla` | Cumulative reputation scoring; designated recorders. |
| [ICCTPReceiver](/interfaces/ICCTPReceiver) | `CCTPReceiverMixin` (abstract) | Receive CCTP v2, mint USDC, dispatch a consumer payload. |
| [IBondYieldVault](/interfaces/IBondYieldVault) | `USYCBondVault` | Yield-bearing principal vault; strict + aggregated subscription policies. |
| [IStableFXSwap](/interfaces/IStableFXSwap) | `FxEscrowAdapter` | Atomic same-block stablecoin FX swap with explicit minOut. |
| [IAttributable](/interfaces/IAttributable) | (marker) | The `bytes32 builder` attribution convention. |
