# HumdRegistry sidecar pattern

HumdRegistry is the immutable trust anchor for any hum subnet built on Reverb Protocol. The registry's identity layer never changes; extensions compose sideways.

## The split

The integrity-critical write path stays physically untouchable. HumdRegistry's advertise + ownership rule cannot be changed by an upgrade because HumdRegistry is not upgradeable. The application-specific state (reputation, bounty, future annotations) lives in upgradeable sibling contracts that are keyed against the same address space.

In Daman's deployment:

- **HumdRegistry**: immutable; the source of truth for bee identity in the daman subnet.
- **ReputationRegistry**: upgradeable sibling. Reads HumdRegistry's identity space; writes cumulative score per bee address. Score formula evolves through upgrade.
- **BountyAccrual**: upgradeable sibling. Reads HumdRegistry's identity space; writes per-claim funding state. Funding policy evolves through upgrade.

Both ReputationRegistry and BountyAccrual are sidecars: separate contracts, separate upgrade paths, no privilege over HumdRegistry's identity surface.

## Why the split

A unified upgradeable registry that both stores identity and tracks application state would have a single owner who can rewrite identity rules at upgrade time. A subnet built on that registry is owner-permissioned, not permissionless.

The split makes the subnet permissionless at the identity layer. Anyone can register a bee against HumdRegistry; no upgrade can revoke that. Application state then evolves freely without compromising the identity guarantee.

## Pattern documentation

The upstream maintainer reasoning is at [adiled/hum#39](https://github.com/adiled/hum/issues/39). The split pattern is the architectural decision Reverb Protocol adopts for any sidecar that wants to compose against HumdRegistry without being subject to its immutability.

## Operational implication

Future Daman annotations (additional scoring dimensions, new reward economies, performance attestations) ship as new sidecar contracts keyed against the same HumdRegistry address space. Bees discover the new surfaces via gossip on the daman subnet without HumdRegistry needing any upgrade.
