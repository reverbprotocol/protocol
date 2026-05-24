# IBondYieldVault

Interface for a yield-bearing vault that takes principal in one asset and returns principal plus accrued yield in the same asset on withdrawal. Implementations decide the underlying yield source (e.g. a tokenized treasury wrapper), the aggregation policy across accounts, and any minimum-subscription thresholds.

## Source

[`src/IBondYieldVault.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IBondYieldVault.sol)

## External functions

```solidity
function depositPrincipal(address asset, uint256 amount, address account) external;
function withdrawPrincipalWithYield(address account) external;
function accruedYield(address account) external view returns (uint256);
```

## Events

```solidity
event PrincipalDeposited(address indexed account, address indexed asset, uint256 amount);
event PrincipalWithdrawn(address indexed account, uint256 principal, uint256 yieldAmount);
```

## Reference implementation

`USYCBondVault` ([`src/reference/USYCBondVault.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/reference/USYCBondVault.sol)) wraps the USYC Teller on Arc testnet at `0x9fdF14c5B14173D74C08Af27AebFf39240dC105A`. Reference: [`developers.circle.com/tokenized/usyc/subscribe-and-redeem`](https://developers.circle.com/tokenized/usyc/subscribe-and-redeem).

USYC enforces a $100,000 minimum subscription per `deposit` call. The aggregation policy is configured at construction:

- **STRICT**: every `depositPrincipal` call must meet the minimum and is subscribed to the Teller immediately. Suitable for institutional tiers.
- **AGGREGATED**: deposits accumulate in a pending bucket; once the bucket crosses the minimum, the entire bucket is subscribed in one call. Suitable for retail tiers where individual contributions are below the minimum.

Accounting is uniform across policies: each account holds a pro-rata claim on the vault's total subscribed shares plus the un-subscribed pending bucket. On withdrawal, the account's share is redeemed from the Teller and any pending portion is returned directly without subscription.

## Conformance

Implementations targeting BUIDL, other tokenized-treasury wrappers, or non-Circle yield sources may declare `is IBondYieldVault` and serve as a drop-in replacement.
