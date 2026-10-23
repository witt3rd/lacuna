---
name: searching-farm-strategy-space
description: Uses lacuna as the whitespace-search stage of a farm economic discovery engine: numeric decision facets, the invertible numeric encoding, decoding void centers into candidate strategies, signal choices and the browser lacuna-wasm build. Use when connecting lacuna to farm or other numeric decision spaces, building lacuna-wasm, or deciding what a void means economically.
metadata:
  home: ~/src/witt3rd/lacuna/skills/searching-farm-strategy-space
  aliases: [farm-economics, lacuna-ag, harvest-discovery]
---

# lacuna as a farm economic discovery engine

lacuna finds **unexplored, feasible regions** of a decision space. It does not
value them. In CAPPZ HARVEST (`cappz/src/lib/harvest/`) the split is:

| Stage | Owner | Output |
|---|---|---|
| Farm economic state, baseline P&L | cappz deterministic TypeScript | `Π_i = A_i(Y_i·P_i − C_v,i) − C_f,i` |
| Candidate generation (rules + optimization) | cappz | parameterized strategies |
| **Whitespace search over strategies** | **lacuna (`lacuna-wasm`, in browser)** | void centers = interpolated strategies no one has evaluated |
| Incremental value, risk, downside | cappz Monte Carlo | `S = E[ΔΠ] − λ·Risk − C_impl` |
| Verification | farmer outcome vs. pre-declared counterfactual | observed result |

An empty region is only a hypothesis. A void becomes a recommendation only after
the economic engine prices it; it becomes *verified* only after an observed outcome.

## Facets (the six-axis schema)

`economic`, `agronomic`, `market`, `risk`, `operational`, `evidence`. Each facet is
a **numeric** facet built with `lacuna_core::numeric` — never hashed text — so a
void center decodes back to concrete parameters:

```json
{"kind":"scalar","id":"n_rate_lb_ac","min":0,"max":250}
{"kind":"categorical","id":"sale_channel","options":["harvest_cash","store_sell_later","forward_contract"]}
```

`encode_numeric` maps scalar `t∈[0,1]` to `(cos tπ/2, sin tπ/2)/√n`; cosine distance is
the mean per-axis distance. `decode_numeric` inverts any positive convex combination,
so `find_interior_voids` centers decode to values strictly inside the anchors' bounds
(geodesic-interior invariant).

## Signals

- `demand_signal` = evaluated expected margin of the strategy (shifted to ≥ 0).
- `quality_signal` = evidence quality in `[0,1]` (actual records > benchmarks > imputed).

High-margin, well-evidenced anchors pull voids toward them; poorly evidenced
anchors cannot seed "gold" voids.

## Config for numeric facets

Numeric encodings are denser than text embeddings. Use a lower anchor window and radius:

```json
{"top_k":8,"min_radius":0.01,"min_void_separation":0.02,"anchor_distance_window":[0.02,1.0]}
```

## Building the browser artifact

```bash
cargo build -p lacuna-wasm --target wasm32-unknown-unknown --profile wasm
# → target/wasm32-unknown-unknown/wasm/lacuna_wasm.wasm (no imports; exports memory, lacuna_alloc, lacuna_call, lacuna_free)
```

cappz vendors this file with a provenance manifest (lacuna commit + sha256) via
`cappz/scripts/build-lacuna-wasm.sh`.
