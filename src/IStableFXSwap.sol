// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

/// @title IStableFXSwap
/// @notice Interface for atomic same-block stablecoin FX swaps. Implementations route through
///         a specific venue (e.g., Circle StableFX FxEscrow on Arc) or aggregate across venues.
/// @dev    `quoteSwap` should be view and side-effect-free; `executeSwap` performs the swap and
///         returns the actual out-amount, which must be greater than or equal to `minOut` or
///         revert.
interface IStableFXSwap {
    /// @notice Emitted on every successful swap.
    event SwapExecuted(
        address indexed account,
        address indexed fromAsset,
        address indexed toAsset,
        uint256 amountIn,
        uint256 amountOut
    );

    /// @notice Quote the expected out-amount for swapping `amount` of `from` into `to`.
    /// @dev    View only. Implementations may return a stale or estimated value depending on
    ///         the venue; consumers should treat the quote as an estimate and pass an explicit
    ///         `minOut` on execution.
    function quoteSwap(address from, address to, uint256 amount) external view returns (uint256);

    /// @notice Swap `amount` of `from` into `to`, requiring at least `minOut` of `to` to be
    ///         received. Caller must have approved the implementation to pull `amount` of `from`.
    /// @return amountOut The actual amount of `to` returned to the caller.
    function executeSwap(address from, address to, uint256 amount, uint256 minOut)
        external
        returns (uint256 amountOut);
}
