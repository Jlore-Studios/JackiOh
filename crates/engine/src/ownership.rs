//! A card changing owner (docs/classic-sets.md B5 E2, E16): the second exception to "a card always
//! goes to its owner's piles" (§3.2), beside R73's library swap. A card taken off the stack as it is
//! cast, out of a hand or out of a deck moves to the thief's hand and its owner becomes the thief, so
//! every pile it reaches afterwards is the thief's. The hand cap applies to the thief's hand, and a
//! card it burns goes to the thief's graveyard, since the thief owns it by then (§2.4).
//!
//! Two ways in, one change of owner (`change_owner`):
//!   - `take_into_hand`: the card goes straight to the thief's hand (E2's countered Spell, E16's cards
//!     handed over, Classic+ #12.3's card out of the deck).
//!   - `draw_from_library_of`: a draw of the thief's own, taken from the other player's library (Classic
//!     #58: "draw the bottom card of your opponent's deck") — §2.4's draw from the moment the card has
//!     left the library, so the thief's hand cap, a cast on draw for the thief and the game draw counter
//!     all apply, and an empty library gives nothing and deals no fatigue to anybody.
//!
//! Hidden information (R466). `stolen` names the card to whoever could read it where it was taken
//! from and to whoever can read it where it is now (`view_for::redact_event`): a card out of a hand is its
//! holder's, a face-down card its controller's, a public one everyone's, and a card out of a library
//! nobody's, so the victim of a deck steal never learns which card left. The victim's own library list (R310) would give it away by what it stops listing, so a card
//! taken out of a library by the other player turns the rest of that library unknown to its owner
//! (R312's `hide_from_owner`): the list may show less than they could piece together, never more.
//!
//! Port of `packages/engine/src/ownership.ts`. TS wrote through the live card it was handed; here a
//! `card: &CardInstance` is the card as the caller holds it, and every write lands on the instance of
//! that id in the state.

use serde::{Deserialize, Serialize};

use crate::config::HAND_CAP;
use crate::draw::DrawOutcome;
use crate::own_library::hide_from_owner;
use crate::preview::is_face_down;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, PlayerId, Zone, ZoneName};

/// `Extract<GameEvent, { type: "stolen" }>["zone"]`: every zone name but "gone".
type StolenFrom = ZoneName;

fn stolen_from(instance: &CardInstance) -> Option<StolenFrom> {
    let zone = instance.zone.z();
    if zone == ZoneName::Gone { None } else { Some(zone) }
}

/// The card as it stands in the state now, or as the caller holds it when the state has no card of
/// its id (TS read the live object either way).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id).cloned().unwrap_or_else(|| card.clone())
}

/// E2, E16: `card` becomes `thief`'s — owner and controller — and `stolen` says so, naming the pile it
/// came from and both players. It does not move the card: the caller puts it in the hand, or draws it.
/// R311's record of what the old owner was shown of the card was theirs, so it goes; R466: a card taken
/// out of the other player's library leaves the rest of that library unknown to them. Returns false,
/// changing nothing, for a card that has ceased to exist.
pub fn change_owner(sink: &mut EngineSink<'_>, card: &CardInstance, thief: PlayerId) -> bool {
    let card = live(sink.state, card);
    let Some(zone) = stolen_from(&card) else {
        return false;
    };
    // R466: who could read the card where it lies, judged before anything about it changes.
    let readable_from = readers_where_it_lies(sink.state, &card);
    let from = card.owner;
    if let Some(taken) = find_instance_mut(sink.state, &card.id) {
        taken.owner = thief;
        taken.controller = thief;
        hide_from_owner(taken);
    }
    if zone == ZoneName::Library && from != thief {
        for left in sink.state.players[from].library.iter_mut() {
            hide_from_owner(left);
        }
    }
    sink.events.push(GameEvent::Stolen {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        from,
        to: thief,
        zone,
        readable_from: Some(readable_from),
    });
    true
}

/// R466: the players who can read a card where it lies (§9.1, §10.8): a hand is its holder's, a library
/// nobody's, a face-down backrow card its controller's (R33), and a face-up card, a graveyard, an exile
/// pile and the resolving zone everyone's (a play is public, R98).
fn readers_where_it_lies(state: &GameState, card: &CardInstance) -> Vec<PlayerId> {
    match card.zone {
        Zone::Hand { player } => vec![player],
        Zone::Library { .. } => Vec::new(),
        Zone::Field { .. } if is_face_down(state, card) => vec![card.controller],
        _ => PLAYER_IDS.to_vec(),
    }
}

/// `"hand" | "burned"`: where a card taken into a hand went — the hand, or its new owner's graveyard
/// because the hand was full (§2.4, R4).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TakenTo {
    #[serde(rename = "hand")]
    Hand,
    #[serde(rename = "burned")]
    Burned,
}

/// E2, E16: move `card` to `thief`'s hand as the thief's own card. `stolen` is emitted first, naming
/// the pile it came from and both players, then the hand's own `addedToHand` or `burned` (§2.4).
/// A card that has ceased to exist is not taken. Returns where it went, or `None` when it was not taken.
pub fn take_into_hand(sink: &mut EngineSink<'_>, card: &CardInstance, thief: PlayerId) -> Option<TakenTo> {
    if !change_owner(sink, card, thief) {
        return None;
    }
    let card = live(sink.state, card);
    // `draw::add_to_hand` burns a card entering a full hand (§2.4, R4): the same test, read before
    // the move, names where it went.
    let burns = sink.state.players[card.owner].hand.len() as i32 >= HAND_CAP;
    crate::draw::add_to_hand(sink, &card);
    Some(if burns { TakenTo::Burned } else { TakenTo::Hand })
}

/// Which end of a library a draw from it takes: `library[0]` is the top (`effects/library.rs`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum LibraryEnd {
    #[serde(rename = "top")]
    Top,
    /// TS's default (`end = "bottom"`).
    #[default]
    #[serde(rename = "bottom")]
    Bottom,
}

/// E16, E2: one draw of `drawer`'s, taken from `from`'s library — its bottom card by default (Classic
/// #58 Common Resources). The card leaves that library and becomes the drawer's (`change_owner`), then
/// §2.4's draw finishes it as the drawer's own (`draw::complete_draw`): the game draw counter, `drawn`,
/// a cast on draw for the drawer (R58) and the drawer's hand cap (R4). An empty library gives nothing
/// and deals no fatigue to anybody: returns `None`.
pub fn draw_from_library_of(
    sink: &mut EngineSink<'_>,
    drawer: PlayerId,
    from: PlayerId,
    end: LibraryEnd,
) -> Option<DrawOutcome> {
    let library = &sink.state.players[from].library;
    if library.is_empty() {
        return None;
    }
    // B5 E3, R457: it is the drawer's draw, so the drawer's draw limit stops it before any card moves.
    if crate::draw::draw_blocked(sink, drawer) {
        return Some(DrawOutcome::Limited);
    }
    let library = &sink.state.players[from].library;
    let at = match end {
        LibraryEnd::Top => 0,
        LibraryEnd::Bottom => library.len() - 1,
    };
    let card = library.get(at)?.clone();
    // B5 E3: it is the drawer's draw, so the drawer's draw limit stops it before the card moves (§2.4).
    if crate::draw::draw_blocked(sink, drawer) {
        return Some(DrawOutcome::Limited);
    }
    // The owner changes while the card still lies where it was taken from, so `stolen` names that pile;
    // then it leaves the library, as `draw::draw_one` takes its card, before §2.4 finishes the draw.
    change_owner(sink, &card, drawer);
    let library = &mut sink.state.players[from].library;
    let index = library.iter().position(|held| held.id == card.id)?;
    let taken = library.remove(index);
    Some(crate::draw::complete_draw(sink, drawer, taken, None))
}
