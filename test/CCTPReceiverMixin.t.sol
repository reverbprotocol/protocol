// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {CCTPReceiverMixin} from "../src/CCTPReceiverMixin.sol";

contract MockUSDC is ERC20 {
    constructor() ERC20("USD Coin", "USDC") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

contract MockMessageTransmitterV2 {
    MockUSDC public usdc;
    uint256 public mintAmount;
    bool public shouldFail;
    bytes public lastMessage;
    bytes public lastAttestation;

    constructor(address _usdc) {
        usdc = MockUSDC(_usdc);
    }

    function setMintAmount(uint256 a) external { mintAmount = a; }
    function setShouldFail(bool b) external { shouldFail = b; }

    function receiveMessage(bytes calldata message, bytes calldata attestation) external returns (bool) {
        lastMessage = message;
        lastAttestation = attestation;
        if (shouldFail) return false;
        if (mintAmount > 0) {
            usdc.mint(msg.sender, mintAmount);
        }
        return true;
    }
}

contract TestConsumer is CCTPReceiverMixin {
    bytes public lastPayload;
    uint256 public lastMinted;
    uint256 public callCount;

    constructor(address _mt, address _usdc) {
        __CCTPReceiver_init(_mt, _usdc);
    }

    function handlePayload(bytes calldata payload, uint256 mintedAmount) internal override {
        lastPayload = payload;
        lastMinted = mintedAmount;
        callCount += 1;
    }
}

contract CCTPReceiverMixinTest is Test {
    MockUSDC public usdc;
    MockMessageTransmitterV2 public mt;
    TestConsumer public consumer;

    function setUp() public {
        usdc = new MockUSDC();
        mt = new MockMessageTransmitterV2(address(usdc));
        consumer = new TestConsumer(address(mt), address(usdc));
    }

    /// @dev Build a synthetic CCTP-shaped message with a known hookData payload at the standard
    ///      v2 offset (376 bytes of fixed prefix, then hookData).
    function _buildMessage(bytes memory hookData) internal pure returns (bytes memory) {
        bytes memory prefix = new bytes(376);
        return bytes.concat(prefix, hookData);
    }

    function test_onCCTPReceive_mintsAndDispatches() public {
        uint256 amount = 1_234;
        mt.setMintAmount(amount);

        bytes memory payload = abi.encode(address(0xBEEF), uint256(42));
        bytes memory message = _buildMessage(payload);

        consumer.onCCTPReceive(message, "attestation-bytes");

        assertEq(consumer.callCount(), 1);
        assertEq(consumer.lastMinted(), amount);
        assertEq(consumer.lastPayload(), payload);
        assertEq(usdc.balanceOf(address(consumer)), amount);
    }

    function test_onCCTPReceive_revertsWhenTransmitterFails() public {
        mt.setShouldFail(true);
        bytes memory message = _buildMessage("");
        vm.expectRevert(CCTPReceiverMixin.CCTPReceiveFailed.selector);
        consumer.onCCTPReceive(message, "attestation");
    }

    function test_onCCTPReceive_balanceAccountingTracksDelta() public {
        // Pre-existing balance on the receiver should not be counted as minted.
        usdc.mint(address(consumer), 999);
        mt.setMintAmount(500);

        bytes memory message = _buildMessage("");
        consumer.onCCTPReceive(message, "");

        assertEq(consumer.lastMinted(), 500);
        assertEq(usdc.balanceOf(address(consumer)), 1_499);
    }

    function test_onCCTPReceive_emptyPayloadWhenMessageBelowHookOffset() public {
        mt.setMintAmount(0);
        bytes memory shortMessage = new bytes(100);
        consumer.onCCTPReceive(shortMessage, "");

        assertEq(consumer.callCount(), 1);
        assertEq(consumer.lastPayload().length, 0);
    }
}
