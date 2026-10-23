#![doc = include_str!("../../../README.md")]

pub mod metric;
pub mod numeric;
pub mod schema;
pub mod steam;
pub mod void;
pub mod voronoi;

pub use metric::{
    cosine_distance, entity_distance, geodesic_simplex_point, min_distance_to_corpus,
    optimize_interior_simplex_center, point_to_entity_distance, point_to_point_distance,
};
pub use numeric::{
    DecodedValue, NumericAxis, NumericValue, decode_numeric, encode_numeric, numeric_dim,
};
pub use schema::{
    CorpusSnapshot, DEFAULT_EMBED_DIM, DomainEntity, DomainSchema, FacetSpec, deterministic_embed,
    normalize_in_place,
};
pub use steam::{generate_steam_catalog, steam_domain_schema, steam_seed_snapshot};
pub use void::{
    BoundaryNeighbor, DEFAULT_ANCHOR_DISTANCE_WINDOW, VoidCandidate, VoidSolverConfig,
    find_interior_voids,
};
pub use voronoi::{
    FieldGrid, FieldSample, MANIFOLD_BOUND, compute_field_grid, compute_voronoi_cell, polygon_area,
    project_entities_2d, project_point_2d,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_seed_snapshot_finds_ranked_interior_voids() {
        let snapshot = steam_seed_snapshot();
        assert_eq!(snapshot.entities.len(), 12);

        let cfg = VoidSolverConfig {
            top_k: 5,
            min_radius: 0.15,
            min_void_separation: 0.15,
            facet_weights: None,
            ..Default::default()
        };
        let voids = find_interior_voids(&snapshot.schema, &snapshot.entities, &cfg);
        assert!(
            !voids.is_empty(),
            "expected interior voids in Steam seed space"
        );
        assert_eq!(voids[0].rank, 1);
        assert_eq!(voids[0].void_id, "void-01");
        assert!(voids[0].radius >= cfg.min_radius);
        assert_eq!(voids[0].boundary_neighbors.len(), 3);
        assert!(voids[0].voronoi_area > 0.0);

        // Verify the void center is genuinely at distance >= radius from ALL corpus entities
        let weights = snapshot.schema.default_weights();
        let actual_min = min_distance_to_corpus(&voids[0].center, &snapshot.entities, &weights);
        assert!(
            (actual_min - voids[0].radius).abs() < 1e-3,
            "reported radius {} must match actual min distance {}",
            voids[0].radius,
            actual_min
        );
    }

    #[test]
    fn field_grid_computes_supply_demand_and_whitespace() {
        let snapshot = steam_seed_snapshot();
        let weights = snapshot.schema.default_weights();
        let coords = project_entities_2d(&snapshot.entities, &weights);
        let grid = compute_field_grid(&snapshot.entities, &coords, 16);
        assert_eq!(grid.samples.len(), 256);
        assert!(grid.samples.iter().any(|s| s.whitespace_potential > 0.25));
    }
}
