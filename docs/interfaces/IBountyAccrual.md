# IBountyAccrual

Interface for the bounty-accrual primitive. A funder credits a recipient with a claim amount; the recipient claims later. Consumer products choose funding policy (percent-of-slash, percent-of-fee, flat, etc.) and call the accrual surface accordingly.

## Source

[`src/IBountyAccrual.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IBountyAccrual.sol)

## External functions

```solidity
function accrueBounty(address recipient, uint256 amount) external returns (uint256 claimId);
function claimBounty(uint256 claimId) external;

function bountyAmount(uint256 claimId) external view returns (uint256);
function bountyRecipient(uint256 claimId) external view returns (address);
function bountyClaimed(uint256 claimId) external view returns (bool);
```

## Events

```solidity
event BountyAccrued(uint256 indexed claimId, address indexed recipient, uint256 amount);
event BountyClaimed(uint256 indexed claimId, address indexed recipient, uint256 amount);
```

## Reference implementation

`BountyAccrualVanilla` ([`src/reference/BountyAccrualVanilla.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/reference/BountyAccrualVanilla.sol)). Funder approves the contract to pull the bounty asset; recipient claims by `claimId`. Funding asset fixed at construction. No admin keys, no decay.

The reference is suitable for direct fork-and-adapt; production-deploy controls (UUPS proxy with `PausableUpgradeable` on the entry points, owner authority routed through a TimelockController behind a Safe multisig) are the consumer's responsibility.

## Conformance

Consumer products with product-specific economics (e.g. a slash-bonded copy-trading product's 10/90 split between slash treasury and watchdog bounty) ship their own implementation declaring `is IBountyAccrual`. Cross-consumer compatibility is the standard's purpose.
