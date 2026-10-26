# Architecture notes — foundation crate audit

Audit date: 2026-10-26. Platform: Windows 11, stable Rust 1.97. Checks run with
`cargo check --no-default-features` on the default branch of each repo, shallow clone.
macOS and Linux builds have not been checked yet.

## Audit summary

| Crate | Commit (short) | Licence | Builds (features off) | Notes |
|-------|----------------|---------|-----------------------|-------|
| `tpt-swarm` | a7d6e0c | **Apache-2.0 only** | `swarm-core` fails | Violates spec §5 (dual licence). Build error in `fed-client`. |
| `tpt-perception` | 5b07204 | MIT OR Apache-2.0 | fails | Path dep expects `tpt-math-geometry` `^0.1.1`, which does not resolve against the checked-out `tpt-math`. |
| `tpt-control` | 10e4320 | MIT OR Apache-2.0 | passes | |
| `tpt-keystone-db` | b861edd | MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`) | passes (`tpt-keystone` crate) | Audited the `tpt-keystone` crate only. |
| `tpt-appfront` | 6146526 | MIT OR Apache-2.0 | passes | |
| `tpt-anima` | 408c2e0 | MIT OR Apache-2.0 | passes | Workspace of 11 crates; only the root was checked. |
| `tpt-math` | a6c659d | MIT OR Apache-2.0 | umbrella fails; sub-crates pass except two | Umbrella does not build: `tpt-math-exact` (two `num-bigint` versions in the tree) and `tpt-math-prob-dist` (`rand_distr` 0.6.0 bump, see below). The crates Fleet uses (`tpt-math-numeric`, `-geometry`, `-linalg-dense`, `-linalg-fixed`) build. Fleet pins `tpt-math-numeric` at `a6c659d`. |
| `tpt-primitives` | 336292c | MIT OR Apache-2.0 | passes | Edition 2024, MSRV 1.88. |
| `tpt-crypto` | 8b73d9b | MIT OR Apache-2.0 | passes | |

All nine repositories exist at `github.com/TPT-Solutions/<crate>`.

## Actions required before Phase 1

- `tpt-swarm`: decide whether to wrap it behind a trait in `fleet-core` (per the "wrap or log"
  rule in `TODO.md`) or ask the owner for a dual licence. Its `swarm-core` build is also broken.
- `tpt-perception`: pin a `tpt-math` revision that provides `tpt-math-geometry` 0.1.x, or
  report upstream.
- `tpt-math` (upstream fixes, owned by this org). A fix commit is ready on branch
  `fix/rand-distr-num-bigint-dedup` in the local audit clone (commit `60a1d28`) and as a patch
  file. It is not pushed yet: pushing needs GitHub credentials this session does not have.
  - `tpt-math-prob-dist` pins `rand_distr` 0.6.0 (Dependabot PR #9, merged 2026-10-20), but its
    comment says `rand` 0.9 pairs with `rand_distr` 0.5. Reverting to 0.5 is the likely fix.
  - `tpt-math-exact` has two `num-bigint` versions (0.4.8 and 0.5.1) and mixes their `BigInt` types.
- `tpt-perception`: expects `tpt-math-geometry` `^0.1.1`, but `tpt-math` at this rev has 0.2.0.
- Re-run the audit on macOS and Linux.
- Record public APIs for the passing crates (not yet done).

## Public API notes

Not yet recorded. To be filled in once the blocking build issues above are resolved.
