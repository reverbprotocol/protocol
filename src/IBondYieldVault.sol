// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

/// @title IBondYieldVault
/// @notice Interface for a yield-bearing vault that takes principal in one asset and returns
///         principal plus accrued yield in the same asset on withdrawal. Implementations decide
///         the underlying yield source (e.g., a tokenized treasury wrapper), the aggregation
///         policy across accounts, and any minimum-subscription thresholds.
/// @dev    `depositPrincipal` takes the asset address explicitly so the same interface can serve
///         multi-asset vaults; single-asset implementations may revert on mismatched asset.
interface IBondYieldVault {
    /// @notice Emitted when principal is credited to `account`.
    event PrincipalDeposited(address indexed account, address indexed asset, uint256 amount);

    /// @notice Emitted when an account withdraws principal plus accrued yield.
    event PrincipalWithdrawn(address indexed account, uint256 principal, uint256 yieldAmount);

    /// @notice Deposit `amount` of `asset` on behalf of `account`. Caller must have approved
    ///         this vault to pull `amount` of `asset`.
    function depositPrincipal(address asset, uint256 amount, address account) external;

    /// @notice Withdraw an account's full principal plus accrued yield to that account.
    function withdrawPrincipalWithYield(address account) external;

    /// @notice Current accrued yield for `account`, denominated in the vault's asset.
    function accruedYield(address account) external view returns (uint256);
}
