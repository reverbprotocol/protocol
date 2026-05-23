// SPDX-License-Identifier: Apache-2.0
/*
 * Vendored verbatim from circlefin/refund-protocol@b506b17 for side-by-side bug-fix proofs.
 * Do not modify. Do not deploy. Test-only artifact.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";

contract RefundProtocolUpstream is EIP712 {
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

    IERC20 public fiatToken;
    uint256 public nonce;
    address public arbiter;
    mapping(address => uint256) public lockupSeconds;
    mapping(uint256 => Payment) public payments;
    mapping(address => uint256) public balances;
    mapping(address => uint256) public debts;
    mapping(bytes32 => bool) public withdrawalHashes;

    error CallerNotAllowed();
    error PaymentIsStillLocked(uint256 paymentID);
    error PaymentDoesNotBelongToRecipient();
    error RefundToIsZeroAddress();
    error InsufficientFunds();
    error InvalidWithdrawalAmount(uint256 paymentID, uint256 withdrawalAmount);
    error InvalidFeeAmount();
    error InvalidSignature();
    error WithdrawalHashAlreadyUsed();
    error WithdrawalHashExpired();
    error PaymentRefunded(uint256 paymentID);
    error LockupSecondsExceedsMax();
    error MismatchedEarlyWithdrawalArrays();

    constructor(address _arbiter, address _usdc, string memory n, string memory v)
        EIP712(n, v)
    {
        arbiter = _arbiter;
        fiatToken = IERC20(_usdc);
    }

    modifier onlyArbiter() {
        if (msg.sender != arbiter) revert CallerNotAllowed();
        _;
    }

    function pay(address to, uint256 amount, address refundTo) external {
        if (refundTo == address(0)) revert RefundToIsZeroAddress();
        uint256 lockup = lockupSeconds[to];
        fiatToken.transferFrom(msg.sender, address(this), amount);
        payments[nonce] = Payment(to, amount, block.timestamp + lockup, refundTo, 0, false);
        balances[to] += amount;
        nonce += 1;
    }

    function refundByRecipient(uint256 paymentID) external {
        Payment memory p = payments[paymentID];
        if (msg.sender != p.to) revert CallerNotAllowed();
        uint256 b = balances[p.to];
        if (p.amount > b) revert InsufficientFunds();
        balances[p.to] = b - p.amount;
        _executeRefund(paymentID, p);
    }

    function depositArbiterFunds(uint256 amount) external onlyArbiter {
        fiatToken.transferFrom(msg.sender, address(this), amount);
        balances[arbiter] += amount;
    }

    function setLockupSeconds(address recipient, uint256 s) external onlyArbiter {
        if (s > MAX_LOCKUP_SECONDS) revert LockupSecondsExceedsMax();
        lockupSeconds[recipient] = s;
    }

    function refundByArbiter(uint256 paymentID) external onlyArbiter {
        Payment memory p = payments[paymentID];
        uint256 rb = balances[p.to];
        if (p.amount <= rb) {
            balances[p.to] = rb - p.amount;
            return _executeRefund(paymentID, p);
        }
        uint256 ab = balances[arbiter];
        if (p.amount > ab) revert InsufficientFunds();
        balances[arbiter] = ab - p.amount;
        debts[p.to] += p.amount;
        _executeRefund(paymentID, p);
    }

    function withdraw(uint256[] calldata paymentIDs) external {
        _settleDebt(msg.sender);
        uint256 total = 0;
        for (uint256 i = 0; i < paymentIDs.length; ++i) {
            Payment memory p = payments[paymentIDs[i]];
            if (p.to != msg.sender) revert CallerNotAllowed();
            if (block.timestamp < p.releaseTimestamp) revert PaymentIsStillLocked(paymentIDs[i]);
            if (p.refunded) revert PaymentRefunded(paymentIDs[i]);
            total += p.amount - p.withdrawnAmount;
            payments[paymentIDs[i]].withdrawnAmount = p.amount;
        }
        uint256 b = balances[msg.sender];
        if (total > b) revert InsufficientFunds();
        balances[msg.sender] = b - total;
        fiatToken.transfer(msg.sender, total);
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
    ) external onlyArbiter {
        bytes32 h = _hashEarlyWithdrawalInfo(paymentIDs, withdrawalAmounts, feeAmount, expiry, salt);
        if (withdrawalHashes[h]) revert WithdrawalHashAlreadyUsed();
        if (ecrecover(h, v, r, s) != recipient) revert InvalidSignature();
        if (block.timestamp > expiry) revert WithdrawalHashExpired();

        uint256 total = 0;
        if (paymentIDs.length != withdrawalAmounts.length) revert MismatchedEarlyWithdrawalArrays();

        for (uint256 i = 0; i < paymentIDs.length; ++i) {
            uint256 pid = paymentIDs[i];
            uint256 wa = withdrawalAmounts[i];
            Payment memory p = payments[pid];
            // BUG: checks against full amount, not remaining
            if (wa > p.amount) revert InvalidWithdrawalAmount(pid, wa);
            if (p.to != recipient) revert PaymentDoesNotBelongToRecipient();
            if (p.refunded) revert PaymentRefunded(pid);
            total += wa;
            payments[pid].withdrawnAmount += wa;
        }
        if (feeAmount > total) revert InvalidFeeAmount();
        uint256 rb = balances[recipient];
        if (rb < total) revert InsufficientFunds();
        balances[recipient] = rb - total;
        balances[arbiter] += feeAmount;
        fiatToken.transfer(recipient, total - feeAmount);
        withdrawalHashes[h] = true;
    }

    function hashEarlyWithdrawalInfo(
        uint256[] calldata paymentIDs,
        uint256[] calldata withdrawalAmounts,
        uint256 feeAmount,
        uint256 expiry,
        uint256 salt
    ) external view returns (bytes32) {
        return _hashEarlyWithdrawalInfo(paymentIDs, withdrawalAmounts, feeAmount, expiry, salt);
    }

    function _executeRefund(uint256 paymentID, Payment memory p) internal {
        if (p.refunded) revert PaymentRefunded(paymentID);
        // BUG: external call before state update
        fiatToken.transfer(p.refundTo, p.amount);
        payments[paymentID].refunded = true;
    }

    function _settleDebt(address recipient) internal {
        uint256 d = debts[recipient];
        uint256 b = balances[recipient];
        uint256 settle = b < d ? b : d;
        balances[recipient] = b - settle;
        balances[arbiter] += settle;
        debts[recipient] = d - settle;
    }

    function _hashEarlyWithdrawalInfo(
        uint256[] calldata paymentIDs,
        uint256[] calldata withdrawalAmounts,
        uint256 feeAmount,
        uint256 expiry,
        uint256 salt
    ) internal view returns (bytes32) {
        bytes32 sh = keccak256(abi.encode(EARLY_WITHDRAWAL_TYPEHASH, paymentIDs, withdrawalAmounts, feeAmount, expiry, salt));
        return _hashTypedDataV4(sh);
    }
}
