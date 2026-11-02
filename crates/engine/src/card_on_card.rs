//! The card-on-card verbs' standing rules (#1850), read by every state check (§4.5) before anything is
//! collected for death, each a whole step after which the check starts over:
//!
//!  - R1850003: a card held beneath another (`effects::hold`) comes back once its holder has left;
//!  - R1865000: a hand card set aside beneath another (`set_aside_from_hand`) is then exiled instead;
//!  - R1850002: an exile aura (`Script.exile_auras`) exiles every card on the field it reaches;
//!  - R1850005: a `Script.radiant_at_keywords` permanent is made Radiant once its keywords reach N.
//!
//! Each answers true only when it changed something, so a board none of them touches settles as before.

use crate::config::{BACKROW_ZONES, UNIT_ZONES};
use crate::effects::move_::matches_cost;
use crate::effects::summon::summon_existing_card;
use crate::effects::targets::cards_in_scope_aimed;
use crate::resolve::{HookOptions, make_context};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, HeldCard, find_instance};
use crate::wire::{KeywordKind, TargetAim, ZoneName};

/// R1850003: whether the card holding `held` is gone — it has left the field since the card was taken,
/// or is on it no longer (a holder that never landed holds nothing). A holder still on the field holds
/// though it does not act there: beneath a Stack pile's top (§3.2) or Dormant (R1839000).
fn holder_gone(sink: &EngineSink<'_>, held: &HeldCard) -> bool {
    crate::stays::left_field_after(sink.state, held.holder_mark, &held.holder_id)
        || !find_instance(sink.state, &held.holder_id)
            .is_some_and(|holder| holder.zone.z() == ZoneName::Field)
}

/// R1850003: summon every held card whose holder is gone, in the order they were taken — for the
/// player it returns to, at R64's placement, summoning sick and with no Cry (R1). One with no zone to
/// enter is exiled, and so is a hand card set aside (R1865000). True when any came back.
pub fn release_held(sink: &mut EngineSink<'_>) -> bool {
    let Some(held) = sink.state.held.as_ref() else {
        return false;
    };
    let gone: Vec<bool> = held.iter().map(|entry| holder_gone(sink, entry)).collect();
    if !gone.contains(&true) {
        return false;
    }
    let mut released: Vec<HeldCard> = Vec::new();
    let mut kept: Vec<HeldCard> = Vec::new();
    for (entry, gone) in sink.state.held.take().into_iter().flatten().zip(gone) {
        if gone {
            released.push(entry);
        } else {
            kept.push(entry);
        }
    }
    sink.state.held = if kept.is_empty() { None } else { Some(kept) };
    for entry in released {
        // R1865000: a set-aside hand card is exiled, the printed card again (R215).
        if entry.exile_on_release == Some(true) {
            let mut card = entry.card;
            crate::zones::reset_instance(&mut card);
            crate::zones::exile_to_pile(sink, card);
            continue;
        }
        let summoned = {
            let mut ctx = make_context(
                sink,
                None,
                HookOptions {
                    controller: Some(entry.to),
                    ..HookOptions::default()
                },
            );
            summon_existing_card(&mut ctx, &entry.card, entry.to)
        };
        if summoned.is_none() {
            crate::zones::exile_to_pile(sink, entry.card);
        }
    }
    true
}

/// The permanents whose standing rules a state check reads, by id, in R68's board order (the active
/// player's side first; within a side the unit tops by lane, then the backrow by lane): those with an
/// exile aura, and those not yet Radiant with a keyword count to reach. One script read per
/// permanent, and no card copied, since every state check pass walks the whole board here.
fn standing_rules(state: &GameState) -> (Vec<String>, Vec<(String, i32)>) {
    let mut exilers: Vec<String> = Vec::new();
    let mut risers: Vec<(String, i32)> = Vec::new();
    for player in [state.active, state.active.opponent()] {
        let side = &state.players[player];
        let tops = side
            .units
            .iter()
            .take(UNIT_ZONES as usize)
            .filter_map(|pile| pile.as_ref().and_then(|pile| pile.first()));
        let backrow = side
            .backrow
            .iter()
            .take(BACKROW_ZONES as usize)
            .filter_map(Option::as_ref);
        for card in tops.chain(backrow) {
            // R1487000, R1839000: a face-down or Dormant card's lasting text is off.
            if crate::preview::lasting_text_off(state, card) {
                continue;
            }
            let script = crate::scripts::script_of(state, card);
            if !script.exile_auras.is_empty() {
                exilers.push(card.id.clone());
            }
            if let Some(needed) = script.radiant_at_keywords
                && !card.radiant
            {
                risers.push((card.id.clone(), needed));
            }
        }
    }
    (exilers, risers)
}

/// R1850002, R1850005: the standing rules of every permanent's text, exile auras first; true when one
/// changed something.
pub fn standing_text(sink: &mut EngineSink<'_>) -> bool {
    let (exilers, risers) = standing_rules(sink.state);
    (!exilers.is_empty() && exile_by_auras(sink, &exilers))
        || (!risers.is_empty() && radiant_by_keywords(sink, &risers))
}

/// R1850002: the exile auras of `sources`, each exiling the cards it reaches as the walk comes to it: a
/// source or a card already gone is passed by. A card never exiles itself, a face-down card is never
/// reached (its cost is hidden), and a card with a keyword kind the aura spares is passed by.
fn exile_by_auras(sink: &mut EngineSink<'_>, sources: &[String]) -> bool {
    let mut changed = false;
    for id in sources {
        let Some(source) = find_instance(sink.state, id).cloned() else {
            continue;
        };
        if !crate::zones::acts_on_field(sink.state, &source) {
            continue;
        }
        let auras = crate::scripts::script_of(sink.state, &source).exile_auras.clone();
        let reached: Vec<CardInstance> = {
            let ctx = make_context(
                sink,
                Some(&source),
                HookOptions {
                    controller: Some(source.controller),
                    ..HookOptions::default()
                },
            );
            let mut reached: Vec<CardInstance> = Vec::new();
            for aura in &auras {
                for card in cards_in_scope_aimed(&ctx, &aura.scope, TargetAim::Harm) {
                    if card.id == source.id
                        || crate::preview::is_face_down(ctx.state, &card)
                        || reached.iter().any(|other| other.id == card.id)
                        || aura
                            .cost
                            .as_ref()
                            .is_some_and(|filter| !matches_cost(&ctx, &card, filter))
                        || aura
                            .not_keywords
                            .iter()
                            .any(|kind| crate::layers::unit_has(ctx.state, &card, *kind))
                    {
                        continue;
                    }
                    reached.push(card);
                }
            }
            reached
        };
        for card in reached {
            let Some(live) = find_instance(sink.state, &card.id).cloned() else {
                continue;
            };
            if !crate::zones::acts_on_field(sink.state, &live) {
                continue;
            }
            crate::zones::exile_to_pile(sink, live);
            changed = true;
        }
    }
    changed
}

/// R1850005: each of `rising` whose distinct keyword kinds, read through the layers (§10.4: printed,
/// granted, aura and position keywords, each kind once), reach its text's N is made Radiant
/// (`effects::radiant`, `radiantSet`). True when any card was made Radiant.
fn radiant_by_keywords(sink: &mut EngineSink<'_>, rising: &[(String, i32)]) -> bool {
    let mut changed = false;
    for (id, needed) in rising {
        let Some(card) = find_instance(sink.state, id).cloned() else {
            continue;
        };
        let mut kinds: Vec<KeywordKind> = Vec::new();
        for keyword in crate::layers::keywords_of(sink.state, &card) {
            if !kinds.contains(&keyword.kind()) {
                kinds.push(keyword.kind());
            }
        }
        if i32::try_from(kinds.len()).unwrap_or(i32::MAX) < *needed {
            continue;
        }
        let mut ctx = make_context(
            sink,
            Some(&card),
            HookOptions {
                controller: Some(card.controller),
                ..HookOptions::default()
            },
        );
        changed |= crate::effects::radiant::make_radiant(&mut ctx, &card);
    }
    changed
}
