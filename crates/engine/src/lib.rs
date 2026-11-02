//! `jackioh_engine`: the JackiOh rules (SPEC.md), the wire types every layer shares and the deck
//! validator, pure and seeded (CLAUDE.md rule 4, SURFACE §3). Module tree: SURFACE §1, §4.1, §6, §16.
//!
//! Re-exports (SURFACE §6.1, §8): every top-level module and `wire` are re-exported whole, so
//! `crate::<name>` (and the testkit's `pub use crate::*`) names every engine item; a name two modules
//! both define is only an error where someone names it through the root. `effects`, `subsystems` and
//! `validator` stay modules (`jackioh_engine::validator::validate_deck`).
//!
//! Three names defined twice resolve explicitly at the root: `UnitView` is `layers`' (§10.4's layered
//! stats; the view's is `wire::UnitView`), `HeroView` is `query`'s (the view's is `wire::HeroView`),
//! and `PlayAction` is `play_choices`' (`play_steps` has its own).

pub mod wire;

// The engine, alphabetical.
pub mod alt_play;
pub mod animated;
pub mod announce;
pub mod attack_summon;
pub mod book_swap;
pub mod brittle;
pub mod brittle_count;
pub mod card_on_card;
pub mod carriers;
pub mod cast_on_draw_now;
pub mod catalog;
pub mod combat;
pub mod condition;
pub mod config;
pub mod cost_rules;
pub mod counter_warning;
pub mod crater;
pub mod credit;
pub mod cry_trigger;
pub mod custom;
pub mod damage;
pub mod draw;
pub mod draw_complete;
pub mod duos;
pub mod duos_life;
pub mod echo;
pub mod effects;
pub mod enchantments;
pub mod faces;
pub mod float;
pub mod freeze;
pub mod game_over;
pub mod game_summary;
pub mod generators;
pub mod grants;
pub mod graveyard_play;
pub mod instance_view;
pub mod kill_credit;
pub mod layers;
pub mod mana;
pub mod marketplace;
pub mod marks;
pub mod modifiers;
pub mod multipliers;
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
pub mod random_targets;
pub mod reduce;
pub mod replacements;
pub mod replay;
pub mod resolve;
pub mod resources;
pub mod restrictions;
pub mod rng;
pub mod sandbox;
pub mod script;
pub mod scripts;
pub mod secrets;
pub mod setup;
pub mod slumber;
pub mod starship;
pub mod state;
pub mod state_check;
pub mod stays;
pub mod subsystems;
pub mod targeted;
pub mod targeting;
pub mod targeting_point;
pub mod temporary;
pub mod times_played;
pub mod trade;
pub mod traps;
pub mod triggers;
pub mod tuning;
pub mod turn;
pub mod view_for;
pub mod win_conditions;
pub mod win_rates;
pub mod work;
pub mod zones;

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
pub use card_on_card::*;
pub use carriers::*;
pub use cast_on_draw_now::*;
pub use catalog::*;
pub use combat::*;
pub use condition::*;
pub use config::*;
pub use cost_rules::*;
pub use counter_warning::*;
pub use credit::*;
pub use cry_trigger::*;
pub use damage::*;
pub use draw::*;
pub use draw_complete::*;
pub use duos::*;
pub use duos_life::*;
pub use echo::*;
pub use enchantments::*;
pub use faces::*;
pub use freeze::*;
pub use game_over::*;
pub use game_summary::*;
pub use generators::*;
pub use grants::*;
pub use graveyard_play::*;
pub use instance_view::*;
pub use kill_credit::*;
pub use layers::*;
pub use mana::*;
pub use marketplace::*;
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
pub use resources::*;
pub use restrictions::*;
pub use rng::*;
pub use sandbox::*;
pub use script::*;
pub use scripts::*;
pub use secrets::*;
pub use setup::*;
pub use slumber::*;
pub use starship::*;
pub use state::*;
pub use state_check::*;
pub use stays::*;
pub use targeted::*;
pub use targeting::*;
pub use targeting_point::*;
pub use temporary::*;
pub use times_played::*;
pub use trade::*;
pub use traps::*;
pub use triggers::*;
pub use tuning::*;
pub use turn::*;
pub use view_for::*;
pub use win_conditions::*;
pub use win_rates::*;
pub use work::*;
pub use zones::*;

// Names two modules define: these win at the root (see the header).
pub use layers::UnitView;
pub use play_choices::PlayAction;
pub use query::HeroView;

// B5 E30, R417: last boards, a setup input; the server reads `last_board_for` as a game ends.
pub use subsystems::last_boards::last_board_for;
// R677: which seat each account plays after a Glitch's swap; the server and practice read it.
pub use subsystems::glitch::{seat_played_by, seats_swapped};
