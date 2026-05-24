---
layout: home

hero:
  name: Reverb Protocol
  text: Substrate for dispute-mediated commerce on Arc
  tagline: Interfaces are the standard; reference implementations are one valid impl. Other parties may deploy conformant alternatives.
  actions:
    - theme: brand
      text: Substrate primitives
      link: /interfaces/IRefundProtocol
    - theme: alt
      text: Reference implementations
      link: /reference/
    - theme: alt
      text: Source on GitHub
      link: https://github.com/reverbprotocol/protocol

features:
  - title: Dispute primitive
    details: RefundProtocolFixed forks circlefin/refund-protocol@b506b17 with four classes of correctness fix. CEI ordering, cumulative over-withdraw guard, debt-settle-before-early-withdraw, zero-recipient guard.
  - title: Substrate primitives
    details: Six namespaced interfaces (IBountyAccrual, IReputationRegistry, ICCTPReceiver, IBondYieldVault, IStableFXSwap, IAttributable) with reference implementations under src/reference/. Each is the cross-consumer standard; conforming implementations are interchangeable.
  - title: Production controls
    details: UUPS upgradeable behind ERC1967Proxy. TimelockController (24h delay) owns upgrade authority; Safe multisig (3-of-5) fronts the Timelock and gates emergency pause. Selector + event freeze tests, stateful fuzz invariants, Slither static analysis in CI.
---

## Live on Arc testnet

Chain ID `5042002`. Deployed configuration in [`.deployments/arc-testnet.json`](https://github.com/reverbprotocol/protocol/blob/main/.deployments/arc-testnet.json).

| Surface | Address |
|---|---|
| RefundProtocolFixed proxy | `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F` |
| RefundProtocolFixed implementation | `0xc4d76141CEA6b8D4b1bBF467e92d6a87F46C53Ed` |
| Owner (TimelockController) | `0xa22510860289751C092e67B15b827020CE09DAbf` |
| Pauser (Safe multisig 3-of-5) | `0x70a34ca4964a16a934432871a593acba5dd63cf1` |

## Consumers

- [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets) — third-party prediction-market operator on Arc. Adopts the `IAttributable` convention on every fill entry point; consumes the dispute primitive on disputed market resolutions.
- [`damanfi/copy-bond`](https://github.com/damanfi/copy-bond) — slash-bonded copy-trading consumer product. Declares conformance to `IBountyAccrual`, `IReputationRegistry`, and the `IAttributable` convention.

Other parties may deploy their own conformant implementations of any interface with different economics, governance, or curation.
