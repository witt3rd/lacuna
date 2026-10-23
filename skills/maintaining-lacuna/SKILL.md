---
name: maintaining-lacuna
description: Maintains the lacuna repository: house git workflow (iterate on master, rebase), CI gates (fmt, clippy, tests, docs), rung dependency pinning and the ladder invariants that must never be bypassed. Use when changing, reviewing, testing or releasing code in the lacuna repo.
metadata:
  home: ~/src/witt3rd/lacuna/skills/maintaining-lacuna
  aliases: [lacuna, lacuna-repo, lacuna-caretaker]
---

# lacuna — how we act in this tree

Charter: repo-root `AGENTS.md`. `README.md` is a symlink to `AGENTS.md` — edit only `AGENTS.md`.
Custody is `AGENTS.md` + `skills/`; no `.agent/` folder, no inhabit, no formal handoff.

## Git

House `fleet_git`. Mainline is `master`. Merge method: rebase.
Stage only files you touched; never `git add -A`.

- **Debugging / iteration**: work on `master`, commit small batches, never leave uncommitted work.
- **Session start**: `git checkout master && git pull origin master`.
- **Parallel features**: `git wt-new` → `lacuna.wt/<branch>/`; after merge `git wt-rm`.

## Invariants

1. **`VerifiedOpportunity` is ladder-gated**: Never add a public bypass constructor for `VerifiedOpportunity`. Every synthesized opportunity must pass through `lacuna_rung::synthesize_void` (`Spec => Abducting => { Mutate -> Abducting | Verified | Rejected }`).
2. **Interior Geodesic Search Only**: Never replace `geodesic_simplex_point` with unconstrained Euclidean perturbation. Points outside the convex hull of domain anchors suffer from high-dimensional boundary drift.
3. **Offline / CI Determinism**: `cargo test --workspace --locked` must pass with zero network calls and no API keys set in the environment. Live OpenRouter / System One calls (`--live-llm`, `--jev`) are opt-in flags or replayed via `rung_std::decide::Recorded` fixtures (`--fixture`).
4. **Doctest Gate**: `AGENTS.md` is compiled via `#![doc = include_str!("../../../README.md")]` in `lacuna-core`. Always label non-Rust fenced code blocks (`bash`, `text`, `json`).

## CI & Verification Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```
