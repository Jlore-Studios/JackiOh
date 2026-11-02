//! What a card script may name (SURFACE §7.1): `use jackioh_engine::prelude::*;` at the top of every
//! `crates/cards/src/scripts/<set>/<file>.rs`. It re-exports the modules scripts read (research A
//! §5.3), the whole effects library, `subsystems` as a module, the wire types, and `json!`, `Value`,
//! `IndexMap`, `Arc` and `json_as`.
//!
//! Left out because their names collide with the effects' verbs, which a card means: `draw` (its
//! `draw`, `add_to_hand`), `turn` (`end_turn`) and `damage` (`lose_health`). `mana` is in for `cost_now`
//! and friends, so its `gain_mana` and `refresh_mana` are resolved to the effects' below. Anything else
//! is named by full path (`jackioh_engine::draw::draws_this_turn`). A script reads through these
//! helpers and writes through effects, never `GameState` fields (CLAUDE.md rule 5).

pub use crate::animated::*;
pub use crate::book_swap::*;
pub use crate::cast_on_draw_now::*;
// `catalog` but its `register_catalog`, which is `jackioh_cards::register_all`'s to call (by path) and
// whose test twin the testkit exports: a card's `mod tests` globs both the prelude and the testkit.
pub use crate::catalog::{
    CatalogQueryArgs, FUSED_DIGEST_MARK, GlitchOdds, RADIANT_INGREDIENT_MARK, TransientHolder,
    catalog_version, def_by_index, def_of, excluding_def_id, find_def, fused_id_parts, fused_id_specs,
    glitch_or_not, held_out, is_digest_id, pick_generated, query, query_cost, registered_catalog, roll_grape,
    roll_weighted, self_def_ids,
};
pub use crate::combat::*;
pub use crate::config::*;
pub use crate::cost_rules::*;
pub use crate::faces::*;
pub use crate::float::*;
pub use crate::grants::*;
pub use crate::graveyard_play::*;
pub use crate::kill_credit::*;
pub use crate::layers::*;
pub use crate::mana::*;
pub use crate::numbers::*;
pub use crate::params::*;
pub use crate::plague::*;
pub use crate::preview::{NOTHING_REMEMBERED, card_preview, count_preview};
pub use crate::prompts::*;
pub use crate::query::*;
pub use crate::replacements::*;
pub use crate::restrictions::*;
pub use crate::rng::*;
pub use crate::script::*;
pub use crate::secrets::*;
// R1839000–R1839002: Dormant's reads and its "When this awakens" (the `awaken` verb is the effects').
pub use crate::slumber::{is_slumbering, slumber_turns_of, when_this_awakens};
// R1852000–R1852007: the Starship's reads and its "When launched" (the launch price is the effects');
// R1859001, R1859003: a fused piece's numbers on its face, and whether a card is a piece now.
pub use crate::starship::{
    STARSHIP_LAUNCH_ABILITY, building_a_ship, dormant_starship_of, is_dormant_starship, is_launched_starship,
    is_starship_piece, launch_cost, launched_starship_of, piece_number, starship_pieces_of, when_launched,
};
pub use crate::state::*;
pub use crate::times_played::*;
// R1864002, R1864003: the trade cards' hand aura and the card a Trade put into the deck.
pub use crate::trade::{tradable_hand_aura, traded_card_of};
pub use crate::traps::*;
pub use crate::tuning::*;
pub use crate::win_rates::*;
pub use crate::zones::*;

pub use crate::effects::*;
pub use crate::subsystems;
pub use crate::wire::*;

// Resolved explicitly: the effects' verbs over `mana`'s engine functions of the same names, and
// `layers::UnitView` and `query::HeroView` over the view's (see `lib.rs`).
pub use crate::effects::{gain_mana, refresh_mana};
pub use crate::layers::UnitView;
pub use crate::query::HeroView;

pub use indexmap::IndexMap;
pub use serde_json::{Value, json};
pub use std::sync::Arc;

/// A JSON literal as a typed argument: `json_as::<SummonArgs>(json!({ "defId": "core-t-rush" }))`
/// (SURFACE §6.6, §7.1). The literal is written once, at the card's build, so a mismatch is a bug in
/// the card file: it panics naming the type and the JSON, which every test of that card shows.
pub fn json_as<T: serde::de::DeserializeOwned>(v: Value) -> T {
    match serde_json::from_value::<T>(v.clone()) {
        Ok(typed) => typed,
        Err(error) => panic!("json_as::<{}>({v}): {error}", std::any::type_name::<T>()),
    }
}
