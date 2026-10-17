//! The learned evaluation's weights, compiled in (#1010; `src/learned.rs`). Reads
//! `src/learned/weights.json`, which `training/ml/jackioh_ml/export.py` writes, and writes
//! `$OUT_DIR/learned_weights.rs`: `LEARNED`, a `Network<'static>` over `const` slices, and
//! `LEARNED_DENSE_NAMES`. So the network reaches this pure crate the way the catalog reaches the cards
//! crate, at compile time, with no static set at run time (CLAUDE.md rule 4).
//!
//! It fails the build, naming the field, when a shape does not add up: `mean`, `scale` and
//! `denseNames` of one length, `denseIn` that many rows of `width`, `sparseIn` one row of `width`
//! per key, the keys sorted and unique, each head layer's `w` its `b`'s length times its input's,
//! `out.w` the last layer's width, and every number finite.

// A build script is not the pure crate: it runs at compile time, where data reaches it (rule 4).
#![allow(clippy::disallowed_methods)]

use std::fmt::Write as _;
use std::path::PathBuf;

use serde_json::Value;

fn fail(message: &str) -> ! {
    panic!("crates/ai/src/learned/weights.json: {message}");
}

fn numbers(value: &Value, field: &str) -> Vec<f64> {
    let Some(items) = value.as_array() else {
        fail(&format!("{field} is not an array"));
    };
    items
        .iter()
        .map(|item| match item.as_f64() {
            Some(x) if x.is_finite() => x,
            _ => fail(&format!("{field} holds a value that is not a finite number: {item}")),
        })
        .collect()
}

fn matrix(value: &Value, field: &str, rows: usize, cols: usize) -> Vec<f64> {
    let Some(items) = value.as_array() else {
        fail(&format!("{field} is not an array"));
    };
    if items.len() != rows {
        fail(&format!("{field} has {} rows, not {rows}", items.len()));
    }
    let mut flat = Vec::with_capacity(rows * cols);
    for (at, row) in items.iter().enumerate() {
        let row = numbers(row, &format!("{field}[{at}]"));
        if row.len() != cols {
            fail(&format!("{field}[{at}] has {} values, not {cols}", row.len()));
        }
        flat.extend(row);
    }
    flat
}

fn literal(x: f64) -> String {
    // `{:?}` is the shortest text that reads back as the same f64, and always a float literal.
    format!("{x:?}")
}

fn slice(values: &[f64]) -> String {
    let mut out = String::from("&[");
    for (at, x) in values.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push_str(&literal(*x));
    }
    out.push(']');
    out
}

fn main() {
    let dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let path = dir.join("src/learned/weights.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| fail(&error.to_string()));
    let json: Value = serde_json::from_str(&text).unwrap_or_else(|error| fail(&error.to_string()));

    let field = |name: &str| -> &Value { json.get(name).unwrap_or_else(|| fail(&format!("no {name}"))) };
    let features_version = field("featuresVersion")
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or_else(|| fail("featuresVersion is not a whole number"));
    let catalog = field("catalog").as_str().unwrap_or_else(|| fail("catalog is not text")).to_string();
    let width = usize::try_from(field("width").as_u64().unwrap_or_else(|| fail("width is not a whole number")))
        .unwrap_or_else(|_| fail("width is too large"));
    if width == 0 {
        fail("width is 0");
    }
    let names: Vec<String> = field("denseNames")
        .as_array()
        .unwrap_or_else(|| fail("denseNames is not an array"))
        .iter()
        .map(|name| name.as_str().unwrap_or_else(|| fail("a dense name is not text")).to_string())
        .collect();
    let dense = names.len();
    let mean = numbers(field("mean"), "mean");
    let scale = numbers(field("scale"), "scale");
    if mean.len() != dense || scale.len() != dense {
        fail(&format!("mean ({}) and scale ({}) are not one per dense name ({dense})", mean.len(), scale.len()));
    }
    if scale.iter().any(|s| *s == 0.0) {
        fail("a scale is 0");
    }
    let keys: Vec<String> = field("keys")
        .as_array()
        .unwrap_or_else(|| fail("keys is not an array"))
        .iter()
        .map(|key| key.as_str().unwrap_or_else(|| fail("a key is not text")).to_string())
        .collect();
    if keys.windows(2).any(|pair| pair[0] >= pair[1]) {
        fail("keys are not sorted and unique");
    }
    let dense_in = matrix(field("denseIn"), "denseIn", dense, width);
    let sparse_in = matrix(field("sparseIn"), "sparseIn", keys.len(), width);
    let bias = numbers(field("bias"), "bias");
    if bias.len() != width {
        fail(&format!("bias has {} values, not {width}", bias.len()));
    }
    let mut layers: Vec<(usize, Vec<f64>, Vec<f64>)> = Vec::new();
    let mut inputs = width;
    for (at, layer) in field("head").as_array().unwrap_or_else(|| fail("head is not an array")).iter().enumerate() {
        let b = numbers(layer.get("b").unwrap_or_else(|| fail("a head layer has no b")), &format!("head[{at}].b"));
        let w = matrix(layer.get("w").unwrap_or_else(|| fail("a head layer has no w")), &format!("head[{at}].w"), b.len(), inputs);
        layers.push((inputs, w, b.clone()));
        inputs = b.len();
    }
    let out = field("out");
    let out_w = numbers(out.get("w").unwrap_or_else(|| fail("out has no w")), "out.w");
    if out_w.len() != inputs {
        fail(&format!("out.w has {} values, not {inputs}", out_w.len()));
    }
    let out_b = out.get("b").and_then(Value::as_f64).unwrap_or_else(|| fail("out.b is not a number"));
    let number = |name: &str| field(name).as_f64().filter(|x| x.is_finite()).unwrap_or_else(|| fail(&format!("{name} is not a number")));
    let (hand_a, hand_b, hand_unit) = (number("handA"), number("handB"), number("handUnit"));

    let mut code = String::new();
    writeln!(code, "/// The dense features the compiled-in network was fitted on, in order.").unwrap();
    write!(code, "pub const LEARNED_DENSE_NAMES: &[&str] = &[").unwrap();
    for name in &names {
        write!(code, "{name:?},").unwrap();
    }
    writeln!(code, "];").unwrap();
    for (at, (inputs, w, b)) in layers.iter().enumerate() {
        writeln!(code, "const LEARNED_HEAD_{at}: Layer<'static> = Layer {{ inputs: {inputs}, w: {}, b: {} }};", slice(w), slice(b)).unwrap();
    }
    write!(code, "const LEARNED_HEAD: &[Layer<'static>] = &[").unwrap();
    for at in 0..layers.len() {
        write!(code, "LEARNED_HEAD_{at},").unwrap();
    }
    writeln!(code, "];").unwrap();
    write!(code, "const LEARNED_KEYS: &[&str] = &[").unwrap();
    for key in &keys {
        write!(code, "{key:?},").unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(code, "/// The network compiled in from `src/learned/weights.json`.").unwrap();
    writeln!(
        code,
        "pub const LEARNED: Network<'static> = Network {{ features_version: {features_version}, catalog: {catalog:?}, \
         width: {width}, mean: {}, scale: {}, keys: LEARNED_KEYS, dense_in: {}, sparse_in: {}, bias: {}, \
         head: LEARNED_HEAD, out_w: {}, out_b: {}, hand_a: {}, hand_b: {}, hand_unit: {} }};",
        slice(&mean),
        slice(&scale),
        slice(&dense_in),
        slice(&sparse_in),
        slice(&bias),
        slice(&out_w),
        literal(out_b),
        literal(hand_a),
        literal(hand_b),
        literal(hand_unit),
    )
    .unwrap();

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out_dir.join("learned_weights.rs"), code).expect("write learned_weights.rs");
}
