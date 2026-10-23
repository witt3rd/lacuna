//! Invertible numeric facet encoding for decision-parameter domains.
//!
//! Text facets are embedded with [`crate::deterministic_embed`]; numeric domains
//! (farm input rates, sale timing, contract mix, portfolio weights) need an
//! embedding that can be *decoded*, so a void center found by
//! [`crate::find_interior_voids`] maps back to concrete parameter values.
//!
//! Each scalar axis with normalized position `t ∈ [0, 1]` becomes a 2-D block
//! `(cos θ, sin θ)` with `θ = t·π/2`. Each categorical axis becomes a one-hot
//! block. Blocks are scaled by `1/√n` so the whole vector is unit length, and
//! the cosine distance between two encodings is the mean per-axis distance:
//! `1 − cos(Δθ)` for scalars (≤ 1) and `0` or `1` for categories.
//!
//! Decoding is scale-invariant per block (`atan2` for scalars, normalized
//! non-negative mass for categories), so any positive convex combination of
//! encodings—exactly what an interior simplex center is—decodes to an
//! interpolated parameter set inside the anchors' bounds.

use std::f64::consts::FRAC_PI_2;

use serde::{Deserialize, Serialize};

/// One decision parameter on a numeric facet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NumericAxis {
    /// Bounded scalar (e.g. nitrogen rate 0–250 lb/ac).
    Scalar {
        /// Machine identifier.
        id: String,
        /// Lower bound (maps to `θ = 0`).
        min: f64,
        /// Upper bound (maps to `θ = π/2`).
        max: f64,
    },
    /// Unordered choice (e.g. sale channel).
    Categorical {
        /// Machine identifier.
        id: String,
        /// Allowed option labels, in block order.
        options: Vec<String>,
    },
}

impl NumericAxis {
    /// Axis identifier.
    pub fn id(&self) -> &str {
        match self {
            Self::Scalar { id, .. } | Self::Categorical { id, .. } => id,
        }
    }

    fn width(&self) -> usize {
        match self {
            Self::Scalar { .. } => 2,
            Self::Categorical { options, .. } => options.len().max(1),
        }
    }
}

/// A raw parameter value supplied for encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumericValue {
    /// Value for a scalar axis.
    Number(f64),
    /// Option label for a categorical axis.
    Label(String),
}

/// A parameter value recovered from a (possibly interpolated) encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecodedValue {
    /// Recovered scalar in axis units.
    Scalar {
        /// Axis identifier.
        id: String,
        /// Value in `[min, max]`.
        value: f64,
    },
    /// Recovered categorical mix.
    Categorical {
        /// Axis identifier.
        id: String,
        /// Option with the largest mass.
        choice: String,
        /// Normalized non-negative mass per option (sums to 1).
        weights: Vec<f64>,
    },
}

/// Total vector dimensionality for a facet built from `axes`.
pub fn numeric_dim(axes: &[NumericAxis]) -> usize {
    axes.iter().map(NumericAxis::width).sum()
}

/// Encodes `values` (one per axis, same order) onto the unit hypersphere.
///
/// Scalars outside `[min, max]` are clamped; an unknown categorical label
/// is an error rather than a silent zero vector.
pub fn encode_numeric(axes: &[NumericAxis], values: &[NumericValue]) -> Result<Vec<f32>, String> {
    if axes.len() != values.len() {
        return Err(format!(
            "expected {} values for {} axes, got {}",
            axes.len(),
            axes.len(),
            values.len()
        ));
    }
    if axes.is_empty() {
        return Err("numeric facet needs at least one axis".into());
    }
    let scale = 1.0 / (axes.len() as f64).sqrt();
    let mut out = Vec::with_capacity(numeric_dim(axes));
    for (axis, value) in axes.iter().zip(values) {
        match (axis, value) {
            (NumericAxis::Scalar { id, min, max }, NumericValue::Number(v)) => {
                if !v.is_finite() {
                    return Err(format!("axis {id}: value is not finite"));
                }
                let span = max - min;
                let t = if span.abs() < 1e-12 {
                    0.0
                } else {
                    ((v - min) / span).clamp(0.0, 1.0)
                };
                let theta = t * FRAC_PI_2;
                out.push((theta.cos() * scale) as f32);
                out.push((theta.sin() * scale) as f32);
            }
            (NumericAxis::Categorical { id, options }, NumericValue::Label(label)) => {
                let idx = options
                    .iter()
                    .position(|o| o == label)
                    .ok_or_else(|| format!("axis {id}: unknown option {label:?}"))?;
                for i in 0..options.len().max(1) {
                    out.push(if i == idx { scale as f32 } else { 0.0 });
                }
            }
            (axis, _) => {
                return Err(format!(
                    "axis {}: value kind does not match axis kind",
                    axis.id()
                ));
            }
        }
    }
    Ok(out)
}

/// Decodes a facet vector (any positive scaling or convex combination of
/// encodings) back into per-axis values.
pub fn decode_numeric(axes: &[NumericAxis], vector: &[f32]) -> Result<Vec<DecodedValue>, String> {
    let need = numeric_dim(axes);
    if vector.len() != need {
        return Err(format!(
            "expected vector of dim {need}, got {}",
            vector.len()
        ));
    }
    let mut cursor = 0;
    let mut out = Vec::with_capacity(axes.len());
    for axis in axes {
        let block = &vector[cursor..cursor + axis.width()];
        cursor += axis.width();
        match axis {
            NumericAxis::Scalar { id, min, max } => {
                let c = f64::from(block[0]).max(0.0);
                let s = f64::from(block[1]).max(0.0);
                let t = if c + s < 1e-12 {
                    0.0
                } else {
                    (s.atan2(c) / FRAC_PI_2).clamp(0.0, 1.0)
                };
                out.push(DecodedValue::Scalar {
                    id: id.clone(),
                    value: min + t * (max - min),
                });
            }
            NumericAxis::Categorical { id, options } => {
                let mass: Vec<f64> = block.iter().map(|&x| f64::from(x).max(0.0)).collect();
                let total: f64 = mass.iter().sum();
                let weights: Vec<f64> = if total < 1e-12 {
                    vec![1.0 / mass.len() as f64; mass.len()]
                } else {
                    mass.iter().map(|m| m / total).collect()
                };
                let best = weights
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                out.push(DecodedValue::Categorical {
                    id: id.clone(),
                    choice: options.get(best).cloned().unwrap_or_default(),
                    weights,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosine_distance;

    fn axes() -> Vec<NumericAxis> {
        vec![
            NumericAxis::Scalar {
                id: "n_rate".into(),
                min: 0.0,
                max: 250.0,
            },
            NumericAxis::Categorical {
                id: "channel".into(),
                options: vec!["harvest".into(), "store".into(), "forward".into()],
            },
        ]
    }

    #[test]
    fn round_trips_and_is_unit_length() {
        let v = encode_numeric(
            &axes(),
            &[
                NumericValue::Number(160.0),
                NumericValue::Label("store".into()),
            ],
        )
        .unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
        let d = decode_numeric(&axes(), &v).unwrap();
        match &d[0] {
            DecodedValue::Scalar { value, .. } => assert!((value - 160.0).abs() < 1e-3),
            other => panic!("{other:?}"),
        }
        match &d[1] {
            DecodedValue::Categorical { choice, .. } => assert_eq!(choice, "store"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn convex_combination_decodes_between_anchors() {
        let a = encode_numeric(
            &axes(),
            &[
                NumericValue::Number(100.0),
                NumericValue::Label("harvest".into()),
            ],
        )
        .unwrap();
        let b = encode_numeric(
            &axes(),
            &[
                NumericValue::Number(200.0),
                NumericValue::Label("store".into()),
            ],
        )
        .unwrap();
        let mix: Vec<f32> = a.iter().zip(&b).map(|(x, y)| 0.3 * x + 0.7 * y).collect();
        let d = decode_numeric(&axes(), &mix).unwrap();
        match &d[0] {
            DecodedValue::Scalar { value, .. } => assert!(*value > 100.0 && *value < 200.0),
            other => panic!("{other:?}"),
        }
        match &d[1] {
            DecodedValue::Categorical {
                choice, weights, ..
            } => {
                assert_eq!(choice, "store");
                assert!((weights[1] - 0.7).abs() < 1e-3);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn distance_is_mean_per_axis_distance() {
        let a = encode_numeric(
            &axes(),
            &[
                NumericValue::Number(0.0),
                NumericValue::Label("harvest".into()),
            ],
        )
        .unwrap();
        let b = encode_numeric(
            &axes(),
            &[
                NumericValue::Number(250.0),
                NumericValue::Label("harvest".into()),
            ],
        )
        .unwrap();
        // scalar axis fully apart (1.0), categorical identical (0.0) → mean 0.5
        assert!((cosine_distance(&a, &b) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn rejects_unknown_label_and_arity_mismatch() {
        assert!(encode_numeric(&axes(), &[NumericValue::Number(1.0)]).is_err());
        assert!(
            encode_numeric(
                &axes(),
                &[
                    NumericValue::Number(1.0),
                    NumericValue::Label("barter".into())
                ]
            )
            .is_err()
        );
    }
}
