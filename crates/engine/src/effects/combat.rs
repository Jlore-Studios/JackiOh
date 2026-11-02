//! Forced attack, Cancel an attack and the AI turn, as card-script verbs (SPEC §6.3's "Forced
//! attack" and "Cancel an attack" rows, §4.2, §4.5, §10.7's AI bullet, R44, R53, R84).
//!
//! Hooks need them (#9 Moths to the Flame from `startOfTurn`, #60 Bear Honeypot and #96 My Pawn from
//! trap triggers), but a hook returns `Vec<Effect>` and has no `EngineSink` (CLAUDE.md rules 4 and 5),
//! so every verb here is a THIN wrapper over the sink-taking `crate::combat` and `play_out_turn`, and
//! nothing here re-implements combat, R53's stop rule or §10.7's policy. `ctx` derefs to its sink.
//!
//! NAMING. The card-facing spellings keep the "d" (`forced_attacks_on`) where `crate::combat` spells
//! them `force_attacks_on`: the effect returns an `Effect`, the engine function takes a sink, and one
//! letter keeps a card file from importing the wrong one.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::destroy::{DestroyArgs, destroy};
use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of, resolve_target};
use crate::combat::{
    AttackAmong, AttackTarget, force_attack_own_hero, force_attacks_on, force_attacks_random, is_enemy_of,
};
use crate::prompts::summoned_so_far;
use crate::restrictions::attack_restriction;
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::state_check::state_check;
use crate::stays::exit_mark;
use crate::subsystems::ai_policy::play_out_turn;
use crate::triggers::{SETTLE_PASS_CAP, SettleSink, dispatch_pending, run_queued_trigger};
use crate::wire::{GameEvent, RedirectWhat, ZoneName, opponent_of};
use crate::work::paused;
use crate::zones::{active_units_of, is_carried};

/// Which side's units are compelled. "any" is both, in R68's order (active side first). (TS
/// `PlayerSpec | "any"`.)
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    out.extend(
        active_units_of(ctx.state, opponent_of(first))
            .into_iter()
            .cloned(),
    );
    out
}

/// `forced_attacks_on`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttacksOnArgs {
    pub target: TargetSpec,
    pub attackers: ForcedSide,
}

/// §6.3 Forced attack: "every enemy unit attacks this" (#9 Moths to the Flame). The named side's
/// active units attack the named target in lane order, and `force_attacks_on` is the whole of R53.
/// A target that resolves to nothing fizzles silently and the card still resolves.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1849007 (Stinkyscape #23 Warp Disruptor): "a random unit from your side attacks it" — one of the
    /// named units, drawn from the match rng among those that may attack the target. Only ever
    /// `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<bool>,
}

/// "summon 2 Rush Tokens; if it was a Unit, they attack it" (#60 Bear Honeypot): "they" is the tokens
/// this effect list just summoned, read back from its `summoned` events (pure, replay-stable). R136: the
/// walk starts at `ctx.events_from` (`resolve::make_context`), so an earlier summon in the same action is
/// not read as this script's own.
fn freshly_summoned(ctx: &EffectContext<'_>) -> IndexSet<String> {
    // A list a prompt split resumes in a later action, whose event list begins after the pause, so
    // what its head summoned comes with the continuation (`prompts::summoned_so_far`, R113).
    summoned_so_far(ctx).into_iter().collect()
}

/// #60 names its target by the instance id its trigger read off the event (R42-style ids).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        let attackers: Vec<CardInstance> =
            attackers_of(ctx, args.attackers.side.unwrap_or(ForcedSide::SelfSide))
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
        // R1849007: one of them at random, drawn only from those that may attack the target — on a
        // carrier, or barred by a restriction (§4.2 step 2), a unit is no candidate — and with none,
        // nothing is drawn.
        let attackers = if args.attackers.random == Some(true) {
            let able: Vec<CardInstance> = attackers
                .into_iter()
                .filter(|unit| {
                    !is_carried(ctx.state, unit)
                        && is_enemy_of(unit, &target)
                        && attack_restriction(ctx.state, unit, &target).is_ok()
                })
                .collect();
            match ctx.sink.rng.pick(&able) {
                Some(unit) => vec![unit.clone()],
                None => return,
            }
        } else {
            attackers
        };

        // R174, R53: the run is the list's, so its stays are the ones the list began with — the tokens
        // it summoned are on them, and a target it took off the field is gone (`force_attacks_on`).
        let since = ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state));
        force_attacks_on(ctx, &attackers, &target, Some(since));
    })
}

/// `cancel_attack`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CancelAttackArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroy_attacker: Option<bool>,
}

/// §6.3 Cancel an attack and R44: inside §4.2 step 4's trap window, mark the open `declaredAttack`
/// canceled (`combat::declare_attack` then skips step 5) and emit `attackCancelled`; the exertion
/// spent declaring is not given back. Outside a window it fizzles, as it does with no `self` for
/// `byInstanceId`, so a trap on a later dispatch cannot cancel a combat already fought. R121: a forced
/// attack opens no window, so this never cancels one (the rule, not an omission).
///
/// `destroyAttacker` (R283, Radiant #96) destroys the attacker as part of the cancel, on the stay it
/// declared from (R174): an Indestructible one is knocked down at the next check (R46), a Reborn one
/// comes back, and only where the cancel happens, so a My Pawn fused onto a My Pawn destroys nothing
/// a second time (R102).
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

/// `redirect_attack`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RedirectAttackArgs {
    /// The instance id the attack is re-aimed at.
    pub to: String,
    /// MD-D20, R1123: once that combat's state check has run, this player gets a fresh copy of each
    /// Unit it destroyed. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_destroyed: Option<bool>,
}

/// §6.3 Redirect, the attack half (MD-D19, R1122): inside §4.2 step 4's trap window, re-aim the open
/// `declaredAttack` at another unit, so §4.3 resolves a combat between allies. The Spell-target half
/// is `targeting_point.rs`'s. Outside a window, or on a canceled declaration, this fizzles, so a trap
/// firing on a later dispatch cannot re-aim a combat already fought.
pub fn redirect_attack(args: RedirectAttackArgs) -> Effect {
    Effect::new("redirectAttack", move |ctx| {
        let Some(by_instance_id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
            return;
        };
        let controller = ctx.controller;
        let Some(open) = ctx.state.declared_attack.as_mut() else {
            return;
        };
        if open.cancelled {
            return;
        }

        let from_id = open.target_id.clone();
        open.target_id = args.to.clone();
        open.redirected = Some(true);
        if args.copy_destroyed == Some(true) {
            open.copies_for = Some(controller);
        }
        ctx.events.push(GameEvent::Redirected {
            what: RedirectWhat::Attack,
            from_id,
            to_id: args.to.clone(),
            by_instance_id: Some(by_instance_id),
        });
    })
}

/// `ai_plays_out_turn`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AiPlaysOutTurnArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_first: Option<bool>,
}

/// R44 and R84: "an AI plays the rest of their turn with random legal actions" (#96 My Pawn). Sets
/// `ai_turn`, R44's lockout (`turn::cleanup` clears it at the end of that turn, R152), and hands the
/// turn to `play_out_turn`, whose uniform draw over `legal_actions` minus `AI_SKIPPED_ACTIONS` IS
/// §10.7's policy; nothing here decides an action. `play_out_turn` adopts a fresh state into the sink
/// field by field, so no later effect may hold a `CardInstance` read before it: #96's list ends here.
///
/// `settleFirst` (R283, R59) settles the board before the AI's first action (Radiant #96): `destroy`
/// only marks (§6.3), and the check collecting the mark would otherwise run inside the playout's first
/// `reduce`, after the AI chose from a board still holding it (a #89 Corpse Eater in the AI's hand eats
/// it before the AI can play it; R212). Off by default.
///
/// One crate calls across its modules directly (SURFACE §6.6); §9.3 and CLAUDE.md rule 4 still ban
/// anything asynchronous inside `reduce`.
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
        play_out_turn(ctx, player, Default::default());
    })
}

/// R1183002 (Meditative+ #18 Cognitive Offloading): "the AI plays the rest of your turn". Engages the
/// caster's autopilot on their own turn only (`subsystems::autopilot::engage`); `reduce` plays that
/// turn by the §10.7 scorer once the engaging action has settled (`reduce::end_due_turns`), never
/// here, so the rest of this list, and of whatever cast it, finishes first.
pub fn autopilot() -> Effect {
    Effect::new("autopilot", move |ctx| {
        let player = ctx.controller;
        crate::subsystems::autopilot::engage(ctx, player);
    })
}

/// R1844001 (Stinkyscape #147 IN33DH3): "for the rest of your turn an AI will help you make the
/// Pareto Optimal decisions". The caster is shown the Pareto hint for the turn running
/// (`PlayerState.pareto_hint`, read by `subsystems::pareto::pareto_hint_ids`) — on their own turn
/// only, so on the opponent's it does nothing — until that turn's cleanup.
pub fn pareto_hint() -> Effect {
    Effect::new("paretoHint", move |ctx| {
        if ctx.sink.state.result.is_some() || ctx.sink.state.active != ctx.controller {
            return;
        }
        let turn = ctx.sink.state.turn;
        ctx.sink.state.players[ctx.controller].pareto_hint = Some(turn);
    })
}

/// R283: what the list before an AI turn has done, settled before the AI acts, as the loop settles
/// between two actions (§10.3, §4.5, R59): the state check, the traps' answers, the triggers it woke.
/// Only the triggers woken here run: what the enclosing action had queued waits for that action's own
/// loop (R117). Stops at a question or a result.
fn settle_before_playout(ctx: &mut EffectContext<'_>) {
    // The same sink; the frontier it shares is the loop's count.
    let settle: &mut SettleSink<'_> = &mut ctx.sink;
    let waiting: IndexSet<String> = settle
        .state
        .trigger_queue
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    for _pass in 0..SETTLE_PASS_CAP {
        state_check(settle);
        if settle.state.result.is_some() || paused(settle) {
            return;
        }
        dispatch_pending(settle);
        if settle.state.result.is_some() || paused(settle) {
            return;
        }
        let Some(at) = settle
            .state
            .trigger_queue
            .iter()
            .position(|entry| !waiting.contains(&entry.id))
        else {
            return;
        };
        let woken = settle.state.trigger_queue.remove(at);
        run_queued_trigger(settle, &woken);
    }
    panic!("the board before the AI turn did not settle in {SETTLE_PASS_CAP} passes (R283)");
}

// B5 E35: forced attacks on "a random enemy" and on the unit's own hero

/// `forced_attack_random`'s arguments (TS's inline object). `among` is TS's `"enemies" | "enemyUnits"`,
/// the type `combat::force_attacks_random` takes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ForcedAttackRandomArgs {
    pub attacker: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub among: Option<AttackAmong>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times: Option<i32>,
}

/// B5 E35: "it attacks a random enemy" (Classic #78 Mutate Spell, twice on its Radiant face) and "this
/// attacks a random enemy Unit" (Classic+ #19.2 Jungle Loser). A forced attack (R53), but each target
/// is drawn (match rng) from those the attacker may attack under the unit restrictions as that attack
/// begins; `times` attacks, each its own combat (`combat::force_attacks_random`).
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// `melee`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MeleeArgs {
    /// The most attacks the melee makes. Absent: `MELEE_MAX_ATTACKS` (Wild System's 20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attacks: Option<i32>,
}

/// R1848005 (Stinkyscape #120 Wild System): "All units attack each other until only one survives" —
/// forced attacks between pairs of Units drawn at random, allies included, each its own combat and
/// state check, until one Unit or none is left, none may attack another, or `max_attacks` have been
/// made (`combat::run_melee`).
pub fn melee(args: MeleeArgs) -> Effect {
    Effect::new("melee", move |ctx| {
        let attacks = args
            .max_attacks
            .unwrap_or(crate::config::MELEE_MAX_ATTACKS)
            .max(0);
        crate::combat::run_melee(ctx, attacks);
    })
}
