//! Face-down plays as Traps: Cloaked (#783, R783004–R783010, R1040–R1046).
//!
//! A Unit, Spell or Field Spell that has Cloaked in its controller's hand (printed, given, or lent by
//! an aura there, §10.4 layer 5: Knowledge Breaker (#45) and its Prime to Units, Paranoia (#99) to
//! Spells, `cloak_hand_aura`) may be set face-down at its own price (R783005). A set card is a Field
//! Trap or a Trap until it reveals, hidden like any set Trap (R33, R1046), its text dormant. It reveals
//! only when its controller activates its Reveal (`CLOAKED_REVEAL`), offered only while the card can
//! come out whole (`reveal_can_complete`). The form ends when the card leaves the field (R78). Under
//! Paranoia a set Spell's text also resolves once as it is set, and the card stays set (R3806000,
//! `resolve_at_set`); a card played face-down runs no Cry at its play.
//!
//! No card grants ME-ALTPLAY's timed reveals: the three timings (`RevealAt::ALL`) are never offered,
//! and the wire keeps them so old records still read.

use crate::script::{AuraEntry, AuraHook, Effect, EngineSink, HookArgs, StatMod, TriggerDef};
use crate::state::{CardInstance, GameState, SetAs, find_instance, find_instance_mut};
use crate::wire::{CardType, GameEvent, Keyword, KeywordKind, PlayerId, RevealAt, Row, Zone, ZoneChoice};

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

/// R783004: the way this card may be set face-down now (`RevealAt::Cloaked`): a Unit, a Spell or a
/// Field Spell with Cloaked in a hand has one.
pub(crate) fn face_down_options(
    state: &GameState,
    card: &CardInstance,
    from_graveyard: bool,
) -> Vec<(SetKind, RevealAt)> {
    if from_graveyard || !is_cloaked(state, card) {
        return Vec::new();
    }
    let kind = match crate::faces::card_type_of_face(state, &card.def_id, card.radiant) {
        CardType::Unit => SetKind::Unit,
        CardType::Spell => SetKind::Spell,
        CardType::FieldSpell => SetKind::FieldSpell,
        CardType::Trap | CardType::FieldTrap => return Vec::new(),
    };
    vec![(kind, RevealAt::Cloaked)]
}

/// R1040, R1044, R783007: "Aura: Your <type>s have Cloaked." — Cloaked for each card of this printed
/// type in its controller's hand while the source acts on their field. A card already set keeps its
/// form when the aura ends.
pub fn cloak_hand_aura(type_: CardType) -> AuraHook {
    crate::script::aura_hook(move |args| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| {
                matches!(card.zone, Zone::Hand { player } if player == controller)
                    && crate::faces::card_type_of_face(state, &card.def_id, card.radiant) == type_
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Cloaked]),
                ..StatMod::default()
            },
        }]
    })
}

/// R1045: the Echo every card acting on this player's field adds to a Spell they reveal now, the
/// largest any of them gives (`Script::reveal_echo`, Radiant Paranoia), 0 with none; a carried Unit is
/// one of them (R446, R1824000). Read as the Spell reveals.
pub fn reveal_echo_bonus(state: &GameState, player: PlayerId) -> i32 {
    let mut out = 0;
    for source in crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane) {
        // R1487000: a face-down card's text sleeps until it is revealed or face-up. A Vanilla card and a
        // set card not yet revealing (`is_dormant`) have no script to ask (`scripts::script_of`).
        if source.controller != player || crate::preview::lasting_text_off(state, source) {
            continue;
        }
        let script = crate::scripts::script_of(state, source);
        let Some(hook) = script.reveal_echo.as_ref() else {
            continue;
        };
        out = out.max(hook(HookArgs {
            state,
            self_: source,
            radiant: source.radiant,
        }));
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

/// R783005, R1044: what a play pays as — a card set face-down as the card it is in hand (a set Spell
/// as a Spell, so Cloaked Toe Cracker's "Your Traps cost (0)" never reaches it), anything else as
/// `card_type_of` reads it.
pub fn priced_type_of(state: &GameState, card: &CardInstance) -> CardType {
    if card.set_as.is_some() {
        return crate::faces::card_type_of_face(state, &card.def_id, card.radiant);
    }
    crate::faces::card_type_of(state, card)
}

/// ME-ALTPLAY: whether the card's own text is dormant — set, not yet revealing, on the field.
pub fn is_dormant(card: &CardInstance) -> bool {
    card.set_as.is_some()
        && card.set_as.as_ref().and_then(|set| set.revealing) != Some(true)
        && matches!(card.zone, Zone::Field { .. })
}

/// R783006: the trigger a set card fires on — one `@reveal` that names no event, so no dispatch ever
/// offers it one, and only its Reveal fires it (`fire_cloaked_reveal`).
pub(crate) fn reveal_triggers(card: &CardInstance) -> Vec<TriggerDef> {
    if card.set_as.is_none() {
        return Vec::new();
    }
    vec![TriggerDef::new("@reveal", &[], |_, _| vec![reveal_effect()])]
}

/// R783006: whether a card set by its Cloaked can come out whole now, which its Reveal waits for: a
/// Unit needs the unit zone it would animate into (R64), a Spell a legal pick for each declaration its
/// play could not go without (R703) and its play gate (R1851001), as `cry_trigger::reveal_set_spell`
/// reads them, and a Field Spell, which stands in its zone already, nothing.
pub fn reveal_can_complete(state: &GameState, card: &CardInstance) -> bool {
    let Some(at) = crate::zones::slot_of(state, card) else {
        return false;
    };
    match crate::faces::card_type_of_face(state, &card.def_id, card.radiant) {
        CardType::Unit => crate::animated::unit_zone_for(state, at.player, at.lane).is_some(),
        CardType::Spell => spell_text_can_resolve(state, card.controller, card),
        _ => true,
    }
}

/// R703, R1851001, R3806000: whether a set Spell's text can resolve now for this player, read off the
/// card as it resolves (stamped set and resolving): a legal pick for each declaration its play could
/// not go without, and its play gate. Its Reveal waits for it (`reveal_can_complete`), its set under
/// Paranoia is offered only with it (`play_choices`), and each resolution reads it as it begins
/// (`cry_trigger::reveal_set_spell`).
pub fn spell_text_can_resolve(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    let mut probe = card.clone();
    let set = probe.set_as.get_or_insert(SetAs {
        reveal: RevealAt::Cloaked,
        set_turn: state.turn,
        echo: None,
        revealing: None,
    });
    set.revealing = Some(true);
    crate::play_choices::play_allowed(state, player, &probe)
        && crate::play_choices::declared_targets(state, &probe)
            .iter()
            .all(|decl| {
                let offered = crate::play_choices::legal_selections_for(state, player, &probe, decl);
                !crate::play_choices::unmet_requirement(decl, offered.len())
            })
}

/// R3806000: whether a Spell this player sets face-down now resolves as it is set: a printed Spell,
/// while a card acting on their field whose text is awake resolves their set Spells
/// (`Script::resolves_set_spells`, Meditative #99 Paranoia). Read as the set resolves (§10.5 step 5).
pub fn resolves_as_set(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    crate::faces::card_type_of_face(state, &card.def_id, card.radiant) == CardType::Spell
        && crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane)
            .into_iter()
            .any(|source| {
                // R1487000: a face-down card's text sleeps until it is revealed or face-up.
                source.controller == player
                    && !source.vanilla
                    && !is_dormant(source)
                    && !crate::preview::lasting_text_off(state, source)
                    && crate::scripts::script_of(state, source).resolves_set_spells
            })
}

/// R3806000: a set Spell resolving as it is set — its text awake (`revealing`) while it is still
/// face-down, which its Reveal never is (`traps::fire_trap` turns it face-up first).
pub fn resolving_at_set(card: &CardInstance) -> bool {
    card.set_as.is_some_and(|set| set.revealing == Some(true)) && card.face_up != Some(true)
}

/// R3806000: a set Spell's resolution as it was set is over: it is a dormant set Trap again, its
/// Reveal offered as before. Nothing for a card that is not resolving so.
pub(crate) fn end_set_resolution(state: &mut GameState, id: &str) {
    if let Some(stored) = find_instance_mut(state, id)
        && resolving_at_set(stored)
        && let Some(set) = stored.set_as.as_mut()
    {
        set.revealing = None;
    }
}

/// R3806000 (Meditative #99 Paranoia): a Spell set face-down by its Cloaked while a Paranoia acts on its
/// controller's field resolves its Spell text as it is set, from its backrow zone, as its Reveal will:
/// its choices asked as prompts (R467), its effects a Trap's (R781001), its own Echo repeated with fresh
/// prompts, Radiant Paranoia's not (R1045). It stays set, hidden and dormant, a Trap on the field; "exile
/// this" and "shuffle this into your deck" wait for its reveal. No play (R70), so nothing is counted.
/// A text it cannot resolve now (R703, R1851001) resolves nothing.
pub(crate) fn resolve_at_set(sink: &mut EngineSink<'_>, card: &CardInstance) {
    let Some(live) = find_instance(sink.state, &card.id).cloned() else {
        return;
    };
    let controller = live.controller;
    if !is_cloaked_set(&live)
        || !matches!(
            live.zone,
            Zone::Field {
                row: Row::Backrow,
                ..
            }
        )
        || !resolves_as_set(sink.state, controller, &live)
        || !spell_text_can_resolve(sink.state, controller, &live)
    {
        return;
    }
    if let Some(stored) = find_instance_mut(sink.state, &live.id)
        && let Some(set) = stored.set_as.as_mut()
    {
        set.revealing = Some(true);
    }
    let Some(live) = find_instance(sink.state, &card.id).cloned() else {
        return;
    };
    let repeats = crate::echo::printed_echo(&live, sink.state).max(0) as u32;
    crate::cry_trigger::reveal_set_spell(sink, &live, controller, repeats);
    // A prompt or a parked tail ends it later (`cry_trigger::run_cry_once`).
    if !crate::work::paused(sink) {
        end_set_resolution(sink.state, &live.id);
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
            // R1045: the Echo stamped as it was set, and Radiant Paranoia's while one acts on its
            // controller's field now.
            let echo = set.echo.unwrap_or(0) + reveal_echo_bonus(ctx.state, controller);
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
