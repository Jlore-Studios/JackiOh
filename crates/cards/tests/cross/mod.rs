//! The cross-card tests (SURFACE §1): one module per `packages/cards/test/<x>.test.ts` that is not
//! a per-card file (those are the `mod tests` of their card's script). Written once by part 1; the
//! files are parts 22–27's.

/// `scenario(...)` over the real cards: what importing the TS harness registered (`registerAll()`).
pub fn scenario(setup: serde_json::Value) -> jackioh_engine::testkit::Scenario {
    jackioh_cards::register_all();
    jackioh_engine::testkit::scenario(setup)
}

pub mod activate_listed;
pub mod after_resolution;
pub mod card_text;
pub mod catalog;
pub mod chinese;
pub mod combat_windows;
pub mod condition_active;
pub mod control_change;
pub mod control_change_carry;
pub mod costs_and_mana;
pub mod deaths_and_reborn;
pub mod echo_and_exile;
pub mod embiggen_choices;
pub mod flavour;
pub mod forced_attacks;
pub mod fuse_registry;
pub mod fused_hooks;
pub mod fused_nested_resume;
pub mod fused_target_checks;
pub mod game_over;
pub mod game_summary;
pub mod hand_returns;
pub mod hidden_information;
pub mod invariants;
pub mod lasting_effects;
pub mod my_pawn;
pub mod params;
pub mod paused_sequences;
pub mod play_choices;
pub mod plays_and_casts;
pub mod pools_and_randomness;
pub mod preview;
pub mod query;
pub mod radiant_standard;
pub mod re_entry;
pub mod references;
pub mod registry;
pub mod resolving_face;
pub mod self_generation;
pub mod setup_and_mulligan;
pub mod stacks_and_reborn;
pub mod tributes;
pub mod trigger_multipliers;
pub mod trigger_stays;
pub mod turn_clock_and_legality;
pub mod turn_stages;
pub mod vanilla_and_positions;
