//! A card changing owner (docs/classic-sets.md B5 E2, E16): the second exception to "a card always
//! goes to its owner's piles" (§3.2), beside R73's library swap. A card taken off the stack, out of a
//! hand or out of a deck moves to the thief's hand and its owner becomes the thief, so its later piles,
//! hand cap and burns are the thief's (§2.4). Two ways in, one `change_owner`: `take_into_hand` (E2's
//! countered Spell, E16, Classic+ #12.3, and a Unit off the field, R1185003, M+ #24 Bob) and
//! `draw_from_library_of` (Classic #58), §2.4's draw for the thief once the card has left the library.
//!
//! Hidden information (R466): `stolen` names the card to whoever could read it where it was taken from
//! and where it is now (`view_for::redact_event`), so the victim of a deck steal never learns which
//! card left; and since the victim's own library list (R310) would give it away, the rest of that
//! library turns unknown to its owner (R312's `hide_from_owner`): less than they could piece together,
//! never more. Every write lands on the instance of the card's id in the state.

use serde::{Deserialize, Serialize};

use crate::draw::DrawOutcome;
use crate::own_library::hide_from_owner;
use crate::preview::is_face_down;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, PlayerId, Zone, ZoneName};
use crate::zones::live_or;

/// The zone a `stolen` event names: every zone name but "gone".
type StolenFrom = ZoneName;

fn stolen_from(instance: &CardInstance) -> Option<StolenFrom> {
    let zone = instance.zone.z();
    if zone == ZoneName::Gone { None } else { Some(zone) }
}

/// E2, E16: `card` becomes `thief`'s, owner and controller, and `stolen` says so; the caller moves it.
/// R311's record of what the old owner was shown goes; R466: a card taken out of the other player's
/// library leaves the rest of it unknown to them. False, changing nothing, for a card that is gone.
pub fn change_owner(sink: &mut EngineSink<'_>, card: &CardInstance, thief: PlayerId) -> bool {
    let card = live_or(sink.state, card);
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
    // R1854002: a card taken from the other player is a steal for the thief's quests.
    if from != thief {
        crate::subsystems::quests::note_steal(sink, thief);
    }
    true
}

/// R466: the players who can read a card where it lies (§9.1, §10.8): a hand is its holder's, a library
/// nobody's, a face-down backrow card its controller's (R33), and a face-up card, a graveyard, an exile
/// pile and the resolving zone everyone's (a play is public, R98).
pub(crate) fn readers_where_it_lies(state: &GameState, card: &CardInstance) -> Vec<PlayerId> {
    match card.zone {
        Zone::Hand { player } => vec![player],
        Zone::Library { .. } => Vec::new(),
        Zone::Field { .. } if is_face_down(state, card) => vec![card.controller],
        _ => PLAYER_IDS.to_vec(),
    }
}

/// Where a card taken into a hand went: the hand, its new owner's graveyard from a full hand (§2.4, R4),
/// or nowhere, a unit token taken off the field having ceased to exist (R11, R1185003).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TakenTo {
    #[serde(rename = "hand")]
    Hand,
    #[serde(rename = "burned")]
    Burned,
    #[serde(rename = "vanished")]
    Vanished,
}

/// E2, E16: move `card` to `thief`'s hand as the thief's own card: `stolen` first, then the hand's
/// `addedToHand` or `burned` (§2.4). Where it went, or `None` for a card that has ceased to exist.
///
/// R1185003: a card on the field leaves it as a Bounce takes it (R747, the thief's hand for the
/// controller's): R78's reset, the pile beneath resuming (R13), the thief's hand cap. A unit token
/// ceases to exist instead (R11).
pub fn take_into_hand(sink: &mut EngineSink<'_>, card: &CardInstance, thief: PlayerId) -> Option<TakenTo> {
    if !change_owner(sink, card, thief) {
        return None;
    }
    let mut card = live_or(sink.state, card);
    if card.zone.z() == ZoneName::Field && crate::zones::is_unit_token(sink.state, &card) {
        crate::zones::move_to_zone(
            sink.state,
            &mut card,
            crate::zones::OffFieldZone::Hand,
            Default::default(),
        );
        return Some(TakenTo::Vanished);
    }
    // `draw::add_to_hand` burns a card entering a full hand (§2.4, R4), and its outcome names where
    // it went.
    Some(match crate::draw::add_to_hand(sink, &mut card) {
        crate::draw::AddToHandOutcome::Hand => TakenTo::Hand,
        crate::draw::AddToHandOutcome::Burned => TakenTo::Burned,
    })
}

/// Which end of a library a draw from it takes: `library[0]` is the top (`effects/library.rs`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum LibraryEnd {
    #[serde(rename = "top")]
    Top,
    /// TS's default (`end = "bottom"`).
    #[default]
    #[serde(rename = "bottom")]
    Bottom,
}

/// E16, E2: one draw of `drawer`'s from `from`'s library, its bottom card by default (Classic #58
/// Common Resources): the card becomes the drawer's (`change_owner`), then §2.4's draw finishes it
/// (`draw::complete_draw`: the draw counter, `drawn`, R58's cast on draw, R4's hand cap). An empty
/// library gives nothing and deals no fatigue to anybody: `None`.
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
