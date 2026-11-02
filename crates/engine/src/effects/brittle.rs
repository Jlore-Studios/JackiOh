//! Brittle X's verbs (docs/classic-sets.md B3.3 rule 4, R385), on a card anywhere (Classic+ #23
//! Dropshipping, T-AI-3 Hallucination, #74). The count lives on the instance (`brittle_count.rs`) and
//! ticks at its controller's start of turn (`brittle.rs`). Events (R440): a named card's is hidden by
//! where it sits (R97); a scope's cards are reported only where both players read them, or the cues
//! would count what its filters let through.

use serde::{Deserialize, Serialize};

use super::card_scope::{CardScope, CardScopeOptions, Readers, cards_in_card_scope};
use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::brittle_count::{active_brittle_count, gain_brittle_count, give_brittle_count};
use crate::damage::DamageTarget;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, ZoneName};

/// Which cards a Brittle verb reaches: one named card (a spec or an id a script captured), or a scope.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BrittleTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CardScope>,
}

/// TS `BrittleTarget & { n: number }`: `give_brittle`'s (and `gain_brittle`'s) arguments, the target's
/// fields beside `n`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GiveBrittleArgs {
    #[serde(flatten)]
    pub on: BrittleTarget,
    pub n: i32,
}

pub type GainBrittleArgs = GiveBrittleArgs;

/// One card a Brittle verb reached, and whether its change is reported.
struct Reached {
    card: CardInstance,
    report: bool,
}

fn reached(ctx: &EffectContext<'_>, args: &BrittleTarget) -> Vec<Reached> {
    if let Some(scope) = &args.scope {
        // MD-B1, R940: Brittle is harmful.
        return cards_in_card_scope(
            ctx,
            scope,
            Some(&CardScopeOptions {
                aim: Some(crate::wire::TargetAim::Harm),
                ..Default::default()
            }),
        )
        .into_iter()
        .map(|entry| Reached {
            report: entry.readers == Readers::Everyone,
            card: entry.card,
        })
        .collect();
    }
    let mut card: Option<CardInstance> = None;
    // R174: a card named by id is aimed at the stay it had when the run began.
    if let Some(instance_id) = &args.instance_id {
        card = instance_on_its_stay(ctx, instance_id);
    } else if let Some(target) = &args.target {
        card = match resolve_target(ctx, target) {
            Some(DamageTarget::Unit { instance }) => Some(instance),
            _ => None,
        };
    }
    // A card that has ceased to exist (R11, R86) is in no pile to take a count.
    match card {
        Some(card) if card.zone.z() != ZoneName::Gone => vec![Reached { card, report: true }],
        _ => Vec::new(),
    }
}

/// The count as the verb left it, read off the card where it stands now.
fn report(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    let Some(live) = find_instance(ctx.state, &card.id) else {
        return;
    };
    let Some(count) = active_brittle_count(live) else {
        return;
    };
    let instance_id = live.id.clone();
    ctx.events.push(GameEvent::CounterChanged {
        instance_id,
        counter: CounterKind::Brittle,
        value: count,
        placed: None,
    });
}

/// Change the count of the card as it stands in the state, through a copy written back.
fn write_count(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    change: impl FnOnce(&GameState, &mut CardInstance),
) {
    let mut live = crate::zones::live_or(ctx.state, card);
    change(ctx.state, &mut live);
    if let Some(stored) = find_instance_mut(&mut *ctx.state, &card.id) {
        stored.brittle = live.brittle;
    }
}

/// B3.3 rule 4: "Give Brittle N": N from now, whatever it was (R385).
pub fn give_brittle(args: GiveBrittleArgs) -> Effect {
    Effect::new("giveBrittle", move |ctx| {
        for entry in reached(ctx, &args.on) {
            let n = args.n;
            write_count(ctx, &entry.card, |state, card| give_brittle_count(state, card, n));
            if entry.report {
                report(ctx, &entry.card);
            }
        }
    })
}

/// B3.3 rule 4: "gain +N Brittle" — N more on the count in force; a card with none starts one now, at
/// N more than it prints (R441).
pub fn gain_brittle(args: GainBrittleArgs) -> Effect {
    Effect::new("gainBrittle", move |ctx| {
        for entry in reached(ctx, &args.on) {
            let n = args.n;
            write_count(ctx, &entry.card, |state, card| gain_brittle_count(state, card, n));
            if entry.report {
                report(ctx, &entry.card);
            }
        }
    })
}

/// R1861001: "lower Brittle by N", never below 0 and only a count in force. A card on the field left
/// at 0 crumbles at once (B3.3 rule 3), an ordinary destroy, so its Death reads the count spent
/// (`query::brittle_spent`).
pub fn lower_brittle(args: GainBrittleArgs) -> Effect {
    Effect::new("lowerBrittle", move |ctx| {
        for entry in reached(ctx, &args.on) {
            let n = args.n.max(0);
            write_count(ctx, &entry.card, |state, card| {
                if active_brittle_count(card).is_some() {
                    gain_brittle_count(state, card, -n);
                }
            });
            if entry.report {
                report(ctx, &entry.card);
            }
            let Some(live) = find_instance(ctx.state, &entry.card.id).cloned() else {
                continue;
            };
            if live.zone.z() == ZoneName::Field && active_brittle_count(&live) == Some(0) {
                crate::brittle::crumble(&mut ctx.sink, &live);
            }
        }
    })
}
