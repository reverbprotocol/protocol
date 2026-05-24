# FAQ

## What is Reverb Protocol?

The on-chain substrate for dispute-mediated commerce on Arc, plus the off-chain operating
model (forager-hive contract + persona-bee scaffolding + cross-product mesh conventions) that
consumer products use to operate at any autonomy level from L0 through L5.

See [Overview](/) and [Architecture](/architecture).

## Why is the dispute primitive forked from `circlefin/refund-protocol`?

The upstream contract had four classes of correctness issue documented at
[IRefundProtocol](/interfaces/IRefundProtocol#four-classes-of-fix): CEI ordering, cumulative
over-withdraw guard, debt-settle-before-early-withdraw, zero-recipient guard. The substrate
forks at commit `b506b17`, applies the four fixes, preserves the external surface, and ships
the result as the production-deploy artifact. The vendored upstream lives in `test/vendor/` for differential tests that prove the fixes change behavior without breaking compatibility.

## What's the difference between the substrate and the consumer products?

The substrate is the published standard: on-chain interfaces + off-chain operating model.
Consumer products (Reverb Markets, Daman) implement the standards with product-specific
behavior. The substrate is operator-trusted infrastructure (TimelockController-owned UUPS
proxies, Safe-multisig-gated pause); consumer products choose their own deployment posture
on top.

## Why a substrate plus consumer products instead of one monolithic protocol?

Two reasons:

1. **Cross-product compatibility.** Two consumer products following both layers get
   architectural compatibility for free. Daman's persona bees can observe Reverb Markets's
   disputes via the shared gossip-topic convention; both products' rulings flow through the
   shared `RefundProtocolFixed` proxy. See [Cooperative equilibrium examples](/OPERATING_MODEL#cooperative-equilibrium-examples).
2. **Independent governance.** The substrate's upgrade authority can be Timelock + Safe; a
   consumer product's upgrade authority can be different (its own Timelock + Safe, or a
   different governance shape entirely). The substrate doesn't tax consumer products with
   its governance assumptions.

## What's an autonomy level?

A position on the spectrum from "deterministic operator code" (L0) to "operator-spawned free
agent swarm" (L5) to "recursive agent economy" (L6, planned). See [Autonomy spectrum](/AUTONOMY_SPECTRUM). The substrate supports L0-L5 today; L6 is future scope.

Choose the level that fits the venture: higher autonomy means lower predictability, higher
coordination cost, and higher novelty signal.

## Why hum?

[hum](https://adiled.github.io/hum/) is the wire substrate for permissionless agent
participation. Reverb Protocol's operating-model layer rides on hum's chi vocabulary, thrum
transport, and HumdRegistry identity. The choice of hum over a bespoke wire is:

- **language-portability** at the wire level (clients in Rust, Go, TypeScript, Python),
  which makes L3 (heterogeneous external agents in any language) architecturally cheap;
- **immutable identity layer** (HumdRegistry on a subnet) that the substrate's reputation
  sidecar pattern composes against;
- **scenario methodology** (5-section prose specs paired 1:1 with sim tests) that the
  substrate inherits for its own scenarios.

## What's a forager?

A thrum-attached process that owns a resource and exposes operations as `chi:"tool-call"`-
addressable tools routed by humd. The substrate's canonical forager is `reverb-arc-fs` which
owns Arc chain state + wallet operations + EIP-712 signing. Consumer products extend it.

See [the forager hive contract](/OPERATING_MODEL#the-forager-hive-contract).

## What's a persona?

A thin asker. It assembles world state from gossip + chain events, opens a sid into a local
LLM worker, prompts the worker with a tight role-overlay system prompt, observes the
worker's tool calls flow through the local forager, and logs. The persona itself contains no
decision logic; the worker decides via tool calls.

See [the persona bee contract](/OPERATING_MODEL#the-persona-bee-contract).

## Where do private keys live?

Per-bee, in the forager's keyring at `~/.config/hum/{forager-name}/keyring.json`. File
permissions must be 0600 on unix; the substrate refuses to load otherwise. Each humd in a
multi-humd ensemble holds only the keys of the bees on that humd; no cross-humd key sharing.

Mainnet pre-requisite (documented but not executed in the current testnet posture): hardware-wallet integration for the keyring.

## How does the substrate handle upgrades?

`RefundProtocolFixed` is a UUPS proxy. Upgrade authority is gated by `_authorizeUpgrade` on
the owner (the TimelockController at `0xa22510860289751C092e67B15b827020CE09DAbf`). The
TimelockController has a 24-hour minimum delay; the Safe multisig is the sole proposer and
executor. Every upgrade is publicly queued for 24h before it can execute.

See [UUPS upgrade lifecycle](/scenarios/uups-upgrade) for the end-to-end flow.

## What happens during a pause?

`pause()` on the substrate is gated by the `pauser` role (the Safe directly, no Timelock delay; emergency stop). Pause-gated functions on `RefundProtocolFixed`: `pay`,
`refundByRecipient`, `refundByArbiter`, `withdraw`, `earlyWithdrawByArbiter`. Cleanup paths
(`settleDebt`, `depositArbiterFunds`, `withdrawArbiterFunds`, `setLockupSeconds`,
`updateRefundTo`) remain live so existing dispute lifecycles can complete.

`unpause()` is owner-gated through the Timelock (24h delay), so the unpause is auditable and
the operator commits to a wait before un-stopping.

## Is the substrate audited?

The current testnet posture has not been third-party audited. The substrate ships its own
hardening: stateful fuzz invariants on the dispute-settlement properties, selector + event
freeze tests, Slither on every PR with `fail-on=high`, Mythril nightly. See [Security posture](/security-posture).

External audit is the standard mainnet pre-requisite; the substrate's documented production-deploy guidance for consumers includes this pre-requisite.

## How do I integrate Reverb Protocol with my own LLM?

The substrate is LLM-agnostic. The worker layer (where the LLM lives) is the consumer's choice: `claude-cli`, `vercel-ai`, `openai-server`, `ollama-server`, or any custom worker that speaks the chi vocabulary on a sid.

The persona-base scaffolding's `AskerLoop` takes a `Transport` trait the runtime injects;
the transport speaks chi to whatever local worker bee the operator runs.

## What's the difference between an L1 consumer product and an L5 one?

At L1, the consumer's agent has hardcoded decision logic with optional LLM-shaped inputs.
The agent's goal structure is operator-fixed; the LLM's stochasticity is the only source of action diversity.

At L5, the consumer's agent receives a minimal seed guide and figures out its own role from
the live state. The agent population adapts faster than the operator can ship code.

Both ride on the same substrate; both inherit the same operating-model crates. The L5 upgrade
from L1 is documented for the Reverb Markets case study in [Reference deployments at L5](/AUTONOMY_SPECTRUM#reference-deployments-at-l5).

## Can I run my own subnet on hum?

Yes. Reverb Protocol's substrate operates on Arc; the off-chain operating model rides on hum. You can deploy your own `HumdRegistry` contract (a new subnet identity space) and operate against the substrate's interfaces with your own persona bees + forager extensions.

See hum's documentation at [adiled.github.io/hum](https://adiled.github.io/hum/) and the
[HumdRegistry sidecar pattern](/humd-registry-sidecar) for the reasoning.

## Where do I report a security issue?

Security disclosures via GitHub Security Advisories on the `reverbprotocol/protocol` repo.
Coordinated disclosure: please do not file public issues for live-deployment vulnerabilities.

## What's the license?

Apache-2.0 across the substrate, the forager crates, and the persona-base crate. Consumer
products choose their own license; both reference deployments (Reverb Markets, Daman) ship
Apache-2.0 as well.

## How can I contribute?

Open a PR against `reverbprotocol/protocol`. The contribution process matches the substrate's existing test discipline: every change should preserve the 53-of-53 Foundry tests + 21-of-21 Rust tests; new functionality needs new tests; the selector + event freeze tests guarantee no ABI drift slips through unintentionally.
