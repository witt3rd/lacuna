//! `lacuna` — agent-first CLI for domain-agnostic whitespace analysis, `rung` ladder
//! synthesis, `rung_std::decide` viability scoring, and WebGPU/SVG manifold rendering.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use lacuna_core::{
    CorpusSnapshot, DEFAULT_EMBED_DIM, DomainEntity, DomainSchema, VoidSolverConfig,
    compute_field_grid, find_interior_voids, generate_steam_catalog, project_entities_2d,
    steam_domain_schema, steam_seed_snapshot,
};
use lacuna_gpu::{ExplorerBundle, render_svg_manifold, render_webgpu_explorer_html};
use lacuna_rung::{
    CalibratedGeometricDecider, SynthesisSpec, jev_from_env, resolve_decider, run_rung_agent_turn,
    run_synthesis_ladder,
};
use rung_std::decide::Decider;
use serde_json::json;

const USAGE: &str = "\
lacuna — domain-agnostic high-dimensional whitespace analyzer, finder & explorer

USAGE:
  lacuna <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
  schema                  Emit the DomainSchema (default: 5-axis Steam Games schema)
  seed                    Generate a Steam Games CorpusSnapshot (--count N, --out PATH)
  ingest                  Ingest custom domain entities (--schema PATH --input PATH [--out PATH])
  voids                   Solve & rank interior Delaunay empty spheres (--input PATH, --top-k K, --weights k=v, --json)
  probe <VOID_ID>         Inspect a void's coordinates, Voronoi polygon, Delaunay neighbors & facet bridge (--json)
  synthesize [VOID_ID]    Run the rung SynthesizeVoid ladder (abduct + rung_std::decide judge + closed-loop proof)
  field                   Compute the 2D supply/demand/whitespace field and render SVG (--svg PATH) or WebGPU HTML (--html PATH)
  demo steam              Run the end-to-end @SyntheticBeef Steam Games Whitespace benchmark
  agent <PROMPT>          Run an autonomous rung-agent turn (--toolset explore|implement|review)

COMMON OPTIONS:
  --input <PATH>          Path to CorpusSnapshot JSON (defaults to built-in Steam archetype snapshot)
  --top-k <N>             Number of top voids to solve/synthesize (default: 5)
  --min-radius <F>        Minimum empty-sphere cosine radius (default: 0.15)
  --weights <K=V,...>     Override facet weights (e.g. core_loop=0.6,social_topology=0.4)
  --live-llm              Dispatch abduction through rung_std::llm::llm_call (uses RUNG_*/OPENROUTER_API_KEY)
  --jev                   Dispatch viability questions through rung_std::decide::JevDecider
  --fixture <PATH>        Replay or record rung_std::decide exchange at PATH
  --svg <PATH>            Write vector SVG topographic manifold map to PATH
  --html <PATH>           Write interactive WebGPU/Shader HTML explorer to PATH
  --json                  Emit machine-readable JSON on stdout
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn run(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        return Ok(USAGE.trim_end().to_string());
    }

    match args[0].as_str() {
        "schema" => cmd_schema(&args[1..]),
        "seed" => cmd_seed(&args[1..]),
        "ingest" => cmd_ingest(&args[1..]),
        "voids" => cmd_voids(&args[1..]),
        "probe" => cmd_probe(&args[1..]),
        "synthesize" => cmd_synthesize(&args[1..]),
        "field" => cmd_field(&args[1..]),
        "demo" => cmd_demo(&args[1..]),
        "agent" => cmd_agent(&args[1..]),
        other => Err(format!(
            "unknown subcommand `{other}` (run `lacuna --help`)"
        )),
    }
}

/// Resolves an output path so bare filenames (e.g. `steam_explorer.html`) are placed inside
/// `out/` rather than cluttering the repository root, and ensures parent directories exist.
fn write_output_file(raw_path: &Path, content: &str) -> Result<PathBuf, String> {
    let resolved = if raw_path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
        PathBuf::from("out").join(raw_path)
    } else {
        raw_path.to_path_buf()
    };
    if let Some(parent) = resolved.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create directory {}: {e}", parent.display()))?;
    }
    fs::write(&resolved, content).map_err(|e| format!("write {}: {e}", resolved.display()))?;
    Ok(resolved)
}

fn cmd_schema(args: &[String]) -> Result<String, String> {
    let _ = args;
    let schema = steam_domain_schema();
    serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())
}

fn cmd_seed(args: &[String]) -> Result<String, String> {
    let mut count = 12_usize;
    let mut out_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                i += 1;
                count = args
                    .get(i)
                    .ok_or("--count requires a number")?
                    .parse()
                    .map_err(|e| format!("invalid --count: {e}"))?;
            }
            "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("--out requires a file path")?,
                ));
            }
            "--json" => {}
            other => return Err(format!("unknown flag `{other}` for `seed`")),
        }
        i += 1;
    }

    let snapshot = generate_steam_catalog(count);
    let json_str = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    if let Some(path) = out_path {
        let written = write_output_file(&path, &json_str)?;
        Ok(json!({
            "status": "ok",
            "domain_id": snapshot.schema.domain_id,
            "entity_count": snapshot.entities.len(),
            "output_path": written,
        })
        .to_string())
    } else {
        Ok(json_str)
    }
}

fn cmd_ingest(args: &[String]) -> Result<String, String> {
    let mut schema_path: Option<PathBuf> = None;
    let mut input_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--schema" => {
                i += 1;
                schema_path = Some(PathBuf::from(
                    args.get(i).ok_or("--schema requires a path")?,
                ));
            }
            "--input" => {
                i += 1;
                input_path = Some(PathBuf::from(args.get(i).ok_or("--input requires a path")?));
            }
            "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(args.get(i).ok_or("--out requires a path")?));
            }
            other => return Err(format!("unknown flag `{other}` for `ingest`")),
        }
        i += 1;
    }

    let schema: DomainSchema = if let Some(sp) = schema_path {
        let raw = fs::read_to_string(&sp).map_err(|e| format!("read {}: {e}", sp.display()))?;
        serde_json::from_str(&raw).map_err(|e| format!("parse schema: {e}"))?
    } else {
        steam_domain_schema()
    };

    let inp = input_path.ok_or("`ingest` requires `--input <entities.json|jsonl>`")?;
    let raw_entities =
        fs::read_to_string(&inp).map_err(|e| format!("read {}: {e}", inp.display()))?;

    let mut entities: Vec<DomainEntity> = if raw_entities.trim_start().starts_with('[') {
        serde_json::from_str(&raw_entities).map_err(|e| format!("parse entities JSON: {e}"))?
    } else {
        let mut list = Vec::new();
        for (line_no, line) in raw_entities.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let ent: DomainEntity =
                serde_json::from_str(line).map_err(|e| format!("line {}: {e}", line_no + 1))?;
            list.push(ent);
        }
        list
    };

    for ent in &mut entities {
        ent.ensure_embedded(&schema, DEFAULT_EMBED_DIM);
    }

    let snapshot = CorpusSnapshot { schema, entities };
    let serialized = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    if let Some(out) = out_path {
        let written = write_output_file(&out, &serialized)?;
        Ok(json!({
            "status": "ok",
            "domain_id": snapshot.schema.domain_id,
            "entity_count": snapshot.entities.len(),
            "output_path": written,
        })
        .to_string())
    } else {
        Ok(serialized)
    }
}

struct CommonOpts {
    input: Option<PathBuf>,
    top_k: usize,
    min_radius: f64,
    weights: Option<BTreeMap<String, f32>>,
    live_llm: bool,
    use_jev: bool,
    fixture: Option<PathBuf>,
    svg_out: Option<PathBuf>,
    html_out: Option<PathBuf>,
    json_output: bool,
    count: Option<usize>,
    resolution: usize,
    positional: Vec<String>,
}

impl Default for CommonOpts {
    fn default() -> Self {
        Self {
            input: None,
            top_k: 5,
            min_radius: 0.15,
            weights: None,
            live_llm: false,
            use_jev: false,
            fixture: None,
            svg_out: None,
            html_out: None,
            json_output: false,
            count: None,
            resolution: 32,
            positional: Vec::new(),
        }
    }
}

fn parse_common_opts(args: &[String]) -> Result<CommonOpts, String> {
    let mut opts = CommonOpts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                opts.input = Some(PathBuf::from(args.get(i).ok_or("--input requires a path")?));
            }
            "--top-k" => {
                i += 1;
                opts.top_k = args
                    .get(i)
                    .ok_or("--top-k requires an integer")?
                    .parse()
                    .map_err(|e| format!("invalid --top-k: {e}"))?;
            }
            "--min-radius" => {
                i += 1;
                opts.min_radius = args
                    .get(i)
                    .ok_or("--min-radius requires a float")?
                    .parse()
                    .map_err(|e| format!("invalid --min-radius: {e}"))?;
            }
            "--weights" => {
                i += 1;
                let spec = args.get(i).ok_or("--weights requires key=val pairs")?;
                opts.weights = Some(parse_weights(spec)?);
            }
            "--live-llm" => opts.live_llm = true,
            "--jev" => opts.use_jev = true,
            "--fixture" => {
                i += 1;
                opts.fixture = Some(PathBuf::from(
                    args.get(i).ok_or("--fixture requires a path")?,
                ));
            }
            "--svg" => {
                i += 1;
                opts.svg_out = Some(PathBuf::from(args.get(i).ok_or("--svg requires a path")?));
            }
            "--html" => {
                i += 1;
                opts.html_out = Some(PathBuf::from(args.get(i).ok_or("--html requires a path")?));
            }
            "--count" => {
                i += 1;
                opts.count = Some(
                    args.get(i)
                        .ok_or("--count requires an integer")?
                        .parse()
                        .map_err(|e| format!("invalid --count: {e}"))?,
                );
            }
            "--resolution" => {
                i += 1;
                opts.resolution = args
                    .get(i)
                    .ok_or("--resolution requires an integer")?
                    .parse()
                    .map_err(|e| format!("invalid --resolution: {e}"))?;
            }
            "--json" => opts.json_output = true,
            pos if !pos.starts_with('-') => opts.positional.push(pos.to_string()),
            other => return Err(format!("unknown option `{other}`")),
        }
        i += 1;
    }
    Ok(opts)
}

fn parse_weights(spec: &str) -> Result<BTreeMap<String, f32>, String> {
    let mut map = BTreeMap::new();
    for pair in spec.split(',') {
        let Some((k, v)) = pair.split_once('=') else {
            return Err(format!(
                "invalid weight pair `{pair}`, expected facet=weight"
            ));
        };
        let w: f32 = v
            .trim()
            .parse()
            .map_err(|e| format!("invalid float weight `{v}`: {e}"))?;
        map.insert(k.trim().to_string(), w);
    }
    Ok(map)
}

fn load_snapshot(opts: &CommonOpts) -> Result<CorpusSnapshot, String> {
    if let Some(ref path) = opts.input {
        let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let mut snap: CorpusSnapshot =
            serde_json::from_str(&raw).map_err(|e| format!("parse snapshot: {e}"))?;
        snap.ensure_all_embedded(DEFAULT_EMBED_DIM);
        Ok(snap)
    } else if let Some(count) = opts.count {
        Ok(generate_steam_catalog(count))
    } else {
        Ok(steam_seed_snapshot())
    }
}

fn cmd_voids(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args)?;
    let snapshot = load_snapshot(&opts)?;
    let cfg = VoidSolverConfig {
        top_k: opts.top_k,
        min_radius: opts.min_radius,
        min_void_separation: 0.16,
        facet_weights: opts.weights,
        ..Default::default()
    };
    let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &cfg);

    if opts.json_output {
        return serde_json::to_string_pretty(&voids).map_err(|e| e.to_string());
    }

    let mut out = format!(
        "Domain: {} ({} entities)\nFound {} interior Delaunay voids:\n",
        snapshot.schema.title,
        snapshot.entities.len(),
        voids.len()
    );
    for v in &voids {
        let names: Vec<&str> = v
            .boundary_neighbors
            .iter()
            .map(|n| n.name.as_str())
            .collect();
        out.push_str(&format!(
            "  #{} {} | Opportunity: {:.4} | Radius: {:.4} | Voronoi Area: {:.4} | Simplex: [{}]\n",
            v.rank,
            v.void_id,
            v.opportunity_score,
            v.radius,
            v.voronoi_area,
            names.join(" × ")
        ));
    }
    Ok(out.trim_end().to_string())
}

fn cmd_probe(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args)?;
    let target_id = opts
        .positional
        .first()
        .map(String::as_str)
        .unwrap_or("void-01");
    let snapshot = load_snapshot(&opts)?;
    let cfg = VoidSolverConfig {
        top_k: opts.top_k.max(10),
        min_radius: opts.min_radius,
        min_void_separation: 0.16,
        facet_weights: opts.weights,
        ..Default::default()
    };
    let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &cfg);
    let void = voids
        .into_iter()
        .find(|v| v.void_id == target_id)
        .ok_or_else(|| format!("void `{target_id}` not found"))?;

    if opts.json_output {
        return serde_json::to_string_pretty(&void).map_err(|e| e.to_string());
    }

    let mut out = format!(
        "VOID {} (Rank #{})\n  Empty-Sphere Radius: {:.4}\n  Opportunity Score:   {:.4}\n  2D Manifold Coords:  [{:.4}, {:.4}] (Voronoi Area: {:.4})\n  Boundary Neighbors:\n",
        void.void_id,
        void.rank,
        void.radius,
        void.opportunity_score,
        void.coords_2d[0],
        void.coords_2d[1],
        void.voronoi_area
    );
    for nb in &void.boundary_neighbors {
        out.push_str(&format!(
            "    - {} ({}) | dist={:.4}, weight={:.3}, demand={:.2}, quality={:.2}\n",
            nb.name,
            nb.entity_id,
            nb.distance,
            nb.simplex_weight,
            nb.demand_signal,
            nb.quality_signal
        ));
    }
    out.push_str("  Facet Bridge Prescription:\n");
    for (facet, bridge) in &void.facet_bridge {
        out.push_str(&format!("    * {facet}: {bridge}\n"));
    }
    Ok(out.trim_end().to_string())
}

fn select_decider(opts: &CommonOpts) -> Arc<dyn Decider> {
    if opts.use_jev {
        Arc::new(jev_from_env())
    } else if let Some(ref p) = opts.fixture {
        Arc::from(resolve_decider(Some(p.as_path())))
    } else {
        Arc::new(CalibratedGeometricDecider)
    }
}

fn cmd_synthesize(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args)?;
    let target_id = opts.positional.first().map(String::as_str);
    let snapshot = load_snapshot(&opts)?;
    let cfg = VoidSolverConfig {
        top_k: opts.top_k,
        min_radius: opts.min_radius,
        min_void_separation: 0.16,
        facet_weights: opts.weights.clone(),
        ..Default::default()
    };
    let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &cfg);
    let selected_voids: Vec<_> = if let Some(id) = target_id {
        vec![
            voids
                .into_iter()
                .find(|v| v.void_id == id)
                .ok_or_else(|| format!("void `{id}` not found"))?,
        ]
    } else {
        voids
    };

    let corpus_arc = Arc::new(snapshot.entities);
    let decider = select_decider(&opts);

    let mut verified_list = Vec::with_capacity(selected_voids.len());
    for void in selected_voids {
        let spec = SynthesisSpec {
            schema: snapshot.schema.clone(),
            corpus: Arc::clone(&corpus_arc),
            void,
            decider: Arc::clone(&decider),
            live_llm: opts.live_llm,
            min_corpus_separation: 0.12,
            min_viability: 0.40,
            max_attempts: 3,
        };
        verified_list.push(run_synthesis_ladder(spec)?);
    }

    if opts.json_output {
        return if verified_list.len() == 1 && target_id.is_some() {
            serde_json::to_string_pretty(&verified_list[0]).map_err(|e| e.to_string())
        } else {
            serde_json::to_string_pretty(&verified_list).map_err(|e| e.to_string())
        };
    }

    let mut out = String::new();
    for opp in &verified_list {
        out.push_str(&format!(
            "═══ {} (Rank #{}) — {} ═══\n  Logline: {}\n  Core 30s Loop: {}\n  Tags: {}\n  Decision Model ({}): Coherent={:.0}%, Novel={:.0}%, Hunger={:.0}%, Scope={} (Composite={:.3})\n  Closed-Loop Proof: verified={} | dist_to_void={:.4} | min_dist_to_existing={:.4} ({})\n\n",
            opp.void_id,
            opp.void_rank,
            opp.hypothesis.title,
            opp.hypothesis.high_concept_logline,
            opp.hypothesis.core_30s_loop,
            opp.hypothesis.tags.join(", "),
            opp.decision.judge_model,
            opp.decision.p_mechanically_coherent * 100.0,
            opp.decision.p_genuinely_novel * 100.0,
            opp.decision.p_commercial_hunger * 100.0,
            opp.decision.scope_tier,
            opp.decision.composite_viability,
            opp.closed_loop_proof.verified_in_void,
            opp.closed_loop_proof.distance_to_void_center,
            opp.closed_loop_proof.min_distance_to_existing_entity,
            opp.closed_loop_proof.closest_existing_entity_name,
        ));
    }
    Ok(out.trim_end().to_string())
}

fn build_explorer_bundle(opts: &CommonOpts) -> Result<ExplorerBundle, String> {
    let snapshot = load_snapshot(opts)?;
    let weights = opts
        .weights
        .clone()
        .unwrap_or_else(|| snapshot.schema.default_weights());
    let entity_coords_2d = project_entities_2d(&snapshot.entities, &weights);
    let field = compute_field_grid(&snapshot.entities, &entity_coords_2d, opts.resolution);
    let cfg = VoidSolverConfig {
        top_k: opts.top_k,
        min_radius: opts.min_radius,
        min_void_separation: 0.16,
        facet_weights: Some(weights),
        ..Default::default()
    };
    let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &cfg);

    let corpus_arc = Arc::new(snapshot.entities.clone());
    let decider = select_decider(opts);
    let mut opportunities = Vec::with_capacity(voids.len());
    for void in &voids {
        let spec = SynthesisSpec {
            schema: snapshot.schema.clone(),
            corpus: Arc::clone(&corpus_arc),
            void: void.clone(),
            decider: Arc::clone(&decider),
            live_llm: opts.live_llm,
            min_corpus_separation: 0.12,
            min_viability: 0.40,
            max_attempts: 3,
        };
        opportunities.push(run_synthesis_ladder(spec)?);
    }

    Ok(ExplorerBundle {
        snapshot,
        entity_coords_2d,
        field,
        voids,
        opportunities,
    })
}

fn cmd_field(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args)?;
    let bundle = build_explorer_bundle(&opts)?;

    let svg_written = if let Some(ref svg_path) = opts.svg_out {
        let svg = render_svg_manifold(&bundle, 960, 720);
        Some(write_output_file(svg_path, &svg)?)
    } else {
        None
    };
    let html_written = if let Some(ref html_path) = opts.html_out {
        let html = render_webgpu_explorer_html(&bundle)?;
        Some(write_output_file(html_path, &html)?)
    } else {
        None
    };

    if opts.json_output {
        return serde_json::to_string_pretty(&json!({
            "domain_id": bundle.snapshot.schema.domain_id,
            "entity_count": bundle.snapshot.entities.len(),
            "void_count": bundle.voids.len(),
            "resolution": bundle.field.resolution,
            "svg_written": svg_written,
            "html_written": html_written,
            "top_voids": bundle.voids,
            "opportunities": bundle.opportunities,
        }))
        .map_err(|e| e.to_string());
    }

    Ok(format!(
        "Computed {}x{} potential field across {} entities ({} verified voids).{}{}",
        bundle.field.resolution,
        bundle.field.resolution,
        bundle.snapshot.entities.len(),
        bundle.opportunities.len(),
        svg_written
            .as_ref()
            .map(|p| format!(" SVG -> {}", p.display()))
            .unwrap_or_default(),
        html_written
            .as_ref()
            .map(|p| format!(" HTML -> {}", p.display()))
            .unwrap_or_default(),
    ))
}

fn cmd_demo(args: &[String]) -> Result<String, String> {
    let sub = args.first().map(String::as_str).unwrap_or("steam");
    if sub != "steam" {
        return Err(format!(
            "unknown demo `{sub}`, expected `lacuna demo steam`"
        ));
    }
    let opts = parse_common_opts(&args[1..])?;
    let bundle = build_explorer_bundle(&opts)?;

    let svg_written = if let Some(ref svg_path) = opts.svg_out {
        let svg = render_svg_manifold(&bundle, 960, 720);
        Some(write_output_file(svg_path, &svg)?)
    } else {
        None
    };
    let html_written = if let Some(ref html_path) = opts.html_out {
        let html = render_webgpu_explorer_html(&bundle)?;
        Some(write_output_file(html_path, &html)?)
    } else {
        None
    };

    if opts.json_output {
        return serde_json::to_string_pretty(&json!({
            "benchmark": "steam-whitespace-explorer",
            "entity_count": bundle.snapshot.entities.len(),
            "opportunities": bundle.opportunities,
            "voids": bundle.voids,
            "svg_written": svg_written,
            "html_written": html_written,
        }))
        .map_err(|e| e.to_string());
    }

    let mut report = format!(
        "╔══════════════════════════════════════════════════════════════════════╗\n\
         ║  LACUNA // STEAM GAMES WHITESPACE EXPLORER ({} games analyzed)       ║\n\
         ╚══════════════════════════════════════════════════════════════════════╝\n\n",
        bundle.snapshot.entities.len()
    );
    for opp in &bundle.opportunities {
        report.push_str(&format!(
            "  [Rank #{} | {}] {}\n    • Logline:   {}\n    • Core Loop: {}\n    • Tags:      {}\n    • Rung Decide ({}): Coherent={:.0}% | Novel={:.0}% | Hunger={:.0}% | Scope={} (Viability={:.3})\n    • Closed-Loop Proof: verified={} | min_dist_to_corpus={:.4} (nearest: {})\n\n",
            opp.void_rank,
            opp.void_id,
            opp.hypothesis.title,
            opp.hypothesis.high_concept_logline,
            opp.hypothesis.core_30s_loop,
            opp.hypothesis.tags.join(", "),
            opp.decision.judge_model,
            opp.decision.p_mechanically_coherent * 100.0,
            opp.decision.p_genuinely_novel * 100.0,
            opp.decision.p_commercial_hunger * 100.0,
            opp.decision.scope_tier,
            opp.decision.composite_viability,
            opp.closed_loop_proof.verified_in_void,
            opp.closed_loop_proof.min_distance_to_existing_entity,
            opp.closed_loop_proof.closest_existing_entity_name,
        ));
    }
    if let Some(ref p) = svg_written {
        report.push_str(&format!("  Wrote SVG  -> {}\n", p.display()));
    }
    if let Some(ref p) = html_written {
        report.push_str(&format!("  Wrote HTML -> {}\n", p.display()));
    }
    Ok(report.trim_end().to_string())
}

fn cmd_agent(args: &[String]) -> Result<String, String> {
    let mut toolset = "explore".to_string();
    let mut prompt_parts = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--toolset" => {
                i += 1;
                toolset = args
                    .get(i)
                    .ok_or("--toolset requires explore|implement|review")?
                    .clone();
            }
            part => prompt_parts.push(part.to_string()),
        }
        i += 1;
    }
    let prompt = prompt_parts.join(" ");
    if prompt.trim().is_empty() {
        return Err("`lacuna agent` requires a prompt".into());
    }
    run_rung_agent_turn(&prompt, &toolset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_demo_steam_json_returns_verified_opportunities() {
        let out = run(&[
            "demo".into(),
            "steam".into(),
            "--top-k".into(),
            "2".into(),
            "--json".into(),
        ])
        .expect("demo steam succeeds");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["benchmark"], "steam-whitespace-explorer");
        assert_eq!(parsed["opportunities"].as_array().unwrap().len(), 2);
        assert_eq!(
            parsed["opportunities"][0]["closed_loop_proof"]["verified_in_void"],
            true
        );
    }
}
