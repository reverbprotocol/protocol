// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {ICCTPReceiver, IMessageTransmitterV2} from "./ICCTPReceiver.sol";

/// @title CCTPReceiverMixin
/// @notice Upgrade-safe abstract reference for consumer contracts that receive CCTP v2 messages
///         on Arc, mint USDC, and dispatch a consumer-supplied payload to the inheriting
///         contract. Inherit and implement `handlePayload`.
///
/// @dev    Storage is namespaced per ERC-7201 so the mixin's slots never collide with the
///         inheriting contract's storage layout. Inheriting contracts call
///         `__CCTPReceiver_init(messageTransmitter, usdc)` from their own `initialize` function.
///         The mixin itself is constructor-free; the inheriting contract is responsible for
///         calling `_disableInitializers()` in its constructor and gating its `initialize` with
///         the `initializer` modifier.
///
///         Default `_decodePayload` assumes the CCTP v2 burn-message layout: 148-byte outer
///         message header + 228-byte burn-message fixed fields = 376 bytes of standard prefix,
///         then variable `hookData`. Override if your relayer uses a different encoding.
///
///         Reference, not for production deploy. The mixin adds no governance surface; it only
///         dispatches CCTP receives to a consumer hook. The inheriting contract is responsible
///         for the production-deploy controls: wrap behind an ERC1967 proxy with UUPS-style
///         upgrade controls, mix in `PausableUpgradeable` on `onCCTPReceive`, route owner
///         authority through a `TimelockController` fronted by a Safe multisig, and run the
///         inheriting contract through static analysis and storage-layout CI before any
///         chain-side deploy.
abstract contract CCTPReceiverMixin is ICCTPReceiver {
    /// @notice Offset in `message` bytes at which the consumer payload begins. Standard CCTP v2
    ///         layout is 376 (148 outer header + 228 burn-message fixed prefix).
    uint256 public constant CCTP_V2_HOOK_OFFSET = 376;

    /// @custom:storage-location erc7201:reverbprotocol.storage.CCTPReceiverMixin
    struct CCTPReceiverStorage {
        IMessageTransmitterV2 messageTransmitter;
        IERC20 usdc;
    }

    /// @dev keccak256(abi.encode(uint256(keccak256("reverbprotocol.storage.CCTPReceiverMixin")) - 1)) & ~bytes32(uint256(0xff))
    bytes32 private constant CCTP_RECEIVER_STORAGE_LOCATION =
        0x009c3710a3eb8a7e5b03d4342e58f75d7df000a9a29f59f713993589837e6200;

    function _getCCTPReceiverStorage() private pure returns (CCTPReceiverStorage storage $) {
        assembly {
            $.slot := CCTP_RECEIVER_STORAGE_LOCATION
        }
    }

    error CCTPReceiveFailed();
    error ZeroAddress();

    /// @notice Initialize the mixin's storage. Called once by the inheriting contract from its
    ///         own `initialize` function.
    function __CCTPReceiver_init(address _messageTransmitter, address _usdc) internal {
        if (_messageTransmitter == address(0) || _usdc == address(0)) revert ZeroAddress();
        CCTPReceiverStorage storage $ = _getCCTPReceiverStorage();
        $.messageTransmitter = IMessageTransmitterV2(_messageTransmitter);
        $.usdc = IERC20(_usdc);
    }

    /// @notice MessageTransmitterV2 on the destination chain (Arc testnet: 0xE737...DC275).
    function messageTransmitter() public view returns (IMessageTransmitterV2) {
        return _getCCTPReceiverStorage().messageTransmitter;
    }

    /// @notice USDC on the destination chain (Arc testnet: 0x36...0000).
    function usdc() public view returns (IERC20) {
        return _getCCTPReceiverStorage().usdc;
    }

    /// @inheritdoc ICCTPReceiver
    function onCCTPReceive(bytes calldata message, bytes calldata attestation) external virtual override {
        CCTPReceiverStorage storage $ = _getCCTPReceiverStorage();
        uint256 balanceBefore = $.usdc.balanceOf(address(this));
        bool ok = $.messageTransmitter.receiveMessage(message, attestation);
        if (!ok) revert CCTPReceiveFailed();
        uint256 mintedAmount = $.usdc.balanceOf(address(this)) - balanceBefore;

        bytes calldata payload = _decodePayload(message);
        handlePayload(payload, mintedAmount);
    }

    /// @notice Decode the consumer payload slice out of the CCTP v2 message. Override if your
    ///         relayer uses a non-standard encoding.
    function _decodePayload(bytes calldata message) internal pure virtual returns (bytes calldata payload) {
        if (message.length <= CCTP_V2_HOOK_OFFSET) {
            return message[message.length:];
        }
        return message[CCTP_V2_HOOK_OFFSET:];
    }

    /// @notice Consumer hook. Called after USDC has been minted to this contract.
    function handlePayload(bytes calldata payload, uint256 mintedAmount) internal virtual;
}
