//! Flicker (docs/classic-sets.md B5 E22, R444): "the card leaves the field and re-enters the same zone
//! at once" — R78's reset, summoning sick, no Cry, no Death — and "it counts as summoned". The move is
//! `zones.flickerInPlace`; `flickerCard` is the whole of it with its events, for an effect list (the
//! `flicker` verb below) and for an engine sequence that holds a sink rather than a context: the "would
//! die" window of §4.5 step 1 flickers the dying units of Classic #14 Shadowstep's Radiant face instead.
//!
//! Port of `packages/engine/src/effects/flicker.ts`. TS's `FieldSink` (`{ state, events }`) is an
//! `EngineSink` here, which an `EffectContext` derefs to.

use serde::{Deserialize, Serialize};

use crate::animated::{animate_on_entry, is_animated};
use crate::catalog::def_of;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope, instance_of};
use crate::script::{Effect, EngineSink};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, rename_in_board_history};
use crate::wire::{CardType, GameEvent, Row, Zone};
use crate::zones::{flicker_in_place, is_carried, lands_face_down};

/// R227 (`zones.freshFaceDownId`, for a card that stands in the state): the card takes the next
/// number as a fresh id, and C+ #35's history follows it (R419). `(former, fresh)`, or `None` for a
/// card in no pile.
fn fresh_face_down_id_in_place(state: &mut GameState, instance_id: &str) -> Option<(String, String)> {
    let fresh = format!("c{}", state.next_id);
    let card = find_instance_mut(state, instance_id)?;
    let former = std::mem::replace(&mut card.id, fresh.clone());
    state.next_id += 1;
    // R419: C+ #35's history names the card by the id it has now.
    rename_in_board_history(state, &former, &fresh);
    Some((former, fresh))
}

/// B5 E22: flicker one card acting on the field. It leaves the field — every effect aimed at it ends
/// (R174), the triggers it queued there go, an animated card's home is let go — and re-enters its zone
/// in the same place at once: reset (R78), summoning sick (R83), in Attack Position, on the same side.
/// No Cry (R1: it was not played) and no Death (it went to no graveyard). A unit token comes back like
/// any card (R444). A Trap re-enters face-down with a fresh id (§3.2, R33, R227); an animated card is
/// still a Unit in its unit zone and stays face-up; a Field Spell is public. Emits `flickered`, then the
/// `summoned` it counts as. False, changing nothing, for a card not acting on the field.
pub fn flicker_card(sink: &mut EngineSink<'_>, card: &CardInstance) -> bool {
    // The card as it stands now (TS held the live object).
    let card = find_instance(sink.state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone());
    let Zone::Field { player, row, lane } = card.zone.clone() else {
        return false;
    };
    let was_animated = is_animated(sink.state, &card);
    let carried = is_carried(sink.state, &card);
    let old_id = card.id.clone();
    if !flicker_in_place(sink.state, &card) {
        return false;
    }
    // The card as `flickerInPlace` left it: reset, sick, in Attack Position.
    let mut card = find_instance(sink.state, &old_id).cloned().unwrap_or(card);

    let face_down = row == Row::Backrow && !carried && lands_face_down(sink.state, &card, Row::Backrow);
    let former_id = if face_down {
        match fresh_face_down_id_in_place(sink.state, &old_id) {
            Some((former, fresh)) => {
                card.id = fresh;
                Some(former)
            }
            None => None,
        }
    } else {
        None
    };
    if was_animated || def_of(Some(&*sink.state), &card.def_id).type_ == CardType::FieldSpell {
        if let Some(live) = find_instance_mut(sink.state, &card.id) {
            live.face_up = Some(true);
        }
        card.face_up = Some(true);
    }

    sink.events.push(GameEvent::Flickered {
        player,
        instance_id: old_id,
        def_id: card.def_id.clone(),
        row,
        lane,
    });
    sink.events.push(GameEvent::Summoned {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        row,
        lane,
        former_id,
        source_id: None,
        arrived_during: None,
        exits_from: None,
    });
    // B3.1 rule 4: an Animated Field Spell animates as it enters the field, and it has just re-entered.
    if row == Row::Backrow && !carried {
        animate_on_entry(sink, &card);
    }
    true
}

/// `flicker`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FlickerArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<BoardScope>,
}

/// B5 E22: flicker a card (`target`, the card running the script by default) or every card a board
/// scope names, read once before any of them moves, in R68's order (§3.1, §3.2).
pub fn flicker(args: FlickerArgs) -> Effect {
    Effect::new("flicker", move |ctx| {
        if let Some(scope) = &args.scope {
            for card in cards_in_scope(ctx, scope) {
                flicker_card(ctx, &card);
            }
            return;
        }
        let spec = args.target.clone().unwrap_or(TargetSpec::SelfCard);
        if let Some(card) = instance_of(ctx, &spec) {
            flicker_card(ctx, &card);
        }
    })
}
