// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

/// @title ICCTPReceiver
/// @notice Interface for consumer contracts that receive CCTP v2 messages on Arc and dispatch a
///         consumer-supplied payload after USDC is minted.
/// @dev    The standard entry point. A relayer (or any external caller) submits the message and
///         attestation; the receiver invokes MessageTransmitterV2.receiveMessage and then
///         dispatches the decoded payload through the implementation's hook.
interface ICCTPReceiver {
    /// @notice Process an attested CCTP v2 message, mint USDC, and dispatch the consumer payload.
    /// @param message     The full CCTP outer message bytes.
    /// @param attestation The attestation bytes from Circle's iris-attestation service.
    function onCCTPReceive(bytes calldata message, bytes calldata attestation) external;
}

/// @title IMessageTransmitterV2
/// @notice Minimal interface against Circle's MessageTransmitterV2 on the destination chain.
/// @dev    The on-chain Arc-testnet deployment is at
///         `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275`.
///         Reference: `developers.circle.com/cctp/quickstarts/transfer-usdc-ethereum-to-arc`.
interface IMessageTransmitterV2 {
    function receiveMessage(bytes calldata message, bytes calldata attestation) external returns (bool);
}
