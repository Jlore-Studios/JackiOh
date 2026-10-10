//! The effects library (SPEC §10.9): every verb a card script composes, one module per
//! `packages/engine/src/effects/<x>.ts`, re-exported whole as the TS barrel (`effects/index.ts`)
//! did with `export *`. A card file reaches them as `jackioh_engine::effects::<verb>` or through
//! `jackioh_engine::prelude` (SURFACE §7.1). Written once by part 1 (SURFACE §1); the verbs are
//! parts 6 and 7's.
//!
//! Collisions: TS kept every exported name distinct across this directory, so the globs below are
//! unambiguous as long as the ports keep the TS names (SURFACE §4.2).

pub mod add_to_hand;
pub mod after_check;
pub mod animate;
pub mod brittle;
pub mod buff;
pub mod card_scope;
pub mod cast;
pub mod choose;
pub mod choose_where;
pub mod coins;
pub mod combat;
pub mod cost;
pub mod counters;
mod craft;
pub mod cry;
pub mod damage;
pub mod datacenter;
pub mod delay;
pub mod destroy;
pub mod draw;
pub mod draw_while;
pub mod each;
pub mod enchant;
pub mod flicker;
pub mod fruit;
pub mod fuse;
pub mod give;
pub mod grant_tag;
pub mod hand_exile;
pub mod heal;
pub mod health;
pub mod jade;
pub mod kill_credit;
pub mod last_board;
pub mod library;
pub mod library_copies;
pub mod locks;
pub mod lose_health;
pub mod mana;
pub mod memory;
pub mod move_;
pub mod perks;
pub mod plague;
pub mod player_mods;
pub mod position;
pub mod radiant;
pub mod random_picks;
pub mod reveal;
pub mod rotate;
pub mod rounds;
pub mod shuffle_card;
pub mod shuffle_into;
pub mod shuffle_random;
pub mod split;
pub mod statuses;
pub mod steal;
pub mod summon;
pub mod summon_this;
pub mod swap;
pub mod targets;
pub mod transform;
pub mod translate;
pub mod tune;
pub mod turn_end;
pub mod turn_hooks;
pub mod turns;
pub mod win;

pub use add_to_hand::*;
pub use after_check::*;
pub use animate::*;
pub use brittle::*;
pub use buff::*;
pub use card_scope::*;
pub use cast::*;
pub use choose::*;
pub use choose_where::*;
pub use coins::*;
pub use combat::*;
pub use cost::*;
pub use counters::*;
pub use craft::*;
pub use cry::*;
pub use damage::*;
pub use datacenter::*;
pub use delay::*;
pub use destroy::*;
pub use draw::*;
pub use draw_while::*;
pub use each::*;
pub use enchant::*;
pub use flicker::*;
pub use fruit::*;
pub use fuse::*;
pub use give::*;
pub use grant_tag::*;
pub use hand_exile::*;
pub use heal::*;
pub use health::*;
pub use jade::*;
pub use kill_credit::*;
pub use last_board::*;
pub use library::*;
pub use library_copies::*;
pub use locks::*;
pub use lose_health::*;
pub use mana::*;
pub use memory::*;
pub use move_::*;
pub use perks::*;
pub use plague::*;
pub use player_mods::*;
pub use position::*;
pub use radiant::*;
pub use random_picks::*;
pub use reveal::*;
pub use rotate::*;
pub use rounds::*;
pub use shuffle_card::*;
pub use shuffle_into::*;
pub use shuffle_random::*;
pub use split::*;
pub use statuses::*;
pub use steal::*;
pub use summon::*;
pub use summon_this::*;
pub use swap::*;
pub use targets::*;
pub use transform::*;
pub use translate::*;
pub use tune::*;
pub use turn_end::*;
pub use turn_hooks::*;
pub use turns::*;
pub use win::*;

// C+ #29's two numbers, which TS's `effects/lastBoard.ts` stated and this barrel exported; they live
// in `crate::config` now (CLAUDE.md rule 9, SURFACE §6.4).
pub use crate::config::{LAST_BOARD_CARD_COST, LAST_BOARD_DISCOVER_OPTIONS};

// Verbs TS's barrel re-exported from `subsystems/` (`effects/index.ts`, its last lines).
// B5 E34, R416: R29's scorer choosing a whole hand (C+ #27 Zephrys Zealotism).
pub use crate::subsystems::perfect_hand::replace_hand_with_perfect;
// B5 E29, R419: return the board, or one side of it, to a snapshot of the last few turns (C+ #35 Rollback).
pub use crate::subsystems::board_history::roll_back;
// The Glitch Easter egg (issue #170, R676): Glitch's one effect, its four outcomes behind one rng draw.
pub use crate::subsystems::glitch::glitch;
