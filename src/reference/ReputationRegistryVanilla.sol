// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

import {IReputationRegistry} from "../IReputationRegistry.sol";

/// @title ReputationRegistryVanilla
/// @notice Minimal reference implementation of `IReputationRegistry`. Recorders are designated
///         at construction; positive and negative deltas are configured at construction. Score
///         updates emit `ReputationUpdated`. No decay, no recorder rotation, no admin keys.
contract ReputationRegistryVanilla is IReputationRegistry {
    int256 public immutable upheldDelta;
    int256 public immutable rejectedDelta;

    mapping(address => bool) public isRecorder;
    mapping(address => int256) internal _score;

    error CallerNotRecorder();
    error ZeroAgent();
    error EmptyRecorderSet();
    error ZeroRecorder();
    error InvalidUpheldDelta();
    error InvalidRejectedDelta();

    constructor(address[] memory recorders, int256 _upheldDelta, int256 _rejectedDelta) {
        if (recorders.length == 0) revert EmptyRecorderSet();
        if (_upheldDelta <= 0) revert InvalidUpheldDelta();
        if (_rejectedDelta >= 0) revert InvalidRejectedDelta();

        upheldDelta = _upheldDelta;
        rejectedDelta = _rejectedDelta;

        for (uint256 i = 0; i < recorders.length; ++i) {
            address recorder = recorders[i];
            if (recorder == address(0)) revert ZeroRecorder();
            isRecorder[recorder] = true;
        }
    }

    modifier onlyRecorder() {
        if (!isRecorder[msg.sender]) revert CallerNotRecorder();
        _;
    }

    /// @inheritdoc IReputationRegistry
    function recordUpheld(address agent) external override onlyRecorder {
        if (agent == address(0)) revert ZeroAgent();
        int256 newScore = _score[agent] + upheldDelta;
        _score[agent] = newScore;
        emit ReputationUpdated(agent, upheldDelta, newScore);
    }

    /// @inheritdoc IReputationRegistry
    function recordRejected(address agent) external override onlyRecorder {
        if (agent == address(0)) revert ZeroAgent();
        int256 newScore = _score[agent] + rejectedDelta;
        _score[agent] = newScore;
        emit ReputationUpdated(agent, rejectedDelta, newScore);
    }

    /// @inheritdoc IReputationRegistry
    function reputationScore(address agent) external view override returns (int256) {
        return _score[agent];
    }
}
