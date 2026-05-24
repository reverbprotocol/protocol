// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import {IBountyAccrual} from "../IBountyAccrual.sol";

/// @title BountyAccrualVanilla
/// @notice Minimal reference implementation of `IBountyAccrual`. Funded by any caller that has
///         approved this contract to pull the bounty asset; recipients claim their accrued
///         amounts on demand. Consumer products choose funding policy by deciding when and how
///         much to call `accrueBounty`; this implementation has no opinion on the source.
/// @dev    Reference, not for production deploy. Wrap behind an ERC1967 proxy with UUPS-style
///         upgrade controls, mix in `PausableUpgradeable` on `accrueBounty` and `claimBounty`,
///         route owner authority through a `TimelockController` fronted by a Safe multisig, and
///         run the contract through static analysis (slither + mythril) and storage-layout CI
///         before any chain-side deploy. This reference is shipped for direct fork-and-adapt;
///         the on-chain controls are the consumer's responsibility.
contract BountyAccrualVanilla is IBountyAccrual, ReentrancyGuard {
    using SafeERC20 for IERC20;

    struct Claim {
        address recipient;
        uint256 amount;
        bool claimed;
    }

    IERC20 public immutable bountyAsset;
    uint256 public nextClaimId;
    mapping(uint256 => Claim) internal _claims;

    error BountyNotFound();
    error BountyAlreadyClaimed();
    error CallerNotRecipient();
    error ZeroRecipient();
    error ZeroAmount();
    error ZeroAsset();

    constructor(address _bountyAsset) {
        if (_bountyAsset == address(0)) revert ZeroAsset();
        bountyAsset = IERC20(_bountyAsset);
    }

    /// @inheritdoc IBountyAccrual
    function accrueBounty(address recipient, uint256 amount)
        external
        override
        nonReentrant
        returns (uint256 claimId)
    {
        if (recipient == address(0)) revert ZeroRecipient();
        if (amount == 0) revert ZeroAmount();

        claimId = nextClaimId;
        unchecked {
            nextClaimId = claimId + 1;
        }

        _claims[claimId] = Claim({recipient: recipient, amount: amount, claimed: false});

        bountyAsset.safeTransferFrom(msg.sender, address(this), amount);

        emit BountyAccrued(claimId, recipient, amount);
    }

    /// @inheritdoc IBountyAccrual
    function claimBounty(uint256 claimId) external override nonReentrant {
        Claim memory claim = _claims[claimId];

        if (claim.recipient == address(0)) revert BountyNotFound();
        if (claim.claimed) revert BountyAlreadyClaimed();
        if (msg.sender != claim.recipient) revert CallerNotRecipient();

        _claims[claimId].claimed = true;

        bountyAsset.safeTransfer(claim.recipient, claim.amount);

        emit BountyClaimed(claimId, claim.recipient, claim.amount);
    }

    /// @inheritdoc IBountyAccrual
    function bountyAmount(uint256 claimId) external view override returns (uint256) {
        return _claims[claimId].amount;
    }

    /// @inheritdoc IBountyAccrual
    function bountyRecipient(uint256 claimId) external view override returns (address) {
        return _claims[claimId].recipient;
    }

    /// @inheritdoc IBountyAccrual
    function bountyClaimed(uint256 claimId) external view override returns (bool) {
        return _claims[claimId].claimed;
    }
}
