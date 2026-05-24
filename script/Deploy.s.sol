// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {RefundProtocolFixed} from "../src/RefundProtocolFixed.sol";

/**
 * Atomic UUPS deploy of RefundProtocolFixed against the shared Safe + TimelockController on
 * Arc testnet. Reads the Safe + Timelock addresses from .deployments/arc-testnet.json (sibling
 * to this script's repo) per the shared-Safe-testnet bonus decision; the deployer EOA never
 * holds owner authority past this script.
 *
 * Pipeline (single broadcast):
 *   1. Deploy RefundProtocolFixed implementation.
 *   2. Deploy ERC1967Proxy pointing at the implementation.
 *   3. Call initialize on the proxy with:
 *        - arbiter        = ARBITER_ADDRESS
 *        - fiatToken      = FIAT_TOKEN_ADDRESS
 *        - eip712Name     = EIP712_NAME
 *        - eip712Version  = EIP712_VERSION
 *        - owner          = TIMELOCK_ADDRESS (Timelock controls upgrades + unpause)
 *        - pauser         = SAFE_ADDRESS     (Safe pauses without Timelock delay)
 *   4. Read proxy.owner() and assert it equals TIMELOCK_ADDRESS.
 *   5. Print proxy + implementation + owner + pauser; caller writes
 *      .deployments/arc-testnet.json post-broadcast.
 *
 * Required env:
 *   DEPLOYER_PRIVATE_KEY   - hex-encoded EVM private key
 *   ARBITER_ADDRESS        - EOA or multisig granted onlyArbiter rights
 *   FIAT_TOKEN_ADDRESS     - the ERC-20 to escrow (USDC on the target chain)
 *   SAFE_ADDRESS           - the deployed Safe multisig (pauser)
 *   TIMELOCK_ADDRESS       - the deployed TimelockController (owner)
 *
 * Optional env:
 *   EIP712_NAME           - default "RefundProtocolFixed"
 *   EIP712_VERSION        - default "1"
 *
 * Deploy with:
 *   forge script script/Deploy.s.sol --rpc-url $RPC_URL --broadcast
 */
contract Deploy is Script {
    error OwnershipMismatch(address expected, address actual);

    function run() external returns (address proxyAddr, address implAddr) {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address arbiter = vm.envAddress("ARBITER_ADDRESS");
        address token = vm.envAddress("FIAT_TOKEN_ADDRESS");
        address safe = vm.envAddress("SAFE_ADDRESS");
        address timelock = vm.envAddress("TIMELOCK_ADDRESS");
        string memory name = _envOr("EIP712_NAME", "RefundProtocolFixed");
        string memory version = _envOr("EIP712_VERSION", "1");

        vm.startBroadcast(pk);

        RefundProtocolFixed impl = new RefundProtocolFixed();

        ERC1967Proxy proxy = new ERC1967Proxy(
            address(impl),
            abi.encodeCall(
                RefundProtocolFixed.initialize,
                (arbiter, token, name, version, timelock, safe)
            )
        );

        vm.stopBroadcast();

        proxyAddr = address(proxy);
        implAddr = address(impl);

        // Verify on-chain that ownership ended at the TimelockController; revert the run if not.
        address actualOwner = RefundProtocolFixed(proxyAddr).owner();
        if (actualOwner != timelock) revert OwnershipMismatch(timelock, actualOwner);

        console2.log("RefundProtocolFixed proxy           :", proxyAddr);
        console2.log("RefundProtocolFixed implementation  :", implAddr);
        console2.log("  owner (TimelockController)       :", actualOwner);
        console2.log("  pauser (Safe)                    :", safe);
        console2.log("  arbiter                          :", arbiter);
        console2.log("  fiatToken                        :", token);
    }

    function _envOr(string memory key, string memory dflt) internal view returns (string memory) {
        try vm.envString(key) returns (string memory v) { return v; } catch { return dflt; }
    }
}
