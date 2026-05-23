// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb (interface)
 *
 * Licensed under the Apache License, Version 2.0.
 *
 * External interface for the refund-protocol primitive. RefundProtocolFixed is
 * the vanilla implementation; other parties may ship conformant implementations.
 */

pragma solidity ^0.8.24;

interface IRefundProtocol {
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
}
