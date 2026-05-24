# IReputationRegistry

Interface for the reputation-registry primitive. A designated recorder records uphold or reject outcomes against an agent address; the registry exposes the cumulative score. Implementations choose the score deltas, the recorder authorization model, and any decay policy.

## Source

[`src/IReputationRegistry.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IReputationRegistry.sol)

## External functions

```solidity
function recordUpheld(address agent) external;
function recordRejected(address agent) external;
function reputationScore(address agent) external view returns (int256);
```

## Events

```solidity
event ReputationUpdated(address indexed agent, int256 delta, int256 newScore);
```

## Reference implementation

`ReputationRegistryVanilla` ([`src/reference/ReputationRegistryVanilla.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/reference/ReputationRegistryVanilla.sol)). Recorders are designated at construction; positive and negative deltas are configured at construction. Score updates emit `ReputationUpdated`. No decay, no recorder rotation, no admin keys.

`reputationScore` returns a signed integer so an agent with more rejections than upholds carries a negative score. Consumers may treat any non-positive score as de-listing without protocol intervention.

## Conformance

A consumer product wanting score decay, ELO-style adjustment, or domain-specific weighting ships its own implementation declaring `is IReputationRegistry`. Tooling that reads `reputationScore(address)` operates against any conforming impl.
