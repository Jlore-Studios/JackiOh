//! Forced attack, Cancel an attack and the AI turn, as card-script verbs (SPEC §6.3's "Forced
//! attack" and "Cancel an attack" rows, §4.2, §4.5, §10.7's AI bullet, R44, R53, R84).
//!
//! WHY THIS FILE EXISTS. `effects/index.ts`'s header said these verbs "live outside it and are not
//! part of the card-script surface", which was true while nothing needed them from a hook. It is
//! not true any more: #9 Moths to the Flame forces attacks from `startOfTurn`, #60 Bear Honeypot
//! from a trap trigger, and #96 My Pawn cancels an attack and hands the turn to the AI from another.
//! A hook returns `Vec<Effect>` and has no `EngineSink` of its own (CLAUDE.md rules 4 and 5), while
//! `crate::combat`'s `force_attack`/`force_attacks_on` and `crate::subsystems::ai_policy`'s
//! `play_out_turn` all take a sink — so every verb here is a THIN wrapper and nothing here
//! re-implements combat, R53's stop rule or §10.7's policy. `EffectContext` derefs to its
//! `EngineSink`, so `ctx` is passed straight through as the sink.
//!
//! NAMING. The card-facing spellings keep the "d" (`forced_attacks_on`, `forced_attacks`) where
//! `crate::combat` spells them `force_attacks_on`/`force_attack`. That is deliberate: the effect
//! returns an `Effect` and the engine function takes a sink, so one letter keeps a card file from
//! importing the wrong one, and an import of both in this file reads unambiguously.
//!
//! Port of `packages/engine/src/effects/combat.ts`.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::destroy::{DestroyArgs, destroy};
use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of, resolve_target};
use crate::combat::{AttackAmong, AttackTarget, force_attack_own_hero, force_attacks_on, force_attacks_random};
use crate::prompts::summoned_so_far;
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::state_check::state_check;
use crate::stays::exit_mark;
use crate::subsystems::ai_policy::play_out_turn;
use crate::triggers::{SETTLE_PASS_CAP, SettleSink, dispatch_pending, run_queued_trigger};
use crate::wire::{GameEvent, ZoneName, opponent_of};
use crate::work::paused;
use crate::zones::active_units_of;

/// Which side's units are compelled. "any" is both, in R68's order (active side first). (TS
/// `PlayerSpec | "any"`.)
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ForcedSide {
    #[serde(rename = "self")]
    SelfSide,
    Enemy,
    Any,
}

impl ForcedSide {
    /// The `PlayerSpec` this side is, or `None` for "any".
    fn player_spec(self) -> Option<PlayerSpec> {
        match self {
            ForcedSide::SelfSide => Some(PlayerSpec::SelfSide),
            ForcedSide::Enemy => Some(PlayerSpec::Enemy),
            ForcedSide::Any => None,
        }
    }
}

/// The units a forced attack may name, in lane order. `active_units_of` is the lane-order walk §4.2's
/// last paragraph asks for, and it reports only the top card of a Stack pile, so a card dormant
/// underneath one neither attacks nor is hit (§3.2, R13).
fn attackers_of(ctx: &EffectContext<'_>, side: ForcedSide) -> Vec<CardInstance> {
    if let Some(spec) = side.player_spec() {
        return active_units_of(ctx.state, player_of(ctx, spec))
            .into_iter()
            .cloned()
            .collect();
    }
    let first = ctx.state.active;
    let mut out: Vec<CardInstance> = active_units_of(ctx.state, first).into_iter().cloned().collect();
    out.extend(active_units_of(ctx.state, opponent_of(first)).into_iter().cloned());
    out
}

/// `forced_attacks_on`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttacksOnArgs {
    pub target: TargetSpec,
    pub attackers: ForcedSide,
}

/// §6.3 Forced attack: "every enemy unit attacks this" (#9 Moths to the Flame). The named side's
/// active units attack the named target in lane order, and `force_attacks_on` is the whole of R53 —
/// §4.2 steps 1 to 3 skipped (position, summoning sickness and Taunt all ignored), no exertion
/// spent, the target still striking back, each attack its own combat with its own state check, and
/// the run stopping as soon as the target has left the field. A target that resolves to nothing
/// fizzles silently and the card still resolves.
pub fn forced_attacks_on(args: ForcedAttacksOnArgs) -> Effect {
    Effect::new("forcedAttacksOn", move |ctx| {
        let Some(target) = resolve_target(ctx, &args.target) else {
            return;
        };
        let attackers = attackers_of(ctx, args.attackers);
        force_attacks_on(ctx, &attackers, &target, None);
    })
}

/// Which attackers #60 compels: a side, narrowed by definition and by "the ones I just made".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttackerFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<ForcedSide>,
    /// §7's token id, so "they" means the Rush Tokens rather than every unit on the side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    /// "the tokens THIS effect list just summoned" — see `freshly_summoned`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summoned_this_script: Option<bool>,
}

/// "summon 2 Rush Tokens; if it was a Unit, they attack it" (#60 Bear Honeypot). "They" is the
/// tokens this same effect list just summoned, not every Rush Token the controller happens to own.
///
/// Nothing in state records that, and nothing needs to: the `summoned` events those summons pushed
/// are already in `ctx.events`, because an effect list appends to the sink's event array as it
/// applies and this effect runs after them. Reading the ids back out of `ctx.events` is therefore
/// pure, deterministic and replay-stable — the event list is the same on every fold of the log — and
/// it needs no new `GameState` field, no callback and no mutable scratch on the context.
///
/// R136 closes the window `ctx.events` alone leaves open: the sink's list is the whole action's, so
/// a `summoned` event from something earlier in it — a second copy of a card, or a trap that fired
/// mid-action — would read as this script's own. `ctx.events_from` is where this script's events
/// begin, set by `resolve::make_context` when the context was built, so the walk starts there and a
/// card reads only what it did itself. A context built by hand (`EffectContext::new`) opens its
/// window where the event list stood then.
fn freshly_summoned(ctx: &EffectContext<'_>) -> IndexSet<String> {
    // A list a prompt split resumes in a later action, whose event list begins after the pause, so
    // what its head summoned comes with the continuation (`prompts::summoned_so_far`, R113).
    summoned_so_far(ctx).into_iter().collect()
}

/// #60 names its target by the instance id its trigger read off the event (R42-style ids).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum ForcedTarget {
    Instance { instance_id: String },
    Spec { spec: TargetSpec },
}

fn target_of(ctx: &EffectContext<'_>, target: &ForcedTarget) -> Option<AttackTarget> {
    match target {
        // R174: the played unit #60 names by the id its trigger read is aimed at its stay as the run
        // began, so a unit the list took off the field before this effect is gone, Reborn body or not.
        ForcedTarget::Instance { instance_id } => resolve_target(
            ctx,
            &TargetSpec::Instance {
                instance_id: instance_id.clone(),
            },
        ),
        ForcedTarget::Spec { spec } => resolve_target(ctx, spec),
    }
}

/// `forced_attacks`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttacksArgs {
    pub attackers: ForcedAttackerFilter,
    pub target: ForcedTarget,
}

/// §6.3 Forced attack with a filtered attacker list (#60 Bear Honeypot). Same delegation as
/// `forced_attacks_on`: this only decides who is named, and `force_attacks_on` is all of R53. A target
/// that has already died during its own resolution simply is not on the field, so the run finds
/// nothing to attack and the tokens are left standing.
pub fn forced_attacks(args: ForcedAttacksArgs) -> Effect {
    Effect::new("forcedAttacks", move |ctx| {
        let Some(target) = target_of(ctx, &args.target) else {
            return;
        };

        let fresh = if args.attackers.summoned_this_script == Some(true) {
            Some(freshly_summoned(ctx))
        } else {
            None
        };
        let attackers: Vec<CardInstance> = attackers_of(ctx, args.attackers.side.unwrap_or(ForcedSide::SelfSide))
            .into_iter()
            .filter(|unit| {
                if let Some(def_id) = &args.attackers.def_id
                    && unit.def_id != *def_id
                {
                    return false;
                }
                fresh.as_ref().is_none_or(|fresh| fresh.contains(&unit.id))
            })
            .collect();

        // R174, R53: the run is the list's, so its stays are the ones the list began with — the tokens
        // it summoned are on them, and a target it took off the field is gone (`force_attacks_on`).
        let since = ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state));
        force_attacks_on(ctx, &attackers, &target, Some(since));
    })
}

/// `cancel_attack`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelAttackArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroy_attacker: Option<bool>,
}

/// §6.3 Cancel an attack and R44: inside §4.2 step 4's trap window, mark the open `declaredAttack`
/// cancelled so no combat resolves, and emit `attackCancelled` in its place. The attacker's
/// exertion was spent on the declaration, so the attack is gone either way — which is why nothing
/// here gives it back. The event names the card that cancelled, so a call with no `self` (nothing
/// for `byInstanceId` to be) fizzles silently rather than inventing a source.
///
/// Marking the record is the whole of the verb: `combat::declare_attack` reads it back when the window
/// closes and skips step 5 (`resolve_declared_attack`). The window is the only moment there is
/// anything to mark — §6.3 says so, "call off an attack already declared, before any damage" — so
/// outside one this fizzles, which is also what keeps a trap that fires on some later dispatch of
/// the same declaration from cancelling a combat that has already happened.
///
/// R121: a forced attack opens no window and writes no `declaredAttack`, so this can never cancel
/// one. That is the rule, not an omission (see `combat::force_attack`).
///
/// `destroyAttacker` (R283, Radiant #96) destroys the attacker of the attack this cancels, as part of
/// the cancel: an ordinary §6.3 destroy of the unit the declaration names, on the stay it declared
/// from (R174), so an Indestructible attacker is knocked down at the next check (R46) and a Reborn
/// one comes back. Tied to the cancel, it happens only where the cancel does — inside the window, on
/// an attack not yet cancelled — so a My Pawn fused onto a My Pawn, whose second half runs once the
/// first has played the turn out and the window has closed (R102), destroys nothing a second time.
pub fn cancel_attack(args: CancelAttackArgs) -> Effect {
    Effect::new("cancelAttack", move |ctx| {
        let Some(by_instance_id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
            return;
        };
        let Some(open) = ctx.state.declared_attack.as_mut() else {
            return;
        };
        if open.cancelled {
            return;
        }

        open.cancelled = true;
        let attacker_id = open.attacker_id.clone();
        let target_id = open.target_id.clone();
        ctx.events.push(GameEvent::AttackCancelled {
            attacker_id: attacker_id.clone(),
            target_id,
            by_instance_id,
        });
        if args.destroy_attacker == Some(true) {
            let effect = destroy(DestroyArgs {
                target: TargetSpec::Instance {
                    instance_id: attacker_id,
                },
            });
            (effect.apply)(ctx);
        }
    })
}

/// `ai_plays_out_turn`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiPlaysOutTurnArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_first: Option<bool>,
}

/// R44 and R84: "an AI plays the rest of their turn with random legal actions" (#96 My Pawn). Two
/// steps, both somebody else's code. `ai_turn` on that player's `PlayerState` is the lockout R44
/// describes — the client refuses to act while it is set, and `turn::cleanup` clears it at the end of
/// the turn it was set for (R152; `start_turn` keeps a backstop clear) — and the turn itself goes to
/// `play_out_turn`, whose uniform draw over `legal_actions` minus `AI_SKIPPED_ACTIONS` IS §10.7's
/// policy, `concede`, `offerDraw` and `answerDraw` included out. Nothing here re-implements the
/// policy or decides an action.
///
/// `play_out_turn` drives `reduce`, which hands back a fresh state that it adopts into the sink's
/// state field by field: `ctx.state` stays the same object, but every instance inside it is new
/// afterwards, so no effect after this one may hold a `CardInstance` it read before it — `#96`'s
/// list ends here for that reason.
///
/// `settleFirst` (R283, R59) settles the board once the lockout is set and before the AI's first
/// action. `destroy` only marks (§6.3), and the check that collects a mark runs after each whole
/// effect (R59) — which, for a list that ends in this effect, is inside the playout's first
/// `reduce`, after the AI has already chosen from a board that still holds the marked unit. Radiant
/// #96 destroys the attacker it stopped and then hands the turn over, and R283 has the AI take over a
/// settled board, so its face asks for `settle_before_playout` here: the check collects the attacker,
/// the traps answer what it said, and the ordinary triggers it woke resolve — a #89 Corpse Eater in
/// the AI's hand eats the attacker before the AI can play it (R212 would have it answer nothing
/// once it has moved). The destroy before this effect is whole, so this is R59's check between two
/// effects, never one between the hits of one. A check that ends the game ends the effect with it,
/// and a question it opens leaves the AI turn to `play_out_turn`, which owes it behind the answer.
/// Off by default, so every other list, base #96's included, plays out exactly as before.
///
/// (TS's note on the static import cycle effects → aiPolicy → reduce → playSteps → effects does not
/// carry over: one Rust crate calls across its modules directly, SURFACE §6.6. §9.3 and CLAUDE.md
/// rule 4 still ban anything asynchronous inside `reduce`.)
pub fn ai_plays_out_turn(args: AiPlaysOutTurnArgs) -> Effect {
    Effect::new("aiPlaysOutTurn", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        // "The rest of their turn": with no turn of theirs running there is nothing to hand over. A
        // #96 fused onto a #96 runs its second half after its first has played that turn out (R102),
        // and a lockout set then would fall on the other player's turn, which no My Pawn took (R152).
        if ctx.state.active != player || ctx.state.result.is_some() {
            return;
        }
        ctx.state.players[player].ai_turn = true;
        if args.settle_first == Some(true) {
            settle_before_playout(ctx);
            if ctx.state.result.is_some() {
                return;
            }
        }
        play_out_turn(ctx, player, None);
    })
}

/// R283: what the list before an AI turn has done, settled before the AI acts — the state check, the
/// traps' answers to its events, and the ordinary triggers those events woke, each followed by the
/// check again (§10.3, §4.5, R59) — as the loop settles between two actions of a turn, which the AI's
/// are. Only the triggers woken here run: whatever the enclosing action had queued before this list
/// keeps its place and waits for that action's own loop (R117). Stops at a question or a result.
fn settle_before_playout(ctx: &mut EffectContext<'_>) {
    // TS `const sink: SettleSink = ctx`: the same sink, with the dispatch count the loop keeps on it.
    let mut settle = SettleSink {
        sink: ctx.sink.reborrow(),
        dispatched: None,
    };
    let waiting: IndexSet<String> = settle
        .sink
        .state
        .trigger_queue
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    for _pass in 0..SETTLE_PASS_CAP {
        state_check(&mut settle.sink);
        if settle.sink.state.result.is_some() || paused(&settle.sink) {
            return;
        }
        dispatch_pending(&mut settle);
        if settle.sink.state.result.is_some() || paused(&settle.sink) {
            return;
        }
        let Some(at) = settle
            .sink
            .state
            .trigger_queue
            .iter()
            .position(|entry| !waiting.contains(&entry.id))
        else {
            return;
        };
        let woken = settle.sink.state.trigger_queue.remove(at);
        run_queued_trigger(&mut settle.sink, &woken);
    }
    panic!("the board before the AI turn did not settle in {SETTLE_PASS_CAP} passes (R283)");
}

// ---------------------------------------------------------------------------
// B5 E35: forced attacks on "a random enemy" and on the unit's own hero
// ---------------------------------------------------------------------------

/// `forced_attack_random`'s arguments (TS's inline object). `among` is TS's `"enemies" | "enemyUnits"`,
/// the type `combat::force_attacks_random` takes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttackRandomArgs {
    pub attacker: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub among: Option<AttackAmong>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times: Option<i32>,
}

/// B5 E35: "it attacks a random enemy" (Classic #78 Mutate Spell, twice on its Radiant face) and "this
/// attacks a random enemy Unit" (Classic+ #19.2 Jungle Loser). A forced attack (R53), so position,
/// sickness and Taunt are waived, but each target is drawn (match rng) from the targets the attacker
/// may attack under the unit restrictions as that attack begins — "an enemy" is a unit or the hero,
/// "an enemy Unit" a unit. `times` attacks, each its own combat and state check; the run ends once the
/// attacker is gone or has nothing it may attack (`combat::force_attacks_random`).
pub fn forced_attack_random(args: ForcedAttackRandomArgs) -> Effect {
    Effect::new("forcedAttackRandom", move |ctx| {
        let Some(attacker) = instance_of(ctx, &args.attacker) else {
            return;
        };
        if attacker.zone.z() != ZoneName::Field {
            return;
        }
        let times = args.times.unwrap_or(1).max(0);
        let since = ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state));
        force_attacks_random(
            ctx,
            &attacker,
            args.among.unwrap_or(AttackAmong::Enemies),
            times,
            Some(since),
        );
    })
}

/// `forced_attack_own_hero`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttackOwnHeroArgs {
    pub attacker: TargetSpec,
}

/// B5 E35: "this attacks your hero" (Classic+ #19.5 Bot Loser while Berserk): a forced attack on the
/// unit's own controller's hero, which does not strike back (`combat::force_attack_own_hero`).
pub fn forced_attack_own_hero(args: ForcedAttackOwnHeroArgs) -> Effect {
    Effect::new("forcedAttackOwnHero", move |ctx| {
        let Some(attacker) = instance_of(ctx, &args.attacker) else {
            return;
        };
        if attacker.zone.z() != ZoneName::Field {
            return;
        }
        force_attack_own_hero(ctx, &attacker);
    })
}
