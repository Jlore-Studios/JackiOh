//! Animated (docs/classic-sets.md B3.1, R383): a Field Spell, Trap or Field Trap that steps into a unit
//! zone as a Unit, and the "Animated on your turn" cards that go back to their backrow zone at their
//! controller's cleanup. `turn.rs` runs `animate_at_turn_start` as a stage of the start of a turn (after
//! the Brittle tick, before the delayed effects) and `return_at_cleanup` at cleanup, after every
//! end-of-turn step (§2.2, R62).
//!
//! What this module owns, rule by rule:
//!   1. printing — `animated_kind_of` reads the keyword off the card's layers (§10.4), so a Vanilla card
//!      has lost it where it stands (R115) and a granted one counts;
//!   2. to animate — `animate_card`: the same lane's unit zone when it is open, else the leftmost open,
//!      unlocked, unreserved one (R64's placement), in Attack Position unless the text says otherwise;
//!      with none open the card stays where it is;
//!   3. while animated it is a Unit for every rule — it stands in its controller's `units` pile, so
//!      every unit walk finds it, and `faces::card_type_of` answers "Unit" there — and it keeps its text:
//!      an animated Field Trap still fires as a trap (`traps::traps_in_order`), which is why the trap
//!      machinery reads `face_type_of` here and never `card_type_of`;
//!   4. when — the trap's own list ends in the `animate` verb (`effects/animate.rs`), a Field Spell as it
//!      enters (`animate_on_entry`), an "on your turn" card at its controller's start of turn and as it
//!      enters on their turn, and back at their cleanup (`return_at_cleanup`);
//!   5. moving is not leaving the field (`zones::step_into_unit_zone`, `zones::step_into_backrow`: no R78
//!      reset, no departure, R174), but entering the unit zone is entering it on that turn: summoning
//!      sick, a fresh exertion (R83, R171);
//!   6. the home zone — held while an "on your turn" card is animated (`zones::reserve_home`), with the
//!      rule's three exceptions: a Lock since stops the return, a new controller has no home on the
//!      old side (the card goes to its new controller's leftmost open backrow zone, or stays), and a
//!      card dormant under a Stack does not return;
//!   7. in the backrow it is not a Unit (it is not in a unit pile);
//!   8. a face-down Animated Trap is hidden like any trap until it fires: nothing here animates a
//!      face-down card except the trap's own firing, which has turned it face-up first (`traps::fire_trap`).
//!
//! R445: animating is not a summon — the card was on the field already (§6.3 Summon puts a card onto
//! the field from anywhere else) — so it emits `animated`, never `summoned`, and nothing that answers
//! a summon (Classic #5 Tesla) answers it.
//!
//! Port of `packages/engine/src/animated.ts`. TS moved the live card it was handed; here `card` is the
//! card as the caller holds it, read again from the state by its id, and every write lands on the
//! instance of that id in the state.

use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::layers::unit_view;
use crate::preview::is_face_down;
use crate::script::EngineSink;
use crate::state::{
    CardInstance, Exertion, GameState, Position, find_instance, find_instance_mut, is_turn_of,
};
use crate::wire::{CardType, GameEvent, KeywordKind, PlayerId, Row, Zone, has_keyword};
use crate::zones::{
    ZoneSlot, acts_on_field, card_at, first_entry_zone, home_of, is_carried, is_open, release_home,
    reserve_home, slot_of, slots_of, step_into_backrow, step_into_unit_zone,
};

/// What the moves here need: the state and the event list — an `EngineSink`, or an effect's context
/// (which derefs to its sink).
pub type FieldSink<'a> = EngineSink<'a>;

/// B3.1 rule 1: the two printings.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimatedKind {
    Animated,
    #[serde(rename = "Animated on your turn")]
    AnimatedOnYourTurn,
}

const ON_YOUR_TURN: KeywordKind = KeywordKind::AnimatedOnYourTurn;

/// `animate_card`'s options: the position it takes (Attack unless the text says otherwise).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnimateOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

/// The card as it stands in the state now, or as the caller holds it when the state has no card of
/// its id (TS read the live object either way).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone())
}

/// B2.7: the type the card's running face is printed as, wherever it stands. An animated card is a Unit
/// while it stands in a unit zone (`faces::card_type_of`, R383), but what its text *is* — a trap that
/// fires, a Field Trap that stays — is its face's, and the trap machinery reads this.
pub fn face_type_of(state: &GameState, card: &CardInstance) -> CardType {
    let def = def_of(Some(state), &card.def_id);
    let face = if card.radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_)
}

/// B3.1 rule 1: the Animated the card has now, read off its keywords as the layers compute them (§10.4:
/// printed unless Vanilla, granted, from an aura), or `None`. "Animated on your turn" is the narrower
/// printing and wins when a card somehow has both.
pub fn animated_kind_of(state: &GameState, card: &CardInstance) -> Option<AnimatedKind> {
    let keywords = unit_view(state, card).keywords;
    if has_keyword(&keywords, ON_YOUR_TURN) {
        return Some(AnimatedKind::AnimatedOnYourTurn);
    }
    if has_keyword(&keywords, KeywordKind::Animated) {
        return Some(AnimatedKind::Animated);
    }
    None
}

/// B3.1 rule 3, R383: a card that is not a Unit by its face, standing in a unit zone as one.
pub fn is_animated(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { row: Row::Units, .. }) && face_type_of(state, card) != CardType::Unit
}

/// B3.1 rule 2: where an animating card goes — its lane's unit zone when open, else R64's leftmost.
fn unit_zone_for(state: &GameState, player: PlayerId, lane: i32) -> Option<ZoneSlot> {
    let same = ZoneSlot {
        player,
        row: Row::Units,
        lane,
    };
    if is_open(state, same) {
        Some(same)
    } else {
        first_entry_zone(state, player, Row::Units)
    }
}

/// B3.1 rules 2, 4 and 5 (R383): animate a card acting in its controller's backrow — move it into a unit
/// zone as a Unit, face-up, summoning sick, with a fresh exertion, in `position` (Attack unless the text
/// says otherwise), without leaving the field. An "Animated on your turn" card's backrow zone is held for
/// its return (rule 6). Emits `animated`, never `summoned` (R445).
///
/// True when the card now stands in a unit zone — including a card that already did, which "does not
/// move or change position" (rule 4). False, changing nothing, when it is not acting in a backrow zone
/// (dormant under a pile, a carried Unit, off the field) or no unit zone of its side is open (rule 2:
/// it stays where it is).
pub fn animate_card(sink: &mut FieldSink<'_>, card: &CardInstance, options: AnimateOptions) -> bool {
    let card = live(sink.state, card);
    if is_animated(sink.state, &card) && acts_on_field(sink.state, &card) {
        return true;
    }
    let Some(from) = slot_of(sink.state, &card) else {
        return false;
    };
    if from.row != Row::Backrow || !acts_on_field(sink.state, &card) || is_carried(sink.state, &card) {
        return false;
    }
    let Some(to) = unit_zone_for(sink.state, from.player, from.lane) else {
        return false;
    };
    if !step_into_unit_zone(sink.state, &card, to) {
        return false;
    }

    let turn = sink.state.turn;
    let Some(animated) = find_instance_mut(sink.state, &card.id) else {
        return false;
    };
    animated.position = Some(options.position.unwrap_or(Position::Atk));
    animated.summoned_turn = Some(turn);
    animated.exertion = Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    };
    animated.face_up = Some(true);
    let animated = animated.clone();
    if animated_kind_of(sink.state, &animated) == Some(AnimatedKind::AnimatedOnYourTurn) {
        reserve_home(sink.state, from, &animated.id);
    }
    sink.events.push(GameEvent::Animated {
        player: from.player,
        instance_id: animated.id.clone(),
        def_id: animated.def_id.clone(),
        backrow_lane: from.lane,
        unit_lane: to.lane,
        carried: None,
    });
    true
}

/// B3.1 rule 6: an animated "Animated on your turn" card goes back to a backrow zone of its side — its
/// home when that is still held for it on this side, Locked since or not (R688: the return is a move,
/// and only plays refuse a Locked zone); its new controller's
/// leftmost open backrow zone when it has changed sides and so has no home here; nowhere, staying a
/// Unit, when neither takes it. A card dormant under a Stack does not return (it is not acting). Its
/// position goes with the unit zone. Emits `deanimated`. True when it went back.
pub fn return_home(sink: &mut FieldSink<'_>, card: &CardInstance) -> bool {
    let card = live(sink.state, card);
    let Some(from) = slot_of(sink.state, &card) else {
        return false;
    };
    if from.row != Row::Units || !acts_on_field(sink.state, &card) || !is_animated(sink.state, &card) {
        return false;
    }

    let home = home_of(sink.state, &card.id).cloned();
    let at_home = home.as_ref().is_some_and(|home| home.zone.player == from.player);
    let to: ZoneSlot = match &home {
        Some(home) if at_home => ZoneSlot {
            player: home.zone.player,
            row: home.zone.row,
            lane: home.zone.lane,
        },
        _ => {
            // A home on the other side belongs to the side the card left: that zone is free again.
            if home.is_some() {
                release_home(sink.state, &card.id);
            }
            match first_entry_zone(sink.state, from.player, Row::Backrow) {
                Some(to) => to,
                None => return false,
            }
        }
    };

    // Its own home is held for it alone; a card dormant there since it left (the pile it animated off,
    // B5 E21) is the one thing the zone can hold, and it goes back on top of that pile.
    release_home(sink.state, &card.id);
    if !step_into_backrow(sink.state, &card, to, at_home) {
        if let Some(home) = &home
            && at_home
        {
            let zone = ZoneSlot {
                player: home.zone.player,
                row: home.zone.row,
                lane: home.zone.lane,
            };
            reserve_home(sink.state, zone, &card.id);
        }
        return false;
    }
    if let Some(returned) = find_instance_mut(sink.state, &card.id) {
        returned.position = None;
    }
    sink.events.push(GameEvent::Deanimated {
        player: from.player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        unit_lane: from.lane,
        backrow_lane: to.lane,
    });
    true
}

/// B3.1 rule 4: a card that entered a backrow zone just now animates as it enters when it is an
/// Animated Field Spell, or an "Animated on your turn" card entering on its controller's turn. A
/// face-down Trap never does (rule 8): an Animated Trap or Field Trap animates as its firing's last step
/// instead. Called by the paths that put a card onto the field — §10.5 step 4 (`play_steps::place_card`,
/// plays and casts) and every summon (`effects/summon::summon_onto`) — right after the event that placed it.
pub fn animate_on_entry(sink: &mut FieldSink<'_>, card: &CardInstance) -> bool {
    let card = live(sink.state, card);
    let Some(at) = slot_of(sink.state, &card) else {
        return false;
    };
    if at.row != Row::Backrow || !acts_on_field(sink.state, &card) || is_carried(sink.state, &card) {
        return false;
    }
    if is_face_down(sink.state, &card) {
        return false;
    }
    match animated_kind_of(sink.state, &card) {
        Some(AnimatedKind::AnimatedOnYourTurn) => {
            is_turn_of(sink.state, card.controller) && animate_card(sink, &card, AnimateOptions::default())
        }
        Some(AnimatedKind::Animated) if face_type_of(sink.state, &card) == CardType::FieldSpell => {
            animate_card(sink, &card, AnimateOptions::default())
        }
        _ => false,
    }
}

/// B3.1 rule 4: `player`'s "Animated on your turn" cards step into their unit zones, backrow lane 1 to
/// 5 (R68's order), each as far as an open unit zone lets it. Only the top of a backrow pile acts (B5
/// E21), and a face-down card stays hidden (rule 8).
pub fn animate_at_turn_start(sink: &mut FieldSink<'_>, player: PlayerId) {
    for slot in slots_of(player, Row::Backrow) {
        let Some(card) = card_at(sink.state, slot).cloned() else {
            continue;
        };
        if is_face_down(sink.state, &card) {
            continue;
        }
        if animated_kind_of(sink.state, &card) != Some(AnimatedKind::AnimatedOnYourTurn) {
            continue;
        }
        animate_card(sink, &card, AnimateOptions::default());
    }
}

/// B3.1 rules 4 and 6: `player`'s animated "on your turn" cards go back to their home zones, unit lane
/// 1 to 5, after every end-of-turn step so their own end-of-turn text ran while they were Units. A card
/// that has lost the keyword since (a Vanilla) stays a Unit for good, and the zone it held is let go.
pub fn return_at_cleanup(sink: &mut FieldSink<'_>, player: PlayerId) {
    for slot in slots_of(player, Row::Units) {
        let Some(card) = card_at(sink.state, slot).cloned() else {
            continue;
        };
        if !is_animated(sink.state, &card) {
            continue;
        }
        if animated_kind_of(sink.state, &card) != Some(AnimatedKind::AnimatedOnYourTurn) {
            release_home(sink.state, &card.id);
            continue;
        }
        return_home(sink, &card);
    }
}
