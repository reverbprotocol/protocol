# Scenarios

Prose specs of substrate-level usage patterns. One story per file, each pairing 1:1 with a
sim test in the runtime that exercises it. Pattern inherited from [hum's scenarios](https://adiled.github.io/hum/scenarios/); five-section structure: setup, happy path, failure modes, success criteria, validation scope.

Scenarios live at the substrate level, not the consumer-product level. They exercise the
on-chain interfaces + the off-chain operating model jointly. Consumer products may author
their own scenarios under their own `docs/scenarios/` sites.

## Current scenarios

| Scenario | Premise |
|---|---|
| [Dispute via hum](/scenarios/dispute-via-hum) | A watchdog persona observes a degradation, gossips a slash-claim, and a forager routes the claim to `RefundProtocolFixed` on Arc. |
| [Cross-product arbiter](/scenarios/cross-product-arbiter) | One arbiter persona rules on disputes from two consumer products on a shared humd ensemble. |
| [UUPS upgrade lifecycle](/scenarios/uups-upgrade) | The full Safe + Timelock + UUPS upgrade path end-to-end, with the queued-upgrade visibility property holding throughout. |

## Why scenarios

The substrate publishes interfaces and contracts. The operating model publishes the off-chain wire. Both are necessary; neither is sufficient as documentation for what the substrate **does** in a typical deployment. Scenarios fill the gap: they show a sequence of events that exercises both layers together, in the same shape every time, so a reader who learns the pattern once can navigate any scenario predictably.

The 1:1 pairing with sim tests enforces that the prose stays accurate. If the test asserts something the prose does not, either document can be corrected to restore alignment; both are equally authoritative.
