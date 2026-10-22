//! The AI integration tests too slow to share the `ai` binary (#873): full `play_match` games
//! (`match_`), the shadow-ban sweeps (`shadow_ban`), the tutorial tier's games (`tutorial_tier`), the
//! lethal solver's walks over played games (`lethal`) and the Duos gate's games (`duos_gate`, #936
//! B32), at AI_GATE_BUDGET. CI runs this as `rust (test ai slow)`; locally `cargo test -p jackioh-ai
//! --test ai_slow`. The many-game sweeps among them are `#[ignore = "daily: …"]`, which super.yml's
//! daily tests run (`cargo test -p jackioh-ai --test ai_slow -- --ignored` here); it was two
//! binaries, `ai_slow_a` and `ai_slow_b`, until those went daily.
//!
//! `#[path]` because the modules live beside the rest in `tests/ai/`, whose `mod.rs` does not name
//! them, so that each runs in one binary only.

#[path = "ai/duos_gate.rs"]
mod duos_gate;
#[path = "ai/lethal.rs"]
mod lethal;
#[path = "ai/match_.rs"]
mod match_;
#[path = "ai/shadow_ban.rs"]
mod shadow_ban;
#[path = "ai/support.rs"]
mod support;
#[path = "ai/tutorial_tier.rs"]
mod tutorial_tier;
