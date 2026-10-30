//! Animated (docs/classic-sets.md B3.1, R383): a Field Spell, Trap or Field Trap that steps into a unit
//! zone as a Unit, and the "Animated on your turn" cards that go back to their backrow zone at their
//! controller's cleanup. `turn.rs` runs `animate_at_turn_start` at the start of a turn (after the
//! Brittle tick, before the delayed effects) and `return_at_cleanup` after every end-of-turn step
//! (§2.2, R62).
//!
//! The keyword is read off the layers (§10.4), so a Vanilla card has lost it (R115). While animated the
//! card is a Unit for every rule but keeps its text: an animated Field Trap still fires as a trap, so
//! the trap machinery reads `face_type_of`, never `card_type_of`. Moving is not leaving the field (no
//! R78 reset, no departure, R174), but entering a unit zone is an entry: summoning sick, a fresh
//! exertion (R83, R171). R445: animating is not a summon (§6.3), so it emits `animated`, never
//! `summoned`, and nothing that answers a summon answers it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::def_of;
use crate::layers::{has_stat_line, unit_view};
use crate::preview::is_face_down;
use crate::script::EngineSink;
use crate::state::{
    CardInstance, Exertion, GameState, Position, active_first, find_instance, find_instance_mut, is_turn_of,
};
use crate::wire::{CardType, GameEvent, KeywordKind, PlayerId, Row, Zone, has_keyword};
use crate::zones::{
    ZoneSlot, acts_on_field, card_at, first_entry_zone, home_of, is_carried, is_open, live_or, release_home,
    reserve_home, slot_of, slots_of, stands_in_backrow, step_into_backrow, step_into_unit_zone,
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

/// R2280001: the mark of a backrow card owed its animation at the next state check (`owe_animation`,
/// `animate_owed`). Engine memory, as `zones::STACKED_KEY` is: R78's reset clears it as the card leaves
/// the field, and the card's own animation or the check that owes it clears it before.
pub const ANIMATE_OWED_KEY: &str = "__animateOwed";

/// `animate_card`'s options: the position it takes (Attack unless the text says otherwise).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnimateOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

/// B2.7: the type the card's running face is printed as, wherever it stands: what an animated card's
/// text *is* (a trap that fires, a Field Trap that stays), which the trap machinery reads (R383).
pub fn face_type_of(state: &GameState, card: &CardInstance) -> CardType {
    if let Some(set) = crate::alt_play::set_type_of(state, card) {
        return set;
    }
    let def = def_of(Some(state), &card.def_id);
    let face = if card.radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_)
}

/// B3.1 rule 1: the Animated the card has now, read off its layered keywords (§10.4), or `None`.
/// "Animated on your turn" is the narrower printing and wins when a card somehow has both.
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

/// R2280000: the Animated the card has when it also has stats (`layers::has_stat_line`), or `None`: a
/// Field Spell, Trap or Field Trap becomes a Unit only with both.
pub fn animated_with_stats(state: &GameState, card: &CardInstance) -> Option<AnimatedKind> {
    if !has_stat_line(state, card) {
        return None;
    }
    animated_kind_of(state, card)
}

/// R2280001: whether the card would animate at a state check now — face-up on top of its controller's
/// backrow zone and acting there, no Unit standing there already (R1728000), a Trap or Field Trap only
/// once it has fired (face-up; a revealed one is still armed), with stats and plain Animated. An
/// "Animated on your turn" card keeps its own moments (R1087).
fn ready_in_backrow(state: &GameState, card: &CardInstance) -> bool {
    let Some(at) = slot_of(state, card) else {
        return false;
    };
    if at.row != Row::Backrow
        || !acts_on_field(state, card)
        || is_carried(state, card)
        || stands_in_backrow(state, card)
        || is_face_down(state, card)
    {
        return false;
    }
    if matches!(face_type_of(state, card), CardType::Trap | CardType::FieldTrap) && card.face_up != Some(true)
    {
        return false;
    }
    animated_with_stats(state, card) == Some(AnimatedKind::Animated)
}

/// R2280001: `ready_in_backrow` for the card of this id as it stands.
pub(crate) fn ready_to_animate(state: &GameState, card_id: &str) -> bool {
    find_instance(state, card_id).is_some_and(|card| ready_in_backrow(state, card))
}

/// R2280001: mark the card owed its animation when it is ready now and was not (`was`, read before the
/// change): the first time it has both conditions. A card that had both already keeps its own moments.
pub(crate) fn owe_animation(state: &mut GameState, card_id: &str, was: bool) {
    if was || !ready_to_animate(state, card_id) {
        return;
    }
    if let Some(card) = find_instance_mut(state, card_id) {
        card.memory
            .insert(ANIMATE_OWED_KEY.to_string(), Value::Bool(true));
    }
}

/// R2280001: the state check's step that animates every backrow card owed its animation, active side
/// first, backrow lane 1 to 5, each as an entry (R383, R64). The mark goes first, so a card with no open
/// unit zone stays and is owed nothing more. True when a card moved.
pub(crate) fn animate_owed(sink: &mut FieldSink<'_>) -> bool {
    let mut moved = false;
    for player in active_first(sink.state) {
        for slot in slots_of(player, Row::Backrow) {
            let Some(id) = card_at(sink.state, slot)
                .filter(|card| card.memory.contains_key(ANIMATE_OWED_KEY))
                .map(|card| card.id.clone())
            else {
                continue;
            };
            let Some(owed) = find_instance_mut(sink.state, &id) else {
                continue;
            };
            owed.memory.shift_remove(ANIMATE_OWED_KEY);
            let owed = owed.clone();
            if ready_in_backrow(sink.state, &owed) && animate_card(sink, &owed, AnimateOptions::default()) {
                moved = true;
            }
        }
    }
    moved
}

/// B3.1 rule 3, R383: a card that is not a Unit by its face, standing in a unit zone as one.
pub fn is_animated(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { row: Row::Units, .. }) && face_type_of(state, card) != CardType::Unit
}

/// B3.1 rule 2: where an animating card goes — its lane's unit zone when open, else R64's leftmost.
pub(crate) fn unit_zone_for(state: &GameState, player: PlayerId, lane: i32) -> Option<ZoneSlot> {
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

/// B3.1 rules 2, 4 and 5 (R383): animate a card with stats (R2280000) acting in its controller's
/// backrow into a unit zone as a Unit, face-up, summoning sick, with a fresh exertion, in `position`;
/// an "Animated on your turn" card's backrow zone is held for its return (rule 6). Emits `animated`
/// (R445). True when it now stands in a unit zone ("does not move or change position", rule 4) or is a
/// Unit standing in its backrow zone (R1728000); false, changing nothing, when it cannot (rule 2).
pub fn animate_card(sink: &mut FieldSink<'_>, card: &CardInstance, options: AnimateOptions) -> bool {
    step_into_units(sink, card, options, true)
}

/// B3.1 rules 2, 4 and 5 shared by `animate_card` (an entry) and `animate_at_turn_start` (none, R1062).
fn step_into_units(
    sink: &mut FieldSink<'_>,
    card: &CardInstance,
    options: AnimateOptions,
    entry: bool,
) -> bool {
    let card = live_or(sink.state, card);
    // R1728000: a Unit standing in its backrow zone (a Flying Unit, R1081000) is a Unit already, as an
    // animated card is: it stays where it stands, in the position it has, and enters nothing.
    if (is_animated(sink.state, &card) || stands_in_backrow(sink.state, &card))
        && acts_on_field(sink.state, &card)
    {
        return true;
    }
    // R2280000: a card with no stats never becomes a Unit.
    if !has_stat_line(sink.state, &card) {
        return false;
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
    if entry {
        animated.summoned_turn = Some(turn);
    }
    animated.exertion = Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    };
    animated.face_up = Some(true);
    animated.memory.shift_remove(ANIMATE_OWED_KEY);
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

/// B3.1 rule 6: an animated "Animated on your turn" card goes back to a backrow zone of its side: its
/// home when still held for it on this side, Locked since or not (R688: only plays refuse a Locked
/// zone); else, having changed sides, its new controller's leftmost open backrow zone; else it stays a
/// Unit. A card dormant under a Stack does not return. Emits `deanimated`. True when it went back.
pub fn return_home(sink: &mut FieldSink<'_>, card: &CardInstance) -> bool {
    let card = live_or(sink.state, card);
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
/// Animated Field Spell, or an "Animated on your turn" card entering on its controller's turn, with
/// stats (R2280000). A face-down Trap never does (rule 8): it animates as its firing's last step.
/// Called right after the placing event by §10.5 step 4 (`play_steps::place_card`) and every summon.
pub fn animate_on_entry(sink: &mut FieldSink<'_>, card: &CardInstance) -> bool {
    let card = live_or(sink.state, card);
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

/// B3.1 rule 4: `player`'s "Animated on your turn" cards with stats (R2280000) step into their unit
/// zones, backrow lane 1 to 5 (R68's order), each as far as an open unit zone lets it. Only the top of
/// a backrow pile acts (B5 E21), and a face-down card stays hidden (rule 8). The step is no entry (R1062).
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
        step_into_units(sink, &card, AnimateOptions::default(), false);
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
