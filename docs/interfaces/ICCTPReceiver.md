# ICCTPReceiver

Interface for consumer contracts that receive CCTP v2 messages on Arc and dispatch a consumer-supplied payload after USDC is minted.

## Source

[`src/ICCTPReceiver.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/ICCTPReceiver.sol) and [`src/CCTPReceiverMixin.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/CCTPReceiverMixin.sol)

## External function

```solidity
function onCCTPReceive(bytes calldata message, bytes calldata attestation) external;
```

A relayer (or any external caller) submits the CCTP outer message and attestation; the receiver invokes `MessageTransmitterV2.receiveMessage` and then dispatches the decoded payload through the implementation's hook.

## Reference implementation

`CCTPReceiverMixin` is the abstract reference. Inherit and implement `handlePayload`:

```solidity
function handlePayload(bytes calldata payload, uint256 mintedAmount) internal virtual;
```

The mixin handles the `MessageTransmitterV2.receiveMessage` call, measures the USDC balance delta as `mintedAmount`, and extracts the consumer payload from the CCTP v2 burn-message layout (148-byte outer header + 228-byte burn-message prefix = 376 bytes of standard fields, then `hookData`). Override `_decodePayload` for non-standard encodings.

## ERC-7201 namespaced storage

The mixin uses ERC-7201 namespaced storage at slot

```
0x009c3710a3eb8a7e5b03d4342e58f75d7df000a9a29f59f713993589837e6200
```

so the mixin's slots never collide with the inheriting contract's storage layout. This makes the mixin safely composable with upgrade-safe inheriting contracts: inheriting contracts call `__CCTPReceiver_init(messageTransmitter, usdc)` from their own initialize function.

## Conformance

Any consumer contract that exposes `onCCTPReceive(bytes,bytes)` with the documented semantics (validate message via MessageTransmitterV2, measure USDC delta, dispatch a consumer payload) is conformant. The mixin is one way; a custom integration that reads `MessageTransmitterV2` directly is another.

## Arc testnet pre-deployed surface

| Surface | Address |
|---|---|
| MessageTransmitterV2 | `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275` |
| USDC | `0x3600000000000000000000000000000000000000` |
| TokenMessengerV2 (CCTP burn on origin) | `0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA` |

Reference: [`developers.circle.com/cctp/quickstarts/transfer-usdc-ethereum-to-arc`](https://developers.circle.com/cctp/quickstarts/transfer-usdc-ethereum-to-arc).
