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
            _ => fail(&format!(
                "{field} holds a value that is not a finite number: {item}"
            )),
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

/// Format 1: a float network (`export.py`), as `Learned::Float`.
fn float_code(
    json: &Value,
    names: &[String],
    keys: &[String],
    width: usize,
    features_version: u32,
    catalog: &str,
    code: &mut String,
) {
    let field = |name: &str| -> &Value { json.get(name).unwrap_or_else(|| fail(&format!("no {name}"))) };
    let dense = names.len();
    let mean = numbers(field("mean"), "mean");
    let scale = numbers(field("scale"), "scale");
    if mean.len() != dense || scale.len() != dense {
        fail(&format!(
            "mean ({}) and scale ({}) are not one per dense name ({dense})",
            mean.len(),
            scale.len()
        ));
    }
    if scale.contains(&0.0) {
        fail("a scale is 0");
    }
    let dense_in = matrix(field("denseIn"), "denseIn", dense, width);
    let sparse_in = matrix(field("sparseIn"), "sparseIn", keys.len(), width);
    let bias = numbers(field("bias"), "bias");
    if bias.len() != width {
        fail(&format!("bias has {} values, not {width}", bias.len()));
    }
    let mut layers: Vec<(usize, Vec<f64>, Vec<f64>)> = Vec::new();
    let mut inputs = width;
    for (at, layer) in field("head")
        .as_array()
        .unwrap_or_else(|| fail("head is not an array"))
        .iter()
        .enumerate()
    {
        let b = numbers(
            layer.get("b").unwrap_or_else(|| fail("a head layer has no b")),
            &format!("head[{at}].b"),
        );
        let w = matrix(
            layer.get("w").unwrap_or_else(|| fail("a head layer has no w")),
            &format!("head[{at}].w"),
            b.len(),
            inputs,
        );
        layers.push((inputs, w, b.clone()));
        inputs = b.len();
    }
    let out = field("out");
    let out_w = numbers(out.get("w").unwrap_or_else(|| fail("out has no w")), "out.w");
    if out_w.len() != inputs {
        fail(&format!("out.w has {} values, not {inputs}", out_w.len()));
    }
    let out_b = out
        .get("b")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| fail("out.b is not a number"));
    let number = |name: &str| {
        field(name)
            .as_f64()
            .filter(|x| x.is_finite())
            .unwrap_or_else(|| fail(&format!("{name} is not a number")))
    };
    let (hand_a, hand_b, hand_unit) = (number("handA"), number("handB"), number("handUnit"));
    // (b)'s linear term, one per dense input; a network without one has none.
    let linear = json
        .get("linear")
        .map(|v| numbers(v, "linear"))
        .unwrap_or_else(|| vec![0.0; dense]);
    if linear.len() != dense {
        fail(&format!(
            "linear has {} values, not one per dense name ({dense})",
            linear.len()
        ));
    }

    for (at, (inputs, w, b)) in layers.iter().enumerate() {
        writeln!(
            code,
            "const LEARNED_HEAD_{at}: Layer<'static> = Layer {{ inputs: {inputs}, w: {}, b: {} }};",
            slice(w),
            slice(b)
        )
        .unwrap();
    }
    write!(code, "const LEARNED_HEAD: &[Layer<'static>] = &[").unwrap();
    for at in 0..layers.len() {
        write!(code, "LEARNED_HEAD_{at},").unwrap();
    }
    writeln!(code, "];").unwrap();
    write!(code, "const LEARNED_KEYS: &[&str] = &[").unwrap();
    for key in keys {
        write!(code, "{key:?},").unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "/// The network compiled in from `src/learned/weights.json`."
    )
    .unwrap();
    writeln!(
        code,
        "pub const LEARNED: Learned<'static> = Learned::Float(Network {{ features_version: {features_version}, catalog: {catalog:?}, \
         width: {width}, mean: {}, scale: {}, keys: LEARNED_KEYS, dense_in: {}, sparse_in: {}, bias: {}, \
         head: LEARNED_HEAD, out_w: {}, out_b: {}, linear: {}, hand_a: {}, hand_b: {}, hand_unit: {} }});",
        slice(&mean),
        slice(&scale),
        slice(&dense_in),
        slice(&sparse_in),
        slice(&bias),
        slice(&out_w),
        literal(out_b),
        slice(&linear),
        literal(hand_a),
        literal(hand_b),
        literal(hand_unit),
    )
    .unwrap();
}

/// Standard base64 (RFC 4648), as `src/learned/int.rs` reads it.
fn base64(text: &str, field: &str) -> Vec<u8> {
    let value = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => fail(&format!("{field} is not base64")),
        }
    };
    let bytes: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if !bytes.len().is_multiple_of(4) {
        fail(&format!("{field} is not base64"));
    }
    let mut out = Vec::new();
    for quad in bytes.chunks(4) {
        let pad = quad.iter().rev().take_while(|c| **c == b'=').count();
        let mut n = 0u32;
        for (at, c) in quad.iter().enumerate() {
            n = (n << 6) | if at >= 4 - pad { 0 } else { value(*c) };
        }
        out.extend_from_slice(&n.to_be_bytes()[1..4 - pad]);
    }
    out
}

fn words(json: &Value, field: &str, size: usize) -> Vec<i64> {
    let text = json
        .as_str()
        .unwrap_or_else(|| fail(&format!("{field} is not base64 text")));
    let bytes = base64(text, field);
    if !bytes.len().is_multiple_of(size) {
        fail(&format!(
            "{field}: a length that is not whole words of {size} bytes"
        ));
    }
    bytes
        .chunks(size)
        .map(|chunk| match size {
            1 => i64::from(chunk[0] as i8),
            2 => i64::from(i16::from_le_bytes([chunk[0], chunk[1]])),
            _ => i64::from_le_bytes(chunk.try_into().expect("eight bytes")),
        })
        .collect()
}

fn ints(values: &[i64]) -> String {
    let items: Vec<String> = values.iter().map(i64::to_string).collect();
    format!("&[{}]", items.join(","))
}

fn whole(json: &Value, field: &str) -> i64 {
    json.as_i64()
        .unwrap_or_else(|| fail(&format!("{field} is not a whole number")))
}

/// Format 2: an integer network (`quantize.py`), as `Learned::Int`.
fn int_code(
    json: &Value,
    names: &[String],
    keys: &[String],
    width: usize,
    features_version: u32,
    catalog: &str,
    code: &mut String,
) {
    let field = |name: &str| -> &Value { json.get(name).unwrap_or_else(|| fail(&format!("no {name}"))) };
    let dense = names.len();
    let dense_in = words(field("denseIn"), "denseIn", 2);
    let sparse_in = words(field("sparseIn"), "sparseIn", 2);
    let bias = words(field("bias"), "bias", 8);
    if dense_in.len() != dense * width || sparse_in.len() != keys.len() * width || bias.len() != width {
        fail("the first layer's shapes do not add up");
    }
    let shift = whole(field("shift"), "shift");
    let activation = whole(field("activation"), "activation");
    if !(0..=24).contains(&shift) || activation <= 0 {
        fail("shift or activation out of range");
    }
    let mut inputs = width;
    let mut heads = Vec::new();
    for (at, layer) in field("head")
        .as_array()
        .unwrap_or_else(|| fail("head is not an array"))
        .iter()
        .enumerate()
    {
        let get = |name: &str| {
            layer
                .get(name)
                .unwrap_or_else(|| fail(&format!("head[{at}] has no {name}")))
        };
        let w = words(get("w"), "head w", 1);
        let b = words(get("b"), "head b", 8);
        let layer_shift = whole(get("shift"), "head shift");
        if whole(get("inputs"), "inputs") != inputs as i64
            || w.len() != b.len() * inputs
            || !(0..=24).contains(&layer_shift)
        {
            fail(&format!("head[{at}]'s shapes do not add up"));
        }
        writeln!(code, "const LEARNED_INT_HEAD_{at}: IntLayer<'static> = IntLayer {{ inputs: {inputs}, shift: {layer_shift}, w: {}, b: {} }};", ints(&w), ints(&b)).unwrap();
        heads.push(at);
        inputs = b.len();
    }
    write!(code, "const LEARNED_INT_HEAD: &[IntLayer<'static>] = &[").unwrap();
    for at in heads {
        write!(code, "LEARNED_INT_HEAD_{at},").unwrap();
    }
    writeln!(code, "];").unwrap();
    let out = field("out");
    let get = |name: &str| {
        out.get(name)
            .unwrap_or_else(|| fail(&format!("out has no {name}")))
    };
    let out_w = words(get("w"), "out w", 1);
    let out_shift = whole(get("shift"), "out shift");
    if out_w.len() != inputs || !(0..=24).contains(&out_shift) {
        fail("out's shapes do not add up");
    }
    let out_b = whole(get("b"), "out b");
    let linear = match json.get("linear") {
        Some(text) => words(text, "linear", 8),
        None => vec![0; dense],
    };
    if linear.len() != dense {
        fail("linear is not one per dense name");
    }
    let number = |name: &str| {
        field(name)
            .as_f64()
            .filter(|x| x.is_finite())
            .unwrap_or_else(|| fail(&format!("{name} is not a number")))
    };
    write!(code, "const LEARNED_KEYS: &[&str] = &[").unwrap();
    for key in keys {
        write!(code, "{key:?},").unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "/// The network compiled in from `src/learned/weights.json`."
    )
    .unwrap();
    writeln!(
        code,
        "pub const LEARNED: Learned<'static> = Learned::Int(IntNetwork {{ features_version: {features_version}, catalog: {catalog:?}, \
         width: {width}, dense: {dense}, activation: {activation}, shift: {shift}, keys: LEARNED_KEYS, dense_in: {}, sparse_in: {}, \
         bias: {}, head: LEARNED_INT_HEAD, out_shift: {out_shift}, out_w: {}, out_b: {out_b}, linear: {}, hand_a: {}, hand_b: {}, hand_unit: {} }});",
        ints(&dense_in),
        ints(&sparse_in),
        ints(&bias),
        ints(&out_w),
        ints(&linear),
        literal(number("handA")),
        literal(number("handB")),
        literal(number("handUnit")),
    )
    .unwrap();
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
    let catalog = field("catalog")
        .as_str()
        .unwrap_or_else(|| fail("catalog is not text"))
        .to_string();
    let width = usize::try_from(
        field("width")
            .as_u64()
            .unwrap_or_else(|| fail("width is not a whole number")),
    )
    .unwrap_or_else(|_| fail("width is too large"));
    if width == 0 {
        fail("width is 0");
    }
    let names: Vec<String> = field("denseNames")
        .as_array()
        .unwrap_or_else(|| fail("denseNames is not an array"))
        .iter()
        .map(|name| {
            name.as_str()
                .unwrap_or_else(|| fail("a dense name is not text"))
                .to_string()
        })
        .collect();
    let keys: Vec<String> = field("keys")
        .as_array()
        .unwrap_or_else(|| fail("keys is not an array"))
        .iter()
        .map(|key| {
            key.as_str()
                .unwrap_or_else(|| fail("a key is not text"))
                .to_string()
        })
        .collect();
    if keys.windows(2).any(|pair| pair[0] >= pair[1]) {
        fail("keys are not sorted and unique");
    }
    let mut code = String::new();
    writeln!(
        code,
        "/// The dense features the compiled-in network was fitted on, in order."
    )
    .unwrap();
    write!(code, "pub const LEARNED_DENSE_NAMES: &[&str] = &[").unwrap();
    for name in &names {
        write!(code, "{name:?},").unwrap();
    }
    writeln!(code, "];").unwrap();
    let format = field("format").as_u64().unwrap_or(1);
    match format {
        1 => float_code(&json, &names, &keys, width, features_version, &catalog, &mut code),
        2 => int_code(&json, &names, &keys, width, features_version, &catalog, &mut code),
        _ => fail(&format!(
            "format {format} is neither a float (1) nor an integer (2) network"
        )),
    }

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out_dir.join("learned_weights.rs"), code).expect("write learned_weights.rs");
}
