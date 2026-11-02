//! The subsystems a card script leans on (BUILD M3-T7): the machinery of a card or of a rule too
//! large for the effects library — Fuse (R77), the rotation rings (R14), the Zephyrs scorer
//! (§10.7), the random AI policy (R44), Heroic Power (R43), Combo-Index (R27), Call to Chaos (R28),
//! the lethal projection, Activate (B3.2, R384), last boards (R417), quests (B5 E33, R404), copied
//! text (B5 E14, R399), and the later sets' (R416, R419, R420, R422, R423, R425, R880, R1000, R1184000,
//! R1183002, R1847000).
//!
//! Laid out per SURFACE §1, each module re-exported whole, so `crate::subsystems::choose_action` is
//! `ai_policy::choose_action` too (SURFACE §4.2). `glitch` (R673–R679) is declared but not globbed: the
//! crate root re-exports the two functions the hosts read (`seat_played_by`, `seats_swapped`).

pub mod activate;
pub mod ai_policy;
pub mod audit;
// R1183002, R1183003: declared, not globbed: `reduce` and the `autopilot` verb name its functions by
// path (`subsystems::autopilot::choose`).
pub mod autopilot;
pub mod board_history;
pub mod call_to_chaos;
pub mod call_to_chaos_meditative;
pub mod call_to_chaos_plus;
pub mod call_to_chaos_scape;
pub mod call_to_chaos_stinky;
pub mod call_to_chaos_training_grounds;
pub mod combo_index;
pub mod copied_text;
pub mod craft;
pub mod feng_shui;
pub mod fuse;
pub mod glitch;
pub mod hero_power;
pub mod ky_competition;
pub mod ky_test;
pub mod last_boards;
pub mod lethal;
pub mod night_market;
pub mod papaya;
pub mod pareto;
pub mod perfect_hand;
pub mod quests;
pub mod rotation;
pub mod scorer;
pub mod starscape;
pub mod twice_forward;
pub mod unit_text;

pub use activate::*;
pub use ai_policy::*;
pub use audit::*;
pub use board_history::*;
pub use call_to_chaos::*;
pub use call_to_chaos_meditative::*;
pub use call_to_chaos_plus::*;
pub use call_to_chaos_scape::*;
pub use call_to_chaos_stinky::*;
pub use call_to_chaos_training_grounds::*;
pub use combo_index::*;
pub use copied_text::*;
pub use craft::*;
pub use fuse::*;
pub use hero_power::*;
pub use ky_competition::*;
pub use ky_test::*;
pub use last_boards::*;
pub use lethal::*;
pub use night_market::*;
pub use papaya::*;
pub use pareto::*;
pub use perfect_hand::*;
pub use quests::*;
pub use rotation::*;
pub use scorer::*;
pub use starscape::*;
pub use twice_forward::*;
pub use unit_text::*;

// The modules' numbers live in `crate::config` (CLAUDE.md rule 9, SURFACE §6.4), re-exported here for
// the cards (`subsystems::FUSE_MIN_INGREDIENTS`); a module here never redefines them.
pub use crate::config::{
    AI_PLAYOUT_STEP_CAP, ARMOR_UP_ARMOR, BRAINSTORM_DISCOUNT, CHAOS_ADDED_CARDS, CHAOS_BACKROW_CARDS,
    CHAOS_COST_DISCOUNT, CHAOS_HEAL, CHAOS_MANA, CHAOS_RUSH_TOKENS, CHAOS_UNIT_COST, CHAOS_UNIT_COUNT,
    CRAFTED_CARD_COST, DIE_INSECT_DAMAGE, DIE_INSECT_LUCKY, FIRST_GRADE, FUSE_MIN_INGREDIENTS,
    GRADE_A_DAMAGE, GRADE_D_CARDS, GRADE_D_DISCOUNT, LIFE_TAP_DAMAGE, LIFE_TAP_DRAW, PING_DAMAGE, PLUCK_COST,
    SCORER_DRY_RUN_PLAYS, SCORER_LOW_HEALTH, SCORER_WEIGHTS, STEADY_SHOT_RAISE, STITCHING_INGREDIENTS,
    STITCHING_MAX_COST, TANK_UP_ARMOR,
};
