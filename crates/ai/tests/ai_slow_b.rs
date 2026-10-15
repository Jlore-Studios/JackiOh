//! The AI integration tests too slow to share the `ai` binary (#873), second half: the tutorial
//! tier's games (`tutorial_tier`), the lethal solver's walks over played games (`lethal`) and the
//! Duos gate's games (`duos_gate`, #936 B32).
//! CI runs this as `rust (test ai slow b)`; locally `cargo test -p jackioh-ai --test ai_slow_b`.
//!
//! `#[path]` because the modules live beside the rest in `tests/ai/`, whose `mod.rs` does not name
//! them, so that each runs in one binary only.

#[path = "ai/duos_gate.rs"]
mod duos_gate;
#[path = "ai/lethal.rs"]
mod lethal;
#[path = "ai/support.rs"]
mod support;
#[path = "ai/tutorial_tier.rs"]
mod tutorial_tier;
