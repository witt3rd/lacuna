---
name: piloting-lacuna-cli
description: Drives the lacuna CLI end-to-end as an autonomous agent with deterministic JSON output: schema inspection, voids with facet-weight steering, probe, rung-ladder synthesis with rung_std::decide viability scoring, and SVG/WebGPU field export. Use when running lacuna from an agent or script, interpreting VoidCandidate or VerifiedOpportunity JSON, or steering a search toward a facet subspace.
metadata:
  home: ~/src/witt3rd/lacuna/skills/piloting-lacuna-cli
  aliases: [pilot-cli, lacuna-pilot, lacuna-cli]
---

# Piloting `lacuna` as an Autonomous Agent

`lacuna` is designed so an agent can drive every stage of whitespace discovery through structured JSON without needing a GUI.

## 1. End-to-End Agent Loop

```bash
# Step 1: Inspect the domain coordinate system (facets & default weights)
lacuna schema

# Step 2: Solve for the top-K interior Delaunay empty spheres in JSON
lacuna voids --top-k 5 --json > voids.json

# Step 3: Re-steer facet weights to hunt in a specific subspace
# Example: weight Core Gameplay Loop (0.7) and Social Topology (0.3), ignoring Setting (0.0)
lacuna voids --weights core_loop=0.7,social_topology=0.3,aesthetic_world=0.0 --top-k 5 --json

# Step 4: Deep-probe a candidate void (Voronoi polygon, Delaunay simplex neighbors, facet bridge)
lacuna probe void-01 --json

# Step 5: Synthesize & verify an opportunity through the rung SynthesizeVoid ladder
# Offline deterministic mode:
lacuna synthesize void-01 --json
# Live OpenRouter LLM + Jev System One Decider mode:
doppler run -p fleet -c dev_work -- lacuna synthesize void-01 --live-llm --jev --json

# Step 6: Render the 2D supply/demand/whitespace potential field to SVG and WebGPU HTML
lacuna field --resolution 48 --svg field.svg --html explorer.html --json
```

## 2. Interpreting JSON Outputs

### `VoidCandidate` (`lacuna voids --json` / `lacuna probe <id> --json`)
- `radius`: High-dimensional cosine distance to the nearest existing corpus entity ($R(x^*) = \min_{e \in \text{Corpus}} d(x^*, e)$). Higher means larger unexplored gap.
- `simplex_diversity`: Mean pairwise distance between the 3 bounding Delaunay anchor entities. Higher means the void bridges more distant clusters.
- `boundary_demand` / `boundary_quality`: Barycentric-weighted demand and sentiment of the 3 bounding neighbors.
- `voronoi_polygon` / `voronoi_area`: Exact 2D Sutherland-Hodgman clipped Voronoi cell around the void in `[-1.0, 1.0]^2`.
- `facet_bridge`: Per-facet recombination prescription showing which boundary neighbor anchors which axis.

### `VerifiedOpportunity` (`lacuna synthesize <id> --json`)
- `hypothesis`: Contains `title`, `high_concept_logline`, `core_30s_loop`, `tags`, and decomposed `facets`.
- `decision`: Output of `rung_std::decide::Decider` (`p_mechanically_coherent`, `p_genuinely_novel`, `p_commercial_hunger`, `scope_tier`, `composite_viability`).
- `closed_loop_proof`:
  - `verified_in_void`: `true` only when the re-embedded hypothesis maintains `min_distance_to_existing_entity >= min_corpus_separation`.
  - `closest_existing_entity_name` & `min_distance_to_existing_entity`: Mathematical proof of how far the generated concept sits from its nearest real-world neighbor.
