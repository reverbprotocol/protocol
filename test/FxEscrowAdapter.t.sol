// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {FxEscrowAdapter, IFxEscrow} from "../src/reference/FxEscrowAdapter.sol";

contract MockStable is ERC20 {
    constructor(string memory name, string memory sym) ERC20(name, sym) {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

/// @dev Simple mock FxEscrow. Rate is `toPerFrom` scaled by 1e18 per unit. Pays `to` to the
///      caller-supplied recipient from its own pre-funded reserves.
contract MockFxEscrow is IFxEscrow {
    uint256 public ratePerUnit; // toToken units per fromToken unit, fixed-point 1e18

    constructor(uint256 _ratePerUnit) { ratePerUnit = _ratePerUnit; }

    function setRate(uint256 r) external { ratePerUnit = r; }

    function quote(address /*from*/, address /*to*/, uint256 amount) external view override returns (uint256) {
        return (amount * ratePerUnit) / 1e18;
    }

    function swap(address from, address to, uint256 amount, address recipient)
        external
        override
        returns (uint256 amountOut)
    {
        MockStable(from).transferFrom(msg.sender, address(this), amount);
        amountOut = (amount * ratePerUnit) / 1e18;
        MockStable(to).transfer(recipient, amountOut);
    }
}

contract FxEscrowAdapterTest is Test {
    MockStable public usdc;
    MockStable public eurc;
    MockFxEscrow public fx;
    FxEscrowAdapter public adapter;

    address public trader = address(0xBEEF);
    uint256 public constant RATE_USDC_TO_EURC = 0.92e18;

    function setUp() public {
        usdc = new MockStable("USD Coin", "USDC");
        eurc = new MockStable("Euro Coin", "EURC");
        fx = new MockFxEscrow(RATE_USDC_TO_EURC);
        adapter = new FxEscrowAdapter(address(fx));

        // Fund trader with USDC, fund FxEscrow with EURC reserves
        usdc.mint(trader, 1_000e6);
        eurc.mint(address(fx), 1_000_000e6);

        vm.prank(trader);
        usdc.approve(address(adapter), type(uint256).max);
    }

    function test_quoteSwap_returnsExpectedAmount() public view {
        uint256 quoted = adapter.quoteSwap(address(usdc), address(eurc), 100e6);
        assertEq(quoted, 92e6);
    }

    function test_executeSwap_movesTokensAtConfiguredRate() public {
        uint256 amountIn = 100e6;

        vm.prank(trader);
        uint256 received = adapter.executeSwap(address(usdc), address(eurc), amountIn, 92e6);

        assertEq(received, 92e6);
        assertEq(usdc.balanceOf(trader), 900e6);
        assertEq(eurc.balanceOf(trader), 92e6);
    }

    function test_executeSwap_revertsBelowMinOut() public {
        uint256 amountIn = 100e6;
        // Demand 95e6 EURC but rate only yields 92e6
        vm.prank(trader);
        vm.expectRevert(abi.encodeWithSelector(FxEscrowAdapter.SlippageExceeded.selector, 92e6, 95e6));
        adapter.executeSwap(address(usdc), address(eurc), amountIn, 95e6);
    }
}
