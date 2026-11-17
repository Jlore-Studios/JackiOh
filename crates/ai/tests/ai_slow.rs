//! The AI integration tests too slow to share the `ai` binary (#873): full `play_match` games
//! (`match_`), the shadow-ban sweeps (`shadow_ban`), the tutorial tier's games (`tutorial_tier`), the
//! lethal solver's walks over played games (`lethal`) and the Duos gate's games (`duos_gate`, #936
//! B32), at AI_GATE_BUDGET. Locally `cargo test -p jackioh-ai --test ai_slow`.
//!
//! This binary is no longer CI's per-pull-request coverage of those games: its slow tests carry
//! `#[ignore = "daily: off per-PR CI by owner decision (#4093), no per-PR stand-in"]` and run in
//! super.yml's daily tests (`cargo test -p jackioh-ai --test ai_slow -- --ignored` here). The fast tests
//! of these modules still run per pull request as `rust (test ai slow)`. If that group is ever empty
//! or merged away, nothing else changes: the daily run keeps every ignored test.
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
