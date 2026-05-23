// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ReputationRegistryVanilla} from "../src/reference/ReputationRegistryVanilla.sol";

contract ReputationRegistryVanillaTest is Test {
    ReputationRegistryVanilla public registry;

    address public recorderA = address(0xAAAA);
    address public recorderB = address(0xBBBB);
    address public stranger = address(0xCAFE);
    address public agent = address(0xBEEF);

    function setUp() public {
        address[] memory recorders = new address[](2);
        recorders[0] = recorderA;
        recorders[1] = recorderB;
        registry = new ReputationRegistryVanilla(recorders, 1, -1);
    }

    function test_recordUpheld_updatesScoreWithPositiveDelta() public {
        vm.prank(recorderA);
        registry.recordUpheld(agent);
        assertEq(registry.reputationScore(agent), 1);

        vm.prank(recorderB);
        registry.recordUpheld(agent);
        assertEq(registry.reputationScore(agent), 2);
    }

    function test_recordRejected_updatesScoreWithNegativeDelta() public {
        vm.prank(recorderA);
        registry.recordRejected(agent);
        assertEq(registry.reputationScore(agent), -1);
    }

    function test_record_revertsWhenCallerNotRecorder() public {
        vm.prank(stranger);
        vm.expectRevert(ReputationRegistryVanilla.CallerNotRecorder.selector);
        registry.recordUpheld(agent);

        vm.prank(stranger);
        vm.expectRevert(ReputationRegistryVanilla.CallerNotRecorder.selector);
        registry.recordRejected(agent);
    }

    function test_unrecordedAgent_hasZeroScore() public view {
        assertEq(registry.reputationScore(agent), 0);
        assertEq(registry.reputationScore(address(0xDEAD)), 0);
    }
}
