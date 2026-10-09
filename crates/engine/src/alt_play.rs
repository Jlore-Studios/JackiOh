//! Face-down plays as Traps (ME-ALTPLAY, R1040–R1046).
//!
//! Knowledge Breaker (#45) and its Prime (#45.1) let their controller play Units face-down into the
//! backrow as Animated Field Traps that reveal at the start of the next turn (MD-C16, MD-C17, MD-C19);
//! Paranoia (#99) lets its controller play Spells face-down as Traps that reveal at a chosen timing
//! (MD-E17, MD-E18). The permission is a hook like `graveyard_play`, live while the granter acts on
//! its controller's field. A set card is a Field Trap (Unit) or Trap (otherwise) until it reveals,
//! hidden like any set Trap (R33, R1046), its text dormant. A set Unit flips, animates and runs its
//! Cry with prompts at its controller's next start of turn, keeping its set turn as `summonedTurn`
//! so it may attack (MD-C17); a set Spell reveals at its point and resolves its Spell text with
//! prompts and Echo repeats (MD-E17). The form ends when the card leaves the field (R78, R1045).

use crate::script::{CostAuraArgs, Effect, FaceDownPlayPermission, HookArgs, TriggerDef};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CardType, GameEvent, GameEventType, PlayerId, RevealAt, Row, Zone, ZoneChoice};

/// What a side grants now: whether Units and Spells may be set, and the Echo a Radiant Paranoia adds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceDownGrant {
    pub units: bool,
    pub spells: bool,
    pub echo: i32,
}

/// ME-ALTPLAY: every face-down permission the player has now — from each card acting on their side
/// of the field whose running face grants one, like `graveyard_play::graveyard_grants_of`.
pub fn grants_of(state: &GameState, player: PlayerId) -> FaceDownGrant {
    let mut out = FaceDownGrant::default();
    for row in [Row::Units, Row::Backrow] {
        for slot in crate::zones::slots_of(player, row) {
            let Some(source) = crate::zones::card_at(state, slot) else {
                continue;
            };
            if source.controller != player {
                continue;
            }
            let script = crate::scripts::script_of(state, source);
            let Some(hook) = script.face_down_play.as_ref() else {
                continue;
            };
            let args = HookArgs {
                state,
                self_: source,
                radiant: source.radiant,
            };
            let args = CostAuraArgs {
                state: args.state,
                self_: args.self_,
                radiant: args.radiant,
            };
            for permission in hook(args) {
                if permission.units == Some(true) {
                    out.units = true;
                }
                if permission.spells == Some(true) {
                    out.spells = true;
                }
                if let Some(echo) = permission.echo {
                    out.echo = out.echo.max(echo);
                }
            }
        }
    }
    out
}

/// What a face-down play sets: a Unit or a Spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetKind {
    Unit,
    Spell,
}

/// ME-ALTPLAY: what this card could be set as, or `None`.
pub(crate) fn face_down_kind(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    from_graveyard: bool,
) -> Option<SetKind> {
    let grants = grants_of(state, player);
    let printed = crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
    if printed == CardType::Unit && !from_graveyard && grants.units {
        return Some(SetKind::Unit);
    }
    if printed == CardType::Spell {
        // Field Spells are permanents already (D1): never set.
        if crate::faces::card_type_of(state, card) == CardType::Spell && grants.spells {
            return Some(SetKind::Spell);
        }
    }
    None
}

/// ME-ALTPLAY: the timings a kind may reveal at — a Unit at the start of the next turn only.
pub(crate) fn timings(kind: SetKind) -> Vec<RevealAt> {
    match kind {
        SetKind::Unit => vec![RevealAt::StartOfNextTurn],
        SetKind::Spell => RevealAt::ALL.to_vec(),
    }
}

/// ME-ALTPLAY: the open backrow zones a face-down play may name.
pub(crate) fn open_backrow_zones(state: &GameState, player: PlayerId) -> Vec<ZoneChoice> {
    crate::zones::slots_of(player, Row::Backrow)
        .into_iter()
        .filter(|slot| crate::zones::is_open(state, *slot))
        .map(|slot| ZoneChoice {
            row: slot.row,
            lane: slot.lane,
        })
        .collect()
}

/// ME-ALTPLAY: what a set card counts as — a Field Trap for a set printed Unit, a Trap for any other
/// set card, `None` without `set_as`. Reads the printed type, never `card_type_of` (no recursion).
pub fn set_type_of(state: &GameState, card: &CardInstance) -> Option<CardType> {
    card.set_as.as_ref()?;
    let printed = crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
    if printed == CardType::Unit {
        Some(CardType::FieldTrap)
    } else {
        Some(CardType::Trap)
    }
}

/// ME-ALTPLAY: what a play pays as — a set printed Unit pays as a Unit (MD-C16), anything else as
/// `card_type_of` reads it (a set Spell as a Trap, MD-E17).
pub fn priced_type_of(state: &GameState, card: &CardInstance) -> CardType {
    if card.set_as.is_some() {
        let printed = crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
        if printed == CardType::Unit {
            return CardType::Unit;
        }
    }
    crate::faces::card_type_of(state, card)
}

/// ME-ALTPLAY: whether the card's own text is dormant — set, not yet revealing, on the field.
pub fn is_dormant(card: &CardInstance) -> bool {
    card.set_as.is_some()
        && card.set_as.as_ref().and_then(|set| set.revealing) != Some(true)
        && matches!(card.zone, Zone::Field { .. })
}

/// ME-ALTPLAY: the triggers a set card fires on — one `@reveal` on turn started and ended.
pub(crate) fn reveal_triggers(card: &CardInstance) -> Vec<TriggerDef> {
    let Some(set) = card.set_as else {
        return Vec::new();
    };
    let reveal = set.reveal;
    let set_turn = set.set_turn;
    let trigger = TriggerDef::new(
        "@reveal",
        &[GameEventType::TurnStarted, GameEventType::TurnEnded],
        |_, _| vec![reveal_effect()],
    )
    .with_when(move |ctx, event| {
        let controller = ctx.controller;
        match reveal {
            RevealAt::EndOfThisTurn => match event {
                GameEvent::TurnEnded { turn, .. } => *turn >= set_turn,
                _ => false,
            },
            RevealAt::StartOfNextTurn => match event {
                GameEvent::TurnStarted { player, turn, .. } => *player == controller && *turn > set_turn,
                _ => false,
            },
            RevealAt::EndOfNextTurn => match event {
                GameEvent::TurnEnded { player, turn, .. } => *player == controller && *turn > set_turn,
                _ => false,
            },
        }
    });
    vec![trigger]
}

/// ME-ALTPLAY: the one effect a reveal runs — a Unit animates then cries, a Spell resolves.
fn reveal_effect() -> Effect {
    Effect::new("revealSet", move |ctx| {
        let Some(snapshot) = ctx.self_.clone() else {
            return;
        };
        let Some(card) = find_instance(ctx.state, &snapshot.id).cloned() else {
            return;
        };
        let Some(set) = card.set_as else {
            return;
        };
        let printed = crate::faces::card_type_of_face(ctx.state, &card.def_id, card.radiant);
        let controller = ctx.controller;
        if printed == CardType::Unit {
            let animated =
                crate::animated::animate_card(ctx, &card, crate::animated::AnimateOptions::default());
            if !animated {
                return;
            }
            if let Some(stored) = find_instance_mut(ctx.state, &snapshot.id) {
                stored.set_as = None;
                stored.summoned_turn = Some(set.set_turn);
            }
            // R1041: the Cry meets the card as it stands revealed — still stamped, it reads as a
            // Field Trap still and `cry_place_of` declines it, so the Cry never runs.
            let Some(live) = find_instance(ctx.state, &snapshot.id).cloned() else {
                return;
            };
            crate::cry_trigger::trigger_cry_of(ctx, &live, controller);
        } else {
            let echo = set.echo.unwrap_or(0);
            if let Some(stored) = find_instance_mut(ctx.state, &snapshot.id)
                && let Some(set) = stored.set_as.as_mut()
            {
                set.revealing = Some(true);
            }
            let Some(live) = find_instance(ctx.state, &snapshot.id).cloned() else {
                return;
            };
            let repeats = (crate::echo::printed_echo(&live, ctx.state) + echo).max(0) as u32;
            crate::cry_trigger::reveal_set_spell(ctx, &live, controller, repeats);
        }
    })
}

/// Silence the unused-permission warning: the hook type is built by card scripts.
#[allow(dead_code)]
fn _uses_permission(_: &FaceDownPlayPermission) {}
