// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {RefundProtocolFixed} from "../src/RefundProtocolFixed.sol";

/**
 * Deploy RefundProtocolFixed.
 *
 * Required env:
 *   DEPLOYER_PRIVATE_KEY  - hex-encoded EVM private key, no 0x prefix tolerated by foundry
 *   ARBITER_ADDRESS       - EOA or multisig that will be granted onlyArbiter rights
 *   FIAT_TOKEN_ADDRESS    - the ERC-20 to escrow (USDC on the target chain)
 *
 * Optional env:
 *   EIP712_NAME           - default "RefundProtocolFixed"
 *   EIP712_VERSION        - default "1"
 *
 * Deploy with:
 *   forge script script/Deploy.s.sol --rpc-url $RPC_URL --broadcast
 */
contract Deploy is Script {
    function run() external returns (RefundProtocolFixed escrow) {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address arbiter = vm.envAddress("ARBITER_ADDRESS");
        address token = vm.envAddress("FIAT_TOKEN_ADDRESS");
        string memory name = _envOr("EIP712_NAME", "RefundProtocolFixed");
        string memory version = _envOr("EIP712_VERSION", "1");

        vm.startBroadcast(pk);
        escrow = new RefundProtocolFixed(arbiter, token, name, version);
        vm.stopBroadcast();

        console2.log("RefundProtocolFixed deployed at:", address(escrow));
        console2.log("  arbiter   :", arbiter);
        console2.log("  fiatToken :", token);
        console2.log("  name      :", name);
        console2.log("  version   :", version);
    }

    function _envOr(string memory key, string memory dflt) internal view returns (string memory) {
        try vm.envString(key) returns (string memory v) { return v; } catch { return dflt; }
    }
}
