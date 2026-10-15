//! The AI's one entry point (SPEC §9.9). The order below is the contract:
//!
//!   1. ai_to_act / redact — the only two reads of the true state (R185).
//!   2. an unanswered draw offer is declined at once (R188).
//!   3. the mulligan keeps the cheap cards.
//!   4. a single candidate is played with no search: listed on a throwaway determinization, since
//!      the seat's own legal actions never depend on the hidden cards, so `options.rng` is untouched.
//!   5. the exact lethal solver, verified on every determinization (a style may pass over it, and
//!      then step 6 keeps off every line that wins this turn, R1452).
//!   6. the beam on determinization 0; its best lines are scored after the opponent's reply
//!      (reply.rs), the best first actions are re-scored the same way on the other determinizations,
//!      and the best mean wins (a style with a temperature picks among them, R1452).
//!   7. a fallback when nothing could be scored.
//!
//! Only the first action of the chosen line is played; the caller asks again after it, so the AI
//! re-plans after every action and a plan that only one sampled world liked never gets past step one.
//! `decide` never panics out: every internal failure becomes a fallback and is counted in simErrors.
//! `decide_styled` is the same with an `AiStyle` (R1452): `decide` is it at `AI_STYLE_EXACT`, which
//! takes no draw of its own, so `decide`'s decisions and cursor are what they were before styles.
//!
//! Port of `packages/ai/src/decide.ts` (SURFACE §9). TS's `try`/`catch` is `catch_unwind` (an engine
//! invariant is a panic in Rust, SURFACE §4.4.9). TS made the node counter after the determinizations;
//! here it is made first, so the counter outlives the guarded body. Nothing differs: a counter is only
//! read through `take()`, which nothing calls before the lethal solver, so its stats are the same.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexMap;
use jackioh_engine::{
    Action, ActionBody, CardInstance, GameState, PLAYER_IDS, PlayerId, Rng, active_units_of,
    mulligan_prompt_for, reduce,
};

use crate::candidates::candidate_actions;
use crate::config::{AI_EVAL, AI_MULLIGAN, AI_MULLIGAN_CURVE, AI_REPLY, AI_SEARCH, AI_STYLE_EXACT};
use crate::determinize::{DeterminizeOptions, det_ground, determinize_on};
use crate::lethal::find_lethal;
use crate::match_::replacements_for;
use crate::mulligan::mulligan_keep_curved;
use crate::observe::{HIDDEN_DEF_ID, ai_to_act, redact, unanswered_draw_offer};
use crate::reply::{hidden_card_ids, reply_score};
use crate::search::{Line, beam_search, score_line};
use crate::simulate::{CountingNodeCounter, create_node_counter, create_sub_counter};
use crate::types::{
    AiOptions, AiStyle, Decision, DecisionReason, NodeCounter, SearchBudget, SearchStats, StoppedBy,
};

fn quiet_stats() -> SearchStats {
    SearchStats {
        nodes: 0,
        determinizations: 0,
        lines: 0,
        sim_errors: 0,
        stopped_by: StoppedBy::Exhausted,
        score: 0.0,
    }
}

fn immediate(action: ActionBody, reason: DecisionReason) -> Decision {
    Decision {
        line: vec![action.clone()],
        action,
        reason,
        stats: quiet_stats(),
    }
}

/// Step 7: endTurn when it is a candidate, else the first candidate, else endTurn regardless.
fn fallback_action(candidates: &[ActionBody]) -> ActionBody {
    candidates
        .iter()
        .find(|action| matches!(action, ActionBody::EndTurn))
        .or_else(|| candidates.first())
        .cloned()
        .unwrap_or(ActionBody::EndTurn)
}

/// Lines in order, at most `perAction` for any one first action, `count` in all.
fn shortlist_lines<'a>(found: &'a [Line], count: usize) -> Vec<&'a Line> {
    let mut taken: IndexMap<&ActionBody, i32> = IndexMap::new();
    let mut out: Vec<&'a Line> = Vec::new();
    for line in found {
        let Some(first) = line.actions.first() else {
            continue;
        };
        let already = taken.get(first).copied().unwrap_or(0);
        if already >= AI_SEARCH.lines_per_action {
            continue;
        }
        taken.insert(first, already + 1);
        out.push(line);
        if out.len() >= count {
            break;
        }
    }
    out
}

/// One line and the score it was given.
#[derive(Clone, Copy)]
struct Scored<'a> {
    line: &'a Line,
    score: f64,
}

/// The best-scoring line for each distinct first action, best first, at most `count` of them.
fn best_per_first_action<'a>(scored: &[Scored<'a>], count: usize) -> Vec<Scored<'a>> {
    let mut best: IndexMap<&'a ActionBody, Scored<'a>> = IndexMap::new();
    for entry in scored {
        let Some(first) = entry.line.actions.first() else {
            continue;
        };
        match best.get_mut(first) {
            None => {
                best.insert(first, *entry);
            }
            Some(held) => {
                if entry.score > held.score {
                    *held = *entry;
                }
            }
        }
    }
    // The map keeps first-seen order (TS's `order` list).
    let mut entries: Vec<Scored<'a>> = best.into_values().collect();
    // Stable, so equal scores keep the beam's order.
    entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    entries.truncate(count);
    entries
}

/// A decision's width (AI_SEARCH.fat_*): the more of its root candidates, or the more cards held and
/// in play across both sides, the more every `reduce` and `legal_actions` under it costs. A prompt's
/// short list of answers hides the state it answers into, so the cards count beside the candidates.
fn fat_width(probe: &GameState, candidates: usize) -> usize {
    let mut size = 0usize;
    for player in PLAYER_IDS {
        let side = &probe.players[player];
        size += side.hand.len();
        size += active_units_of(probe, player).len();
        size += side.backrow.iter().flatten().count();
    }
    candidates.max(size)
}

/// The budget a decision of `fat` width spends (AI_SEARCH.fat_*): `nodes` in inverse proportion to
/// the width past fat_reference, floored at fat_min_nodes; the lethal solver keeps its share of it,
/// floored at fat_min_lethal. The search's shape (determinizations, widths, depths, finalists) is
/// the caller's unchanged.
fn scaled_budget(budget: SearchBudget, fat: usize) -> SearchBudget {
    let reference = AI_SEARCH.fat_reference.max(1) as usize;
    let nodes = if fat <= reference {
        budget.nodes
    } else {
        (budget.nodes * reference / fat).max(AI_SEARCH.fat_min_nodes.max(0) as usize)
    };
    // A budget that gives the lethal solver nothing (lethal_nodes 0) is a caller that does not
    // want it run at all, not one to be floored up to fat_min_lethal.
    let lethal_nodes = if budget.lethal_nodes == 0 {
        0
    } else {
        (budget.lethal_nodes * nodes / budget.nodes.max(1))
            .max(AI_SEARCH.fat_min_lethal.max(0) as usize)
            .min(nodes)
    };
    SearchBudget {
        nodes,
        lethal_nodes,
        ..budget
    }
}

/// R1452: whether this decision passes over the lethal line the solver found. A style that never
/// misses takes no draw.
fn passes_over_lethal(style: &AiStyle, rng: &mut Rng) -> bool {
    style.missed_lethal_chance > 0.0 && rng.chance(style.missed_lethal_chance)
}

/// R1452: the beam's lines a decision that passed over lethal may still play. The beam scores a won
/// game far above anything else, so left alone it would find the same lethal the solver did; a
/// missed lethal keeps off every line that wins on the determinization it was searched on and every
/// line that opens with the solver's first lethal action. When that leaves nothing, every line.
fn lines_missing_lethal(found: Vec<Line>, first_lethal: &ActionBody, seat: PlayerId) -> Vec<Line> {
    let kept: Vec<Line> = found
        .iter()
        .filter(|line| {
            let wins = line
                .end
                .result
                .as_ref()
                .is_some_and(|result| result.winner.player() == Some(seat));
            let opens_lethal = line.actions.first().is_some_and(|action| action == first_lethal);
            !wins && !opens_lethal
        })
        .cloned()
        .collect();
    if kept.is_empty() { found } else { kept }
}

/// R1452: the finalist a decision plays, given each finalist's mean score. At a temperature of 0
/// or below, or with fewer than two finalists, the first best, with no draw. Otherwise a softmax
/// over the means: each weighs `e^((mean − best) / temperature)`, and one draw lands on the first
/// finalist whose running sum of weights passes it.
pub fn pick_finalist(means: &[f64], temperature: f64, rng: &mut Rng) -> usize {
    let mut best_index = 0usize;
    for (index, mean) in means.iter().enumerate() {
        if means.get(best_index).is_some_and(|best| mean > best) {
            best_index = index;
        }
    }
    if temperature <= 0.0 || means.len() < 2 {
        return best_index;
    }
    let best = means.get(best_index).copied().unwrap_or(0.0);
    let weights: Vec<f64> = means
        .iter()
        .map(|mean| ((mean - best) / temperature).exp())
        .collect();
    let draw = rng.next() * weights.iter().sum::<f64>();
    let mut running = 0.0;
    for (index, weight) in weights.iter().enumerate() {
        running += weight;
        if running > draw {
            return index;
        }
    }
    best_index
}

/// What the guarded body decided, before the stats are read off the counter.
enum Plan {
    /// Answered at once, with quiet stats.
    Immediate(ActionBody, DecisionReason),
    /// Searched: the stats are the counter's, with this score and, when given, this stop reason.
    Searched {
        action: ActionBody,
        reason: DecisionReason,
        line: Vec<ActionBody>,
        score: f64,
        stopped_by: Option<StoppedBy>,
    },
}

/// What the guarded body leaves behind for the fallback when it panics (TS's outer `let`s).
struct Scratch {
    candidates: Vec<ActionBody>,
    determinizations: usize,
    lines: usize,
}

/// Steps 1–7 on the redacted copy; every number `stats` reads beside the counter goes in `scratch`.
fn plan(
    state: &GameState,
    seat: PlayerId,
    budget: SearchBudget,
    style: &AiStyle,
    rng: &mut Rng,
    counter: &dyn NodeCounter,
    scratch: &mut Scratch,
) -> Plan {
    // 1. Everything below reads the redacted copy.
    let public = redact(state, seat);

    // 2. R188.
    if unanswered_draw_offer(&public, seat) {
        return Plan::Immediate(
            ActionBody::AnswerDraw { accept: false },
            DecisionReason::DrawOffer,
        );
    }

    // 3. The mulligan: its own, at once, whether or not the other seat has answered (R265).
    if public.pending.is_none() && mulligan_prompt_for(&public, seat).is_some() {
        let keep = mulligan_keep_curved(&public, seat, AI_MULLIGAN.keep_max_cost, &AI_MULLIGAN_CURVE);
        return Plan::Immediate(ActionBody::Mulligan { keep }, DecisionReason::Mulligan);
    }

    // 4. Forced: a throwaway world lists the candidates without touching options.rng. Every
    // determinization of this decision starts from the same public state, so the read-only part
    // of the determinization (the pools, what the opponent has shown, each side's face-down
    // grants) is computed once (det_ground) and handed to each.
    let ground = det_ground(&public, seat);
    let probe = determinize_on(
        &public,
        seat,
        &mut Rng::new(AI_SEARCH.probe_seed, 0),
        DeterminizeOptions::default(),
        &ground,
    );
    scratch.candidates = candidate_actions(&probe, seat);
    if scratch.candidates.len() == 1
        && let Some(only) = scratch.candidates.first()
    {
        return Plan::Immediate(only.clone(), DecisionReason::Forced);
    }

    // A decision's nodes cost more the wider the state is, so the searched part of the budget
    // shrinks where the state is fat (AI_SEARCH.fat_*): the 2 s the sweep flags a decision over
    // (R186) buys a fraction of the nodes there that it buys in the opening. The whole searched
    // section — the lethal solver's slice, the beam's and the replies' — is drawn from one
    // sub-counter of that size, so the replies cannot spend what the beam was denied.
    let budget = scaled_budget(budget, fat_width(&probe, scratch.candidates.len()));
    let stage = create_sub_counter(counter, budget.nodes);
    let counter: &dyn NodeCounter = &stage;

    // The determinizations' draws: with a style's own two (passing over lethal, picking a
    // finalist), the only draws decide ever takes from options.rng.
    let k = budget.determinizations.max(1);
    let mut dets: Vec<GameState> = Vec::new();
    for _ in 0..k {
        dets.push(determinize_on(
            &public,
            seat,
            rng,
            DeterminizeOptions::default(),
            &ground,
        ));
    }
    scratch.determinizations = k;

    // 5. Lethal, verified on every determinization, unless the style passes over it (R1452): then
    // the search below plays on as if it had not seen it, kept off every line that wins this turn.
    let mut passed_over: Option<ActionBody> = None;
    if let Some(lethal) = find_lethal(&dets, seat, counter, budget.lethal_nodes)
        && let Some(first_lethal) = lethal.first().cloned()
    {
        if !passes_over_lethal(style, rng) {
            return Plan::Searched {
                action: first_lethal,
                reason: DecisionReason::Lethal,
                line: lethal,
                score: AI_EVAL.win - f64::from(public.turn),
                stopped_by: None,
            };
        }
        passed_over = Some(first_lethal);
    }

    // 6. The beam on determinization 0, keeping enough nodes back to score the finalists after the
    // opponent's reply on every determinization.
    let Some(det0) = dets.first() else {
        let action = fallback_action(&scratch.candidates);
        return Plan::Searched {
            line: vec![action.clone()],
            action,
            reason: DecisionReason::Fallback,
            score: 0.0,
            stopped_by: None,
        };
    };
    let root_turn = det0.turn;
    let remaining = budget.nodes.saturating_sub(counter.used());
    let finalist_count = budget.finalists.max(1);
    let per_finalist = (AI_SEARCH.lines_per_action * AI_REPLY.reserve_steps) as usize
        + (k - 1) * (budget.max_depth + 1 + AI_REPLY.reserve_steps as usize);
    let reserve = (remaining / 2).min(finalist_count * per_finalist);
    // The beam's own slice of the counter. Its stop reason is read as the beam ends: nothing takes a
    // node through the slice afterwards, so it is the reason TS read off it at the end.
    let (found, beam_stopped_by) = {
        let beam_counter = create_sub_counter(counter, remaining - reserve);
        let found = beam_search(det0, seat, &beam_counter, budget);
        (found, beam_counter.stopped_by())
    };
    scratch.lines = found.len();

    if found.is_empty() {
        let action = fallback_action(&scratch.candidates);
        let stopped_by = if counter.stopped_by() != StoppedBy::Exhausted {
            counter.stopped_by()
        } else {
            beam_stopped_by
        };
        return Plan::Searched {
            line: vec![action.clone()],
            action,
            reason: DecisionReason::Fallback,
            score: 0.0,
            stopped_by: Some(stopped_by),
        };
    }

    let found = match &passed_over {
        Some(first_lethal) => lines_missing_lethal(found, first_lethal, seat),
        None => found,
    };

    // Only a line's first action is ever played. The best lines (at most AI_SEARCH.linesPerAction
    // for any one first action) are scored after the opponent's reply on determinization 0; each
    // first action keeps its best line; the best `finalists` first actions are scored again on every
    // other determinization, and the best mean wins.
    let shortlist = shortlist_lines(&found, finalist_count * AI_SEARCH.lines_per_action as usize);
    let hidden0 = hidden_card_ids(det0, seat);
    let mut replied: Vec<Scored<'_>> = Vec::new();
    for line in shortlist {
        let Some(score) = reply_score(&line.end, seat, root_turn, counter, &hidden0) else {
            break;
        };
        replied.push(Scored { line, score });
    }
    let unreplied: Vec<Scored<'_>> = found
        .iter()
        .map(|line| Scored {
            line,
            score: line.score,
        })
        .collect();
    let finalists = best_per_first_action(
        if !replied.is_empty() { &replied } else { &unreplied },
        finalist_count,
    );

    let mut totals: Vec<f64> = finalists.iter().map(|entry| entry.score).collect();
    let mut scored_on: i32 = 1;
    for world in dets.iter().skip(1) {
        let hidden = hidden_card_ids(world, seat);
        let mut row: Vec<f64> = Vec::new();
        for entry in &finalists {
            let Some(score) = score_line(
                world,
                seat,
                &entry.line.actions,
                counter,
                !replied.is_empty(),
                Some(&hidden),
            ) else {
                break;
            };
            row.push(score);
        }
        // A world the counter could not finish is left out for every finalist alike.
        if row.len() < finalists.len() {
            break;
        }
        for (index, score) in row.into_iter().enumerate() {
            if let Some(total) = totals.get_mut(index) {
                *total += score;
            }
        }
        scored_on += 1;
    }

    let means: Vec<f64> = (0..finalists.len())
        .map(|index| totals.get(index).copied().unwrap_or(0.0) / f64::from(scored_on))
        .collect();
    let chosen_index = pick_finalist(&means, style.temperature, rng);
    let chosen_mean = means.get(chosen_index).copied().unwrap_or(0.0);

    let chosen = finalists.get(chosen_index).map(|entry| entry.line);
    let Some((chosen, action)) =
        chosen.and_then(|line| line.actions.first().map(|action| (line, action.clone())))
    else {
        let fallback = fallback_action(&scratch.candidates);
        return Plan::Searched {
            line: vec![fallback.clone()],
            action: fallback,
            reason: DecisionReason::Fallback,
            score: 0.0,
            stopped_by: None,
        };
    };
    let stopped_by = if counter.stopped_by() != StoppedBy::Exhausted {
        counter.stopped_by()
    } else {
        beam_stopped_by
    };
    let reason = if public
        .pending
        .as_ref()
        .is_some_and(|pending| pending.player_id == seat)
    {
        DecisionReason::Prompt
    } else {
        DecisionReason::Search
    };
    Plan::Searched {
        action,
        reason,
        line: chosen.actions.clone(),
        score: chosen_mean,
        stopped_by: Some(stopped_by),
    }
}

/// The body a decision submits: the chosen one when the reducer accepts it on the true state, and
/// the first of the match's replacements (`replacements_for`: endTurn, then an answer, then any
/// other legal action) when it does not. A chosen action can be legal on every determinization and
/// still refuse on the true state — a face-down aura trap taxes the hand the determinization could
/// not see (R403) — and a submitted refusal is recorded against the AI and replaced anyway, so the
/// same replacement is better submitted directly: the trajectory is the same one the match would
/// have made of it, minus the rejection.
fn submitted(state: &GameState, seat: PlayerId, decision: Decision) -> Decision {
    if holds_placeholder(state) {
        // `decide` is handed the observed view outside the gate's drivers (the worker and arena
        // send the redacted state), and its `ai:hidden` placeholders have no definition the
        // reducer could resolve — an endTurn's opponent draw throws on one. Only a true state
        // proves a refusal, so here the chosen action goes in as decided and the match's own
        // replacement machinery stands in for the refusal, exactly as it does without this check.
        return decision;
    }
    let trial = |action: &ActionBody| {
        catch_unwind(AssertUnwindSafe(|| {
            reduce(state, &Action::new(action.clone(), seat, "submitted".to_string()))
                .error
                .is_none()
        }))
        .unwrap_or(false)
    };
    if trial(&decision.action) {
        return decision;
    }
    for replacement in replacements_for(state, seat) {
        if trial(&replacement) {
            return Decision {
                line: vec![replacement.clone()],
                action: replacement,
                reason: DecisionReason::Fallback,
                ..decision
            };
        }
    }
    decision
}

/// Any `ai:hidden` placeholder anywhere in the state (determinize.rs `is_placeholder`): a state the
/// observer redacted is not one a trial `reduce` can run on.
fn holds_placeholder(state: &GameState) -> bool {
    fn any(pile: &[CardInstance]) -> bool {
        pile.iter().any(|card| card.def_id == HIDDEN_DEF_ID)
    }
    PLAYER_IDS.iter().any(|seat| {
        let player = &state.players[*seat];
        any(&player.hand)
            || any(&player.library)
            || any(&player.graveyard)
            || any(&player.exile)
            || any(&player.resolving)
            || player.units.iter().flatten().any(|pile| any(pile))
            || player
                .backrow
                .iter()
                .flatten()
                .any(|card| card.def_id == HIDDEN_DEF_ID)
            || player
                .backrow_piles
                .iter()
                .flatten()
                .flatten()
                .any(|card| card.def_id == HIDDEN_DEF_ID)
            || player
                .carried
                .iter()
                .flatten()
                .flatten()
                .any(|card| card.def_id == HIDDEN_DEF_ID)
    })
}

/// The AI's one entry point. `None` when !ai_to_act(state, seat). Never panics out.
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision> {
    decide_styled(state, seat, options, &AI_STYLE_EXACT)
}

/// `decide` under `style` (R1452): the ladder bots' entry point. At `AI_STYLE_EXACT` it is
/// `decide`, draw for draw.
pub fn decide_styled(
    state: &GameState,
    seat: PlayerId,
    options: &mut AiOptions,
    style: &AiStyle,
) -> Option<Decision> {
    match catch_unwind(AssertUnwindSafe(|| ai_to_act(state, seat))) {
        Ok(true) => {}
        _ => return None,
    }

    let budget = options.budget;
    let counter = create_node_counter(budget.nodes, options.should_stop);
    let mut scratch = Scratch {
        candidates: Vec::new(),
        determinizations: 0,
        lines: 0,
    };
    let rng = &mut options.rng;

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        plan(state, seat, budget, style, rng, &counter, &mut scratch)
    }));

    let stats = |counter: &CountingNodeCounter, thrown: usize, score: f64, stopped_by: Option<StoppedBy>| {
        SearchStats {
            nodes: counter.used(),
            determinizations: scratch.determinizations,
            lines: scratch.lines,
            sim_errors: counter.sim_errors() + thrown,
            stopped_by: stopped_by.unwrap_or_else(|| counter.stopped_by()),
            score,
        }
    };

    match outcome {
        Ok(Plan::Immediate(action, reason)) => Some(submitted(state, seat, immediate(action, reason))),
        Ok(Plan::Searched {
            action,
            reason,
            line,
            score,
            stopped_by,
        }) => Some(submitted(
            state,
            seat,
            Decision {
                action,
                reason,
                line,
                stats: stats(&counter, 0, score, stopped_by),
            },
        )),
        Err(_) => {
            let action = fallback_action(&scratch.candidates);
            Some(submitted(
                state,
                seat,
                Decision {
                    line: vec![action.clone()],
                    action,
                    reason: DecisionReason::Fallback,
                    stats: stats(&counter, 1, 0.0, None),
                },
            ))
        }
    }
}
