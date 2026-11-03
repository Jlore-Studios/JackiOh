//! The engine's rule tests from `p` through `r`, in the `rules_2` binary (`tests/rules_2.rs`);
//! `mod.rs` has the ones from `a` to `o` and `mod_3.rs` the ones from `s` on (#2957). All three
//! take `fixtures`, so a module reaches it as `crate::rules::fixtures` in any of them.

// A fixture only the other binary's modules use is dead code in this one.
#[allow(dead_code)]
pub mod fixtures;
pub mod papaya;
pub mod params;
pub mod pareto;
pub mod pauses;
pub mod perfect_hand;
pub mod play_choices;
pub mod play_choices_filters;
pub mod play_counts;
pub mod play_gate;
pub mod play_pipeline_b_replay;
pub mod play_reason;
pub mod play_step3;
pub mod play_watch;
pub mod pool_hold;
pub mod pools;
pub mod preview;
pub mod preview_ids;
pub mod prompt_kinds;
pub mod prompts;
pub mod query;
pub mod quests;
pub mod queued_when;
pub mod random_entries;
pub mod ranged_flying;
pub mod recruit_variants;
pub mod reduce;
pub mod replacements;
pub mod replay;
pub mod replay_scripted;
pub mod replay_steps;
pub mod resources;
pub mod restrictions;
pub mod rng;
pub mod rotation;
pub mod rounds;
pub mod rule_benders;
pub mod rulings_a;
pub mod rulings_b;
pub mod rulings_c;
pub mod rulings_config;
