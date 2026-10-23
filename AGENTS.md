# AGENTS.md — lacuna

Domain-agnostic high-dimensional whitespace analyzer, finder, visualizer, and explorer.
Built in Rust on [`rung`](https://github.com/witt3rd/rung) (`ladder!`, `rung_std::decide`, `rung_std::llm`, `rung-agent-core`) and `wgpu`/WGSL shaders.

**Live Interactive Demo (GitHub Pages):** <https://witt3rd.github.io/lacuna/>

> `README.md` is a **symlink to this file**. Edit only `AGENTS.md`.
> Custody is `AGENTS.md` + `skills/`; no `.agent/` folder, no inhabit, no formal handoff.

---

## Goals (the problem)

If you embed 100,000 domain entities (Steam games, startups, patents, molecules) into a single vector space and naively pick a point "far away from neighbors," three failure modes immediately occur:

1. **Off-Manifold Boundary Drift**: In high dimensions ($D \ge 64$), unconstrained distance maximization pushes points onto the exterior boundary of the hypersphere—orthogonal to the entire domain manifold.
2. **Surface vs. Structural Entanglement**: Standard text embeddings conflate surface theme/setting (e.g., *cyberpunk*, *medieval*, *zombies*) with structural mechanics (e.g., *core gameplay loop*, *progression*, *social topology*).
3. **Dead Voids vs. Gold Voids**: Many regions of concept space are empty because they are physically impossible, un-fun, or commercially dead.

`lacuna` solves all three:
- **Four-Layer Universal Input Contract**: Decomposes every entity into orthogonal semantic facets (`FacetSpec`), unit-normalized per-facet embeddings, and explicit demand + quality signals.
- **Interior Delaunay Empty-Sphere Solver**: Restricts search to the *geodesic interior simplex* of diverse, high-demand anchor clusters (`optimize_interior_simplex_center`), maximizing the empty-sphere radius $R(x^*) = \min_{e \in \text{Corpus}} d(x^*, e)$ while guaranteeing $x^*$ remains surrounded by valid domain clusters.
- **`rung::ladder!` Closed-Loop Synthesis & `rung_std::decide` Viability Gate**: Abducts concrete opportunity hypotheses (`DraftHypothesis`), scores them on atomic probability questions via `rung_std::decide::Decider` (`JevDecider` / `Recorded` / `CalibratedGeometricDecider`), and **re-embeds the generated description** to prove mathematically that its vector lands inside the void and stays separated from all existing corpus entities (`VerifiedOpportunity`).
- **Continuous Topographic Potential Field (`wgpu` WGSL + SVG/WebGPU HTML)**: Evaluates incumbent saturation density against annular demand gravity to illuminate glowing cyan whitespace basins and bounded Voronoi cells.

---

## Merits (what is load-bearing)

- **The compiler gates `VerifiedOpportunity`.** `lacuna_rung::synthesize_void` is a `rung::ladder!`. You cannot construct a `VerifiedOpportunity` without passing through `Abducting`, the `rung_std::decide::Decider` viability check, and the closed-loop re-embedding distance proof.
- **Interior geodesic containment.** Candidate void centers are strictly positive convex combinations of anchor facet vectors on the unit hypersphere (`geodesic_simplex_point`), preventing off-manifold drift.
- **Agent-first CLI (`lacuna`).** Every operation—schema inspection, corpus ingestion, high-D void solving, single-void probing, `rung` ladder synthesis, and topographic field rendering—is exposed as a deterministic CLI subcommand with `--json` output so an autonomous agent (`rung-agent`) can pilot the entire workflow.
- **Zero-key offline determinism + live `rung` scaling.** Deterministic locality-sensitive n-gram embeddings (`deterministic_embed`) and `CalibratedGeometricDecider` let every test and CLI command run with zero network or credentials; passing `--live-llm` and `--jev` dispatches through `rung_std::llm::llm_call` and `rung_std::decide::JevDecider` (OpenRouter / System One).

---

## Workspace

```text
crates/lacuna-core   Pure-Rust (wasm32-compatible) schema, multi-facet metric space,
                     interior Delaunay empty-sphere solver, 2D Sutherland-Hodgman
                     bounded Voronoi tessellation, potential field grid, and Steam benchmark
crates/lacuna-rung   rung::ladder! (SynthesizeVoid), rung_std::decide viability judge,
                     rung_std::llm abduction, and rung-agent-core turn engine bridge
crates/lacuna-gpu    WGSL topographic potential-field shader (field.wgsl), SVG vector
                     manifold renderer, and self-contained WebGPU/Shader HTML explorer
crates/lacuna-cli    The `lacuna` binary: 100% agent-pilotable CLI (--json everywhere)
crates/lacuna-wasm   wasm32 C-ABI JSON bridge (`lacuna_call`) to lacuna-core for in-browser
                     discovery; no wasm-bindgen, no imports, deterministic
```

Numeric decision domains (farm inputs, sale timing, contract mix) use
`lacuna_core::numeric`: an invertible encoding whose void centers decode back to
concrete parameter values. See [`searching-farm-strategy-space`](skills/searching-farm-strategy-space/SKILL.md).

---

## Getting Started (Compiled Doctest)

The crate rustdoc compiles and runs `README.md` (`AGENTS.md`). Unlabeled fences are treated as Rust—label non-Rust blocks.

```rust
use lacuna_core::{VoidSolverConfig, find_interior_voids, min_distance_to_corpus, steam_seed_snapshot};

fn main() {
    let snapshot = steam_seed_snapshot();
    let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &VoidSolverConfig::default());
    assert!(!voids.is_empty());

    let top = &voids[0];
    let weights = snapshot.schema.default_weights();
    let min_d = min_distance_to_corpus(&top.center, &snapshot.entities, &weights);
    assert!((min_d - top.radius).abs() < 1e-3);
}
```

---

## Agent-Pilotable CLI (`lacuna`)

```bash
# 1. Inspect the 5-axis domain schema
cargo run -q -p lacuna-cli -- schema

# 2. Solve & rank top interior Delaunay empty spheres (human table or --json)
cargo run -q -p lacuna-cli -- voids --top-k 5
cargo run -q -p lacuna-cli -- voids --top-k 5 --json

# 3. Probe a specific void's boundary neighbors, Voronoi polygon & facet bridge
cargo run -q -p lacuna-cli -- probe void-01 --json

# 4. Run the rung::ladder! synthesis + rung_std::decide judge + closed-loop proof
cargo run -q -p lacuna-cli -- synthesize void-01 --json

# 5. Run the flagship @SyntheticBeef Steam Games Whitespace Explorer & emit SVG + WebGPU HTML into out/
cargo run -q -p lacuna-cli -- demo steam --count 60 --top-k 5 --svg out/steam_manifold.svg --html out/steam_explorer.html
```

---

## Commands & CI Gate

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo build -p lacuna-wasm --target wasm32-unknown-unknown --profile wasm --locked
```

---

## Skills (`skills/`)

| Skill | One Line |
|---|---|
| [`maintaining-lacuna`](skills/maintaining-lacuna/SKILL.md) | Repo caretaker: house git (`master` iteration, rebase), CI gates, `rung` tag pinning, release procedure |
| [`piloting-lacuna-cli`](skills/piloting-lacuna-cli/SKILL.md) | How an agent pilots `lacuna` via CLI (`ingest → voids → probe → synthesize → field`) with `--json` |
| [`onboarding-lacuna-domains`](skills/onboarding-lacuna-domains/SKILL.md) | Onboarding any domain (VC, Patents, Pharma, SaaS, Research) onto the 4-layer input contract |
| [`searching-farm-strategy-space`](skills/searching-farm-strategy-space/SKILL.md) | lacuna as the whitespace-search stage of the CAPPZ HARVEST farm economic discovery engine |
| [`running-steam-benchmark`](skills/running-steam-benchmark/SKILL.md) | Playbook for the 100,000+ Steam Games Whitespace Explorer (`@SyntheticBeef` challenge) |
