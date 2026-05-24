---
layout: home

hero:
  name: Reverb Protocol
  text: Substrate for dispute-mediated commerce on Arc
  tagline: Two layers, published as standard. On-chain interfaces + off-chain operating model. Consumer products inherit both for free.
  actions:
    - theme: brand
      text: Quickstart
      link: /quickstart
    - theme: alt
      text: Architecture
      link: /architecture
    - theme: alt
      text: Source on GitHub
      link: https://github.com/reverbprotocol/protocol

features:
  - title: Dispute primitive
    details: RefundProtocolFixed forks circlefin/refund-protocol@b506b17 with four classes of correctness fix. CEI ordering, cumulative over-withdraw guard, debt-settle-before-early-withdraw, zero-recipient guard. Live on Arc testnet behind a UUPS proxy.
  - title: Substrate primitives
    details: Seven interfaces (IRefundProtocol, IBountyAccrual, IReputationRegistry, ICCTPReceiver, IBondYieldVault, IStableFXSwap, IAttributable) with reference implementations under src/reference/. Each is the cross-consumer standard; conforming implementations are interchangeable.
  - title: Operating model
    details: Off-chain standard. reverb-arc-fs forager hive (wallet keyring, six-stage safety pipeline, scoping config) + persona-base scaffolding (PersonaBee trait, AskerLoop, ToolCallObserver, BustDetector). Consumer products extend with product-specific tools and personas.
  - title: Production controls
    details: UUPS upgradeable behind ERC1967Proxy. TimelockController (24h delay) owns upgrade authority; Safe multisig (3-of-5) fronts the Timelock and gates emergency pause. Selector + event freeze tests, stateful fuzz invariants, Slither static analysis in CI.
  - title: Autonomy spectrum
    details: Supports L0 (static operator code) through L5 (operator-spawned free agent swarm) on the agentic-autonomy spectrum. Two reference deployments operate at L5 on a shared humd ensemble.
  - title: Scenarios + guides
    details: Documentation pattern inherited from hum's scenarios (five-section prose specs paired 1:1 with sim tests). Three scenarios cover the substrate end-to-end; two guides walk through building consumer-product foragers + personas.
---

## Live on Arc testnet

Chain ID `5042002`. Deployed configuration in [`.deployments/arc-testnet.json`](https://github.com/reverbprotocol/protocol/blob/main/.deployments/arc-testnet.json).

| Surface | Address |
|---|---|
| RefundProtocolFixed proxy | `0xc8bF99c55703bc682a3Efd5c8A728EaEda3E121F` |
| RefundProtocolFixed implementation | `0xc4d76141CEA6b8D4b1bBF467e92d6a87F46C53Ed` |
| Owner (TimelockController) | `0xa22510860289751C092e67B15b827020CE09DAbf` |
| Pauser (Safe multisig 3-of-5) | `0x70a34ca4964a16a934432871a593acba5dd63cf1` |
| Safe singleton (v1.4.1) | `0x41675C099F32341bf84BFc5382aF534df5C7461a` |

## Consumers

Two reference deployments operate against this substrate:

- [`reverbprotocol/markets`](https://github.com/reverbprotocol/markets) — third-party prediction-market operator on Arc. Adopts the `IAttributable` convention on every fill entry point; consumes the dispute primitive on disputed market resolutions. Operates at L5 via `reverb-markets-arc-fs` + four sovereign persona bees.
- [`damanfi/copy-bond`](https://github.com/damanfi/copy-bond) — slash-bonded copy-trading consumer product. Declares conformance to `IBountyAccrual`, `IReputationRegistry`, and the `IAttributable` convention. Operates at L5 via `daman-arc-fs` + the Daman persona set.

Other parties may deploy their own conformant implementations of any interface with different economics, governance, or curation. See [Build your own forager](/guides/build-your-own-forager) and [Build your own persona](/guides/build-your-own-persona).

## Navigation

| Section | Content |
|---|---|
| [Quickstart](/quickstart) | Clone → configure → first persona bee sending a transaction in ~15 minutes |
| [Architecture](/architecture) | Three-layer conceptual overview + request lifecycle + multi-humd ensemble |
| [Autonomy spectrum](/AUTONOMY_SPECTRUM) | L0-L5 framework with per-level substrate-primitive mapping + decision criteria |
| [Operating model](/OPERATING_MODEL) | The off-chain published standard: forager hive contract + persona bee contract + cross-product mesh conventions |
| [Substrate primitives](/interfaces/) | Per-interface documentation: signatures, events, reference impls, conformance notes |
| [Scenarios](/scenarios/) | Three end-to-end scenarios in the five-section shape: dispute via hum, cross-product arbiter, UUPS upgrade lifecycle |
| [Guides](/guides/build-your-own-forager) | Walkthroughs for extending the operating model in your consumer product |
| [Security posture](/security-posture) | UUPS + Safe + Timelock + Pausable + freeze tests + fuzz invariants + Slither/Mythril CI |
| [HumdRegistry sidecar pattern](/humd-registry-sidecar) | The architectural split between immutable identity and upgradeable application state |
| [Glossary](/glossary) | Substrate + operating-model + production-deploy vocabulary |
| [FAQ](/faq) | Common questions: why this design, what's audited, how to integrate your own LLM |
