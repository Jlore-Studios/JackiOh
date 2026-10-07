//! Port of `packages/engine/test/fixtures/rng-child.ts`.
//!
//! Run by test/rng.test.ts in a separate process: prints `count` draws from (seed, cursor),
//! which proves a resumed cursor reproduces the sequence anywhere (BUILD M1-T2).
//!
//! The TS file was a child process reading `process.argv`; a pure test crate reads no environment
//! (SURFACE §3), so the child is this function: it takes the three arguments the child took, with the
//! child's defaults (`seed = ""`, `cursor = "0"`, `count = "1"`), and returns exactly what the child
//! wrote to stdout, `{"draws":[…],"cursor":n}`.

use jackioh_engine::testkit::*;

/// The child's stdout for `argv.slice(2)` = `args`: `count` draws from `create_rng(seed, cursor)`,
/// then the cursor the generator stands at.
pub fn rng_child(args: &[&str]) -> String {
    let seed = args.first().copied().unwrap_or("");
    let cursor_arg = args.get(1).copied().unwrap_or("0");
    let count_arg = args.get(2).copied().unwrap_or("1");
    let mut rng = create_rng(seed, number_of(cursor_arg) as u32);
    let draws: Vec<f64> = (0..number_of(count_arg) as usize).map(|_| rng.next()).collect();
    // `JSON.stringify({ draws, cursor })` keeps that key order; serde_json's map would sort it.
    format!(
        "{{\"draws\":{},\"cursor\":{}}}",
        serde_json::to_string(&draws).expect("draws serialise"),
        rng.cursor()
    )
}

/// JS `Number(text)` on the child's arguments: a whole number, or 0 for anything else (TS's `NaN`
/// would make no draw and a cursor of 0).
fn number_of(text: &str) -> f64 {
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value >= 0.0 => value.trunc(),
        _ => 0.0,
    }
}
