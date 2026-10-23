---
name: running-steam-benchmark
description: Runs the flagship Steam games whitespace benchmark (the @SyntheticBeef challenge) with lacuna and rung-agent: the built-in seed and procedural catalogs, live abduction with the Jev viability judge, and ingesting a full external Steam dataset into five-axis DomainEntity JSONL. Use when reproducing or scaling the Steam demo, or benchmarking lacuna on a large catalog.
metadata:
  home: ~/src/witt3rd/lacuna/skills/running-steam-benchmark
  aliases: [steam-benchmark, steam-whitespace, synthetic-beef-test]
---

# Flagship Benchmark: The Steam Games Whitespace Explorer

This skill implements the `@SyntheticBeef` challenge ([Tweet](https://x.com/SyntheticBeef/status/2112904858718367750)):

> *"Take all 100,000+ games on steam, scrape their descriptions and metadata, semantically plot them in vector space using embeddings, mathematically choose a location in vector space far away from neighbors, convert that back to a description and metadata, boom, brand new ideas ranked by how unlike any other ideas they are."*

## 1. Quick Benchmark Run (Built-in Seed or Procedural Scale)

Bare filenames passed to `--svg`, `--html`, or `--out` automatically write into `out/` (gitignored) so the repository root stays clean.

```bash
# Run on the 12 canonical Steam archetypes and emit SVG + interactive WebGPU HTML into out/
cargo run --release -p lacuna-cli -- demo steam \
  --top-k 5 \
  --svg out/steam_voids.svg \
  --html out/steam_explorer.html

# Scale up to a larger clustered catalog (e.g., 500 or 10,000 games)
cargo run --release -p lacuna-cli -- demo steam \
  --count 1000 \
  --top-k 5 \
  --json > out/steam_1k_results.json
```

## 2. Live Abduction + Jev System One Viability Judge

With Doppler (`OPENROUTER_API_KEY`) configured on `roger`:

```bash
doppler run -p fleet -c dev_work -- \
  cargo run --release -p lacuna-cli -- demo steam \
  --live-llm \
  --jev \
  --top-k 3 \
  --html steam_live_explorer.html \
  --json
```

## 3. Ingesting a Full External Steam Dataset via `rung-agent`

Because `rung-agent` has built-in Python execution (`--tools read,write,python,web`), you can instruct `lacuna agent` (or `rung-agent` directly) to convert any raw Steam CSV/Parquet dump (e.g., HuggingFace `FronkonGames/steam-games-dataset`) into `lacuna`'s 5-axis `DomainEntity` JSONL format:

- `core_loop`: Moment-to-moment player verbs and physics/simulation rules.
- `progression`: Run structure, knowledge-gating, tech trees, or extraction loops.
- `social_topology`: Solo, proximity-voice co-op, asymmetric roles, or asynchronous world-sharing.
- `aesthetic_world`: Camera perspective, sensory atmosphere, and art direction.
- `player_hook`: Primary cognitive/emotional satisfaction.

Then pass `--input steam_100k.json` to `lacuna voids`, `lacuna synthesize`, or `lacuna field`.
