# Substrate primitives — overview

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

## Marker

| Interface | Purpose |
|---|---|
| [IAttributable](/interfaces/IAttributable) | The `bytes32 builder` attribution convention for third-party UI surfaces. Marker only; no required methods. |

## Reference implementations are not for production deploy

The reference impls under `src/reference/` are shipped for direct fork-and-adapt. Each carries a NatSpec block clarifying production-deploy guidance: wrap behind an ERC1967 proxy with UUPS-style upgrade controls, mix in `PausableUpgradeable` on state-mutating entry points, route owner authority through a `TimelockController` fronted by a Safe multisig, and run the contract through static analysis and storage-layout CI before any chain-side deploy.

The exception is `RefundProtocolFixed` itself, which IS the production-deploy artifact and is itself UUPS upgradeable with the full production controls described in the [Security posture](/security-posture).

## Arc testnet pre-deployed surface used by the references

| Surface | Address |
|---|---|
| USDC (native gas, 18 decimals) | `0x3600000000000000000000000000000000000000` |
| EURC | `0x89B50855Aa3bE2F677cD6303Cec089B5F319D72a` |
| USYC | `0xe9185F0c5F296Ed1797AaE4238D26CCaBEadb86C` |
| USYC Teller | `0x9fdF14c5B14173D74C08Af27AebFf39240dC105A` |
| TokenMessengerV2 (CCTP burn) | `0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA` |
| MessageTransmitterV2 (CCTP receive) | `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275` |
| FxEscrow (StableFX) | `0x867650F5eAe8df91445971f14d89fd84F0C9a9f8` |
