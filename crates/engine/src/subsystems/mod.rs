//! The subsystems a card script leans on (BUILD M3-T7): the machinery of a card or of a rule too
//! large for the effects library — Fuse (R77), the rotation rings (R14), the Zephyrs scorer
//! (§10.7), the random AI policy (R44), Heroic Power (R43), Combo-Index (R27), Call to Chaos (R28),
//! the lethal projection (R44), Activate (B3.2, R384), last boards (R417), quests (B5 E33, R404),
//! copied text (B5 E14, R399), the Classic+ machinery (R416, R419, R420, R422, R423, R425) and the
//! Meditative set's (the craft, R880; the night market, R1000).
//!
//! One module per `packages/engine/src/subsystems/<x>.ts`, re-exported whole as TS's barrel
//! (`subsystems/index.ts`) did, so `subsystems.chooseAction` is both
//! `crate::subsystems::ai_policy::choose_action` and `crate::subsystems::choose_action` (SURFACE
//! §4.2). `glitch` (R673–R679) is declared but not globbed: TS's barrel left it out, and the crate
//! root re-exports the two functions the hosts read (`seat_played_by`, `seats_swapped`). Written
//! once by part 1 (SURFACE §1); the modules are part 8's.

pub mod activate;
pub mod ai_policy;
pub mod audit;
pub mod board_history;
pub mod call_to_chaos;
pub mod call_to_chaos_meditative;
pub mod call_to_chaos_plus;
pub mod combo_index;
pub mod copied_text;
pub mod craft;
pub mod feng_shui;
pub mod fuse;
pub mod glitch;
pub mod hero_power;
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
pub mod twice_forward;

pub use activate::*;
pub use ai_policy::*;
pub use audit::*;
pub use board_history::*;
pub use call_to_chaos::*;
pub use call_to_chaos_meditative::*;
pub use call_to_chaos_plus::*;
pub use combo_index::*;
pub use copied_text::*;
pub use craft::*;
pub use fuse::*;
pub use hero_power::*;
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
pub use twice_forward::*;

// The numbers these modules stated in TS live in `crate::config` now (CLAUDE.md rule 9, SURFACE
// §6.4); re-exported here so TS's `subsystems.X` path still names them (a card reads
// `subsystems::FUSE_MIN_INGREDIENTS`). A module of this directory uses them from `crate::config`
// and does not redefine them.
pub use crate::config::{
    AI_PLAYOUT_STEP_CAP, ARMOR_UP_ARMOR, BRAINSTORM_DISCOUNT, CHAOS_ADDED_CARDS, CHAOS_BACKROW_CARDS,
    CHAOS_COST_DISCOUNT, CHAOS_HEAL, CHAOS_MANA, CHAOS_RUSH_TOKENS, CHAOS_UNIT_COST, CHAOS_UNIT_COUNT,
    CRAFTED_CARD_COST, DIE_INSECT_DAMAGE, DIE_INSECT_LUCKY, FIRST_GRADE, FUSE_MIN_INGREDIENTS,
    GRADE_A_DAMAGE, GRADE_D_CARDS, GRADE_D_DISCOUNT, LIFE_TAP_DAMAGE, LIFE_TAP_DRAW, PING_DAMAGE, PLUCK_COST,
    SCORER_DRY_RUN_PLAYS, SCORER_LOW_HEALTH, SCORER_WEIGHTS, STEADY_SHOT_RAISE, STITCHING_INGREDIENTS,
    STITCHING_MAX_COST, TANK_UP_ARMOR,
};
