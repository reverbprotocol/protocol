// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import {IStableFXSwap} from "../IStableFXSwap.sol";

/// @title IFxEscrow
/// @notice Minimal interface against Circle StableFX FxEscrow. Arc-testnet deployment at
///         `0x867650F5eAe8df91445971f14d89fd84F0C9a9f8`.
///         Reference: `developers.circle.com/stablefx`.
interface IFxEscrow {
    function quote(address fromAsset, address toAsset, uint256 amount) external view returns (uint256);
    function swap(address fromAsset, address toAsset, uint256 amount, address recipient)
        external
        returns (uint256 amountOut);
}

/// @title FxEscrowAdapter
/// @notice Reference adapter routing `IStableFXSwap` calls through the StableFX FxEscrow engine.
///         Pulls the caller's `from` asset, approves FxEscrow, executes the swap, and forwards
///         the resulting `to` asset back to the caller.
/// @dev    Intended for atomic same-block USDC <-> EURC settlement on Arc. The reference engine
///         enforces venue-side liquidity and pricing; this adapter is a thin wrapper.
///
///         Reference, not for production deploy. Wrap behind an ERC1967 proxy with UUPS-style
///         upgrade controls, mix in `PausableUpgradeable` on `executeSwap`, route owner
///         authority through a `TimelockController` fronted by a Safe multisig, and run the
///         contract through static analysis and storage-layout CI before any chain-side deploy.
///         The slippage guard is the minimum production control; additional controls (per-asset
///         caps, per-account rate limits, oracle deviation guards on the quoted price) live in
///         the consumer's wrapper. This reference is shipped for direct fork-and-adapt;
///         production controls are the consumer's responsibility.
contract FxEscrowAdapter is IStableFXSwap, ReentrancyGuard {
    using SafeERC20 for IERC20;

    IFxEscrow public immutable fxEscrow;

    error SlippageExceeded(uint256 received, uint256 minOut);
    error ZeroAddress();
    error ZeroAmount();

    constructor(address _fxEscrow) {
        if (_fxEscrow == address(0)) revert ZeroAddress();
        fxEscrow = IFxEscrow(_fxEscrow);
    }

    /// @inheritdoc IStableFXSwap
    function quoteSwap(address from, address to, uint256 amount) external view override returns (uint256) {
        return fxEscrow.quote(from, to, amount);
    }

    /// @inheritdoc IStableFXSwap
    function executeSwap(address from, address to, uint256 amount, uint256 minOut)
        external
        override
        nonReentrant
        returns (uint256 amountOut)
    {
        if (from == address(0) || to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();

        IERC20(from).safeTransferFrom(msg.sender, address(this), amount);
        IERC20(from).forceApprove(address(fxEscrow), amount);

        amountOut = fxEscrow.swap(from, to, amount, address(this));
        if (amountOut < minOut) revert SlippageExceeded(amountOut, minOut);

        IERC20(to).safeTransfer(msg.sender, amountOut);

        emit SwapExecuted(msg.sender, from, to, amount, amountOut);
    }
}
