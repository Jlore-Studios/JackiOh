//! The rng child: `count` draws from (seed, cursor), which prove a resumed cursor reproduces the
//! sequence anywhere (BUILD M1-T2). A pure test crate reads no environment (SURFACE §3), so the child
//! is a function of its three arguments that returns its stdout.

use jackioh_engine::testkit::*;

/// The child's stdout: `count` draws from `create_rng(seed, cursor)`, then the cursor it stands at.
pub fn rng_child(args: &[&str]) -> String {
    let seed = args.first().copied().unwrap_or("");
    let cursor_arg = args.get(1).copied().unwrap_or("0");
    let count_arg = args.get(2).copied().unwrap_or("1");
    let mut rng = create_rng(seed, number_of(cursor_arg) as u32);
    let draws: Vec<f64> = (0..number_of(count_arg) as usize).map(|_| rng.next()).collect();
    // Written by hand for the key order `draws`, `cursor`: serde_json's map would sort it.
    format!(
        "{{\"draws\":{},\"cursor\":{}}}",
        serde_json::to_string(&draws).expect("draws serialize"),
        rng.cursor()
    )
}

/// A child's argument as a whole number, or 0 for anything else.
fn number_of(text: &str) -> f64 {
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value >= 0.0 => value.trunc(),
        _ => 0.0,
    }
}
