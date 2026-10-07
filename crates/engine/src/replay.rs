//! Replay: (seed, decks, action log) rebuilds a match exactly, and a state hash makes two folds
//! comparable (SPEC §9.2, §9.3). A practice game adds its handicaps to that tuple (§9.9, R180, R187),
//! a game with a dealt deck the seats that were dealt one (R433), and a match its seats' last boards
//! (R417) and its Glitch boards (R678): they are setup, not actions, so
//! the fold hands them to `create_game` exactly as the live game did.
//!
//! Port of `packages/engine/src/replay.ts` (part 5). `canonical`, `fnv1a32_utf16` and `hash_state`
//! follow SURFACE §5.2 to the bit: practice saves on players' devices store the hash, the hotseat
//! fixture pins `"a798906b"`, and the golden traces (§13) and the server's migration checksum use the
//! same two functions. `fold` is SURFACE §6.1's `fold(&FoldArgs) -> FoldResult`; `FoldArgs` and
//! `FoldResult` are TS's `ReplayInput` and `ReplayResult` under the names SURFACE gives them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::Handicap;
use crate::state::{CreateGameOptions, GameState, LastBoardInput, create_game};
use crate::wire::{Action, CardDefs, PerPlayerOpt, PlayerId};

/// Canonical JSON: keys sorted, so two equal states always produce the same text.
///
/// SURFACE §5.2 step 2: a string, number, bool or null as `JSON.stringify` writes it; an array as
/// `[` + items joined by `,` + `]`; an object as `{` + `"key":canonical(value)` pairs, keys sorted
/// ascending by UTF-16 code units (TS's `<`), joined by `,` + `}`. TS drops `undefined` values; a
/// serialised Rust value has none (an absent `Option` is no key at all, SURFACE §4.4.7).
pub fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&js_number(number)),
        Value::String(text) => out.push_str(&json_string(text)),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            // TS: `.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))`, a comparison of UTF-16 code
            // units. A stable sort, as `Array.prototype.sort` is (SURFACE §4.4.1); keys are unique.
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (at, (key, item)) in entries.into_iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(key));
                out.push(':');
                write_canonical(item, out);
            }
            out.push('}');
        }
    }
}

/// `JSON.stringify(text)`: serde_json writes the same escapes (`\" \\ \b \f \n \r \t`, `\u00xx` in
/// lower-case hex for the other control characters, everything else raw), SURFACE §5.2.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
}

/// `JSON.stringify(n)` for a JSON number: an integer as its digits; a float with no fraction as an
/// integer (JS has one number type, so `2.0` prints `2`); any other float as serde_json writes it,
/// which is the shortest round-trip form, as JS's is. The engine's state holds integers only.
fn js_number(number: &serde_json::Number) -> String {
    if number.is_i64() || number.is_u64() {
        return number.to_string();
    }
    match number.as_f64() {
        Some(float) if float.is_finite() && float.fract() == 0.0 && float.abs() < 1e21 => {
            format!("{float:.0}")
        }
        Some(float) if !float.is_finite() => String::from("null"),
        _ => number.to_string(),
    }
}

/// FNV-1a 32 over the UTF-16 code units of `text` (TS's `charCodeAt` loop), printed as 8 lower-case
/// hex digits (SURFACE §5.2 step 3). UTF-8 bytes would differ: state strings carry `× − – ♾ ³ ²`.
pub fn fnv1a32_utf16(text: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for unit in text.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{hash:08x}")
}

/// FNV-1a over the canonical state, minus the nonce log, which is bookkeeping, and the opening a Glitch
/// reset deals again (R676), which is a copy of the fold's own input, so no hash moved when it came.
pub fn hash_state(state: &GameState) -> String {
    let mut value = match serde_json::to_value(state) {
        Ok(value) => value,
        Err(error) => panic!("hash_state: a GameState did not serialise: {error}"),
    };
    if let Value::Object(map) = &mut value {
        map.remove("applied");
        map.remove("opening");
    }
    fnv1a32_utf16(&canonical(&value))
}

/// TS `ReplayInput`: what a fold starts from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplayInput {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    pub log: Vec<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CardDefs>,
    /// R180, R187: the same handicaps the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handicaps: Option<PerPlayerOpt<Handicap>>,
    /// R433: the same dealt seats the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dealt: Option<Vec<PlayerId>>,
    /// R417: the same last boards the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_boards: Option<LastBoardInput>,
    /// R678: the same Glitch boards the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glitch_boards: Option<LastBoardInput>,
}

/// SURFACE §6.1's name for `ReplayInput`.
pub type FoldArgs = ReplayInput;

/// One rejected action of a folded log: its nonce and the reducer's refusal, verbatim.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FoldError {
    pub nonce: String,
    pub error: String,
}

/// TS `ReplayResult`: `{ state, errors: [{ nonce, error }] }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub state: GameState,
    pub errors: Vec<FoldError>,
}

/// SURFACE §6.1's name for `ReplayResult`.
pub type FoldResult = ReplayResult;

/// Fold a recorded log from scratch. Errors are collected, not thrown: a log may hold rejects. The
/// setup is not: `create_game` throws on a deck its seat's handicap does not allow, so a handicapped
/// game folded without its handicaps fails loudly instead of replaying a different game (R180).
pub fn fold(input: &ReplayInput) -> ReplayResult {
    let start = create_game(&CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        catalog: input.catalog.clone(),
        handicaps: input.handicaps.clone(),
        dealt: input.dealt.clone(),
        last_boards: input.last_boards.clone(),
        glitch_boards: input.glitch_boards.clone(),
    });
    let mut state = crate::reduce::begin_game(&start).state;
    let mut errors: Vec<FoldError> = Vec::new();

    for action in &input.log {
        let result = crate::reduce::reduce(&state, action);
        if let Some(error) = result.error {
            errors.push(FoldError {
                nonce: action.nonce.clone(),
                error,
            });
            continue;
        }
        state = result.state;
    }

    ReplayResult { state, errors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys_and_writes_json_stringify_text() {
        let value = json!({ "b": [1, "x\n", null, true], "a": { "z": 0, "y": "×" } });
        assert_eq!(canonical(&value), r#"{"a":{"y":"×","z":0},"b":[1,"x\n",null,true]}"#);
        assert_eq!(canonical(&json!(2.0)), "2");
        assert_eq!(canonical(&json!("\u{1}")), r#""\u0001""#);
    }

    #[test]
    fn fnv_runs_over_utf16_code_units() {
        // FNV-1a 32 of the empty string is its offset basis.
        assert_eq!(fnv1a32_utf16(""), "811c9dc5");
        // "a" = 0x61: (0x811c9dc5 ^ 0x61) * 0x01000193 mod 2^32.
        assert_eq!(fnv1a32_utf16("a"), "e40c292c");
        // One UTF-16 unit for "×" (U+00D7), not its two UTF-8 bytes.
        let by_unit = {
            let mut hash: u32 = 0x811c_9dc5;
            hash ^= 0xd7;
            hash = hash.wrapping_mul(0x0100_0193);
            format!("{hash:08x}")
        };
        assert_eq!(fnv1a32_utf16("×"), by_unit);
    }
}
