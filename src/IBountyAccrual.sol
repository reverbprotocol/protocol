// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

/// @title IBountyAccrual
/// @notice Interface for the bounty-accrual primitive. A funder credits a recipient with a
///         claim amount; the recipient claims later. Consumer products decide funding policy
///         (percent-of-slash, percent-of-fee, flat, etc.) and call the accrual surface
///         accordingly. Implementations choose the bounty asset and any aggregation policy.
/// @dev    The claim surface is the user-facing direction; the accrual surface is included so a
///         meta-funder can interact with any conforming implementation. Consumer products are
///         free to expose richer accrual entry points in addition to `accrueBounty`.
interface IBountyAccrual {
    /// @notice Emitted when a funder accrues a new bounty claim.
    /// @param claimId   Sequential identifier assigned by the implementation.
    /// @param recipient The address eligible to claim the bounty.
    /// @param amount    Bounty notional in the implementation's funding asset.
    event BountyAccrued(uint256 indexed claimId, address indexed recipient, uint256 amount);

    /// @notice Emitted when a recipient claims an accrued bounty.
    /// @param claimId   The accrual identifier being claimed.
    /// @param recipient The address that received the bounty notional.
    /// @param amount    Bounty notional released.
    event BountyClaimed(uint256 indexed claimId, address indexed recipient, uint256 amount);

    /// @notice Credit `recipient` with `amount` of the implementation's bounty asset, callable by
    ///         a funder that has approved this contract to transfer `amount` of the asset.
    /// @param  recipient Address eligible to claim the bounty.
    /// @param  amount    Bounty notional, in the implementation's funding asset.
    /// @return claimId   The sequential identifier assigned to the new accrual.
    function accrueBounty(address recipient, uint256 amount) external returns (uint256 claimId);

    /// @notice Withdraw an accrued bounty as the designated recipient.
    /// @param  claimId Identifier returned from a prior `accrueBounty` call.
    function claimBounty(uint256 claimId) external;

    /// @notice Notional of an accrued bounty, regardless of claim status.
    function bountyAmount(uint256 claimId) external view returns (uint256);

    /// @notice Recipient eligible to claim a given bounty.
    function bountyRecipient(uint256 claimId) external view returns (address);

    /// @notice Whether a given bounty has already been claimed.
    function bountyClaimed(uint256 claimId) external view returns (bool);
}
