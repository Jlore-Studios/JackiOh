//! Delayed effects (SPEC §2.2's turn loop, §10.1, §10.6, R62, R68, R76).
//!
//! "At the start of your next turn …" and "End of turn: …" are not triggers. §2.2 gives them their
//! own two points in the turn — after the mana refresh and before the start-of-turn triggers, and
//! after the end-of-turn trap window and before cleanup — and R62 fixes that order. `state.delayed`
//! is where one waits, `modifiers.scheduleDelayed` puts it there, `modifiers.dueDelayed` picks the
//! ones due in R68's creation order and `turn.runDelayed` re-enters them. All of that already
//! exists; this file is only the missing verb in front of it, because a card script may not write
//! state itself (CLAUDE.md rule 5) and `effects/index.ts` is the whole card-script vocabulary.
//!
//! WHAT IS STORED IS A `Resume`, NEVER A CLOSURE (§9.3, §10.6). `prompts.resumeSelf` builds it out
//! of the running context — this script's def id, the face that is running (§5.2) and the instance
//! when it still exists — plus the data this step captures, so the continuation survives a JSON
//! round-trip and a replay re-enters the same step with the same data. `instanceId` is deliberately
//! optional: R76 has #50 K-Pop Fanatic's steal fire "even if K-Pop Fanatic died", so the continuation
//! must be able to outlive its card, which is why a card carries what it needs in `data` rather
//! than reaching back through `ctx.self`.
//!
//! Port of `packages/engine/src/effects/delay.ts`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::effects::destroy::{DestroyArgs, destroy, destroy_all};
use crate::effects::move_::{DiscardHandArgs, discard_hand};
use crate::effects::targets::{
    BoardScope, PlayerSpec, TargetSpec, cards_in_scope, instance_of, player_of, stands_since_script_began,
};
use crate::marks::{mark_delayed, sync_marks};
use crate::modifiers::{add_start_of_turn_effect, schedule_delayed};
use crate::prompts::{SELF_KEY, resume_self};
use crate::resolve::{HookOptions, make_context};
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{DelayedAt, DelayedEffect, Resume, find_instance, is_turn_of};
use crate::wire::{CardMark, Phase, PlayerId, ZoneName};
use crate::work::RUN_MARKS_KEY;

/// The `Script` key a delayed continuation lands on unless the card names another. `script.ts`
/// documents `delayed` as "a delayed effect this card scheduled, resolved at its R62 point", and it
/// is a `Hook` — a function — which is the only shape `turn.runDelayed` can re-enter today (it goes
/// through `resolve.runHook`, which calls `script[hook]`). A card whose continuation is an entry in
/// its `resume` step table passes `hook: prompts.RESUME_HOOK`; see the gap noted in
/// `test/effects-delay.test.ts`, which `turn.ts` must fix before that spelling works.
pub const DELAYED_HOOK: &str = "delayed";

/// R350: `DelayAt.player` for the turn that is running, whoever's it is.
pub const THIS_TURN: &str = "turn";

/// R1140: the `ctx.data` key under which a delayed step that watches hand cards (`DelayArgs.handWatch`)
/// finds the ones still watched as it runs, in the order they were picked (`due_resume`).
pub const HAND_WATCH_KEY: &str = "handWatch";

/// `DelayAt.player`: a `PlayerSpec`, relative to the controller like every other, or `"turn"`
/// (`THIS_TURN`): the player whose turn is running as the effect is made.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DelayPlayer {
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
    #[serde(rename = "turn")]
    Turn,
}

impl DelayPlayer {
    /// The `PlayerSpec` this names, or `None` for `"turn"`.
    fn spec(self) -> Option<PlayerSpec> {
        match self {
            DelayPlayer::SelfSide => Some(PlayerSpec::SelfSide),
            DelayPlayer::Enemy => Some(PlayerSpec::Enemy),
            DelayPlayer::Turn => None,
        }
    }
}

/// When a delayed effect comes due, in the vocabulary a card file writes: §2.2's two points. The
/// player is relative to the controller like every other `PlayerSpec`, or `"turn"`: the player whose
/// turn is running as the effect is made — R350's "at the end of this turn", whoever's turn that is
/// (#90.1 CN-Virus, cast on a draw that may come on either player's turn).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DelayAt {
    /// Only `Phase::Start` or `Phase::End` (TS `DelayedEffect["at"]["phase"]`).
    pub phase: Phase,
    pub player: DelayPlayer,
}

/// The player a `DelayAt` waits for. During setup, which is no player's turn, "turn" is p1's first (§2.1).
fn delay_player(ctx: &EffectContext<'_>, at: &DelayAt) -> PlayerId {
    match at.player.spec() {
        None => ctx.sink.state.active,
        Some(spec) => player_of(ctx, spec),
    }
}

/// `delay`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DelayArgs {
    pub at: DelayAt,
    /// The step the continuation re-enters (§10.6: "script id + step + captured data").
    pub step: String,
    /// The `Script` key that step lives under; `DELAYED_HOOK` unless the card says otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    /// What the continuation carries across the boundary — the only place it may keep anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
    /// R174: the card on the field this effect is aimed at, if any. The effect is forgotten the moment
    /// that card leaves the field, so it never lands on a card that left and came back (#50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<String>,
    /// B5 E27, R458: "at the start / end of your *next* turn" rather than the next such boundary — the
    /// boundary of the turn it is made on passes it by (`DelayedEffect.notBefore`), so an end-of-turn
    /// clause made on the controller's own turn waits for the end of their next one (Classic #37
    /// Radiant). R241 does not drop one made on the other player's turn: that player's next turn exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<bool>,
    /// R437: the mark the watched card carries in both views while this effect waits (#50's pending
    /// steal, `{ mark: "steal", color: "purple" }`). Ignored without `watch` or `handWatch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<CardMark>,
    /// R1140 (ME-HANDMARK, Meditative #76 Do or Die): hand cards this effect is aimed at, by instance id.
    /// Each is watched for its stay in that hand: it is dropped as it leaves (played, discarded,
    /// shuffled away, stolen, R174), the effect with it once none is left, and a card that comes back is
    /// a new stay nobody watches. Those of them not in a hand now are left out, and with none left
    /// nothing is scheduled. `mark` marks each (R1141). The step finds the ones still watched under
    /// `HAND_WATCH_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_watch: Option<Vec<String>>,
}

/// §6.2's "at the start of your next turn" and "end of turn" as one verb (#39 Recycling Initiative,
/// #50 K-Pop Fanatic, #78 /fullsend). `at.player` says whose turn boundary it waits for, relative to
/// the controller like every other `PlayerSpec`; `owner` is the controller, which is both who the
/// continuation runs as and, through R68's creation `seq`, where it sits among several due at once.
///
/// Scheduling never fizzles: a delay can be armed by a card that is about to exile or sacrifice
/// itself in the same effect list, which is exactly what #39 does, and the entry stays whatever
/// happens to the card afterwards (R76, R86).
pub fn delay(args: DelayArgs) -> Effect {
    Effect::new("delay", move |ctx| {
        // R174, R76: an effect aimed at a card on the field is aimed at that stay. A target an earlier
        // effect of the same list has already taken off the field — a fused card's other part bounced
        // it (#52) or sacrificed it (#22) — has no stay left to watch, so the delayed effect fizzles
        // now rather than waiting for whatever later stands under the same id (R78, R83).
        if let Some(watch) = &args.watch
            && !stands_since_script_began(ctx, watch)
        {
            return;
        }
        // R241: "End of turn" is the controller's turn end (§6.2), and an end-of-turn clause is the
        // turn's it was made on (#39, #78). One made on the other player's turn — a cast on draw there
        // (R70) — has no end of its controller's turn to wait for, and waiting for the next one would
        // run it at the end of a turn the card was never played on, as R155 keeps a return Spell cast
        // then in the graveyard: it is not scheduled at all.
        if args.next != Some(true) && ends_other_players_turn(ctx, &args.at) {
            return;
        }
        // R1140: a hand watch is aimed at cards in a hand now; with none of them there, nothing waits.
        let hand_watch: Option<Vec<String>> = args.hand_watch.as_ref().map(|ids| {
            ids.iter()
                .filter(|id| find_instance(ctx.state, id).is_some_and(|card| card.zone.z() == ZoneName::Hand))
                .cloned()
                .collect()
        });
        if hand_watch.as_ref().is_some_and(Vec::is_empty) {
            return;
        }
        // `resumeSelf` is the one builder for the def id, the face and the instance id, so a delay
        // and a prompt store the same shape; only the hook differs, and only when a card says so.
        let resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(DELAYED_HOOK),
        );
        let at = DelayedAt {
            phase: args.at.phase,
            player: delay_player(ctx, &args.at),
        };
        let not_before = if args.next == Some(true) {
            Some(next_turn_mark(ctx))
        } else {
            None
        };
        let controller = ctx.controller;
        let entry = schedule_delayed(ctx, controller, at, resume, args.watch.clone(), not_before);
        if let Some(ids) = &hand_watch
            && let Some(stored) = ctx.state.delayed.iter_mut().find(|due| due.id == entry.id)
        {
            stored.hand_watch = Some(ids.clone());
        }
        // R437: the card it waits for carries the mark until it resolves, fizzles or is forgotten; R1141:
        // each hand card it watches, while it is watched.
        if let Some(mark) = &args.mark {
            match &hand_watch {
                Some(ids) => sync_marks(ctx, &entry.id, mark, ids),
                None => mark_delayed(ctx, &entry, mark),
            }
        }
    })
}

/// R1140: the continuation a due entry re-enters — its stored `Resume`, and for a hand watch the cards
/// still watched under `HAND_WATCH_KEY`, read off the entry as it runs, so a card that left the hand
/// while it waited is not among them.
pub fn due_resume(entry: &DelayedEffect) -> Resume {
    let mut resume = entry.resume.clone();
    if let Some(ids) = &entry.hand_watch {
        resume
            .data
            .insert(HAND_WATCH_KEY.to_string(), Value::from(ids.clone()));
    }
    resume
}

/// The continuation a delayed effect stores: this script's step (`prompts.resumeSelf`) as a run of its
/// own. R127: it re-enters as whatever is left of its card then, so a Death hook's snapshot (R89) is
/// not carried past the hook that read it; nor does it carry the run it was made in
/// (`work.RUN_MARKS_KEY`): it resolves at its own R62 point, and a card it watches is watched through
/// `watch` (R174).
fn delayed_resume(ctx: &EffectContext<'_>, step: &str, data: IndexMap<String, Value>, hook: &str) -> Resume {
    let mut built = resume_self(ctx, step, data);
    built.data.shift_remove(SELF_KEY);
    built.data.shift_remove(RUN_MARKS_KEY);
    Resume {
        hook: hook.to_string(),
        ..built
    }
}

/// R458: the first turn a "next turn" clause may run on — any turn after the one it is made on.
fn next_turn_mark(ctx: &EffectContext<'_>) -> i32 {
    ctx.sink.state.turn + 1
}

/// R241: an end-of-turn clause of the controller's own, made on a turn that is not the controller's —
/// the other player's, or setup's, which is no player's turn though `active` names p1 there (§2.1).
fn ends_other_players_turn(ctx: &EffectContext<'_>, at: &DelayAt) -> bool {
    // R350: "this turn" is the turn running, whoever's it is, so there is no other player's turn to miss.
    let Some(spec) = at.player.spec() else {
        return false;
    };
    at.phase == Phase::End
        && player_of(ctx, spec) == ctx.controller
        && !is_turn_of(ctx.sink.state, ctx.controller)
}

// ---------------------------------------------------------------------------
// ---- v0.2.0 verbs: activate and turn (B5 E27 delayed kinds, E28 rest of the game) ----
// ---------------------------------------------------------------------------

/// B5 E27: the delayed effects the engine itself resolves, by the `Resume.hook` they are stored under —
/// a verb, not a card's step, so any card can say "destroyed at the start of your next turn" without a
/// `delayed` hook of its own. `turn.ts`'s delayed stage runs these (`runEngineDelayed`) and re-enters a
/// card's step for every other entry (R126).
pub const DELAYED_DESTROY_HOOK: &str = "@delayedDestroy";
pub const DELAYED_DISCARD_HAND_HOOK: &str = "@delayedDiscardHand";

/// A card's def id for the record an engine delayed effect keeps: the text that made it, if any — the
/// running `ctx.defId` first, as `prompts.resumeSelf` names it (B5 E14, R546). `(defId, radiant)`.
fn maker_of(ctx: &EffectContext<'_>) -> (String, bool) {
    let def_id = ctx
        .def_id
        .clone()
        .or_else(|| ctx.self_.as_ref().map(|card| card.def_id.clone()))
        .unwrap_or_default();
    (def_id, ctx.radiant)
}

/// `destroyAtNextTurnStart`'s argument: one unit by a `TargetSpec`, or a board scope read when it
/// resolves (TS `{ target; mark? } | { scope; mark? }`, told apart by `"scope" in args`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum DestroyAtNextTurnStartArgs {
    Scope {
        scope: BoardScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mark: Option<CardMark>,
    },
    Target {
        target: TargetSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mark: Option<CardMark>,
    },
}

/// B5 E27, R458, R174 (Classic #20 The Power to Punish): the unit `target` names is destroyed at the
/// start of its controller's next turn — a destroy (§6.3), so Indestructible ignores it (R46) and the
/// state check after the delayed effect collects it (R59). Aimed at that unit's stay on the field: it
/// fizzles the moment the unit leaves the field, whatever comes back under its id (`watch`, R76, R83).
/// With no unit on the field to aim at, nothing is scheduled.
///
/// `scope` instead is "all enemy Units are destroyed at the start of your next turn" (the Radiant face):
/// the Units the scope names *then*, read as the delayed effect resolves, not a list fixed now — sides
/// relative to the controller who made it. R750: its `mark` goes on every Unit the scope names while it
/// waits, those that arrive meanwhile too (`refreshScopeMarks`).
pub fn destroy_at_next_turn_start(args: DestroyAtNextTurnStartArgs) -> Effect {
    Effect::new("destroyAtNextTurnStart", move |ctx| {
        let (def_id, radiant) = maker_of(ctx);
        let controller = ctx.controller;
        let at = DelayedAt {
            phase: Phase::Start,
            player: controller,
        };
        match &args {
            DestroyAtNextTurnStartArgs::Scope { scope, mark } => {
                let mut data: IndexMap<String, Value> = IndexMap::new();
                data.insert(
                    "scope".to_string(),
                    serde_json::to_value(scope).expect("a board scope is JSON"),
                );
                if let Some(mark) = mark {
                    data.insert(
                        "mark".to_string(),
                        serde_json::to_value(mark).expect("a card mark is JSON"),
                    );
                }
                let resume = Resume {
                    def_id,
                    hook: DELAYED_DESTROY_HOOK.to_string(),
                    step: "scope".to_string(),
                    radiant,
                    instance_id: None,
                    data,
                };
                schedule_delayed(ctx, controller, at, resume, None, None);
            }
            DestroyAtNextTurnStartArgs::Target { target, mark } => {
                let Some(unit) = instance_of(ctx, target) else {
                    return;
                };
                if unit.zone.z() != ZoneName::Field {
                    return;
                }
                let mut data: IndexMap<String, Value> = IndexMap::new();
                data.insert("instanceId".to_string(), Value::String(unit.id.clone()));
                let resume = Resume {
                    def_id,
                    hook: DELAYED_DESTROY_HOOK.to_string(),
                    step: "unit".to_string(),
                    radiant,
                    instance_id: None,
                    data,
                };
                let entry = schedule_delayed(ctx, controller, at, resume, Some(unit.id.clone()), None);
                // R437: the watched unit carries the mark while the destroy waits (Classic #20's red aura).
                if let Some(mark) = mark {
                    mark_delayed(ctx, &entry, mark);
                }
            }
        }
    })
}

/// `discardHandAtTurnEnd`'s `turn`: the end of this turn, or of the controller's next one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum DiscardHandTurn {
    This,
    Next,
}

impl DiscardHandTurn {
    /// The literal, which the stored continuation names as its step.
    fn as_str(self) -> &'static str {
        match self {
            DiscardHandTurn::This => "this",
            DiscardHandTurn::Next => "next",
        }
    }
}

/// `discardHandAtTurnEnd`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscardHandAtTurnEndArgs {
    pub turn: DiscardHandTurn,
}

/// B5 E27, R458 (Classic #37 Last Hurrah): the controller discards their whole hand at the end of
/// `this` turn — whoever's turn is running, as R350 reads "this turn" — or at the end of their `next`
/// turn (Radiant), which the end of the turn it is made on passes by. A discard (§6.3), so "whenever
/// you discard" sees each card.
pub fn discard_hand_at_turn_end(args: DiscardHandAtTurnEndArgs) -> Effect {
    Effect::new("discardHandAtTurnEnd", move |ctx| {
        let (def_id, radiant) = maker_of(ctx);
        let resume = Resume {
            def_id,
            hook: DELAYED_DISCARD_HAND_HOOK.to_string(),
            step: args.turn.as_str().to_string(),
            radiant,
            instance_id: None,
            data: IndexMap::new(),
        };
        let controller = ctx.controller;
        if args.turn == DiscardHandTurn::This {
            let at = DelayedAt {
                phase: Phase::End,
                player: ctx.sink.state.active,
            };
            schedule_delayed(ctx, controller, at, resume, None, None);
            return;
        }
        let at = DelayedAt {
            phase: Phase::End,
            player: controller,
        };
        let not_before = next_turn_mark(ctx);
        schedule_delayed(ctx, controller, at, resume, None, Some(not_before));
    })
}

fn scope_in(data: &IndexMap<String, Value>) -> Option<BoardScope> {
    match data.get("scope") {
        Some(raw @ Value::Object(_)) => serde_json::from_value::<BoardScope>(raw.clone()).ok(),
        _ => None,
    }
}

/// R750: the mark a delayed destroy of a scope puts on the Units it names, if it was made with one.
fn mark_in(data: &IndexMap<String, Value>) -> Option<CardMark> {
    let Some(Value::Object(raw)) = data.get("mark") else {
        return None;
    };
    match (raw.get("mark"), raw.get("color")) {
        (Some(Value::String(mark)), Some(Value::String(color))) => Some(CardMark {
            mark: mark.clone(),
            color: color.clone(),
        }),
        _ => None,
    }
}

/// R750, R437: every waiting delayed destroy of a scope made with a mark marks the Units its scope names
/// now — the read `runEngineDelayed` makes as it resolves — so a Unit that arrives while it waits is
/// marked and one that leaves is not. Called where the resolution loop collects events
/// (`triggers.collectEvents`), before `marks.sweepMarks` drops the marks of an effect that is gone.
pub fn refresh_scope_marks(sink: &mut EngineSink<'_>) {
    // Read off the list first: neither the context nor `syncMarks` touches `state.delayed`.
    let waiting: Vec<(String, PlayerId, BoardScope, CardMark)> = sink
        .state
        .delayed
        .iter()
        .filter(|entry| entry.resume.hook == DELAYED_DESTROY_HOOK)
        .filter_map(|entry| {
            let scope = scope_in(&entry.resume.data)?;
            let mark = mark_in(&entry.resume.data)?;
            Some((entry.id.clone(), entry.owner, scope, mark))
        })
        .collect();
    for (id, owner, scope, mark) in waiting {
        let ids: Vec<String> = {
            let ctx = make_context(
                sink,
                None,
                HookOptions {
                    controller: Some(owner),
                    ..HookOptions::default()
                },
            );
            cards_in_scope(&ctx, &scope)
                .iter()
                .map(|card| card.id.clone())
                .collect()
        };
        sync_marks(sink, &id, &mark, &ids);
    }
}

/// Run one of the engine's own delayed kinds, as its owner. Returns false for an entry that is a card's
/// step, which the caller re-enters through the card's script instead (R126).
pub fn run_engine_delayed(sink: &mut EngineSink<'_>, effect: &DelayedEffect) -> bool {
    let mut ctx = make_context(
        sink,
        None,
        HookOptions {
            controller: Some(effect.owner),
            ..HookOptions::default()
        },
    );
    let data = &effect.resume.data;
    match effect.resume.hook.as_str() {
        DELAYED_DESTROY_HOOK => {
            if let Some(scope) = scope_in(data) {
                (destroy_all(scope).apply)(&mut ctx);
            } else if let Some(instance_id) = data.get("instanceId").and_then(Value::as_str) {
                let target = TargetSpec::Instance {
                    instance_id: instance_id.to_string(),
                };
                (destroy(DestroyArgs { target }).apply)(&mut ctx);
            }
            true
        }
        DELAYED_DISCARD_HAND_HOOK => {
            let discard = discard_hand(DiscardHandArgs {
                player: Some(PlayerSpec::SelfSide),
            });
            (discard.apply)(&mut ctx);
            true
        }
        _ => false,
    }
}

/// `forRestOfGame`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForRestOfGameArgs {
    pub step: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// B5 E28, R458 (Classic+ #52): "For the rest of the game: at the start of your turn, …". The card's
/// `step` (under `hook`, `DELAYED_HOOK` unless the card names another) is re-entered at the start of
/// each of `player`'s turns from the next one on, in R62's delayed stage with the delayed effects, in
/// creation order; several stack, each running once per turn. It is the player's effect now, not the
/// card's: it resolves with no `self` whatever became of the card (R127), under the face it was made
/// with, carrying `data`. `label` is its badge (R169), in the card's own words.
pub fn for_rest_of_game(args: ForRestOfGameArgs) -> Effect {
    Effect::new("forRestOfGame", move |ctx| {
        let mut resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(DELAYED_HOOK),
        );
        resume.instance_id = None;
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_start_of_turn_effect(ctx, player, resume, &args.label);
    })
}
