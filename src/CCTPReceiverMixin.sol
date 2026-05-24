// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {ICCTPReceiver, IMessageTransmitterV2} from "./ICCTPReceiver.sol";

/// @title CCTPReceiverMixin
/// @notice Abstract reference for consumer contracts that receive CCTP v2 messages on Arc, mint
///         USDC, and dispatch a consumer-supplied payload to the inheriting contract.
/// @dev    Inherit and implement `handlePayload`. The mixin measures the USDC balance delta
///         across the `MessageTransmitterV2.receiveMessage` call and passes the minted amount
///         to the consumer hook.
///
///         Default `_decodePayload` assumes the message body follows the CCTP v2 burn-message
///         layout (`developers.circle.com/cctp/quickstarts/transfer-usdc-ethereum-to-arc`):
///         148 bytes of outer message header + 228 bytes of burn-message fixed fields = 376
///         bytes of standard prefix, then variable `hookData`. Override `_decodePayload` if your
///         relayer uses a different encoding.
///
///         Abstract reference. The mixin adds no governance surface; it only dispatches CCTP
///         receives to a consumer hook. The inheriting contract is responsible for the
///         production-deploy controls: wrap behind an ERC1967 proxy with UUPS-style upgrade
///         controls, mix in `PausableUpgradeable` on `onCCTPReceive`, route owner authority
///         through a `TimelockController` fronted by a Safe multisig, and run the inheriting
///         contract through static analysis and storage-layout CI before any chain-side deploy.
abstract contract CCTPReceiverMixin is ICCTPReceiver {
    /// @notice MessageTransmitterV2 on the destination chain (Arc testnet: 0xE737...DC275).
    IMessageTransmitterV2 public immutable messageTransmitter;

    /// @notice USDC on the destination chain (Arc testnet: 0x36...0000).
    IERC20 public immutable usdc;

    /// @notice Offset in `message` bytes at which the consumer payload begins. Standard CCTP v2
    ///         layout is 376 (148 outer header + 228 burn-message fixed prefix).
    uint256 public constant CCTP_V2_HOOK_OFFSET = 376;

    error CCTPReceiveFailed();
    error ZeroAddress();

    constructor(address _messageTransmitter, address _usdc) {
        if (_messageTransmitter == address(0) || _usdc == address(0)) revert ZeroAddress();
        messageTransmitter = IMessageTransmitterV2(_messageTransmitter);
        usdc = IERC20(_usdc);
    }

    /// @inheritdoc ICCTPReceiver
    function onCCTPReceive(bytes calldata message, bytes calldata attestation) external override {
        uint256 balanceBefore = usdc.balanceOf(address(this));
        bool ok = messageTransmitter.receiveMessage(message, attestation);
        if (!ok) revert CCTPReceiveFailed();
        uint256 mintedAmount = usdc.balanceOf(address(this)) - balanceBefore;

        bytes calldata payload = _decodePayload(message);
        handlePayload(payload, mintedAmount);
    }

    /// @notice Decode the consumer payload slice out of the CCTP v2 message. Override if your
    ///         relayer uses a non-standard encoding.
    /// @param  message The full CCTP outer message bytes.
    /// @return payload The consumer-supplied portion of the message.
    function _decodePayload(bytes calldata message) internal pure virtual returns (bytes calldata payload) {
        if (message.length <= CCTP_V2_HOOK_OFFSET) {
            return message[message.length:];
        }
        return message[CCTP_V2_HOOK_OFFSET:];
    }

    /// @notice Consumer hook. Called after USDC has been minted to this contract.
    /// @param payload      The consumer-supplied payload extracted from the CCTP message.
    /// @param mintedAmount The USDC amount minted to this contract during the receive.
    function handlePayload(bytes calldata payload, uint256 mintedAmount) internal virtual;
}
