// SPDX-License-Identifier: Apache-2.0
/*
 * Copyright 2026 project-reverb
 * Licensed under the Apache License, Version 2.0.
 */

pragma solidity ^0.8.24;

/// @title IReputationRegistry
/// @notice Interface for the reputation-registry primitive. A designated recorder records uphold
///         or reject outcomes against an agent address; the registry exposes the cumulative score.
///         Implementations choose the score deltas, the recorder authorization model, and any
///         decay policy.
/// @dev    `reputationScore` returns a signed integer so an agent with more rejections than
///         upholds can carry a negative score. Consumers may treat any non-positive score as
///         de-listing without protocol intervention.
interface IReputationRegistry {
    /// @notice Emitted whenever the score for `agent` changes.
    /// @param agent    Address whose score was updated.
    /// @param delta    Signed change applied to the previous score.
    /// @param newScore Resulting cumulative score after the delta.
    event ReputationUpdated(address indexed agent, int256 delta, int256 newScore);

    /// @notice Record an upheld outcome against `agent`; applies the implementation's positive delta.
    /// @dev    Callable only by an address authorized as a recorder at construction.
    function recordUpheld(address agent) external;

    /// @notice Record a rejected outcome against `agent`; applies the implementation's negative delta.
    /// @dev    Callable only by an address authorized as a recorder at construction.
    function recordRejected(address agent) external;

    /// @notice Cumulative reputation score for `agent`. Zero is the default for never-recorded agents.
    function reputationScore(address agent) external view returns (int256);
}
