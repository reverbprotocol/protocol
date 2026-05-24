// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {RefundProtocolFixed} from "../src/RefundProtocolFixed.sol";

contract InvUSDC is ERC20 {
    constructor() ERC20("USDC", "USDC") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

/// @dev Stateful fuzz handler. Drives the escrow with bounded random pay() and withdraw() calls.
contract RefundProtocolHandler is Test {
    RefundProtocolFixed public escrow;
    InvUSDC public usdc;
    address[] public actors;
    uint256 public payCallCount;
    uint256 public withdrawCallCount;

    constructor(RefundProtocolFixed _escrow, InvUSDC _usdc, address[] memory _actors) {
        escrow = _escrow;
        usdc = _usdc;
        for (uint256 i = 0; i < _actors.length; i++) {
            actors.push(_actors[i]);
            usdc.mint(_actors[i], 1_000_000_000);
            vm.prank(_actors[i]);
            usdc.approve(address(escrow), type(uint256).max);
        }
    }

    function pay(uint256 actorSeed, uint256 toSeed, uint256 amountSeed) external {
        address actor = actors[actorSeed % actors.length];
        address to = actors[toSeed % actors.length];
        uint256 amount = bound(amountSeed, 1, 10_000);
        vm.prank(actor);
        escrow.pay(to, amount, actor);
        payCallCount++;
    }

    function withdraw(uint256 actorSeed, uint256 idSeed) external {
        if (escrow.nonce() == 0) return;
        address actor = actors[actorSeed % actors.length];
        uint256 paymentId = idSeed % escrow.nonce();
        uint256[] memory ids = new uint256[](1);
        ids[0] = paymentId;
        try escrow.withdraw(ids) {} catch {}
        withdrawCallCount++;
    }
}

contract RefundProtocolInvariantTest is Test {
    RefundProtocolFixed public escrow;
    InvUSDC public usdc;
    RefundProtocolHandler public handler;

    address public arbiter = address(0xA);
    address public owner = address(0x10);
    address public pauser = address(0x20);

    function setUp() public {
        usdc = new InvUSDC();
        RefundProtocolFixed impl = new RefundProtocolFixed();
        ERC1967Proxy proxy = new ERC1967Proxy(
            address(impl),
            abi.encodeCall(
                RefundProtocolFixed.initialize,
                (arbiter, address(usdc), "InvTest", "1", owner, pauser)
            )
        );
        escrow = RefundProtocolFixed(address(proxy));

        address[] memory actors = new address[](3);
        actors[0] = address(0xA11CE);
        actors[1] = address(0xB0B);
        actors[2] = address(0xCAFE);

        handler = new RefundProtocolHandler(escrow, usdc, actors);

        targetContract(address(handler));
    }

    /// @notice FIX-2 invariant: cumulative withdrawnAmount on any payment is bounded by its
    ///         original amount. The upstream contract violated this; the fix enforces it.
    function invariant_noOverWithdraw() public view {
        uint256 total = escrow.nonce();
        for (uint256 i = 0; i < total; i++) {
            (, uint256 amount,, , uint256 withdrawn,) = escrow.payments(i);
            assertLe(withdrawn, amount, "cumulative withdraw exceeded payment amount");
        }
    }

    /// @notice Aggregate balance integrity: the sum of per-account balances tracked in the
    ///         contract never exceeds the contract's token balance.
    function invariant_aggregateBalanceBoundedByTokenBalance() public view {
        uint256 totalBalances = 0;
        // arbiter balance + actor balances
        totalBalances += escrow.balances(arbiter);
        totalBalances += escrow.balances(address(0xA11CE));
        totalBalances += escrow.balances(address(0xB0B));
        totalBalances += escrow.balances(address(0xCAFE));
        assertLe(totalBalances, usdc.balanceOf(address(escrow)), "accounted balances exceed token holdings");
    }
}
