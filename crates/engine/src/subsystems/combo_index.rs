//! Combo-Index (SPEC §8 #93, R27): the grade counter, its end-of-turn threshold and the E→S cascade.
//!
//! The grade is state, not script: `instance.counters.grade`, 1..6 for E..S (§10.1), so a paused game, a
//! replay and `view_for` read the same number. At the end of its controller's turn (§10.3, R62), at
//! `cardsPlayed >= grade` the grade rises by one and every step from E up to the new grade runs, in
//! order (R27); S is terminal, and its step is "run E–A again". Every step is an effect from
//! `crate::effects`; the numbers live in `crate::config` (CLAUDE.md rule 9).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{FIRST_GRADE, GRADE_A_DAMAGE, GRADE_D_CARDS, GRADE_D_DISCOUNT};
use crate::effects::add_to_hand::add_to_hand;
use crate::effects::cost::set_cost_mod;
use crate::effects::damage::damage;
use crate::effects::move_::exile;
use crate::effects::radiant::set_radiant_random;
use crate::prelude::json_as;
use crate::query::cards_played_this_turn;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, PlayerId, Selection, opponent_of};

/// §8 #93: a grade's letter, by hand: `wire::string_union!` would export a client type the wire does
/// not have (the view carries the letter as a string).
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

/// The number a letter stands for.
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

/// The cards this player played this turn that a copy can still be made from: a card that has ceased
/// to exist (a unit token that left the field, R11) is dropped rather than making the E step fizzle at
/// random (R86, in R60's shape). R133: deduped by id in first-play order, so a replayed card is one
/// candidate and the pick replays.
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
/// "Cards played this turn" is the controller's own `turnLog`, so a card the opponent cast during this
/// turn counts on their log, not on this one.
pub fn grade_rises(state: &GameState, instance: &CardInstance) -> bool {
    let grade = grade_of(instance);
    if is_terminal_grade(grade) {
        return false;
    }
    cards_played_this_turn(state, instance.controller) >= grade
}

/// Writes the grade on the card in the state, by id. The event names the card even when it is nowhere now.
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

/// The card an effect of this module names: the running card as it stands now, or the instance by id.
fn card_of(ctx: &EffectContext<'_>, instance_id: Option<&str>) -> Option<CardInstance> {
    match instance_id {
        None => ctx.live_self().cloned(),
        Some(id) => find_instance(ctx.sink.state, id).cloned(),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartGradeArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// "Grade counter, starts at E" (§8 #93): write the counter as the card arrives, so the client has
/// a grade to show before the first end of turn.
pub fn start_grade(args: StartGradeArgs) -> Effect {
    Effect::new("comboIndexStartGrade", move |ctx| {
        let Some(card) = card_of(ctx, args.instance_id.as_deref()) else {
            return;
        };
        write_grade(ctx, &card, FIRST_GRADE);
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaiseGradeArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
}

/// The one direct state change this subsystem owns, the grade counter (§10.1); R27 caps it at S.
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

/// Hands a random pick to a library effect as the `{ of: "chosen" }` selection a player would have
/// made (R81), so the library effect does the work (CLAUDE.md rule 5).
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

/// Grade E: "add a copy of a random card played this turn to your hand": a fresh copy with the radiant
/// flag kept (R27, R57), picked as the step runs, so the second E of an S cascade re-reads the turn log.
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

/// Grade D: "2 different random hand cards cost 1 less": different (R27), all of them when fewer exist
/// (R60). The discount is a `costMod` on the instance, so it travels with the card (R78, R766) and R65
/// applies it.
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

/// Grade B: "a random hand card becomes Radiant": a non-Radiant one, none when none is left (R60).
pub fn step_b() -> Effect {
    set_radiant_random(json_as(json!({ "zones": "hand", "count": 1, "player": "self" })))
}

/// Grade A: "8 damage to the enemy hero with Lifesteal". §4.4 step 8 keys Lifesteal off the source's
/// keywords and #93 prints none, so the effect states it (R85): the controller heals by what landed
/// after Armor and the Anti-oneshot cap (§4.4 steps 2 and 3), nothing when it was 0 (R63).
pub fn step_a() -> Effect {
    damage(json_as(
        json!({ "to": { "of": "enemyHero" }, "amount": GRADE_A_DAMAGE, "lifesteal": true }),
    ))
}

/// The steps E–A, in cascade order.
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

/// The card's end-of-turn hook (§2.2, R62): the rise and every step from E up to the new grade, returned
/// rather than applied, so the cascade is one effect list to the resolution loop (§10.3, R59) and the
/// card file stays pure (CLAUDE.md rule 5).
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
