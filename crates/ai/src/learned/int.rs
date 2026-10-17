//! The integer network (#1011, `docs/learned-eval.md` D7), the one that ships:
//! `training/ml/jackioh_ml/quantize.py` makes it from a float one, and `quantize.correction` is its
//! reference.
//!
//! The first layer takes the raw dense features (the fit's standardizer folded into its weights),
//! i16 weights at the scale `A·2^shift`, `A` a clipped activation's one (127). Each head layer and the
//! output are i8 at a `2^shift` of their own. Every sum is an i64, every shift an arithmetic shift
//! right (a floor), every clip to `[0, A]`, and the output's sum is divided once, in IEEE doubles,
//! into the correction's logit: the same answer on every target, by construction (R1011000).

use crate::features::Features;

/// One head layer: `out = clip((w · input + b) >> shift, 0, A)`, `w` i8 row-major
/// `outputs × inputs`, `b` at the layer's scale `A·2^shift`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntLayer<'a> {
    pub inputs: usize,
    pub shift: u32,
    pub w: &'a [i8],
    pub b: &'a [i64],
}

/// An integer network, borrowed: `LEARNED` holds the compiled-in one, and `OwnedIntNetwork::view`
/// lends one read at run time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntNetwork<'a> {
    pub features_version: u32,
    pub catalog: &'a str,
    pub width: usize,
    /// How many dense features it reads.
    pub dense: usize,
    /// A clipped activation's one (127).
    pub activation: i64,
    pub shift: u32,
    /// The sparse keys it has a row for, sorted.
    pub keys: &'a [&'a str],
    /// `dense × width`, row-major by input.
    pub dense_in: &'a [i16],
    /// `keys.len() × width`, row-major by key.
    pub sparse_in: &'a [i16],
    pub bias: &'a [i64],
    pub head: &'a [IntLayer<'a>],
    pub out_shift: u32,
    pub out_w: &'a [i8],
    pub out_b: i64,
    /// (b)'s linear term over the raw dense features, at the output's scale `A·2^out_shift`.
    pub linear: &'a [i64],
    pub hand_a: f64,
    pub hand_b: f64,
    pub hand_unit: f64,
}

/// An integer activation clipped to `[0, top]`, as comparisons.
fn clip_int(x: i64, top: i64) -> i64 {
    if x < 0 {
        0
    } else if x > top {
        top
    } else {
        x
    }
}

impl IntNetwork<'_> {
    /// `net(x)` in integers, then one division into a logit. Zero for features of another layout.
    pub fn correction(&self, features: &Features) -> f64 {
        let width = self.width;
        if features.dense.len() != self.dense || self.bias.len() != width {
            return 0.0;
        }
        let mut acc: Vec<i64> = self.bias.to_vec();
        let mut linear = 0i64;
        for (j, value) in features.dense.iter().enumerate() {
            if *value == 0 {
                continue;
            }
            let x = i64::from(*value);
            for (a, w) in acc.iter_mut().zip(&self.dense_in[j * width..(j + 1) * width]) {
                *a += x * i64::from(*w);
            }
            linear += x * self.linear.get(j).copied().unwrap_or(0);
        }
        for (key, count) in &features.sparse {
            if let Ok(r) = self.keys.binary_search(&key.as_str()) {
                let count = i64::from(*count);
                for (a, w) in acc.iter_mut().zip(&self.sparse_in[r * width..(r + 1) * width]) {
                    *a += count * i64::from(*w);
                }
            }
        }
        let mut hidden: Vec<i64> = acc
            .iter()
            .map(|a| clip_int(a >> self.shift, self.activation))
            .collect();
        for layer in self.head {
            let mut next: Vec<i64> = Vec::with_capacity(layer.b.len());
            for (k, bias) in layer.b.iter().enumerate() {
                let mut total = *bias;
                for (w, h) in layer.w[k * layer.inputs..(k + 1) * layer.inputs]
                    .iter()
                    .zip(&hidden)
                {
                    total += i64::from(*w) * h;
                }
                next.push(clip_int(total >> layer.shift, self.activation));
            }
            hidden = next;
        }
        let mut total = self.out_b;
        for (w, h) in self.out_w.iter().zip(&hidden) {
            total += i64::from(*w) * h;
        }
        total += linear;
        // Both are exact in an f64 (far under 2^53), and the one division is IEEE's.
        total as f64 / (self.activation << self.out_shift) as f64
    }

    /// The position's value in the hand eval's points: `hand + (b + net(x))·U/a`.
    pub fn points(&self, hand: f64, features: &Features) -> f64 {
        if self.hand_a == 0.0 {
            return hand;
        }
        hand + (self.hand_b + self.correction(features)) * self.hand_unit / self.hand_a
    }
}

/// Standard base64 (RFC 4648, `=` padded) decoded, or `None` for text that is not.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for quad in bytes.chunks(4) {
        let pad = quad.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 {
            return None;
        }
        let mut n = 0u32;
        for (at, c) in quad.iter().enumerate() {
            let v = if at >= 4 - pad { 0 } else { value(*c)? };
            n = (n << 6) | v;
        }
        let three = n.to_be_bytes();
        out.extend_from_slice(&three[1..4 - pad]);
    }
    Some(out)
}

/// Little-endian words of `N` bytes, read by `read`; `None` for a length that is not a multiple.
fn words<const N: usize, T>(bytes: &[u8], read: fn([u8; N]) -> T) -> Option<Vec<T>> {
    if !bytes.len().is_multiple_of(N) {
        return None;
    }
    Some(
        bytes
            .chunks(N)
            .map(|chunk| {
                let mut word = [0u8; N];
                word.copy_from_slice(chunk);
                read(word)
            })
            .collect(),
    )
}

/// An integer network read at run time from weights.json's object (`"format": 2`), owned: the
/// parity fixture's, natively and in the WASM build (R1011000). The compiled-in network never comes
/// this way: `build.rs` makes it constant.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedIntNetwork {
    features_version: u32,
    catalog: String,
    width: usize,
    dense: usize,
    activation: i64,
    shift: u32,
    keys: Vec<String>,
    dense_in: Vec<i16>,
    sparse_in: Vec<i16>,
    bias: Vec<i64>,
    head: Vec<(usize, u32, Vec<i8>, Vec<i64>)>,
    out_shift: u32,
    out_w: Vec<i8>,
    out_b: i64,
    linear: Vec<i64>,
    hand_a: f64,
    hand_b: f64,
    hand_unit: f64,
}

fn whole(value: &serde_json::Value, name: &str) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| format!("{name} is not a whole number"))
}

fn shift_of(value: &serde_json::Value, name: &str) -> Result<u32, String> {
    whole(value, name).and_then(|s| u32::try_from(s).map_err(|_| format!("{name} is not a shift")))
}

fn base64_field(value: &serde_json::Value, name: &str) -> Result<Vec<u8>, String> {
    value
        .as_str()
        .and_then(base64_decode)
        .ok_or_else(|| format!("{name} is not base64"))
}

impl OwnedIntNetwork {
    /// Reads weights.json's object; `Err` names what is missing or does not add up.
    pub fn from_json(json: &serde_json::Value) -> Result<OwnedIntNetwork, String> {
        let field = |name: &str| json.get(name).ok_or_else(|| format!("no {name}"));
        let float = |name: &str| {
            field(name)?
                .as_f64()
                .ok_or_else(|| format!("{name} is not a number"))
        };
        let i16s = |name: &str| {
            words::<2, i16>(&base64_field(field(name)?, name)?, i16::from_le_bytes)
                .ok_or_else(|| format!("{name}: a length that is not whole i16s"))
        };
        if whole(field("format")?, "format")? != 2 {
            return Err("not an integer network (format 2)".to_string());
        }
        let keys: Vec<String> = field("keys")?
            .as_array()
            .ok_or("keys is not an array")?
            .iter()
            .map(|key| key.as_str().map(str::to_string).ok_or("a key is not text"))
            .collect::<Result<_, _>>()?;
        let mut head = Vec::new();
        for (at, layer) in field("head")?
            .as_array()
            .ok_or("head is not an array")?
            .iter()
            .enumerate()
        {
            let get = |name: &str| layer.get(name).ok_or_else(|| format!("head[{at}] has no {name}"));
            let inputs =
                usize::try_from(whole(get("inputs")?, "inputs")?).map_err(|_| "inputs is negative")?;
            let w = words::<1, i8>(&base64_field(get("w")?, "head w")?, i8::from_le_bytes).ok_or("head w")?;
            let b = words::<8, i64>(&base64_field(get("b")?, "head b")?, i64::from_le_bytes)
                .ok_or("head b: a length that is not whole i64s")?;
            head.push((inputs, shift_of(get("shift")?, "head shift")?, w, b));
        }
        let out = field("out")?;
        let get = |name: &str| out.get(name).ok_or_else(|| format!("out has no {name}"));
        let net = OwnedIntNetwork {
            features_version: u32::try_from(whole(field("featuresVersion")?, "featuresVersion")?)
                .map_err(|_| "featuresVersion is negative")?,
            catalog: field("catalog")?.as_str().unwrap_or("").to_string(),
            width: usize::try_from(whole(field("width")?, "width")?).map_err(|_| "width is negative")?,
            dense: field("denseNames")?
                .as_array()
                .ok_or("denseNames is not an array")?
                .len(),
            activation: whole(field("activation")?, "activation")?,
            shift: shift_of(field("shift")?, "shift")?,
            dense_in: i16s("denseIn")?,
            sparse_in: i16s("sparseIn")?,
            bias: words::<8, i64>(&base64_field(field("bias")?, "bias")?, i64::from_le_bytes)
                .ok_or("bias: a length that is not whole i64s")?,
            keys,
            head,
            out_shift: shift_of(get("shift")?, "out shift")?,
            out_w: words::<1, i8>(&base64_field(get("w")?, "out w")?, i8::from_le_bytes).ok_or("out w")?,
            out_b: whole(get("b")?, "out b")?,
            linear: match json.get("linear") {
                Some(text) => words::<8, i64>(&base64_field(text, "linear")?, i64::from_le_bytes)
                    .ok_or("linear: a length that is not whole i64s")?,
                None => Vec::new(),
            },
            hand_a: float("handA")?,
            hand_b: float("handB")?,
            hand_unit: float("handUnit")?,
        };
        if net.dense_in.len() != net.dense * net.width
            || net.sparse_in.len() != net.keys.len() * net.width
            || net.bias.len() != net.width
        {
            return Err("the first layer's shapes do not add up".to_string());
        }
        Ok(net)
    }

    /// The keys as borrowed text, for `view`.
    pub fn key_refs(&self) -> Vec<&str> {
        self.keys.iter().map(String::as_str).collect()
    }

    /// The head layers borrowed, for `view`.
    pub fn layers(&self) -> Vec<IntLayer<'_>> {
        self.head
            .iter()
            .map(|(inputs, shift, w, b)| IntLayer {
                inputs: *inputs,
                shift: *shift,
                w,
                b,
            })
            .collect()
    }

    /// The network lent as an `IntNetwork`, over `keys` (from `key_refs`) and `layers` (`layers`).
    pub fn view<'a>(&'a self, keys: &'a [&'a str], layers: &'a [IntLayer<'a>]) -> IntNetwork<'a> {
        IntNetwork {
            features_version: self.features_version,
            catalog: &self.catalog,
            width: self.width,
            dense: self.dense,
            activation: self.activation,
            shift: self.shift,
            keys,
            dense_in: &self.dense_in,
            sparse_in: &self.sparse_in,
            bias: &self.bias,
            head: layers,
            out_shift: self.out_shift,
            out_w: &self.out_w,
            out_b: self.out_b,
            linear: &self.linear,
            hand_a: self.hand_a,
            hand_b: self.hand_b,
            hand_unit: self.hand_unit,
        }
    }
}

/// R1011000: the parity fixture's answers (`crates/ai/tests/fixtures/learned-parity.json`): its
/// network's correction for each case, as JSON numbers. Natively and in the WASM build alike.
pub fn parity_answers(fixture: &serde_json::Value) -> Result<Vec<f64>, String> {
    let owned = OwnedIntNetwork::from_json(fixture.get("network").ok_or("no network")?)?;
    let keys = owned.key_refs();
    let layers = owned.layers();
    let net = owned.view(&keys, &layers);
    let mut answers = Vec::new();
    for case in fixture
        .get("cases")
        .and_then(|c| c.as_array())
        .ok_or("no cases")?
    {
        let features: Features = serde_json::from_value(serde_json::json!({
            "dense": case.get("dense").ok_or("a case has no dense")?,
            "sparse": case.get("sparse").ok_or("a case has no sparse")?,
        }))
        .map_err(|error| error.to_string())?;
        answers.push(net.correction(&features));
    }
    Ok(answers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_reads_rfc_4648() {
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(base64_decode("Zm9v").unwrap(), b"foo");
        assert_eq!(base64_decode("+/8=").unwrap(), vec![0xfb, 0xff]);
        assert!(base64_decode("Zm9").is_none());
        assert!(base64_decode("Z===").is_none());
    }

    #[test]
    fn clip_int_is_comparisons() {
        assert_eq!(clip_int(-5, 127), 0);
        assert_eq!(clip_int(64, 127), 64);
        assert_eq!(clip_int(500, 127), 127);
    }
}
