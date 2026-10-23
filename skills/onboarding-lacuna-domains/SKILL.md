---
name: onboarding-lacuna-domains
description: Onboards a new domain (for example venture capital, patents, drug discovery, SaaS features or research literature) onto lacuna's four-layer input contract: a DomainSchema with orthogonal facets, DomainEntity JSONL, then ingest, voids and field. Use when a user wants to apply lacuna to a new dataset or domain, or asks how to write schema.json or entities.jsonl.
metadata:
  home: ~/src/witt3rd/lacuna/skills/onboarding-lacuna-domains
  aliases: [domain-onboarding, lacuna-domain, lacuna-schema]
---

# Onboarding a New Domain into `lacuna`

`lacuna` is completely domain-agnostic. To apply it to any domain, define a `DomainSchema` JSON file and ingest a JSON/JSONL stream of `DomainEntity` records via `lacuna ingest`.

## 1. Define `DomainSchema` (`schema.json`)

Separate **structural mechanics** from **surface context** so your facets are orthogonal:

```json
{
  "domain_id": "vc-startups",
  "title": "Venture Capital Whitespace Atlas",
  "description": "Maps startup capabilities, target workflows, and unit-economic models to identify unfunded market gaps.",
  "facets": [
    {
      "id": "problem_workflow",
      "name": "Target Workflow & Pain Point",
      "description": "The specific bottleneck, manual process, or failure mode being eliminated.",
      "default_weight": 0.35
    },
    {
      "id": "core_mechanism",
      "name": "Core Technical Mechanism",
      "description": "How the product solves the problem (e.g., formal verification, bio-catalysis, edge ASIC).",
      "default_weight": 0.35
    },
    {
      "id": "buyer_persona",
      "name": "Buyer & Deployment Context",
      "description": "Who buys it, regulatory regime, and deployment environment.",
      "default_weight": 0.15
    },
    {
      "id": "economic_moat",
      "name": "Distribution & Defensibility Model",
      "description": "Network effects, proprietary data flywheel, or switching-cost structure.",
      "default_weight": 0.15
    }
  ],
  "demand_metric_name": "log_funding_usd",
  "quality_metric_name": "revenue_efficiency_score"
}
```

## 2. Prepare Entities (`entities.jsonl`)

Each line in `entities.jsonl` is a `DomainEntity` (if `vectors` is `{}` or omitted, `lacuna ingest` computes deterministic embeddings automatically):

```json
{"id":"vc:01","name":"AcmeVerify","summary":"Formal verification for Verilog chip designs","tags":["EDA","Semiconductors"],"facets":{"problem_workflow":"Post-synthesis silicon bug escapes in ASIC tapeout","core_mechanism":"SMT-solver bounded model checking over RTL","buyer_persona":"Hyperscaler silicon engineering teams","economic_moat":"Deep integration into CI synthesis flows"},"vectors":{},"demand_signal":16.5,"quality_signal":0.92,"metadata":{}}
```

## 3. Ingest, Solve, and Synthesize

```bash
# Ingest into a portable CorpusSnapshot
lacuna ingest --schema schema.json --input entities.jsonl --out vc_snapshot.json

# Find ranked interior voids
lacuna voids --input vc_snapshot.json --top-k 5 --json

# Synthesize verified opportunities & render the WebGPU explorer
lacuna field --input vc_snapshot.json --svg vc_field.svg --html vc_explorer.html
```
