// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {USYCBondVault, IUSYCTeller} from "../src/reference/USYCBondVault.sol";

contract MockUSDC is ERC20 {
    constructor() ERC20("USD Coin", "USDC") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

/// @dev Mock Teller that mints share-tokens 1:1 with assets at deposit-time, then accrues yield
///      via a configurable bump that increases the per-share asset value over time.
contract MockTeller is IUSYCTeller {
    MockUSDC public usdc;
    uint256 public totalShares;
    uint256 public totalAssetsHeld;

    constructor(address _usdc) { usdc = MockUSDC(_usdc); }

    function deposit(uint256 assets, address /*receiver*/) external override returns (uint256 shares) {
        usdc.transferFrom(msg.sender, address(this), assets);
        // 1:1 share-price at issuance
        shares = totalShares == 0 ? assets : (assets * totalShares) / totalAssetsHeld;
        totalShares += shares;
        totalAssetsHeld += assets;
        return shares;
    }

    function redeem(uint256 shares, address receiver, address /*owner*/) external override returns (uint256 assets) {
        assets = (shares * totalAssetsHeld) / totalShares;
        totalShares -= shares;
        totalAssetsHeld -= assets;
        usdc.transfer(receiver, assets);
        return assets;
    }

    function convertToAssets(uint256 shares) external view override returns (uint256) {
        if (totalShares == 0) return 0;
        return (shares * totalAssetsHeld) / totalShares;
    }

    /// Simulate yield by inflating the asset-side of the pool. Mints to itself so per-share
    /// asset value rises.
    function accrueYield(uint256 amount) external {
        usdc.mint(address(this), amount);
        totalAssetsHeld += amount;
    }
}

contract USYCBondVaultTest is Test {
    MockUSDC public usdc;
    MockTeller public teller;

    address public funder = address(0xFEED);
    address public alice = address(0xA11CE);
    address public bob = address(0xB0B);

    uint256 public constant MIN_SUB = 100_000e6; // $100K with 6 decimals

    function setUp() public {
        usdc = new MockUSDC();
        teller = new MockTeller(address(usdc));
        usdc.mint(funder, 10_000_000e6);
    }

    function _newVault(USYCBondVault.AggregationPolicy policy) internal returns (USYCBondVault v) {
        v = new USYCBondVault(address(usdc), address(teller), policy, MIN_SUB);
        vm.prank(funder);
        usdc.approve(address(v), type(uint256).max);
    }

    function test_deposit_strict_subscribesImmediately() public {
        USYCBondVault vault = _newVault(USYCBondVault.AggregationPolicy.STRICT);
        vm.prank(funder);
        vault.depositPrincipal(address(usdc), MIN_SUB, alice);

        assertEq(vault.principalOf(alice), MIN_SUB);
        assertEq(vault.totalShares(), MIN_SUB);
        assertEq(vault.pendingPrincipal(), 0);
    }

    function test_deposit_strict_revertsBelowMinimum() public {
        USYCBondVault vault = _newVault(USYCBondVault.AggregationPolicy.STRICT);
        vm.prank(funder);
        vm.expectRevert(USYCBondVault.BelowMinimumSubscription.selector);
        vault.depositPrincipal(address(usdc), MIN_SUB - 1, alice);
    }

    function test_deposit_aggregated_batchesAtThreshold() public {
        USYCBondVault vault = _newVault(USYCBondVault.AggregationPolicy.AGGREGATED);

        vm.prank(funder);
        vault.depositPrincipal(address(usdc), 60_000e6, alice);
        assertEq(vault.totalShares(), 0);
        assertEq(vault.pendingPrincipal(), 60_000e6);

        vm.prank(funder);
        vault.depositPrincipal(address(usdc), 50_000e6, bob);
        assertEq(vault.pendingPrincipal(), 0);
        assertEq(vault.totalShares(), 110_000e6);
    }

    function test_withdraw_returnsPrincipalPlusYield() public {
        USYCBondVault vault = _newVault(USYCBondVault.AggregationPolicy.STRICT);
        vm.prank(funder);
        vault.depositPrincipal(address(usdc), MIN_SUB, alice);

        teller.accrueYield(10_000e6); // 10% yield on the pool

        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vault.withdrawPrincipalWithYield(alice);
        uint256 received = usdc.balanceOf(alice) - aliceBalanceBefore;

        assertGt(received, MIN_SUB);
        assertEq(received, MIN_SUB + 10_000e6);
        assertEq(vault.principalOf(alice), 0);
    }

    function test_accruedYield_reportsCurrentValue() public {
        USYCBondVault vault = _newVault(USYCBondVault.AggregationPolicy.STRICT);
        vm.prank(funder);
        vault.depositPrincipal(address(usdc), MIN_SUB, alice);

        assertEq(vault.accruedYield(alice), 0);
        teller.accrueYield(5_000e6);
        assertEq(vault.accruedYield(alice), 5_000e6);
    }
}
