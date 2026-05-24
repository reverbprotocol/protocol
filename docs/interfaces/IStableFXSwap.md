# IStableFXSwap

Interface for atomic same-block stablecoin FX swaps. Implementations route through a specific venue (e.g. Circle StableFX FxEscrow on Arc) or aggregate across venues.

## Source

[`src/IStableFXSwap.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/IStableFXSwap.sol)

## External functions

```solidity
function quoteSwap(address from, address to, uint256 amount) external view returns (uint256);
function executeSwap(address from, address to, uint256 amount, uint256 minOut) external returns (uint256 amountOut);
```

`quoteSwap` is view and side-effect-free; consumers should treat the quote as an estimate and pass an explicit `minOut` on execution. `executeSwap` performs the swap and returns the actual out-amount, which must be at least `minOut` or revert.

## Events

```solidity
event SwapExecuted(
    address indexed account,
    address indexed fromAsset,
    address indexed toAsset,
    uint256 amountIn,
    uint256 amountOut
);
```

## Reference implementation

`FxEscrowAdapter` ([`src/reference/FxEscrowAdapter.sol`](https://github.com/reverbprotocol/protocol/blob/main/src/reference/FxEscrowAdapter.sol)) routes through Circle StableFX FxEscrow on Arc testnet at `0x867650F5eAe8df91445971f14d89fd84F0C9a9f8`. Reference: [`developers.circle.com/stablefx`](https://developers.circle.com/stablefx).

The adapter pulls the caller's `from` asset, approves FxEscrow, executes the swap, and forwards the resulting `to` asset back to the caller. Slippage guard via `minOut`. Intended for atomic same-block USDC <-> EURC settlement.

## Conformance

A multi-venue aggregator can declare `is IStableFXSwap` and route across StableFX, Uniswap, native AMMs, or other venues based on price. The consumer-facing surface stays stable.
