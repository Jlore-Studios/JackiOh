//! Brittle X's verbs (docs/classic-sets.md B3.3 rule 4, R385): "Give Brittle N" sets a card's count to
//! N, starting now; "gain +N Brittle" adds N to it. A card anywhere may take one — on the field, in a
//! hand, in a deck (Classic+ #23 Dropshipping gives Brittle 2 to the cards it adds to your hand, T-AI-3
//! Hallucination to its copy, #74 Twice Forward One Step Backwards gains +1 on the field). The count
//! lives on the instance (`brittle_count.rs`) and ticks at its controller's start of turn (`brittle.rs`).
//!
//! Events (R440): a named card's count is reported by `counterChanged` "brittle" wherever it is, hidden
//! by where it sits (R97); a scope's cards are reported only where both players read them, since a cue
//! on each hidden card a scope reached would count the ones its filters let through.
//!
//! Port of `packages/engine/src/effects/brittle.ts`.

use serde::{Deserialize, Serialize};

use super::card_scope::{CardScope, Readers, cards_in_card_scope};
use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::brittle_count::{active_brittle_count, gain_brittle_count, give_brittle_count};
use crate::damage::DamageTarget;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, ZoneName};

/// Which cards a Brittle verb reaches: one named card (a spec or an id a script captured), or a scope.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
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
#[serde(rename_all = "camelCase")]
pub struct GiveBrittleArgs {
    #[serde(flatten)]
    pub on: BrittleTarget,
    pub n: i32,
}

/// `gain_brittle`'s arguments: the same shape as `give_brittle`'s.
pub type GainBrittleArgs = GiveBrittleArgs;

/// One card a Brittle verb reached, and whether its change is reported.
struct Reached {
    card: CardInstance,
    report: bool,
}

fn reached(ctx: &EffectContext<'_>, args: &BrittleTarget) -> Vec<Reached> {
    if let Some(scope) = &args.scope {
        return cards_in_card_scope(ctx, scope, None)
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

/// The count as the verb left it: read off the card where it stands now (TS read the live object the
/// verb had just written).
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

/// Change the count of the card as it stands in the state (TS wrote through the live object the scope or
/// target handed back): the verb runs on a copy of the live card, and the count it leaves is written back.
fn write_count(ctx: &mut EffectContext<'_>, card: &CardInstance, change: impl FnOnce(&GameState, &mut CardInstance)) {
    let mut live = find_instance(ctx.state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone());
    change(ctx.state, &mut live);
    if let Some(stored) = find_instance_mut(&mut *ctx.state, &card.id) {
        stored.brittle = live.brittle;
    }
}

/// B3.3 rule 4: "Give Brittle N" — the count is N from now, whatever it was (a given count, R385).
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
