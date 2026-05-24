// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import {IBondYieldVault} from "../IBondYieldVault.sol";

/// @title IUSYCTeller
/// @notice Minimal interface against the USYC Teller. Arc-testnet deployment at
///         `0x9fdF14c5B14173D74C08Af27AebFf39240dC105A`.
///         Reference: `developers.circle.com/tokenized/usyc/subscribe-and-redeem`.
interface IUSYCTeller {
    function deposit(uint256 assets, address receiver) external returns (uint256 shares);
    function redeem(uint256 shares, address receiver, address owner) external returns (uint256 assets);
    function convertToAssets(uint256 shares) external view returns (uint256);
}

/// @title USYCBondVault
/// @notice Reference implementation of `IBondYieldVault` wrapping the USYC Teller. Subscribes
///         the underlying asset (USDC) to USYC via the Teller; tracks per-account principal
///         contribution; computes per-account claims pro-rata of the pool's USYC share balance.
///
/// @dev    USYC enforces a $100,000 minimum subscription per `deposit` call. The aggregation
///         policy decided at construction:
///
///         - STRICT: every `depositPrincipal` call must be at least `minSubscription` and is
///           subscribed to the Teller immediately. Suitable for institutional tiers.
///         - AGGREGATED: deposits accumulate in a pending bucket; once the bucket crosses
///           `minSubscription`, the entire bucket is subscribed in one call. Suitable for retail
///           tiers where individual contributions are below the minimum.
///
///         The accounting model is uniform across policies: each account holds a pro-rata claim
///         on the vault's total subscribed shares plus the un-subscribed pending bucket. On
///         withdrawal, the account's share is redeemed from the Teller and any pending portion
///         is returned directly without subscription.
///
///         Reference, not for production deploy. Wrap behind an ERC1967 proxy with UUPS-style
///         upgrade controls, mix in `PausableUpgradeable` on `depositPrincipal` and
///         `withdrawPrincipalWithYield`, route owner authority through a `TimelockController`
///         fronted by a Safe multisig, and run the contract through static analysis and
///         storage-layout CI before any chain-side deploy. The yield-bearing surface raises the
///         stakes: a bug here corrupts principal. This reference is shipped for direct
///         fork-and-adapt; production controls are the consumer's responsibility.
contract USYCBondVault is IBondYieldVault, ReentrancyGuard {
    using SafeERC20 for IERC20;

    enum AggregationPolicy {
        STRICT,
        AGGREGATED
    }

    IERC20 public immutable asset;
    IUSYCTeller public immutable teller;
    AggregationPolicy public immutable policy;
    uint256 public immutable minSubscription;

    mapping(address => uint256) public principalOf;
    uint256 public totalPrincipal;
    uint256 public pendingPrincipal;
    uint256 public totalShares;

    error WrongAsset();
    error ZeroAccount();
    error ZeroAmount();
    error BelowMinimumSubscription();
    error NoBalance();
    error ZeroAddress();

    constructor(address _asset, address _teller, AggregationPolicy _policy, uint256 _minSubscription) {
        if (_asset == address(0) || _teller == address(0)) revert ZeroAddress();
        asset = IERC20(_asset);
        teller = IUSYCTeller(_teller);
        policy = _policy;
        minSubscription = _minSubscription;
    }

    /// @inheritdoc IBondYieldVault
    function depositPrincipal(address depositAsset, uint256 amount, address account)
        external
        override
        nonReentrant
    {
        if (depositAsset != address(asset)) revert WrongAsset();
        if (amount == 0) revert ZeroAmount();
        if (account == address(0)) revert ZeroAccount();

        asset.safeTransferFrom(msg.sender, address(this), amount);

        principalOf[account] += amount;
        totalPrincipal += amount;

        if (policy == AggregationPolicy.STRICT) {
            if (amount < minSubscription) revert BelowMinimumSubscription();
            _subscribe(amount);
        } else {
            pendingPrincipal += amount;
            if (pendingPrincipal >= minSubscription) {
                uint256 toSubscribe = pendingPrincipal;
                pendingPrincipal = 0;
                _subscribe(toSubscribe);
            }
        }

        emit PrincipalDeposited(account, address(asset), amount);
    }

    /// @inheritdoc IBondYieldVault
    function withdrawPrincipalWithYield(address account) external override nonReentrant {
        uint256 accountPrincipal = principalOf[account];
        if (accountPrincipal == 0) revert NoBalance();

        uint256 sharesToRedeem = (accountPrincipal * totalShares) / totalPrincipal;
        uint256 pendingShare = (accountPrincipal * pendingPrincipal) / totalPrincipal;

        principalOf[account] = 0;
        totalPrincipal -= accountPrincipal;
        totalShares -= sharesToRedeem;
        pendingPrincipal -= pendingShare;

        uint256 redeemed = 0;
        if (sharesToRedeem > 0) {
            redeemed = teller.redeem(sharesToRedeem, address(this), address(this));
        }

        uint256 totalOut = redeemed + pendingShare;
        asset.safeTransfer(account, totalOut);

        uint256 yieldAmount = totalOut > accountPrincipal ? totalOut - accountPrincipal : 0;
        emit PrincipalWithdrawn(account, accountPrincipal, yieldAmount);
    }

    /// @inheritdoc IBondYieldVault
    function accruedYield(address account) external view override returns (uint256) {
        uint256 accountPrincipal = principalOf[account];
        if (accountPrincipal == 0 || totalPrincipal == 0) return 0;

        uint256 accountShares = (accountPrincipal * totalShares) / totalPrincipal;
        uint256 redeemableEquivalent = teller.convertToAssets(accountShares);
        uint256 pendingShare = (accountPrincipal * pendingPrincipal) / totalPrincipal;
        uint256 totalValue = redeemableEquivalent + pendingShare;

        if (totalValue <= accountPrincipal) return 0;
        return totalValue - accountPrincipal;
    }

    function _subscribe(uint256 amount) internal {
        asset.forceApprove(address(teller), amount);
        uint256 shares = teller.deposit(amount, address(this));
        totalShares += shares;
    }
}
