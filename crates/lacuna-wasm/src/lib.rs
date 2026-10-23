//! wasm32 C-ABI JSON bridge to `lacuna-core`.
//!
//! One entry point, [`lacuna_call`], takes a UTF-8 JSON request and returns a
//! length-prefixed UTF-8 JSON response, so a host needs no `wasm-bindgen`
//! glue—only `memory`, [`lacuna_alloc`], [`lacuna_free`] and [`lacuna_call`].
//!
//! Requests (`op` selects the operation):
//!
//! ```json
//! {"op":"version"}
//! {"op":"find_voids","schema":{...},"entities":[...],"config":{"top_k":5}}
//! {"op":"encode_numeric","axes":[...],"values":[160.0,"store"]}
//! {"op":"decode_numeric","axes":[...],"vector":[0.1,0.2]}
//! {"op":"embed","texts":["text facet", ...]}   (deterministic text embedding, DEFAULT_EMBED_DIM)
//! ```
//!
//! Responses are `{"ok":true,"result":...}` or `{"ok":false,"error":"..."}`.
//! Every operation is deterministic: the same request bytes always produce
//! the same response bytes.

use lacuna_core::{
    DEFAULT_EMBED_DIM, DomainEntity, DomainSchema, NumericAxis, NumericValue, VoidSolverConfig,
    decode_numeric, deterministic_embed, encode_numeric, find_interior_voids,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// Crate version reported by `{"op":"version"}`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    Version,
    FindVoids {
        schema: DomainSchema,
        entities: Vec<DomainEntity>,
        #[serde(default)]
        config: VoidSolverConfig,
    },
    EncodeNumeric {
        axes: Vec<NumericAxis>,
        values: Vec<NumericValue>,
    },
    DecodeNumeric {
        axes: Vec<NumericAxis>,
        vector: Vec<f32>,
    },
    /// The same deterministic text embedding `find_voids` applies to text facets,
    /// so callers can place entities and void centers on one map.
    Embed {
        texts: Vec<String>,
    },
}

/// Pure request handler shared by the wasm export and native tests.
pub fn handle(request: &str) -> String {
    let outcome = serde_json::from_str::<Request>(request)
        .map_err(|e| format!("invalid request: {e}"))
        .and_then(dispatch);
    match outcome {
        Ok(result) => json!({ "ok": true, "result": result }).to_string(),
        Err(error) => json!({ "ok": false, "error": error }).to_string(),
    }
}

fn dispatch(req: Request) -> Result<Value, String> {
    match req {
        Request::Version => Ok(json!({ "lacuna_wasm": VERSION })),
        Request::FindVoids {
            schema,
            mut entities,
            config,
        } => {
            for e in &mut entities {
                e.ensure_embedded(&schema, DEFAULT_EMBED_DIM);
            }
            let voids = find_interior_voids(&schema, &entities, &config);
            serde_json::to_value(voids).map_err(|e| e.to_string())
        }
        Request::EncodeNumeric { axes, values } => encode_numeric(&axes, &values).map(|v| json!(v)),
        Request::DecodeNumeric { axes, vector } => decode_numeric(&axes, &vector)
            .and_then(|d| serde_json::to_value(d).map_err(|e| e.to_string())),
        Request::Embed { texts } => Ok(json!(
            texts
                .iter()
                .map(|t| deterministic_embed(t, DEFAULT_EMBED_DIM))
                .collect::<Vec<_>>()
        )),
    }
}

/// Allocates `len` bytes in linear memory for the host to write a request into.
#[unsafe(no_mangle)]
pub extern "C" fn lacuna_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees a buffer previously returned by [`lacuna_alloc`] or [`lacuna_call`].
///
/// # Safety
/// `ptr` and `len` must be exactly a pair handed out by this module, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lacuna_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        // SAFETY: caller contract — (ptr, len) came from a forgotten Vec with capacity ≥ len.max(1).
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
    }
}

/// Handles the JSON request at `(ptr, len)` and returns a buffer laid out as
/// `[u32 little-endian byte length][UTF-8 JSON]`; free it with
/// `lacuna_free(out, 4 + length)`.
///
/// # Safety
/// `(ptr, len)` must be readable linear memory holding the request bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lacuna_call(ptr: *const u8, len: usize) -> *mut u8 {
    // SAFETY: caller contract — the host wrote `len` bytes at `ptr`.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let response = match std::str::from_utf8(bytes) {
        Ok(text) => handle(text),
        Err(_) => json!({ "ok": false, "error": "request is not UTF-8" }).to_string(),
    };
    let body = response.into_bytes();
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    let ptr = out.as_mut_ptr();
    std::mem::forget(out);
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn version_and_bad_request() {
        assert_eq!(parse(&handle(r#"{"op":"version"}"#))["ok"], true);
        let bad = parse(&handle(r#"{"op":"nope"}"#));
        assert_eq!(bad["ok"], false);
    }

    #[test]
    fn numeric_corpus_voids_decode_inside_bounds() {
        let axes = json!([
            {"kind":"scalar","id":"n_rate","min":0.0,"max":250.0},
            {"kind":"scalar","id":"store_months","min":0.0,"max":9.0}
        ]);
        let mut entities = Vec::new();
        let grid = [
            (60.0, 0.0),
            (220.0, 0.0),
            (60.0, 9.0),
            (220.0, 9.0),
            (140.0, 4.0),
            (100.0, 7.0),
        ];
        for (i, (n, m)) in grid.iter().enumerate() {
            let enc = parse(&handle(
                &json!({"op":"encode_numeric","axes":axes,"values":[n,m]}).to_string(),
            ));
            entities.push(json!({
                "id": format!("s{i}"), "name": format!("strategy {i}"), "summary": "", "tags": [],
                "facets": {}, "vectors": {"economic": enc["result"]},
                "demand_signal": 5.0, "quality_signal": 0.8, "metadata": {}
            }));
        }
        let schema = json!({
            "domain_id":"farm-test","title":"t","description":"t",
            "facets":[{"id":"economic","name":"Economic","description":"","default_weight":1.0}],
            "demand_metric_name":"m","quality_metric_name":"q"
        });
        let req = json!({"op":"find_voids","schema":schema,"entities":entities,
            "config":{"top_k":3,"min_radius":0.005,"min_void_separation":0.01,"anchor_distance_window":[0.01,1.0]}});
        let out = parse(&handle(&req.to_string()));
        assert_eq!(out["ok"], true, "{out}");
        let voids = out["result"].as_array().unwrap();
        assert!(!voids.is_empty());
        let center = &voids[0]["center"]["economic"];
        let dec = parse(&handle(
            &json!({"op":"decode_numeric","axes":axes,"vector":center}).to_string(),
        ));
        let n = dec["result"][0]["value"].as_f64().unwrap();
        let m = dec["result"][1]["value"].as_f64().unwrap();
        assert!((60.0..=220.0).contains(&n), "n={n}");
        assert!((0.0..=9.0).contains(&m), "m={m}");
    }

    #[test]
    fn embed_matches_find_voids_text_embedding() {
        let out = parse(&handle(
            r#"{"op":"embed","texts":["3d printed pet toys","3d printed pet toys"]}"#,
        ));
        let v = out["result"].as_array().unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], v[1]);
        assert_eq!(v[0].as_array().unwrap().len(), DEFAULT_EMBED_DIM);
    }
}
