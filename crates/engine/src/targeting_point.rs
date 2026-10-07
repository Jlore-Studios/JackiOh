//! The targeting point (docs/classic-sets.md B5 E5 "a friendly unit is targeted", E9's target redirect;
//! R450): what happens the moment a player targets a card, before anything resolves at it.
//!
//! Two things answer a targeting, in this order:
//!   1. A targeting cost (Classic #89 Paul Allen's Ghost: "to target this with anything but an attack,
//!      a player must also discard N cards"). The discards are random at pay time (R682); §10.5 step
//!      2 pays them (`pay_targeting_discards`), and a prompt answer naming such a card pays them before
//!      it goes on. It binds both players. With fewer cards than the cost in hand the card is not a
//!      legal target at all (`targeting::can_pay_to_target`), so it is never offered.
//!   2. An interception (Classic #33 Joro: "While this is in your hand: when your opponent targets one
//!      of your Units with a Spell, summon this and make it the new target"). The first card in the
//!      targeted unit's controller's hand whose `replacements` declare `{ on: "targeted", where:
//!      "hand" }` and answer this source (`by: "spell"` needs a Spell, R651) is
//!      summoned (R64's leftmost open unit zone; with none, nothing happens), with no Cry and
//!      summoning sick, and the pick moves to it — `redirected` "target". One interceptor answers one targeting: a play naming several of
//!      that player's units redirects the first. A declared pick moves only when the interceptor is
//!      itself a legal pick of that declaration (Hearthstone's Spellbender); a cost already paid for
//!      the first pick stays paid.
//!      "Targeting" is choosing: declared `target` picks of a play, a cast and an activation, and every
//!      answer to a `target` prompt. Random picks, "all" effects, Tributes, hand picks and zone picks target
//!      nothing (R450). The attack half (§4.2 step 2, `combat.rs`) calls `intercept_targeting` with
//!      `what: "attack"`.
//!
//! A prompt answer reaches this through `prompts.rs`'s calls into this module (TS
//! `prompts.registerTargetingHooks`, which goes, SURFACE §6.6: `prompts.rs` calls
//! `targetable_options`, `why_target_answer_refused` and `target_answer` here directly) and through the
//! play pipeline's own answerer (`play_steps::answer_play_prompt`), which owns its prompts (R122).
//!
//! Port of `packages/engine/src/targetingPoint.ts` (part 3).

use serde_json::json;

use crate::prelude::json_as;
use crate::prompts::OpenPromptArgs;
use crate::script::EngineSink;
use crate::state::{CardInstance, EngineError, GameState, PendingChoice, PromptOption, find_instance};
use crate::wire::{CardType, GameEvent, PlayerId, PromptKind, RedirectWhat, Selection, ZoneName};

/// What a redirect moved: a chosen target here, an attack at §4.2 step 2 (`combat.rs`). TS's
/// `"target" | "attack"`, the two literals of the event's own `redirected.what` that a targeting can
/// move, so it is that type.
pub type RedirectKind = RedirectWhat;

/// `InterceptArgs.accepts`: whether this card may stand as pick `index`, read on the state as it stands.
pub type InterceptorFilter<'a> = dyn Fn(&GameState, &CardInstance, usize) -> bool + 'a;

/// TS `InterceptArgs`. The two predicates borrow what their caller closes over; `accepts` is also
/// handed the state as it stands, because the caller's state is the sink this call writes to.
pub struct InterceptArgs<'a> {
    /// The player who targeted.
    pub chooser: PlayerId,
    /// The picks, in the order they were made.
    pub picks: Vec<Selection>,
    /// Which picks are targetings (a declared `target` pick); every pick when absent.
    pub targeting: Option<&'a dyn Fn(usize) -> bool>,
    /// Whether the interceptor may stand as pick `index` (a declaration's filter); always when absent.
    pub accepts: Option<&'a InterceptorFilter<'a>>,
    pub what: Option<RedirectKind>,
    /// The targeting card's type; an attack carries none, so a `by: "spell"` card never answers it (R651).
    pub source: Option<CardType>,
}

/// Classic #33 Joro, R450: summon the card that answers this targeting and move the first pick it
/// answers to it. Returns the picks, with at most one changed. The summon is §6.3's (`effects::summon`):
/// the leftmost open unit zone of its controller, no Cry, summoning sick, `summoned`; then `redirected`.
pub fn intercept_targeting(sink: &mut EngineSink<'_>, args: InterceptArgs<'_>) -> Vec<Selection> {
    let mut picks = args.picks.clone();
    for index in 0..picks.len() {
        if let Some(targeting) = args.targeting
            && !targeting(index)
        {
            continue;
        }
        let Selection::Instance { instance_id } = &picks[index] else {
            continue;
        };
        let Some(targeted) = find_instance(sink.state, instance_id).cloned() else {
            continue;
        };
        let Some(interceptor) =
            crate::targeting::interceptor_for(sink.state, args.chooser, &targeted, args.source).cloned()
        else {
            continue;
        };
        if let Some(accepts) = args.accepts
            && !accepts(sink.state, &interceptor, index)
        {
            continue;
        }

        let owner = interceptor.controller;
        {
            let mut ctx = crate::resolve::make_context(
                sink,
                Some(&interceptor),
                crate::resolve::HookOptions {
                    controller: Some(owner),
                    ..crate::resolve::HookOptions::default()
                },
            );
            let effect = crate::effects::summon::summon(json_as(json!({
                "instance": { "of": "instance", "instanceId": interceptor.id }
            })));
            (effect.apply)(&mut ctx);
        }
        let on_field =
            find_instance(sink.state, &interceptor.id).is_some_and(|card| card.zone.z() == ZoneName::Field);
        if !on_field {
            return picks;
        }
        picks[index] = Selection::Instance {
            instance_id: interceptor.id.clone(),
        };
        sink.events.push(GameEvent::Redirected {
            what: args.what.unwrap_or(RedirectWhat::Target),
            from_id: targeted.id.clone(),
            to_id: interceptor.id.clone(),
            by_instance_id: Some(interceptor.id.clone()),
        });
        return picks;
    }
    picks
}

/// R450, R682, §6.3 Discard: pay a targeting cost of `count` discards — random cards from the
/// player's hand outside `keep`, drawn through the match rng. Fewer cards than the cost ends it; a
/// cost nobody can pay is never listed or offered (`can_pay_to_target`, `why_targeting_discards_unpayable`),
/// so the keep is what the refusal kept: the card a play is taking out of that hand, and any hand
/// card the same play or activation picks. A prompt answer keeps nothing, as its refusal does (TS
/// `keep = []`: pass `&[]`).
pub fn pay_targeting_discards(sink: &mut EngineSink<'_>, player: PlayerId, count: i32, keep: &[String]) {
    for _ in 0..count.max(0) {
        let hand: Vec<CardInstance> = sink.state.players[player]
            .hand
            .iter()
            .filter(|card| !keep.contains(&card.id))
            .cloned()
            .collect();
        if hand.is_empty() {
            return;
        }
        let Some(card) = sink.rng.pick(&hand).cloned() else {
            return;
        };
        crate::effects::move_::discard_from_hand(sink, &card);
    }
}

// ---------------------------------------------------------------------------------------------
// A prompt's answer (R450): the cost, then the interception
// ---------------------------------------------------------------------------------------------

/// R450: why a `target` answer cannot stand — its picks cost more cards than its chooser holds (two
/// costly picks of one prompt, each payable alone). `Ok` for any other prompt.
pub fn why_target_answer_refused(
    state: &GameState,
    pending: &PendingChoice,
    picks: &[Selection],
) -> Result<(), EngineError> {
    if pending.kind != PromptKind::Target {
        return Ok(());
    }
    let cost = crate::targeting::targeting_discards_for(state, picks, None);
    let hand = state.players[pending.player_id].hand.len() as i32;
    if cost > hand {
        return Err(EngineError::new(format!(
            "targeting those costs {cost} discards, and you hold {hand} cards"
        )));
    }
    Ok(())
}

/// R651: the targeting card's type behind a `target` prompt — the card whose prompt it is
/// (`resume.instance_id`), read as the pick is answered. `None` when the prompt names no card.
fn source_of(state: &GameState, instance_id: Option<&str>) -> Option<CardType> {
    let source = find_instance(state, instance_id?)?;
    Some(crate::faces::card_type_of(state, source))
}

/// R450, R682: the targeting point of a `target` prompt's answer, which the caller has validated and
/// closed. A costly answer pays its random discards first, then the interception answers.
pub fn target_answer(
    sink: &mut EngineSink<'_>,
    pending: &PendingChoice,
    picks: &[Selection],
) -> Vec<Selection> {
    if pending.kind != PromptKind::Target {
        return picks.to_vec();
    }
    let chooser = pending.player_id;
    let cost = crate::targeting::targeting_discards_for(sink.state, picks, None);
    if cost > 0 {
        pay_targeting_discards(sink, chooser, cost, &[]);
    }
    let source = source_of(sink.state, pending.resume.instance_id.as_deref());
    intercept_targeting(
        sink,
        InterceptArgs {
            chooser,
            picks: picks.to_vec(),
            targeting: None,
            accepts: None,
            what: Some(RedirectWhat::Target),
            source,
        },
    )
}

/// R450, E35: the options a `target` prompt may offer its chooser — none they could not pay to target,
/// and none Immune to Spells when a Spell asks. Every other prompt keeps its options. (Private in TS,
/// where `registerTargetingHooks` handed it to `prompts.ts`; `prompts.rs` calls it here.)
pub fn targetable_options(state: &GameState, args: &OpenPromptArgs) -> Vec<PromptOption> {
    if args.kind != PromptKind::Target {
        return args.options.to_vec();
    }
    let source = args
        .resume
        .instance_id
        .as_deref()
        .and_then(|id| find_instance(state, id));
    let spell_source = source.filter(|card| crate::faces::card_type_of(state, card) == CardType::Spell);
    args.options
        .iter()
        .filter(|option| {
            let Selection::Instance { instance_id } = &option.selection else {
                return true;
            };
            let Some(card) = find_instance(state, instance_id) else {
                return true;
            };
            if card.zone.z() != ZoneName::Field {
                return true;
            }
            crate::targeting::may_target(state, args.player, spell_source, card, None)
        })
        .cloned()
        .collect()
}
