//! The engine's rule tests from `s` on, in the `rules_3` binary (`tests/rules_3.rs`);
//! `mod.rs` has the ones from `a` to `o` and `mod_2.rs` the ones from `p` through `r` (#2957).
//! Both take `fixtures`, so a module reaches it as `crate::rules::fixtures` in either.
//! (`mod_2.rs` says why the rules tests are three binaries.)

// A fixture only another binary's modules use is dead code in this one.
#[allow(dead_code)]
pub mod fixtures;
pub mod sandbox;
pub mod scorer;
pub mod secrets;
pub mod self_tribute;
pub mod sets_not_shipped;
pub mod setup;
pub mod setup_aside;
pub mod shuffle_random;
pub mod sick_view;
pub mod slumber;
pub mod spell_power;
pub mod spell_verbs;
pub mod staged_quests;
pub mod starscape;
pub mod starship;
pub mod starship_base_piece;
pub mod start_of_opponent_turn;
pub mod state;
pub mod statecheck;
pub mod stays;
pub mod stinkyscape_verbs;
pub mod tallies;
pub mod targeting;
pub mod temporary;
pub mod terminus;
pub mod tg_spells;
pub mod tradable;
pub mod transform_riders;
pub mod transform_variants;
pub mod trap_cardresolved;
pub mod trap_window_pause;
pub mod tribute;
pub mod tribute_zones;
pub mod trigger_multipliers;
pub mod trigger_zones;
pub mod triggers;
pub mod turn;
pub mod turn_cap;
pub mod turn_wiring;
pub mod twice_forward;
pub mod uncovered;
pub mod unit_text;
pub mod untributable;
pub mod validator;
pub mod validator_drafts;
pub mod view_for;
pub mod view_hidden_ids;
pub mod view_marks;
pub mod weapon_kit;
pub mod when_attacks;
pub mod whenever_kills;
pub mod windfury;
pub mod zones;
