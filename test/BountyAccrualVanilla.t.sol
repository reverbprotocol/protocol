// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {BountyAccrualVanilla} from "../src/reference/BountyAccrualVanilla.sol";

contract MockERC20 is ERC20 {
    constructor() ERC20("USD Coin", "USDC") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

contract BountyAccrualVanillaTest is Test {
    BountyAccrualVanilla public bounty;
    MockERC20 public usdc;

    address public funder = address(0xFEED);
    address public recipient = address(0xBEEF);
    address public stranger = address(0xCAFE);

    function setUp() public {
        usdc = new MockERC20();
        bounty = new BountyAccrualVanilla(address(usdc));
        usdc.mint(funder, 10_000);
        vm.prank(funder);
        usdc.approve(address(bounty), type(uint256).max);
    }

    function test_accrue_thenClaim_happyPath() public {
        vm.prank(funder);
        uint256 claimId = bounty.accrueBounty(recipient, 500);

        assertEq(claimId, 0);
        assertEq(bounty.bountyAmount(claimId), 500);
        assertEq(bounty.bountyRecipient(claimId), recipient);
        assertFalse(bounty.bountyClaimed(claimId));
        assertEq(usdc.balanceOf(address(bounty)), 500);

        vm.prank(recipient);
        bounty.claimBounty(claimId);

        assertTrue(bounty.bountyClaimed(claimId));
        assertEq(usdc.balanceOf(recipient), 500);
        assertEq(usdc.balanceOf(address(bounty)), 0);
    }

    function test_claim_revertsWhenCallerNotRecipient() public {
        vm.prank(funder);
        uint256 claimId = bounty.accrueBounty(recipient, 100);

        vm.prank(stranger);
        vm.expectRevert(BountyAccrualVanilla.CallerNotRecipient.selector);
        bounty.claimBounty(claimId);
    }

    function test_claim_revertsWhenAlreadyClaimed() public {
        vm.prank(funder);
        uint256 claimId = bounty.accrueBounty(recipient, 100);

        vm.prank(recipient);
        bounty.claimBounty(claimId);

        vm.prank(recipient);
        vm.expectRevert(BountyAccrualVanilla.BountyAlreadyClaimed.selector);
        bounty.claimBounty(claimId);
    }

    function test_accrue_revertsOnZeroAmount() public {
        vm.prank(funder);
        vm.expectRevert(BountyAccrualVanilla.ZeroAmount.selector);
        bounty.accrueBounty(recipient, 0);
    }
}
