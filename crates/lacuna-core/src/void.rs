//! High-dimensional interior empty-sphere solver and Gold Whitespace ranker.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::metric::{
    cosine_distance, entity_distance, optimize_interior_simplex_center, point_to_entity_distance,
    point_to_point_distance,
};
use crate::schema::{DomainEntity, DomainSchema};
use crate::voronoi::{
    MANIFOLD_BOUND, compute_voronoi_cell, polygon_area, project_entities_2d, project_point_2d,
};

/// One existing corpus entity sitting on the boundary of an empty sphere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryNeighbor {
    pub entity_id: String,
    pub name: String,
    pub summary: String,
    pub tags: Vec<String>,
    /// Multi-facet cosine distance from the void center to this neighbor.
    pub distance: f64,
    /// Barycentric weight in the interior simplex that spans this void.
    pub simplex_weight: f64,
    pub demand_signal: f64,
    pub quality_signal: f64,
    pub facets: BTreeMap<String, String>,
}

/// A mathematically verified interior whitespace void in the domain manifold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoidCandidate {
    /// Deterministic identifier (e.g., `"void-01"`).
    pub void_id: String,
    /// 1-based rank by `opportunity_score`.
    pub rank: usize,
    /// Radius of the empty sphere in high-dimensional space (`min_{e in corpus} dist(center, e)`).
    pub radius: f64,
    /// Composite Gold Whitespace score combining `radius`, `simplex_diversity`,
    /// `boundary_demand`, and `voronoi_area`.
    pub opportunity_score: f64,
    /// Mean pairwise distance between the Delaunay simplex anchors (measures cross-genre bridge span).
    pub simplex_diversity: f64,
    /// Weighted demand signal across the boundary neighbors.
    pub boundary_demand: f64,
    /// Weighted quality/sentiment signal across the boundary neighbors.
    pub boundary_quality: f64,
    /// High-dimensional unit-normalized vector coordinates per facet.
    pub center: BTreeMap<String, Vec<f32>>,
    /// Projected 2D manifold coordinates `[x, y]` in `[-1.0, 1.0]^2`.
    pub coords_2d: [f64; 2],
    /// Exact bounded 2D Voronoi cell vertices around `coords_2d`.
    pub voronoi_polygon: Vec<[f64; 2]>,
    /// Area of the 2D Voronoi cell.
    pub voronoi_area: f64,
    /// Surrounding Delaunay/Voronoi boundary entities that frame this void.
    pub boundary_neighbors: Vec<BoundaryNeighbor>,
    /// Per-facet bridge prescription: which neighbor is closest on each semantic axis
    /// and what novel recombination is required.
    pub facet_bridge: BTreeMap<String, String>,
}

/// Default admissible pairwise anchor distance window, tuned for text embeddings.
pub const DEFAULT_ANCHOR_DISTANCE_WINDOW: [f64; 2] = [0.35, 0.92];

/// Configuration knobs for [`find_interior_voids`].
///
/// Every field has a serde default, so a JSON config may name only the knobs it changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoidSolverConfig {
    /// Number of top-ranked voids to return.
    pub top_k: usize,
    /// Minimum empty-sphere radius required.
    pub min_radius: f64,
    /// Minimum distance between two returned void centers (de-duplication threshold).
    pub min_void_separation: f64,
    /// Optional custom facet weights (defaults to `schema.default_weights()`).
    pub facet_weights: Option<BTreeMap<String, f32>>,
    /// Inclusive `[min, max]` pairwise distance two anchors must have to span a simplex.
    /// Text embeddings use [`DEFAULT_ANCHOR_DISTANCE_WINDOW`]; numeric facets
    /// ([`crate::numeric`]) are usually denser and need a lower floor.
    pub anchor_distance_window: [f64; 2],
}

impl Default for VoidSolverConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            min_radius: 0.15,
            min_void_separation: 0.18,
            facet_weights: None,
            anchor_distance_window: DEFAULT_ANCHOR_DISTANCE_WINDOW,
        }
    }
}

/// Finds and ranks interior high-dimensional empty spheres bounded by diverse,
/// high-demand corpus clusters.
pub fn find_interior_voids(
    schema: &DomainSchema,
    entities: &[DomainEntity],
    config: &VoidSolverConfig,
) -> Vec<VoidCandidate> {
    if entities.len() < 3 {
        return Vec::new();
    }

    let weights = config
        .facet_weights
        .clone()
        .unwrap_or_else(|| schema.default_weights());

    let coords_2d = project_entities_2d(entities, &weights);
    let [lo, hi] = config.anchor_distance_window;
    let admissible = |d: f64| (lo..=hi).contains(&d);

    // Select diverse, high-signal anchor candidates (up to 28 representatives)
    // to form Delaunay simplex triples.
    let anchor_indices = select_diverse_anchors(entities, &weights, 28);

    let mut raw_candidates: Vec<VoidCandidate> = Vec::new();

    for a_idx in 0..anchor_indices.len() {
        for b_idx in (a_idx + 1)..anchor_indices.len() {
            let i = anchor_indices[a_idx];
            let j = anchor_indices[b_idx];
            let dij = entity_distance(&entities[i], &entities[j], &weights);
            if !admissible(dij) {
                continue;
            }
            for &k in &anchor_indices[(b_idx + 1)..] {
                let dik = entity_distance(&entities[i], &entities[k], &weights);
                let djk = entity_distance(&entities[j], &entities[k], &weights);
                if !admissible(dik) || !admissible(djk) {
                    continue;
                }

                let anchors = [&entities[i], &entities[j], &entities[k]];
                let (center, bary, radius) =
                    optimize_interior_simplex_center(&anchors, entities, schema, &weights);

                if radius < config.min_radius {
                    continue;
                }

                let simplex_diversity = (dij + dik + djk) / 3.0;
                let boundary_demand = anchors
                    .iter()
                    .zip(bary.iter())
                    .map(|(e, &w)| w * e.demand_signal.max(0.1))
                    .sum::<f64>();
                let boundary_quality = anchors
                    .iter()
                    .zip(bary.iter())
                    .map(|(e, &w)| w * e.quality_signal.clamp(0.1, 1.0))
                    .sum::<f64>();

                let pt_2d = project_point_2d(&center, entities, &coords_2d, &weights);
                let vor_poly = compute_voronoi_cell(pt_2d, &coords_2d, MANIFOLD_BOUND);
                let vor_area = polygon_area(&vor_poly);

                let mut boundary_neighbors = Vec::with_capacity(3);
                for (anchor, &w) in anchors.iter().zip(bary.iter()) {
                    let dist = point_to_entity_distance(&center, anchor, &weights);
                    boundary_neighbors.push(BoundaryNeighbor {
                        entity_id: anchor.id.clone(),
                        name: anchor.name.clone(),
                        summary: anchor.summary.clone(),
                        tags: anchor.tags.clone(),
                        distance: (dist * 10000.0).round() / 10000.0,
                        simplex_weight: (w * 1000.0).round() / 1000.0,
                        demand_signal: anchor.demand_signal,
                        quality_signal: anchor.quality_signal,
                        facets: anchor.facets.clone(),
                    });
                }

                let facet_bridge = build_facet_bridge(schema, &center, &anchors);

                // Opportunity score rewards large empty-sphere radius, cross-cluster
                // simplex diversity, strong market demand/sentiment, and 2D Voronoi basin area.
                let demand_factor = (boundary_demand / 10.0).clamp(0.25, 1.5);
                let area_bonus = 1.0 + 0.35 * vor_area.sqrt().min(1.0);
                let opportunity_score = radius
                    * simplex_diversity.powf(0.65)
                    * demand_factor
                    * boundary_quality
                    * area_bonus;

                raw_candidates.push(VoidCandidate {
                    void_id: String::new(),
                    rank: 0,
                    radius: (radius * 10000.0).round() / 10000.0,
                    opportunity_score: (opportunity_score * 10000.0).round() / 10000.0,
                    simplex_diversity: (simplex_diversity * 10000.0).round() / 10000.0,
                    boundary_demand: (boundary_demand * 1000.0).round() / 1000.0,
                    boundary_quality: (boundary_quality * 1000.0).round() / 1000.0,
                    center,
                    coords_2d: [
                        (pt_2d[0] * 10000.0).round() / 10000.0,
                        (pt_2d[1] * 10000.0).round() / 10000.0,
                    ],
                    voronoi_polygon: vor_poly,
                    voronoi_area: (vor_area * 10000.0).round() / 10000.0,
                    boundary_neighbors,
                    facet_bridge,
                });
            }
        }
    }

    raw_candidates.sort_by(|a, b| {
        b.opportunity_score
            .partial_cmp(&a.opportunity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Greedy non-maximum suppression so returned voids cover distinct basins
    let mut selected: Vec<VoidCandidate> = Vec::new();
    for mut cand in raw_candidates {
        if selected.len() >= config.top_k {
            break;
        }
        let too_close = selected.iter().any(|existing| {
            point_to_point_distance(&cand.center, &existing.center, &weights)
                < config.min_void_separation
        });
        if !too_close {
            let rank = selected.len() + 1;
            cand.rank = rank;
            cand.void_id = format!("void-{rank:02}");
            selected.push(cand);
        }
    }

    selected
}

fn select_diverse_anchors(
    entities: &[DomainEntity],
    weights: &BTreeMap<String, f32>,
    max_anchors: usize,
) -> Vec<usize> {
    if entities.len() <= max_anchors {
        return (0..entities.len()).collect();
    }

    // Start with highest demand*quality entity, then iteratively add entities that combine
    // high signal with distance from already-picked anchors (farthest-point + quality seeding).
    let mut chosen: Vec<usize> = Vec::with_capacity(max_anchors);
    let mut best_first = 0_usize;
    let mut best_score = -1.0_f64;
    for (i, e) in entities.iter().enumerate() {
        let s = e.demand_signal * e.quality_signal;
        if s > best_score {
            best_score = s;
            best_first = i;
        }
    }
    chosen.push(best_first);

    while chosen.len() < max_anchors {
        let mut next_idx = None;
        let mut next_val = -1.0_f64;
        for (i, e) in entities.iter().enumerate() {
            if chosen.contains(&i) {
                continue;
            }
            let min_d = chosen
                .iter()
                .map(|&c| entity_distance(e, &entities[c], weights))
                .fold(f64::INFINITY, f64::min);
            let val =
                min_d * (0.5 + 0.5 * (e.demand_signal / 12.0).clamp(0.1, 1.2) * e.quality_signal);
            if val > next_val {
                next_val = val;
                next_idx = Some(i);
            }
        }
        match next_idx {
            Some(idx) => chosen.push(idx),
            None => break,
        }
    }

    chosen
}

fn build_facet_bridge(
    schema: &DomainSchema,
    center: &BTreeMap<String, Vec<f32>>,
    anchors: &[&DomainEntity],
) -> BTreeMap<String, String> {
    let mut bridge = BTreeMap::new();
    for (f_idx, facet) in schema.facets.iter().enumerate() {
        let Some(c_vec) = center.get(&facet.id) else {
            continue;
        };
        // Assign primary and secondary contributor for this facet so the bridge
        // systematically recombines orthogonal axes across the simplex anchors.
        let mut ranked: Vec<(&DomainEntity, f64)> = anchors
            .iter()
            .map(|&a| {
                let d = a
                    .vectors
                    .get(&facet.id)
                    .map(|v| cosine_distance(c_vec, v))
                    .unwrap_or(1.0);
                (a, d)
            })
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let primary = ranked[f_idx % ranked.len()].0;
        let secondary = ranked[(f_idx + 1) % ranked.len()].0;
        let p_text = primary
            .facets
            .get(&facet.id)
            .cloned()
            .unwrap_or_else(|| primary.summary.clone());
        let s_text = secondary
            .facets
            .get(&facet.id)
            .cloned()
            .unwrap_or_else(|| secondary.summary.clone());

        bridge.insert(
            facet.id.clone(),
            format!(
                "Fuse [{}] ({}) with [{}] ({})",
                primary.name, p_text, secondary.name, s_text
            ),
        );
    }
    bridge
}
