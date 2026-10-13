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
//!
//! Cloaked (#783, R783004–R783008) is the keyword form of the same play: a Unit, Spell or Field
//! Spell that has Cloaked in its controller's hand (printed, given, or lent by an aura that reaches
//! the cards in a hand, §10.4 layer 5) may be set face-down at its own price (R783005), and has no
//! timing: it reveals only when its controller activates its Reveal (`CLOAKED_REVEAL`,
//! `subsystems::activate`), which is offered only while the card can come out whole
//! (`reveal_can_complete`). The reveal is the same firing as a timed one (`fire_cloaked_reveal`).

use crate::script::{CostAuraArgs, Effect, EngineSink, FaceDownPlayPermission, HookArgs, TriggerDef};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{
    CardType, GameEvent, GameEventType, KeywordKind, PlayerId, RevealAt, Row, Zone, ZoneChoice,
};

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

/// What a face-down play sets: a Unit, a Spell, or (Cloaked, R783004) a Field Spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetKind {
    Unit,
    Spell,
    FieldSpell,
}

/// R783006: the id of the Reveal ability every Cloaked set card has (`subsystems::activate`).
pub const CLOAKED_REVEAL: &str = "reveal";

/// R783004: whether the card has Cloaked where it is now, read through the layers (§10.4): printed
/// or given, or lent by an aura that reaches the cards in its controller's hand (R783007).
pub fn is_cloaked(state: &GameState, card: &CardInstance) -> bool {
    crate::layers::unit_has(state, card, KeywordKind::Cloaked)
}

/// R783006: a card set face-down by its Cloaked that has not begun to reveal.
pub fn is_cloaked_set(card: &CardInstance) -> bool {
    card.set_as
        .is_some_and(|set| set.reveal == RevealAt::Cloaked && set.revealing != Some(true))
}

/// R783009: every catalog definition that prints Cloaked on a face that is not a Trap or a Field
/// Trap, in catalog order (`catalog::query`) — what a hidden backrow card may be, set by its Cloaked,
/// whatever the board shows.
pub fn printed_cloaked_def_ids() -> Vec<String> {
    crate::catalog::query(&crate::catalog::CatalogQueryArgs::default())
        .into_iter()
        .filter(|def| {
            [&def.base, &def.radiant].into_iter().any(|face| {
                !matches!(
                    face.type_.unwrap_or(def.type_),
                    CardType::Trap | CardType::FieldTrap
                ) && crate::wire::has_keyword(&face.keywords, KeywordKind::Cloaked)
            })
        })
        .map(|def| def.id.clone())
        .collect()
}

/// R783007, R783009: every catalog definition an aura on the field lends Cloaked to in this player's
/// hand now, in catalog order — probed as a fresh card of it there, and only while some aura's entry
/// carries Cloaked at all, so a board with none costs one walk of its auras.
pub fn lent_cloaked_def_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    let lends =
        crate::layers::aura_sources(state).into_iter().any(|source| {
            !source.vanilla
                && crate::scripts::script_of(state, source)
                    .aura
                    .as_ref()
                    .is_some_and(|aura| {
                        aura(HookArgs {
                            state,
                            self_: source,
                            radiant: source.radiant,
                        })
                        .iter()
                        .any(|entry| {
                            entry.mod_.keywords.as_ref().is_some_and(|keywords| {
                                crate::wire::has_keyword(keywords, KeywordKind::Cloaked)
                            })
                        })
                    })
        });
    if !lends {
        return Vec::new();
    }
    let mut next_id = state.next_id;
    crate::catalog::query(&crate::catalog::CatalogQueryArgs::default())
        .into_iter()
        .filter(|def| {
            let probe = crate::state::new_instance(&mut next_id, &def.id, player, Zone::Hand { player });
            !matches!(
                crate::faces::card_type_of_face(state, &def.id, false),
                CardType::Trap | CardType::FieldTrap
            ) && is_cloaked(state, &probe)
        })
        .map(|def| def.id.clone())
        .collect()
}

/// ME-ALTPLAY: what a permission lets this card be set as, or `None`.
fn permission_kind(
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

/// ME-ALTPLAY: the timings a permission's kind may reveal at — a Unit at the start of the next turn only.
fn timings(kind: SetKind) -> Vec<RevealAt> {
    match kind {
        SetKind::Unit => vec![RevealAt::StartOfNextTurn],
        SetKind::Spell => RevealAt::ALL.to_vec(),
        SetKind::FieldSpell => Vec::new(),
    }
}

/// ME-ALTPLAY, R1040, R1044, R783004: every way this card may be set face-down now, as a kind and
/// a timing — a permission's timings first, then Cloaked's own (`RevealAt::Cloaked`), which a Unit,
/// a Spell or a Field Spell in a hand has and a Trap or a Field Trap never does.
pub(crate) fn face_down_options(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    from_graveyard: bool,
) -> Vec<(SetKind, RevealAt)> {
    let mut out: Vec<(SetKind, RevealAt)> = Vec::new();
    if let Some(kind) = permission_kind(state, player, card, from_graveyard) {
        out.extend(timings(kind).into_iter().map(|reveal| (kind, reveal)));
    }
    if !from_graveyard && is_cloaked(state, card) {
        let kind = match crate::faces::card_type_of_face(state, &card.def_id, card.radiant) {
            CardType::Unit => Some(SetKind::Unit),
            CardType::Spell => Some(SetKind::Spell),
            CardType::FieldSpell => Some(SetKind::FieldSpell),
            CardType::Trap | CardType::FieldTrap => None,
        };
        if let Some(kind) = kind {
            out.push((kind, RevealAt::Cloaked));
        }
    }
    out
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

/// ME-ALTPLAY: what a set card counts as — a Field Trap for a set printed Unit or (R783004) Field
/// Spell, which stay on the field as they reveal, a Trap for any other set card, `None` without
/// `set_as`. Reads the printed type, never `card_type_of` (no recursion).
pub fn set_type_of(state: &GameState, card: &CardInstance) -> Option<CardType> {
    card.set_as.as_ref()?;
    let printed = crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
    if printed == CardType::Unit || printed == CardType::FieldSpell {
        Some(CardType::FieldTrap)
    } else {
        Some(CardType::Trap)
    }
}

/// ME-ALTPLAY: what a play pays as — a set printed Unit pays as a Unit (MD-C16), a card set by its
/// Cloaked as the card it is in hand (R783005), anything else as `card_type_of` reads it (a set
/// Spell as a Trap, MD-E17).
pub fn priced_type_of(state: &GameState, card: &CardInstance) -> CardType {
    if let Some(set) = card.set_as {
        let printed = crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
        if printed == CardType::Unit || set.reveal == RevealAt::Cloaked {
            return printed;
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

/// ME-ALTPLAY: the triggers a set card fires on — one `@reveal` on turn started and ended. A card
/// set by its Cloaked has no timing (R783006): its `@reveal` names no event, so no dispatch ever
/// offers it one, and only its Reveal fires it (`fire_cloaked_reveal`).
pub(crate) fn reveal_triggers(card: &CardInstance) -> Vec<TriggerDef> {
    let Some(set) = card.set_as else {
        return Vec::new();
    };
    let reveal = set.reveal;
    if reveal == RevealAt::Cloaked {
        return vec![TriggerDef::new("@reveal", &[], |_, _| vec![reveal_effect()])];
    }
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
            RevealAt::Cloaked => false,
        }
    });
    vec![trigger]
}

/// R783006: whether a card set by its Cloaked can come out whole now, which its Reveal waits for — a
/// Unit needs the unit zone it would animate into (its own lane's, else R64's leftmost open one), a
/// Spell what playing it needs (a legal pick for each declaration its play could not go without,
/// R703, read as its reveal reads it, `cry_trigger::reveal_set_spell`), and a Field Spell, which
/// stands in its zone already, nothing.
pub fn reveal_can_complete(state: &GameState, card: &CardInstance) -> bool {
    let Some(at) = crate::zones::slot_of(state, card) else {
        return false;
    };
    match crate::faces::card_type_of_face(state, &card.def_id, card.radiant) {
        CardType::Unit => crate::animated::unit_zone_for(state, at.player, at.lane).is_some(),
        CardType::Spell => {
            let mut probe = card.clone();
            if let Some(set) = probe.set_as.as_mut() {
                set.revealing = Some(true);
            }
            crate::play_choices::declared_targets(state, &probe)
                .iter()
                .all(|decl| {
                    let offered =
                        crate::play_choices::legal_selections_for(state, card.controller, &probe, decl);
                    !crate::play_choices::unmet_requirement(decl, offered.len())
                })
        }
        _ => true,
    }
}

/// R783006: fire a card set by its Cloaked as its Reveal is activated — the trap firing a timed
/// reveal is (R1041, R1044), answering the `activated` event, for its controller.
pub fn fire_cloaked_reveal(sink: &mut EngineSink<'_>, card: &CardInstance, event: &GameEvent) {
    let reveal = crate::traps::TrapMatch {
        trap: card.clone(),
        triggers: reveal_triggers(card),
    };
    crate::traps::fire_trap(sink, &reveal, event, Some(card.controller));
}

/// ME-ALTPLAY: the one effect a reveal runs — a Unit animates then cries, a Spell resolves. A Field
/// Spell set by its Cloaked (R783006) runs its Cry from where it stands, as a Spell's text runs, and
/// the firing's end turns it into its own card there (`traps::consume_trap`).
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
            // R1041, R171: it counts as summoned on its set turn, or on the later turn it came back
            // onto the field after it was set (a Rollback restoring it, a steal's fresh id).
            let entered = card
                .summoned_turn
                .map_or(set.set_turn, |turn| turn.max(set.set_turn));
            if let Some(stored) = find_instance_mut(ctx.state, &snapshot.id) {
                stored.set_as = None;
                stored.summoned_turn = Some(entered);
            }
            // R783010: a Crater Unit Locks the unit zone it came out into, before its Cry.
            crate::crater::lock_on_arrival(ctx, &snapshot.id);
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
