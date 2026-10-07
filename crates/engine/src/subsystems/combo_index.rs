//! Combo-Index (SPEC §8 #93, R27): the grade counter, its end-of-turn threshold and the E→S cascade.
//!
//! The grade is state, not script: it lives in `instance.counters.grade` as 1..6 for E..S (§10.1),
//! so a paused game, a replay and `view_for` all read the same number, and the letter comes from
//! `GRADES` here. At the end of its controller's turn — an ordinary end-of-turn trigger (§10.3,
//! R62) — the card compares the cards that player has played this turn with the current grade: at
//! `cardsPlayed >= grade` the grade rises by one and every step from E up to the new grade runs, in
//! order (R27). Grade S is terminal, so at S the check does nothing at all, and the S step is "run
//! E–A again", which is the one extra round the cascade ever does.
//!
//! The card file (M4) stays a list of effects: `end_of_turn: hook(|ctx| combo_index_end_of_turn(ctx, …))`.
//! Every step is an effect from `crate::effects`; the only direct state change here is the counter
//! itself, because no effect in the library owns `counters.grade`. The radiant text ("Start of
//! turn: add a Combo-Fodder to your hand", §8) is one `add_to_hand` on the card's own `start_of_turn`
//! hook and waits for the catalog id of #93.1, so it lives in the card file, not here.
//!
//! Port of `packages/engine/src/subsystems/comboIndex.ts`. `FIRST_GRADE`, `GRADE_D_CARDS`,
//! `GRADE_D_DISCOUNT` and `GRADE_A_DAMAGE` live in `crate::config` (CLAUDE.md rule 9; part 1 moved
//! them, and `subsystems/mod.rs` re-exports them under TS's `subsystems.X` path); `LAST_GRADE` is
//! derived from `GRADES` here, as TS derived it.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{FIRST_GRADE, GRADE_A_DAMAGE, GRADE_D_CARDS, GRADE_D_DISCOUNT};
use crate::effects::add_to_hand::add_to_hand;
use crate::effects::cost::set_cost_mod;
use crate::effects::damage::damage;
use crate::effects::move_::exile;
use crate::effects::radiant::set_radiant_random;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, PlayerId, Selection, opponent_of};

/// §8 #93: a grade's letter (TS `Grade = (typeof GRADES)[number]`). Written by hand rather than with
/// `wire::string_union!`, which would export a client type the wire does not have (the view carries
/// the letter as a string).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Grade {
    E,
    D,
    C,
    B,
    A,
    S,
}

impl Grade {
    /// The letter itself, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::E => "E",
            Grade::D => "D",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::S => "S",
        }
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// §8 #93: the six grades, in the order the cascade runs them.
pub const GRADES: &[Grade] = &[Grade::E, Grade::D, Grade::C, Grade::B, Grade::A, Grade::S];

/// S: the terminal grade (R27).
pub const LAST_GRADE: i32 = GRADES.len() as i32;

fn clamp_grade(grade: i32) -> i32 {
    grade.clamp(FIRST_GRADE, LAST_GRADE)
}

/// The letter a grade number shows, for the card file and for `view_for` (§10.8).
pub fn grade_name(grade: i32) -> Grade {
    usize::try_from(clamp_grade(grade) - 1)
        .ok()
        .and_then(|at| GRADES.get(at).copied())
        .unwrap_or(Grade::E)
}

/// The number a letter stands for, so a test or a tooltip can say "A" and mean 5.
pub fn grade_value(name: Grade) -> i32 {
    match GRADES.iter().position(|grade| *grade == name) {
        Some(at) => at as i32 + 1,
        None => FIRST_GRADE,
    }
}

/// §10.1: the counter is the whole model, and an instance that never set one is at E.
pub fn grade_of(instance: &CardInstance) -> i32 {
    clamp_grade(instance.counters.grade.unwrap_or(FIRST_GRADE))
}

pub fn grade_name_of(instance: &CardInstance) -> Grade {
    grade_name(grade_of(instance))
}

/// R27: "grade S is terminal", so at S nothing rises and no step runs.
pub fn is_terminal_grade(grade: i32) -> bool {
    clamp_grade(grade) >= LAST_GRADE
}

/// The bookkeeping §10.5 step 4 keeps; `start_turn` clears it, so it is this turn's count alone.
pub fn plays_this_turn(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].turn_log.cards_played
}

/// The cards this player played this turn that a copy can still be made from. `turnLog.playedIds`
/// holds instance ids (§10.1), so a card that has ceased to exist — a unit token that left the
/// field (R11) — leaves nothing to copy.
///
/// R86: such an id drops out of the pool instead of staying in it and making the E step fizzle at
/// random, following R60's shape for a random pick over existing cards ("or all of them if fewer
/// exist"): "a card you played this turn" is the card, not the object that card left behind.
///
/// R133: the log records one entry per play, so a card played, bounced and replayed appears twice.
/// The pool is the *set* of cards played, not the list of plays, so each id is weighed once and a
/// replayed card is one candidate for `rng.pick`. Deduping by id before the lookup keeps R86's half
/// intact — a set of ids cannot resurrect a card that has ceased to exist — and the first play's
/// position is kept, so the pool stays in play order and the pick stays replayable.
pub fn played_cards_this_turn(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out: Vec<&CardInstance> = Vec::new();
    for id in &state.players[player].turn_log.played_ids {
        if seen.contains(&id.as_str()) {
            continue;
        }
        seen.push(id);
        if let Some(card) = find_instance(state, id) {
            out.push(card);
        }
    }
    out
}

/// Whether this end of turn raises the grade: not at S (R27), and only at or above the threshold.
/// "Cards played this turn" is the controller's own `turnLog` (§10.1) — the only per-turn record
/// there is — so a card the opponent cast during this turn counts on their log, not on this one.
pub fn grade_rises(state: &GameState, instance: &CardInstance) -> bool {
    let grade = grade_of(instance);
    if is_terminal_grade(grade) {
        return false;
    }
    plays_this_turn(state, instance.controller) >= grade
}

/// TS wrote `card.counters.grade` through the live object; here the card in the state, by id. The
/// event names the card even when it is nowhere now (TS's object outlived its zone).
fn write_grade(ctx: &mut EffectContext<'_>, card: &CardInstance, next: i32) {
    let value = clamp_grade(next);
    if value == grade_of(card) && card.counters.grade.is_some() {
        return;
    }
    if let Some(live) = find_instance_mut(ctx.sink.state, &card.id) {
        live.counters.grade = Some(value);
    }
    ctx.sink.events.push(GameEvent::CounterChanged {
        instance_id: card.id.clone(),
        counter: CounterKind::Grade,
        value,
        placed: None,
    });
}

/// The card an effect of this module names: the running card as it stands now (TS's live
/// `ctx.self`), or the instance by id.
fn card_of(ctx: &EffectContext<'_>, instance_id: Option<&str>) -> Option<CardInstance> {
    match instance_id {
        None => ctx.live_self().cloned(),
        Some(id) => find_instance(ctx.sink.state, id).cloned(),
    }
}

/// `start_grade`'s argument (TS `{ instanceId? } = {}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartGradeArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// "Grade counter, starts at E" (§8 #93): write the counter as the card arrives, so the client has
/// a grade to show before the first end of turn. Reading a Combo-Index that never ran this still
/// gives E, because `grade_of` defaults to it.
pub fn start_grade(args: StartGradeArgs) -> Effect {
    Effect::new("comboIndexStartGrade", move |ctx| {
        let Some(card) = card_of(ctx, args.instance_id.as_deref()) else {
            return;
        };
        write_grade(ctx, &card, FIRST_GRADE);
    })
}

/// `raise_grade`'s argument (TS `{ instanceId?; to? } = {}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaiseGradeArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
}

/// The one direct state change this subsystem owns: the grade counter itself (§10.1). Nothing else
/// in the effects library writes `counters.grade`, and R27 caps it at S.
pub fn raise_grade(args: RaiseGradeArgs) -> Effect {
    Effect::new("comboIndexRaiseGrade", move |ctx| {
        let Some(card) = card_of(ctx, args.instance_id.as_deref()) else {
            return;
        };
        let grade = grade_of(&card);
        if is_terminal_grade(grade) {
            return;
        }
        write_grade(ctx, &card, args.to.unwrap_or(grade + 1));
    })
}

/// Name one existing card for an effects-library `TargetSpec`. The library resolves `{ of: "chosen" }`
/// against `ctx.targets` (R81), so a random pick this subsystem made is handed over as the selection
/// it would have been if a player had picked it, and the library effect does the work (CLAUDE.md
/// rule 5: nothing here duplicates an effect that already exists).
///
/// TS spread a fresh context with that one target; here the context's own `targets` are swapped for
/// the one selection around the effect's `apply` and restored after.
fn naming(ctx: &mut EffectContext<'_>, card_id: &str, effect: &Effect) {
    let held = std::mem::replace(
        &mut ctx.targets,
        vec![Selection::Instance {
            instance_id: card_id.to_string(),
        }],
    );
    (effect.apply)(ctx);
    ctx.targets = held;
}

/// Grade E: "add a copy of a random card played this turn to your hand". R27 makes it a fresh copy
/// with the radiant flag kept, which is exactly what `add_to_hand` builds (R57: a fresh instance
/// carrying only the radiant flag). The pick happens when the step runs, so the second E of an S
/// cascade re-reads the turn log.
pub fn step_e() -> Effect {
    Effect::new("comboIndexStepE", |ctx| {
        let picked = {
            let played = played_cards_this_turn(ctx.sink.state, ctx.controller);
            ctx.sink
                .rng
                .pick(&played)
                .map(|card| (card.def_id.clone(), card.radiant))
        };
        let Some((def_id, radiant)) = picked else {
            return;
        };
        let copy = add_to_hand(json_as(
            json!({ "defId": def_id, "player": "self", "radiant": radiant }),
        ));
        (copy.apply)(ctx);
    })
}

/// Grade D: "2 different random hand cards cost 1 less". R27 says the two are different and R60
/// gives all of them when fewer exist, which is what a shuffle-and-take does. The discount is a
/// `costMod` on the instance, so it travels with the card between zones (R78) and R65 applies it.
pub fn step_d() -> Effect {
    Effect::new("comboIndexStepD", |ctx| {
        let controller = ctx.controller;
        let picked: Vec<String> = {
            let hand = &ctx.sink.state.players[controller].hand;
            if hand.is_empty() {
                return;
            }
            ctx.sink
                .rng
                .shuffle(hand)
                .into_iter()
                .take(GRADE_D_CARDS.max(0) as usize)
                .map(|card| card.id)
                .collect()
        };
        for id in picked {
            let discount = set_cost_mod(json_as(
                json!({ "target": { "of": "chosen" }, "amount": -GRADE_D_DISCOUNT }),
            ));
            naming(ctx, &id, &discount);
        }
    })
}

/// Grade C: "the opponent exiles a random hand card". Random, so it is never a prompt (§10.6, R16's
/// shape for a discard), and the exile goes through the library effect, counters included (R55).
pub fn step_c() -> Effect {
    Effect::new("comboIndexStepC", |ctx| {
        let opponent = opponent_of(ctx.controller);
        let picked = {
            let hand = &ctx.sink.state.players[opponent].hand;
            ctx.sink.rng.pick(hand).map(|card| card.id.clone())
        };
        let Some(id) = picked else {
            return;
        };
        let banish = exile(json_as(json!({ "target": { "of": "chosen" } })));
        naming(ctx, &id, &banish);
    })
}

/// Grade B: "a random hand card becomes Radiant". R60 narrows the pool to the non-Radiant cards and
/// does nothing when none are left, which `set_radiant_random` already implements.
pub fn step_b() -> Effect {
    set_radiant_random(json_as(json!({ "zones": "hand", "count": 1, "player": "self" })))
}

/// Grade A: "8 damage to the enemy hero with Lifesteal". §4.4 step 8 keys Lifesteal off the source's
/// keywords and #93 prints none, so the effect states it instead (R85): the pipeline heals the
/// controller by the amount that actually landed, after Armor and the Anti-oneshot cap (§4.4 steps 2
/// and 3), and by nothing at all when the hit was reduced to 0 (R63).
pub fn step_a() -> Effect {
    damage(json_as(
        json!({ "to": { "of": "enemyHero" }, "amount": GRADE_A_DAMAGE, "lifesteal": true }),
    ))
}

/// The steps E–A, in cascade order; S is "run E–A again", so it is these five once more.
const STEPS: [fn() -> Effect; 5] = [step_e, step_d, step_c, step_b, step_a];

/// One grade's step (§8 #93). S runs E–A again and never itself, so this recursion is one deep.
pub fn grade_step_effects(grade: i32) -> Vec<Effect> {
    let value = clamp_grade(grade);
    if value == LAST_GRADE {
        return STEPS.iter().map(|step| step()).collect();
    }
    match usize::try_from(value - 1).ok().and_then(|at| STEPS.get(at)) {
        Some(step) => vec![step()],
        None => Vec::new(),
    }
}

/// R27: "steps run E→new grade in order".
pub fn cascade_effects(up_to: i32) -> Vec<Effect> {
    let top = clamp_grade(up_to);
    let mut out: Vec<Effect> = Vec::new();
    for grade in FIRST_GRADE..=top {
        out.extend(grade_step_effects(grade));
    }
    out
}

/// The card's end-of-turn hook (§2.2, R62): nothing at S (R27), nothing below the threshold, and
/// otherwise the rise followed by every step from E up to the new grade. The list is returned rather
/// than applied, so the whole cascade is one effect list to the resolution loop (§10.3, R59) and the
/// card file stays pure (CLAUDE.md rule 5). A pure read of the sink (a card passes its `ctx`).
pub fn combo_index_end_of_turn(sink: &EngineSink<'_>, instance: &CardInstance) -> Vec<Effect> {
    if !grade_rises(sink.state, instance) {
        return Vec::new();
    }
    let next = grade_of(instance) + 1;
    let mut out = vec![raise_grade(RaiseGradeArgs {
        instance_id: Some(instance.id.clone()),
        to: Some(next),
    })];
    out.extend(cascade_effects(next));
    out
}
