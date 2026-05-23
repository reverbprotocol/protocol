// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {RefundProtocolFixed} from "../src/RefundProtocolFixed.sol";
import {RefundProtocolUpstream} from "./vendor/RefundProtocolUpstream.sol";

contract MockERC20 is ERC20 {
    constructor() ERC20("USD Coin", "USDC") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

contract RefundProtocolFixedTest is Test {
    RefundProtocolFixed public escrow;
    MockERC20 public usdc;

    uint256 public constant USER_PK = 0x1234;
    uint256 public constant RECEIVER_PK = 0x5678;
    address public user = vm.addr(USER_PK);
    address public receiver = vm.addr(RECEIVER_PK);
    address public arbiter = address(0xABCD);
    address public refundTo = address(0x9ABC);
    uint256 public expiry;

    function setUp() public {
        usdc = new MockERC20();
        escrow = new RefundProtocolFixed(arbiter, address(usdc), "Refund Protocol", "1.0");
        usdc.mint(user, 1_000);
        usdc.mint(arbiter, 1_000);
        vm.prank(user);
        usdc.approve(address(escrow), type(uint256).max);
        vm.prank(arbiter);
        usdc.approve(address(escrow), type(uint256).max);
        expiry = block.timestamp + 9_999_999;
    }

    // ---------- core flows: parity with upstream ----------

    function test_pay_storesPaymentAndIncrementsNonce() public {
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo);
        (address to, uint256 amount,, address rt,, bool refunded) = escrow.payments(0);
        assertEq(to, receiver);
        assertEq(amount, 100);
        assertEq(rt, refundTo);
        assertFalse(refunded);
        assertEq(escrow.balances(receiver), 100);
        assertEq(escrow.nonce(), 1);
    }

    function test_pay_revertsOnZeroRefundTo() public {
        vm.prank(user);
        vm.expectRevert(RefundProtocolFixed.RefundToIsZeroAddress.selector);
        escrow.pay(receiver, 100, address(0));
    }

    function test_pay_revertsOnZeroRecipient() public {
        vm.prank(user);
        vm.expectRevert(RefundProtocolFixed.RecipientIsZeroAddress.selector);
        escrow.pay(address(0), 100, refundTo);
    }

    function test_withdraw_afterLockupExpires() public {
        vm.prank(arbiter);
        escrow.setLockupSeconds(receiver, 3600);
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo);

        uint256[] memory ids = _ids(0);
        vm.prank(receiver);
        vm.expectRevert(abi.encodeWithSelector(RefundProtocolFixed.PaymentIsStillLocked.selector, 0));
        escrow.withdraw(ids);

        vm.warp(block.timestamp + 3601);
        vm.prank(receiver);
        escrow.withdraw(ids);
        assertEq(usdc.balanceOf(receiver), 100);
        assertEq(escrow.balances(receiver), 0);
    }

    function test_refundByRecipient_returnsToRefundTo() public {
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo);
        vm.prank(receiver);
        escrow.refundByRecipient(0);
        assertEq(usdc.balanceOf(refundTo), 100);
        assertEq(escrow.balances(receiver), 0);
    }

    function test_earlyWithdrawByArbiter_partialThenLockedWithdrawCompletes() public {
        vm.prank(arbiter);
        escrow.setLockupSeconds(receiver, 3600);
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo);

        uint256[] memory ids = _ids(0);
        uint256[] memory amts = _amts(90);
        (uint8 v, bytes32 r, bytes32 s) = _signEarlyWithdraw(ids, amts, 0, expiry, 0, RECEIVER_PK, address(escrow));

        vm.prank(arbiter);
        escrow.earlyWithdrawByArbiter(ids, amts, 0, expiry, 0, receiver, v, r, s);
        assertEq(escrow.balances(receiver), 10);
        assertEq(usdc.balanceOf(receiver), 90);

        vm.warp(block.timestamp + 3601);
        vm.prank(receiver);
        escrow.withdraw(ids);
        assertEq(escrow.balances(receiver), 0);
        assertEq(usdc.balanceOf(receiver), 100);
    }

    // ---------- FIX-1: CEI ordering on _executeRefund ----------

    function test_FIX1_executeRefund_marksRefundedBeforeTransfer() public {
        // Sentinel ERC-20 that asserts state-on-transfer.
        StateProbeToken probe = new StateProbeToken();
        RefundProtocolFixed e = new RefundProtocolFixed(arbiter, address(probe), "P", "1");
        probe.mint(user, 1000);
        vm.prank(user);
        probe.approve(address(e), type(uint256).max);

        vm.prank(user);
        e.pay(receiver, 100, refundTo);

        // Tell the probe to read e.payments(0).refunded inside transfer and bind expectation.
        probe.bind(address(e), 0);

        vm.prank(receiver);
        e.refundByRecipient(0);

        // probe.observedRefundedFlag MUST be true (state set before external call).
        assertTrue(probe.observedRefundedFlag(), "FIX-1: refunded flag must be set before transfer");
    }

    function test_FIX1_upstream_doesNotMarkRefundedBeforeTransfer() public {
        StateProbeUpstreamToken probe = new StateProbeUpstreamToken();
        RefundProtocolUpstream u = new RefundProtocolUpstream(arbiter, address(probe), "P", "1");
        probe.mint(user, 1000);
        vm.prank(user);
        probe.approve(address(u), type(uint256).max);
        vm.prank(user);
        u.pay(receiver, 100, refundTo);
        probe.bind(address(u), 0);

        vm.prank(receiver);
        u.refundByRecipient(0);

        // Upstream sets refunded AFTER the transfer, so probe sees false.
        assertFalse(probe.observedRefundedFlag(), "upstream observed pre-CEI ordering");
    }

    // ---------- FIX-2: cumulative over-withdraw guard ----------

    function test_FIX2_upstream_drainsPastFullPaymentAmount() public {
        // Setup: receiver has TWO payments of 100, balance = 200.
        // Drain >100 from a single payment via two distinct-salt sigs.
        RefundProtocolUpstream u = new RefundProtocolUpstream(arbiter, address(usdc), "P", "1");
        vm.prank(user);
        usdc.approve(address(u), type(uint256).max);
        vm.prank(user);
        u.pay(receiver, 100, refundTo); // id 0
        vm.prank(user);
        u.pay(receiver, 100, refundTo); // id 1
        assertEq(u.balances(receiver), 200);

        uint256[] memory ids = _ids(0); // only id 0 referenced
        uint256[] memory amts = _amts(90);

        // First session, salt 0
        (uint8 v1, bytes32 r1, bytes32 s1) = _signUpstream(ids, amts, 0, expiry, 0, RECEIVER_PK, address(u));
        vm.prank(arbiter);
        u.earlyWithdrawByArbiter(ids, amts, 0, expiry, 0, receiver, v1, r1, s1);

        // Second session, salt 1, same paymentID, same amount
        (uint8 v2, bytes32 r2, bytes32 s2) = _signUpstream(ids, amts, 0, expiry, 1, RECEIVER_PK, address(u));
        vm.prank(arbiter);
        u.earlyWithdrawByArbiter(ids, amts, 0, expiry, 1, receiver, v2, r2, s2);

        // Withdrawn 180 from a payment whose amount was 100. The other payment's balance is now drained.
        assertEq(usdc.balanceOf(receiver), 180);
        (,,,, uint256 withdrawnAmount,) = u.payments(0);
        assertEq(withdrawnAmount, 180, "upstream tolerated cumulative over-withdraw past payment.amount");
    }

    function test_FIX2_fixed_revertsCumulativeOverWithdraw() public {
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo); // id 0
        vm.prank(user);
        escrow.pay(receiver, 100, refundTo); // id 1

        uint256[] memory ids = _ids(0);
        uint256[] memory amts = _amts(90);

        (uint8 v1, bytes32 r1, bytes32 s1) = _signEarlyWithdraw(ids, amts, 0, expiry, 0, RECEIVER_PK, address(escrow));
        vm.prank(arbiter);
        escrow.earlyWithdrawByArbiter(ids, amts, 0, expiry, 0, receiver, v1, r1, s1);

        (uint8 v2, bytes32 r2, bytes32 s2) = _signEarlyWithdraw(ids, amts, 0, expiry, 1, RECEIVER_PK, address(escrow));
        vm.prank(arbiter);
        // 90 > (100 - 90) remaining, must revert
        vm.expectRevert(abi.encodeWithSelector(RefundProtocolFixed.InvalidWithdrawalAmount.selector, 0, 90));
        escrow.earlyWithdrawByArbiter(ids, amts, 0, expiry, 1, receiver, v2, r2, s2);
    }

    // ---------- FIX-3: debt settlement before earlyWithdraw ----------

    function test_FIX3_upstream_earlyWithdrawBypassesDebtSettlement() public {
        RefundProtocolUpstream u = new RefundProtocolUpstream(arbiter, address(usdc), "P", "1");
        vm.prank(user);
        usdc.approve(address(u), type(uint256).max);
        vm.prank(arbiter);
        usdc.approve(address(u), type(uint256).max);

        // Build a debt: receiver had a payment, withdrew, then arbiter refunds out of pocket.
        vm.prank(user);
        u.pay(receiver, 100, refundTo); // id 0
        uint256[] memory ids0 = _ids(0);
        vm.prank(receiver);
        u.withdraw(ids0);
        vm.prank(arbiter);
        u.depositArbiterFunds(100);
        vm.prank(arbiter);
        u.refundByArbiter(0);
        assertEq(u.debts(receiver), 100);

        // New payment lands; upstream earlyWithdraw does NOT call _settleDebt first.
        vm.prank(user);
        u.pay(receiver, 100, refundTo); // id 1
        assertEq(u.balances(receiver), 100);
        assertEq(u.debts(receiver), 100);

        uint256[] memory ids1 = _ids(1);
        uint256[] memory amts = _amts(100);
        (uint8 v, bytes32 r, bytes32 s) = _signUpstream(ids1, amts, 0, expiry, 0, RECEIVER_PK, address(u));
        vm.prank(arbiter);
        u.earlyWithdrawByArbiter(ids1, amts, 0, expiry, 0, receiver, v, r, s);

        // Debt remains undisturbed (bug); receiver got their 100 anyway.
        assertEq(u.debts(receiver), 100, "upstream did not touch debts on early withdraw");
        assertEq(usdc.balanceOf(receiver), 200);
    }

    function test_FIX3_fixed_earlyWithdrawSettlesDebtFirst() public {
        // Build a 50 debt against the fixed contract.
        vm.prank(user);
        escrow.pay(receiver, 50, refundTo); // id 0
        uint256[] memory ids0 = _ids(0);
        vm.prank(receiver);
        escrow.withdraw(ids0);
        vm.prank(arbiter);
        escrow.depositArbiterFunds(50);
        vm.prank(arbiter);
        escrow.refundByArbiter(0);
        assertEq(escrow.debts(receiver), 50);

        // New 200 payment lands; debt is 50; early-withdraw 100.
        // Sequence under the fix: _settleDebt drains 50 to arbiter, balance becomes 150, debt 0.
        // Then earlyWithdraw 100 lands cleanly: balance 50, payout 100 to receiver.
        vm.prank(user);
        escrow.pay(receiver, 200, refundTo); // id 1
        assertEq(escrow.balances(receiver), 200);

        uint256[] memory ids1 = _ids(1);
        uint256[] memory amts = _amts(100);
        (uint8 v, bytes32 r, bytes32 s) = _signEarlyWithdraw(ids1, amts, 0, expiry, 0, RECEIVER_PK, address(escrow));

        uint256 receiverUSDCBefore = usdc.balanceOf(receiver);
        vm.prank(arbiter);
        escrow.earlyWithdrawByArbiter(ids1, amts, 0, expiry, 0, receiver, v, r, s);

        assertEq(escrow.debts(receiver), 0, "FIX-3: debt settled before early-withdraw");
        assertEq(escrow.balances(receiver), 50, "200 - (50 settled) - (100 withdrawn)");
        assertEq(escrow.balances(arbiter), 50, "arbiter received 50 settled (deposit was already withdrawn for refund)");
        assertEq(usdc.balanceOf(receiver) - receiverUSDCBefore, 100);
    }

    // ---------- FIX-4: zero-recipient guard ----------

    function test_FIX4_fixed_revertsOnZeroRecipient() public {
        uint256[] memory ids = _ids(0);
        uint256[] memory amts = _amts(1);
        (uint8 v, bytes32 r, bytes32 s) = _signEarlyWithdraw(ids, amts, 0, expiry, 0, RECEIVER_PK, address(escrow));
        vm.prank(arbiter);
        vm.expectRevert(RefundProtocolFixed.RecipientIsZeroAddress.selector);
        escrow.earlyWithdrawByArbiter(ids, amts, 0, expiry, 0, address(0), v, r, s);
    }

    // ---------- helpers ----------

    function _ids(uint256 a) internal pure returns (uint256[] memory out) {
        out = new uint256[](1);
        out[0] = a;
    }
    function _amts(uint256 a) internal pure returns (uint256[] memory out) {
        out = new uint256[](1);
        out[0] = a;
    }

    function _signEarlyWithdraw(
        uint256[] memory ids,
        uint256[] memory amts,
        uint256 fee,
        uint256 _expiry,
        uint256 salt,
        uint256 pk,
        address contractAddr
    ) internal returns (uint8 v, bytes32 r, bytes32 s) {
        bytes32 h = RefundProtocolFixed(contractAddr).hashEarlyWithdrawalInfo(ids, amts, fee, _expiry, salt);
        (v, r, s) = vm.sign(pk, h);
    }

    function _signUpstream(
        uint256[] memory ids,
        uint256[] memory amts,
        uint256 fee,
        uint256 _expiry,
        uint256 salt,
        uint256 pk,
        address contractAddr
    ) internal returns (uint8 v, bytes32 r, bytes32 s) {
        bytes32 h = RefundProtocolUpstream(contractAddr).hashEarlyWithdrawalInfo(ids, amts, fee, _expiry, salt);
        (v, r, s) = vm.sign(pk, h);
    }
}

// ---------- probe tokens for FIX-1 ----------

contract StateProbeToken is ERC20 {
    address public boundEscrow;
    uint256 public boundPaymentID;
    bool public observedRefundedFlag;
    bool public bound;

    constructor() ERC20("Probe", "PRB") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
    function bind(address e, uint256 id) external { boundEscrow = e; boundPaymentID = id; bound = true; }

    function transfer(address to, uint256 amount) public override returns (bool) {
        if (bound && msg.sender == boundEscrow) {
            (,,,,, bool refunded) = RefundProtocolFixed(boundEscrow).payments(boundPaymentID);
            observedRefundedFlag = refunded;
        }
        return super.transfer(to, amount);
    }
}

contract StateProbeUpstreamToken is ERC20 {
    address public boundEscrow;
    uint256 public boundPaymentID;
    bool public observedRefundedFlag;
    bool public bound;

    constructor() ERC20("ProbeU", "PRBU") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
    function bind(address e, uint256 id) external { boundEscrow = e; boundPaymentID = id; bound = true; }

    function transfer(address to, uint256 amount) public override returns (bool) {
        if (bound && msg.sender == boundEscrow) {
            (,,,,, bool refunded) = RefundProtocolUpstream(boundEscrow).payments(boundPaymentID);
            observedRefundedFlag = refunded;
        }
        return super.transfer(to, amount);
    }
}
