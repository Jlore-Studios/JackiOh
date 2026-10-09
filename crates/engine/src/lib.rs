//! `jackioh_engine`: the JackiOh rules (SPEC.md), the wire types every layer shares and the deck
//! validator, pure and seeded (CLAUDE.md rule 4, SURFACE §3). Port of `packages/engine/src/index.ts`,
//! `packages/shared/src/index.ts` (as `wire`) and `packages/validator/src/index.ts` (as `validator`).
//!
//! Module tree (SURFACE §1, §6), written once by part 1: one module per
//! `packages/engine/src/<camelName>.ts`, snake_cased (SURFACE §4.1). A Wave 1 part fills module files
//! and never edits this one; a module it wishes existed goes under `GAPS` in its notes (SURFACE §16).
//!
//! Re-exports (SURFACE §6.1, §8):
//!
//! - Every top-level module is re-exported whole (`pub use reduce::*;` …), the ones
//!   `packages/engine/src/index.ts` re-exported and the ones it did not, so `crate::<name>` (and the
//!   testkit's `pub use crate::*`) names every engine item, as SURFACE §8 needs. TS kept every name
//!   it exported from these files distinct; Rust keeps that contract, and a name two modules both
//!   define is only an error where someone names it through the root.
//! - `wire` (← `@jackioh/shared`) is re-exported whole too, so `jackioh_engine::GameEvent` works.
//! - `effects` and `subsystems` stay modules (TS `export * as effects`, `export * as subsystems`).
//! - `validator` stays a module (TS `@jackioh/validator` was its own package, and its
//!   `validateDeck` is not `state.ts`'s): `jackioh_engine::validator::validate_deck`.
//!
//! Three names are defined twice in TS and resolved here explicitly, in `@jackioh/engine`'s favour,
//! because that is what a TS import from `@jackioh/engine` meant: `UnitView` is `layers`' (§10.4's
//! layered stats; the view's is `wire::UnitView`), `HeroView` is `query`'s (`heroOf`'s answer; the
//! view's is `wire::HeroView`), and `PlayAction` is `play_choices`' (`play_steps` has its own).

// The wire types (← packages/shared/src).
pub mod wire;

// The engine (← packages/engine/src), alphabetical.
pub mod alt_play;
pub mod animated;
pub mod announce;
pub mod book_swap;
pub mod brittle;
pub mod brittle_count;
pub mod carriers;
pub mod cast_on_draw_now;
pub mod catalog;
pub mod combat;
pub mod condition;
pub mod config;
pub mod cost_rules;
pub mod counter_warning;
pub mod cry_trigger;
pub mod damage;
pub mod draw;
pub mod draw_complete;
pub mod echo;
pub mod effects;
pub mod enchantments;
pub mod faces;
pub mod game_over;
pub mod game_summary;
pub mod graveyard_play;
pub mod instance_view;
pub mod kill_credit;
pub mod layers;
pub mod mana;
pub mod marks;
pub mod modifiers;
pub mod numbers;
pub mod own_library;
pub mod ownership;
pub mod params;
pub mod plague;
pub mod play_choices;
pub mod play_counts;
pub mod play_steps;
pub mod preview;
pub mod prompts;
pub mod query;
pub mod random_cast;
pub mod reduce;
pub mod replacements;
pub mod replay;
pub mod resolve;
pub mod restrictions;
pub mod rng;
pub mod script;
pub mod scripts;
pub mod setup;
pub mod state;
pub mod state_check;
pub mod stays;
pub mod subsystems;
pub mod targeting;
pub mod targeting_point;
pub mod temporary;
pub mod times_played;
pub mod traps;
pub mod triggers;
pub mod tuning;
pub mod turn;
pub mod view_for;
pub mod work;
pub mod zones;

// The deck validator (← packages/validator/src/index.ts).
pub mod validator;

// What a card script may name (SURFACE §7.1).
pub mod prelude;

// `scenario()` and the invariant monitor, for tests and the fuzz CLI (SURFACE §8).
#[cfg(feature = "testkit")]
pub mod testkit;

pub use wire::*;

pub use animated::*;
pub use announce::*;
pub use book_swap::*;
pub use brittle::*;
pub use brittle_count::*;
pub use carriers::*;
pub use cast_on_draw_now::*;
pub use catalog::*;
pub use combat::*;
pub use condition::*;
pub use config::*;
pub use cost_rules::*;
pub use counter_warning::*;
pub use cry_trigger::*;
pub use damage::*;
pub use draw::*;
pub use draw_complete::*;
pub use echo::*;
pub use enchantments::*;
pub use faces::*;
pub use game_over::*;
pub use game_summary::*;
pub use graveyard_play::*;
pub use instance_view::*;
pub use kill_credit::*;
pub use layers::*;
pub use mana::*;
pub use marks::*;
pub use modifiers::*;
pub use numbers::*;
pub use own_library::*;
pub use ownership::*;
pub use params::*;
pub use plague::*;
pub use play_choices::*;
pub use play_counts::*;
pub use play_steps::*;
pub use preview::*;
pub use prompts::*;
pub use query::*;
pub use random_cast::*;
pub use reduce::*;
pub use replacements::*;
pub use replay::*;
pub use resolve::*;
pub use restrictions::*;
pub use rng::*;
pub use script::*;
pub use scripts::*;
pub use setup::*;
pub use state::*;
pub use state_check::*;
pub use stays::*;
pub use targeting::*;
pub use targeting_point::*;
pub use temporary::*;
pub use times_played::*;
pub use traps::*;
pub use triggers::*;
pub use tuning::*;
pub use turn::*;
pub use view_for::*;
pub use work::*;
pub use zones::*;

// Defined twice in TS; `@jackioh/engine`'s meaning wins at the root (see the header).
pub use layers::UnitView;
pub use play_choices::PlayAction;
pub use query::HeroView;

// B5 E30, R417: last boards, a setup input; the server reads `last_board_for` as a game ends.
pub use subsystems::last_boards::last_board_for;
// R677: which seat each account plays after a Glitch's swap; the server and practice read it.
pub use subsystems::glitch::{seat_played_by, seats_swapped};
