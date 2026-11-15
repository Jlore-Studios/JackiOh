//! Where a Buff or a Nerf is live (SPEC §6.3, R3888000): the one table every row of the menu reads
//! (`tune.rs`, `params::steppable_params_in`), so each change is drawn only where it can still change
//! what the card does in the game, from the zone it is in or one it reaches in the normal course: a
//! deck card reaches a hand, and a field card a graveyard, unless it is a unit token. A bounce, a
//! resurrect or a "Trigger a Cry" is not the normal course, as R1748000 reasoned about the cost.
//!
//! ```text
//!   | TuneActs      | Deck | Hand | Played | Field | FaceDown | Dormant | Graveyard | Exile
//!   | Price         |  ✓   |  ✓   |   –    |   –   |    –     |    –    |     –     |   –
//!   | InHand        |  ✓   |  ✓   |   –    |   –   |    –     |    –    |     –     |   –
//!   | HandOrDeck    |  ✓   |  ✓   |   –    |   –   |    –     |    –    |     –     |   –
//!   | OnDraw        |  ✓   |  –   |   ✓¹   |   –   |    –     |    –    |     –     |   –
//!   | AtResolve     |  ✓   |  ✓   |   ✓    |   –   |    –     |    –    |    ✓²     |   –
//!   | AtPlay        |  ✓   |  ✓   |   ✓    |   –   |    ✓     |    –    |    ✓²     |   –
//!   | EntryTurn     |  ✓   |  ✓   |   ✓    |  ✓³   |    ✓     |    ✓    |    ✓²     |   –
//!   | OnAwaken      |  ✓⁴  |  ✓⁴  |   ✓⁴   |   –   |    ✓⁴    |    ✓    |    ✓²⁴    |   –
//!   | OnEvolve      |  ✓⁵  |  ✓⁵  |   ✓⁵   |  ✓⁵   |    ✓⁵    |    ✓⁵   |    ✓⁵     |   –
//!   | Lasting       |  ✓   |  ✓   |   ✓    |   ✓   |    ✓     |    ✓    |    ✓²     |   –
//!   | FromGraveyard |  ✓⁶  |  ✓⁶  |   ✓⁶   |  ✓⁶   |    ✓⁶    |    ✓⁶   |    ✓      |   –
//!   | Plague        |  –   |  –   |   –    |   ✓   |    –     |    ✓    |     –     |   –
//! ```
//!
//! 1. Only while its draw casts it (`cast_on_draw_now::is_cast_on_draw`).
//! 2. Only while a play may take it from there (`graveyard_play::playable_from_graveyard`).
//! 3. Only while it is summoning sick (`combat::is_sick`), or has a Reborn of its own that would bring
//!    it back sick (R83).
//! 4. Only while it has never awoken (`awakened`, R3039000).
//! 5. Only while it has not evolved (`evolved`, R3536000); an Evolve reaches a graveyard card where it
//!    lies (`evolution::evolve_now`).
//! 6. Never on a unit token (`zones::is_unit_token`), which no graveyard keeps (R11).
//!
//! Each row keeps its own conditions besides (types, bounds, R65, Vanilla, Immutable). Pure reads.

use crate::config::{
    TUNE_ACTS_AT_PLAY, TUNE_ACTS_FROM_GRAVEYARD, TUNE_ACTS_IN_HAND, TUNE_ACTS_ON_ENTRY_TURN,
};
use crate::numbers::NumberedKey;
use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, KeywordKind, Param, ParamRead, ZoneName};

/// R3888000: the table's columns — where the card is, as a Buff or a Nerf reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuneZone {
    /// In a deck (a library).
    Deck,
    /// In a hand.
    Hand,
    /// Being played or cast: §10.5 step 3, where hidden Nerfs land (R1845001), its price paid and its
    /// Cry still to come, or resolving.
    Played,
    /// On the field, face-up and not Dormant.
    Field,
    /// On the field face-down: a set Trap, or a card set by its Cloaked (R783004), not yet revealed.
    FaceDown,
    /// On the field and Dormant (§6.1, R1839000).
    Dormant,
    /// In a graveyard.
    Graveyard,
    /// In an exile pile.
    Exile,
}

/// R3888000: the table's rows — when a change acts, which says in which zones it is live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuneActs {
    /// The cost row and a Tribute: paid as the card is played.
    Price,
    /// A keyword that acts only from a hand (`TUNE_ACTS_IN_HAND`).
    InHand,
    /// A number only read while the card is in a hand or a deck (`read: "handOrDeck"`).
    HandOrDeck,
    /// A number only a Unit's or a Field Spell's "Cast on draw:" line reads (`read: "castOnDraw"`).
    OnDraw,
    /// An X-cost card's X, read as the card resolves.
    AtResolve,
    /// A keyword that acts as the card is played or enters (`TUNE_ACTS_AT_PLAY`), Echo, and a number
    /// only the Cry or the Combo reads (or a Spell's Cast on draw, its resolution).
    AtPlay,
    /// Rush and Charge (`TUNE_ACTS_ON_ENTRY_TURN`): they act on the turn the Unit enters.
    EntryTurn,
    /// A number only "When awoken:" reads (`read: "awaken"`).
    OnAwaken,
    /// A number only the Evolve text reads (`read: "evolve"`).
    OnEvolve,
    /// The stats, every other keyword and numbered keyword, and a number the lasting text reads.
    Lasting,
    /// Flashback (`TUNE_ACTS_FROM_GRAVEYARD`) and a number only read in a graveyard (`read:
    /// "graveyard"`).
    FromGraveyard,
    /// The Plague row (R1748002): a card on the field that uses its own counters.
    Plague,
}

/// R3888000: the column the card is in now; `None` for a card that has ceased to exist.
pub fn tune_zone_of(state: &GameState, card: &CardInstance) -> Option<TuneZone> {
    Some(match card.zone.z() {
        ZoneName::Library => TuneZone::Deck,
        ZoneName::Hand => TuneZone::Hand,
        ZoneName::Resolving => TuneZone::Played,
        ZoneName::Field if crate::slumber::is_slumbering(card) => TuneZone::Dormant,
        ZoneName::Field if crate::preview::is_face_down(state, card) => TuneZone::FaceDown,
        ZoneName::Field => TuneZone::Field,
        ZoneName::Graveyard => TuneZone::Graveyard,
        ZoneName::Exile => TuneZone::Exile,
        ZoneName::Gone => return None,
    })
}

/// R3888000: when a keyword the keyword row adds or removes acts.
pub fn keyword_acts(kind: KeywordKind) -> TuneActs {
    if TUNE_ACTS_IN_HAND.contains(&kind) {
        TuneActs::InHand
    } else if TUNE_ACTS_AT_PLAY.contains(&kind) {
        TuneActs::AtPlay
    } else if TUNE_ACTS_ON_ENTRY_TURN.contains(&kind) {
        TuneActs::EntryTurn
    } else if TUNE_ACTS_FROM_GRAVEYARD.contains(&kind) {
        TuneActs::FromGraveyard
    } else {
        TuneActs::Lasting
    }
}

/// R3888000: when a numbered keyword on the X row acts: a Tribute is paid with the price (R2261000),
/// an Echo repeats the play, and the rest last.
pub fn numbered_acts(key: NumberedKey) -> TuneActs {
    match key {
        NumberedKey::Tribute => TuneActs::Price,
        NumberedKey::Echo => TuneActs::AtPlay,
        _ => TuneActs::Lasting,
    }
}

/// R3888000, R3888001: when a declared number is read, by its `read` mark on the face the card
/// shows. A Spell's "Cast on draw:" line is its resolution, run wherever it is cast, so it is read at
/// play; a Unit's runs only as its draw casts it (`cast_on_draw_now`).
pub fn param_acts(state: &GameState, card: &CardInstance, param: &Param) -> TuneActs {
    match param.read_on_face(card.radiant) {
        None => TuneActs::Lasting,
        Some(ParamRead::Cry | ParamRead::Combo) => TuneActs::AtPlay,
        Some(ParamRead::CastOnDraw) => {
            if crate::alt_play::priced_type_of(state, card) == CardType::Spell {
                TuneActs::AtPlay
            } else {
                TuneActs::OnDraw
            }
        }
        Some(ParamRead::Awaken) => TuneActs::OnAwaken,
        Some(ParamRead::Evolve) => TuneActs::OnEvolve,
        Some(ParamRead::HandOrDeck) => TuneActs::HandOrDeck,
        Some(ParamRead::Graveyard) => TuneActs::FromGraveyard,
    }
}

/// R3888000: whether a change that acts as `acts` says is live on `card` in `zone`: the table's cell
/// and its footnotes.
pub fn is_live(state: &GameState, card: &CardInstance, zone: TuneZone, acts: TuneActs) -> bool {
    use TuneZone::{Deck, Dormant, Exile, FaceDown, Field, Graveyard, Hand, Played};
    let from_graveyard = || crate::graveyard_play::playable_from_graveyard(state, card);
    match acts {
        TuneActs::Price | TuneActs::InHand | TuneActs::HandOrDeck => matches!(zone, Deck | Hand),
        TuneActs::OnDraw => match zone {
            Deck => true,
            Played => crate::cast_on_draw_now::is_cast_on_draw(state, card),
            _ => false,
        },
        TuneActs::AtResolve => match zone {
            Deck | Hand | Played => true,
            Graveyard => from_graveyard(),
            Field | FaceDown | Dormant | Exile => false,
        },
        TuneActs::AtPlay => match zone {
            Deck | Hand | Played | FaceDown => true,
            Graveyard => from_graveyard(),
            Field | Dormant | Exile => false,
        },
        TuneActs::EntryTurn => match zone {
            Deck | Hand | Played | FaceDown | Dormant => true,
            Field => {
                crate::combat::is_sick(state, card)
                    || crate::layers::unit_has(state, card, KeywordKind::Reborn)
            }
            Graveyard => from_graveyard(),
            Exile => false,
        },
        TuneActs::OnAwaken => {
            card.awakened != Some(true)
                && match zone {
                    Deck | Hand | Played | FaceDown | Dormant => true,
                    Graveyard => from_graveyard(),
                    Field | Exile => false,
                }
        }
        TuneActs::OnEvolve => card.evolved != Some(true) && zone != Exile,
        TuneActs::Lasting => match zone {
            Graveyard => from_graveyard(),
            Exile => false,
            _ => true,
        },
        TuneActs::FromGraveyard => zone != Exile && !crate::zones::is_unit_token(state, card),
        TuneActs::Plague => matches!(zone, Field | Dormant),
    }
}
