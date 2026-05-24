// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {RefundProtocolFixed} from "../src/RefundProtocolFixed.sol";

/// @notice Selector + event-topic freeze tests for RefundProtocolFixed.
///
/// Every external function in the contract's interface and every emitted event has its on-chain
/// signature locked here. Any upgrade that changes a function name, parameter list, or event
/// shape will fail this test before merge.
///
/// Pattern: the left side uses the Solidity-resolved `.selector` accessor on the contract type
/// (function selector or event topic). The right side computes the hash of the explicit
/// signature string. If the function or event signature drifts in the contract, the left side
/// resolves to a different hash; the literal string on the right does not move. Assertion fails.
contract SelectorFreezeRefundProtocolFixedTest is Test {
    // ---------- external function selectors ----------

    function test_selectorFrozen_DOMAIN_SEPARATOR() public pure {
        assertEq(RefundProtocolFixed.DOMAIN_SEPARATOR.selector, bytes4(keccak256("DOMAIN_SEPARATOR()")));
    }

    function test_selectorFrozen_pay() public pure {
        assertEq(RefundProtocolFixed.pay.selector, bytes4(keccak256("pay(address,uint256,address)")));
    }

    function test_selectorFrozen_refundByRecipient() public pure {
        assertEq(RefundProtocolFixed.refundByRecipient.selector, bytes4(keccak256("refundByRecipient(uint256)")));
    }

    function test_selectorFrozen_refundByArbiter() public pure {
        assertEq(RefundProtocolFixed.refundByArbiter.selector, bytes4(keccak256("refundByArbiter(uint256)")));
    }

    function test_selectorFrozen_settleDebt() public pure {
        assertEq(RefundProtocolFixed.settleDebt.selector, bytes4(keccak256("settleDebt(address)")));
    }

    function test_selectorFrozen_depositArbiterFunds() public pure {
        assertEq(RefundProtocolFixed.depositArbiterFunds.selector, bytes4(keccak256("depositArbiterFunds(uint256)")));
    }

    function test_selectorFrozen_withdrawArbiterFunds() public pure {
        assertEq(RefundProtocolFixed.withdrawArbiterFunds.selector, bytes4(keccak256("withdrawArbiterFunds(uint256)")));
    }

    function test_selectorFrozen_setLockupSeconds() public pure {
        assertEq(RefundProtocolFixed.setLockupSeconds.selector, bytes4(keccak256("setLockupSeconds(address,uint256)")));
    }

    function test_selectorFrozen_withdraw() public pure {
        assertEq(RefundProtocolFixed.withdraw.selector, bytes4(keccak256("withdraw(uint256[])")));
    }

    function test_selectorFrozen_earlyWithdrawByArbiter() public pure {
        assertEq(
            RefundProtocolFixed.earlyWithdrawByArbiter.selector,
            bytes4(keccak256("earlyWithdrawByArbiter(uint256[],uint256[],uint256,uint256,uint256,address,uint8,bytes32,bytes32)"))
        );
    }

    function test_selectorFrozen_updateRefundTo() public pure {
        assertEq(RefundProtocolFixed.updateRefundTo.selector, bytes4(keccak256("updateRefundTo(uint256,address)")));
    }

    function test_selectorFrozen_hashEarlyWithdrawalInfo() public pure {
        assertEq(
            RefundProtocolFixed.hashEarlyWithdrawalInfo.selector,
            bytes4(keccak256("hashEarlyWithdrawalInfo(uint256[],uint256[],uint256,uint256,uint256)"))
        );
    }

    // ---------- event topic hashes ----------

    function test_eventTopicFrozen_PaymentCreated() public pure {
        assertEq(
            RefundProtocolFixed.PaymentCreated.selector,
            keccak256("PaymentCreated(uint256,address,uint256,uint256,address)")
        );
    }

    function test_eventTopicFrozen_Refund() public pure {
        assertEq(RefundProtocolFixed.Refund.selector, keccak256("Refund(uint256,address,uint256)"));
    }

    function test_eventTopicFrozen_RefundToUpdated() public pure {
        assertEq(
            RefundProtocolFixed.RefundToUpdated.selector,
            keccak256("RefundToUpdated(uint256,address,address)")
        );
    }

    function test_eventTopicFrozen_Withdrawal() public pure {
        assertEq(RefundProtocolFixed.Withdrawal.selector, keccak256("Withdrawal(address,uint256)"));
    }

    function test_eventTopicFrozen_WithdrawalFeePaid() public pure {
        assertEq(RefundProtocolFixed.WithdrawalFeePaid.selector, keccak256("WithdrawalFeePaid(address,uint256)"));
    }

    function test_eventTopicFrozen_DebtSettled() public pure {
        assertEq(RefundProtocolFixed.DebtSettled.selector, keccak256("DebtSettled(address,uint256)"));
    }
}
