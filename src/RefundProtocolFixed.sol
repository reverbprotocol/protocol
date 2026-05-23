// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2025 Circle Internet Group, Inc. (upstream)
 * Copyright 2026 project-reverb (fixes)
 *
 * Licensed under the Apache License, Version 2.0.
 *
 * Forked from circlefin/refund-protocol@b506b17 (src/RefundProtocol.sol).
 * Four classes of fix applied; see CHANGELOG.md and the inline FIX-{N} markers.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import {IRefundProtocol} from "./IRefundProtocol.sol";

contract RefundProtocolFixed is EIP712, ReentrancyGuard, IRefundProtocol {
    using SafeERC20 for IERC20;

    struct Payment {
        address to;
        uint256 amount;
        uint256 releaseTimestamp;
        address refundTo;
        uint256 withdrawnAmount;
        bool refunded;
    }

    uint256 public constant MAX_LOCKUP_SECONDS = 60 * 60 * 24 * 180;
    bytes32 public constant EARLY_WITHDRAWAL_TYPEHASH = keccak256(
        "EarlyWithdrawalByArbiter(uint256[] paymentIDs,uint256[] withdrawalAmounts,uint256 feeAmount,uint256 expiry,uint256 salt)"
    );

    IERC20 public immutable fiatToken;
    address public immutable arbiter;
    uint256 public nonce;
    mapping(address => uint256) public lockupSeconds;
    mapping(uint256 => Payment) public payments;
    mapping(address => uint256) public balances;
    mapping(address => uint256) public debts;
    mapping(bytes32 => bool) public withdrawalHashes;

    event PaymentCreated(
        uint256 indexed paymentID,
        address indexed to,
        uint256 amount,
        uint256 releaseTimestamp,
        address indexed refundTo
    );
    event Refund(uint256 indexed paymentID, address indexed refundTo, uint256 amount);
    event RefundToUpdated(uint256 indexed paymentID, address indexed oldRefundTo, address indexed newRefundTo);
    event Withdrawal(address indexed to, uint256 amount);
    event WithdrawalFeePaid(address indexed recipient, uint256 amount);
    event DebtSettled(address indexed recipient, uint256 amount);

    error CallerNotAllowed();
    error PaymentIsStillLocked(uint256 paymentID);
    error PaymentDoesNotBelongToRecipient();
    error RefundToIsZeroAddress();
    error RecipientIsZeroAddress();
    error InsufficientFunds();
    error InvalidWithdrawalAmount(uint256 paymentID, uint256 withdrawalAmount);
    error InvalidFeeAmount();
    error InvalidSignature();
    error WithdrawalHashAlreadyUsed();
    error WithdrawalHashExpired();
    error PaymentRefunded(uint256 paymentID);
    error LockupSecondsExceedsMax();
    error MismatchedEarlyWithdrawalArrays();
    error ZeroAddress();

    constructor(address _arbiter, address _fiatToken, string memory eip712Name, string memory eip712Version)
        EIP712(eip712Name, eip712Version)
    {
        if (_arbiter == address(0) || _fiatToken == address(0)) revert ZeroAddress();
        arbiter = _arbiter;
        fiatToken = IERC20(_fiatToken);
    }

    modifier onlyArbiter() {
        if (msg.sender != arbiter) revert CallerNotAllowed();
        _;
    }

    // solhint-disable-next-line func-name-mixedcase
    function DOMAIN_SEPARATOR() external view override returns (bytes32) {
        return _domainSeparatorV4();
    }

    function pay(address to, uint256 amount, address refundTo) external override nonReentrant {
        if (refundTo == address(0)) revert RefundToIsZeroAddress();
        if (to == address(0)) revert RecipientIsZeroAddress();

        uint256 lockup = lockupSeconds[to];
        uint256 releaseAt = block.timestamp + lockup;

        fiatToken.safeTransferFrom(msg.sender, address(this), amount);

        uint256 paymentID = nonce;
        payments[paymentID] = Payment(to, amount, releaseAt, refundTo, 0, false);
        balances[to] += amount;
        unchecked { nonce = paymentID + 1; }

        emit PaymentCreated(paymentID, to, amount, releaseAt, refundTo);
    }

    function refundByRecipient(uint256 paymentID) external override nonReentrant {
        Payment memory payment = payments[paymentID];
        if (msg.sender != payment.to) revert CallerNotAllowed();

        uint256 recipientBalance = balances[payment.to];
        if (payment.amount > recipientBalance) revert InsufficientFunds();

        balances[payment.to] = recipientBalance - payment.amount;
        _executeRefund(paymentID, payment);
    }

    function refundByArbiter(uint256 paymentID) external override onlyArbiter nonReentrant {
        Payment memory payment = payments[paymentID];
        uint256 recipientBalance = balances[payment.to];

        if (payment.amount <= recipientBalance) {
            balances[payment.to] = recipientBalance - payment.amount;
            _executeRefund(paymentID, payment);
            return;
        }

        uint256 arbiterBalance = balances[arbiter];
        if (payment.amount > arbiterBalance) revert InsufficientFunds();

        balances[arbiter] = arbiterBalance - payment.amount;
        debts[payment.to] += payment.amount;

        _executeRefund(paymentID, payment);
    }

    function settleDebt(address recipient) external override {
        _settleDebt(recipient);
    }

    function depositArbiterFunds(uint256 amount) external override onlyArbiter nonReentrant {
        fiatToken.safeTransferFrom(msg.sender, address(this), amount);
        balances[arbiter] += amount;
    }

    function withdrawArbiterFunds(uint256 amount) external override onlyArbiter nonReentrant {
        uint256 arbiterBalance = balances[arbiter];
        if (amount > arbiterBalance) revert InsufficientFunds();

        balances[arbiter] = arbiterBalance - amount;
        fiatToken.safeTransfer(arbiter, amount);
    }

    function setLockupSeconds(address recipient, uint256 recipientLockupSeconds) external override onlyArbiter {
        if (recipientLockupSeconds > MAX_LOCKUP_SECONDS) revert LockupSecondsExceedsMax();
        lockupSeconds[recipient] = recipientLockupSeconds;
    }

    function withdraw(uint256[] calldata paymentIDs) external override nonReentrant {
        _settleDebt(msg.sender);

        uint256 totalAmount = 0;
        for (uint256 i = 0; i < paymentIDs.length; ++i) {
            uint256 paymentID = paymentIDs[i];
            Payment memory payment = payments[paymentID];
            if (payment.to != msg.sender) revert CallerNotAllowed();
            if (block.timestamp < payment.releaseTimestamp) revert PaymentIsStillLocked(paymentID);
            if (payment.refunded) revert PaymentRefunded(paymentID);

            totalAmount += payment.amount - payment.withdrawnAmount;
            payments[paymentID].withdrawnAmount = payment.amount;
        }

        uint256 recipientBalance = balances[msg.sender];
        if (totalAmount > recipientBalance) revert InsufficientFunds();
        balances[msg.sender] = recipientBalance - totalAmount;

        fiatToken.safeTransfer(msg.sender, totalAmount);
        emit Withdrawal(msg.sender, totalAmount);
    }

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
    ) external override onlyArbiter nonReentrant {
        // FIX-4: reject zero-recipient explicitly. Upstream allowed an arbiter-crafted call
        // where ecrecover-degenerate signatures could collide with address(0).
        if (recipient == address(0)) revert RecipientIsZeroAddress();

        if (paymentIDs.length != withdrawalAmounts.length) revert MismatchedEarlyWithdrawalArrays();

        bytes32 withdrawalInfoHash = _hashEarlyWithdrawalInfo(paymentIDs, withdrawalAmounts, feeAmount, expiry, salt);
        if (withdrawalHashes[withdrawalInfoHash]) revert WithdrawalHashAlreadyUsed();
        if (block.timestamp > expiry) revert WithdrawalHashExpired();
        if (ecrecover(withdrawalInfoHash, v, r, s) != recipient) revert InvalidSignature();

        // FIX-3: settle outstanding debts before early-withdrawal. Upstream `withdraw()` does this;
        // upstream `earlyWithdrawByArbiter` did not, so a recipient with debt could route around
        // settlement by signing an early-withdraw request.
        _settleDebt(recipient);

        uint256 totalAmount = 0;
        for (uint256 i = 0; i < paymentIDs.length; ++i) {
            uint256 paymentID = paymentIDs[i];
            uint256 withdrawalAmount = withdrawalAmounts[i];

            Payment memory payment = payments[paymentID];

            if (payment.to != recipient) revert PaymentDoesNotBelongToRecipient();
            if (payment.refunded) revert PaymentRefunded(paymentID);

            // FIX-2: cumulative over-withdraw guard. Upstream checked `withdrawalAmount > payment.amount`,
            // which permitted multiple distinct-salt sessions to each withdraw up to the full amount,
            // collectively draining past 100% of the original payment as long as the recipient's
            // aggregate balance covered it.
            uint256 remaining = payment.amount - payment.withdrawnAmount;
            if (withdrawalAmount > remaining) revert InvalidWithdrawalAmount(paymentID, withdrawalAmount);

            totalAmount += withdrawalAmount;
            payments[paymentID].withdrawnAmount = payment.withdrawnAmount + withdrawalAmount;
        }

        if (feeAmount > totalAmount) revert InvalidFeeAmount();

        uint256 recipientBalance = balances[recipient];
        if (recipientBalance < totalAmount) revert InsufficientFunds();
        balances[recipient] = recipientBalance - totalAmount;
        balances[arbiter] += feeAmount;

        // mark hash used before external transfer (CEI; FIX-1 sibling)
        withdrawalHashes[withdrawalInfoHash] = true;

        fiatToken.safeTransfer(recipient, totalAmount - feeAmount);
        emit Withdrawal(recipient, totalAmount);
        emit WithdrawalFeePaid(recipient, feeAmount);
    }

    function updateRefundTo(uint256 paymentID, address newRefundTo) external override {
        if (newRefundTo == address(0)) revert RefundToIsZeroAddress();
        Payment memory payment = payments[paymentID];
        if (msg.sender != payment.refundTo) revert CallerNotAllowed();
        emit RefundToUpdated(paymentID, payment.refundTo, newRefundTo);
        payments[paymentID].refundTo = newRefundTo;
    }

    function hashEarlyWithdrawalInfo(
        uint256[] calldata paymentIDs,
        uint256[] calldata withdrawalAmounts,
        uint256 feeAmount,
        uint256 expiry,
        uint256 salt
    ) external view override returns (bytes32) {
        return _hashEarlyWithdrawalInfo(paymentIDs, withdrawalAmounts, feeAmount, expiry, salt);
    }

    // FIX-1: CEI ordering. Upstream transferred tokens before marking the payment refunded,
    // exposing a reentrancy surface against any non-standard ERC-20 with a transfer hook.
    function _executeRefund(uint256 paymentID, Payment memory payment) internal {
        if (payment.refunded) revert PaymentRefunded(paymentID);
        payments[paymentID].refunded = true;
        fiatToken.safeTransfer(payment.refundTo, payment.amount);
        emit Refund(paymentID, payment.refundTo, payment.amount);
    }

    function _settleDebt(address recipient) internal {
        uint256 recipientDebt = debts[recipient];
        if (recipientDebt == 0) return;

        uint256 recipientBalance = balances[recipient];
        uint256 settleAmount = recipientBalance < recipientDebt ? recipientBalance : recipientDebt;
        if (settleAmount == 0) return;

        balances[recipient] = recipientBalance - settleAmount;
        balances[arbiter] += settleAmount;
        debts[recipient] = recipientDebt - settleAmount;

        emit DebtSettled(recipient, settleAmount);
    }

    function _hashEarlyWithdrawalInfo(
        uint256[] calldata paymentIDs,
        uint256[] calldata withdrawalAmounts,
        uint256 feeAmount,
        uint256 expiry,
        uint256 salt
    ) internal view returns (bytes32) {
        bytes32 structHash =
            keccak256(abi.encode(EARLY_WITHDRAWAL_TYPEHASH, paymentIDs, withdrawalAmounts, feeAmount, expiry, salt));
        return _hashTypedDataV4(structHash);
    }
}
