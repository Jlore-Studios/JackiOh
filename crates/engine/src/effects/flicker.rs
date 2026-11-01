//! Flicker (docs/classic-sets.md B5 E22, R444): "the card leaves the field and re-enters the same zone
//! at once" — R78's reset, summoning sick, no Cry, no Death — and "it counts as summoned". `flicker_card`
//! takes a sink, not a context, so the "would die" window of §4.5 step 1 can flicker the dying units of
//! Classic #14 Shadowstep's Radiant face too.

use serde::{Deserialize, Serialize};

use crate::animated::{animate_on_entry, is_animated};
use crate::catalog::def_of;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope, instance_of};
use crate::script::{Effect, EngineSink};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, rename_in_board_history};
use crate::wire::{CardType, GameEvent, Row, Zone};
use crate::zones::{flicker_in_place, is_carried, lands_face_down};

/// R227: a card in the state takes the next number as a fresh id: `(former, fresh)`.
fn fresh_face_down_id_in_place(state: &mut GameState, instance_id: &str) -> Option<(String, String)> {
    let fresh = format!("c{}", state.next_id);
    let card = find_instance_mut(state, instance_id)?;
    let former = std::mem::replace(&mut card.id, fresh.clone());
    state.next_id += 1;
    // R419: C+ #35's history names the card by the id it has now.
    rename_in_board_history(state, &former, &fresh);
    crate::generators::rename_generator(state, &former, &fresh);
    Some((former, fresh))
}

/// B5 E22: flicker one card acting on the field: it leaves (every effect aimed at it ends, R174) and
/// re-enters in place reset (R78), summoning sick (R83), in Attack Position. No Cry (R1), no Death; a
/// unit token comes back too (R444). A Trap re-enters face-down with a fresh id (§3.2, R33, R227); an
/// animated card or a Field Spell stays face-up. Emits `flickered`, then the `summoned` it counts as.
pub fn flicker_card(sink: &mut EngineSink<'_>, card: &CardInstance) -> bool {
    // The card as it stands now (TS held the live object).
    let card = crate::zones::live_or(sink.state, card);
    let Zone::Field { player, row, lane } = card.zone.clone() else {
        return false;
    };
    let was_animated = is_animated(sink.state, &card);
    let carried = is_carried(sink.state, &card);
    let old_id = card.id.clone();
    if !flicker_in_place(sink.state, &card) {
        return false;
    }
    // As `flicker_in_place` left it: reset, sick, in Attack Position.
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
        creator: None,
    });
    // R783010: a Crater Unit Locks the zone it re-entered.
    crate::crater::lock_on_arrival(sink, &card.id);
    // B3.1 rule 4: an Animated Field Spell animates as it enters the field, and it has just re-entered.
    if row == Row::Backrow && !carried {
        animate_on_entry(sink, &card);
    }
    true
}

/// `flicker`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
