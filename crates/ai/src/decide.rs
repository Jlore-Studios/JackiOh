//! The AI's one entry point (SPEC §9.9). The order below is the contract:
//!
//!   1. ai_to_act / redact — the only two reads of the true state (R185).
//!   2. an unanswered draw offer is declined at once (R188).
//!   3. the mulligan keeps the cheap cards; a question of a sequence (Meditative+ #20 KY's
//!      Competition) is answered at once with the option its statement works out to (R1184002): no
//!      search, so thirty questions cost what one redaction does, and never Stop, since a miss costs
//!      nothing a Stop keeps.
//!   4. a single candidate is played with no search: listed on a throwaway determinization, since
//!      the seat's own legal actions never depend on the hidden cards, so `options.rng` is untouched.
//!   5. the exact lethal solver, verified on every determinization (a style may pass over it, and
//!      then step 6 keeps off every line that wins this turn, R1452).
//!   6. the beam on determinization 0; its best lines are scored after the opponent's reply
//!      (reply.rs), the best first actions are re-scored the same way on the other determinizations,
//!      and the best mean wins (a style with a temperature picks among them, R1452).
//!   7. a fallback when nothing could be scored.
//!
//! The chosen action is tried on the true state before it is returned (`submitted`): when the
//! reducer refuses it, the first action of the next finalist it accepts goes in its place, best mean
//! first, and only when none is accepted the match's replacement does, as a fallback (#1575).
//!
//! Only the first action of the chosen line is played; the caller asks again after it, so the AI
//! re-plans after every action and a plan that only one sampled world liked never gets past step one.
//! `decide` never panics out: every internal failure becomes a fallback and is counted in simErrors,
//! a panic in simPanics too (#1429), since in WebAssembly it would have lost the whole decision.
//! `decide_styled` is the same with an `AiStyle` (R1452): `decide` is it at `AI_STYLE_EXACT`, which
//! takes no draw of its own, so `decide`'s decisions and cursor are what they were before styles.
//!
//! Port of `packages/ai/src/decide.ts` (SURFACE §9). TS's `try`/`catch` is `catch_unwind` (an engine
//! invariant is a panic in Rust, SURFACE §4.4.9). TS made the node counter after the determinizations;
//! here it is made first, so the counter outlives the guarded body. Nothing differs: a counter is only
//! read through `take()`, which nothing calls before the lethal solver, so its stats are the same.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexMap;
use jackioh_engine::subsystems::ky_competition::worked_answer;
use jackioh_engine::{
    ANSWER_STOP_KEY, Action, ActionBody, CardInstance, GameState, PLAYER_IDS, PlayerId, PromptKind,
    PromptOption, Rng, active_units_of, mulligan_prompt_for, reduce_unpiloted,
};

use crate::candidates::candidate_actions;
use crate::config::{
    AI_EVAL, AI_KY_COMPETITION_ACCURACY, AI_KY_COMPETITION_RADIANT_ACCURACY, AI_MULLIGAN, AI_MULLIGAN_CURVE,
    AI_REPLY, AI_SEARCH, AI_STYLE_EXACT,
};
use crate::determinize::{DeterminizeOptions, det_ground, determinize_on};
use crate::lethal::find_lethal;
use crate::match_::replacements_for;
use crate::mulligan::mulligan_keep_curved;
use crate::observe::{HIDDEN_DEF_ID, ai_to_act, redact, unanswered_draw_offer};
use crate::reply::{hidden_card_ids, reply_score};
use crate::search::{Line, beam_search, score_line};
use crate::simulate::{CountingNodeCounter, create_node_counter, create_sub_counter, note_sim_panic};
use crate::types::{
    AiOptions, AiStyle, Decision, DecisionReason, NodeCounter, SearchBudget, SearchStats, StoppedBy,
};

fn quiet_stats() -> SearchStats {
    SearchStats {
        nodes: 0,
        determinizations: 0,
        lines: 0,
        sim_errors: 0,
        sim_panics: 0,
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
        first_sim_panic: None,
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

/// Step 3, R1184002: the answer to a question of a sequence open for `seat` (Meditative+ #20 KY's
/// Competition): the option whose words are what the statement works out to (`worked_answer`), read
/// off the redacted prompt, so the key is never read (R465) — a player with a pencil answers so. Its
/// redacted copy judges no answer right, so a search would score the four answers and Stop alike and
/// spend a decision's budget on a guess, up to `KY_COMPETITION_MAX` times in one turn, past the turn
/// clock (R79); worked out, each answer costs one redaction. It never stops: a miss would end the
/// sequence with the streak a Stop keeps, so answering is never worse. R1563005: like a player in a
/// hurry it slips: it answers right `AI_KY_COMPETITION_ACCURACY` of the time (the Radiant face's easier
/// sums `AI_KY_COMPETITION_RADIANT_ACCURACY`), and otherwise gives one of the wrong answers, both drawn
/// from its own stream. `None` for any other prompt, or a statement it cannot work out, which the
/// search then answers.
fn worked_answer_action(public: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    let pending = public.pending.as_ref()?;
    if pending.player_id != seat || pending.kind != PromptKind::Answer || pending.streak.is_none() {
        return None;
    }
    let answer = worked_answer(&pending.prompt)?.to_string();
    let option = pending.options.iter().find(|option| option.label == answer)?;
    let accuracy = if pending.resume.radiant {
        AI_KY_COMPETITION_RADIANT_ACCURACY
    } else {
        AI_KY_COMPETITION_ACCURACY
    };
    let selection = if rng.chance(accuracy) {
        option.selection.clone()
    } else {
        let wrong: Vec<&PromptOption> = pending
            .options
            .iter()
            .filter(|other| other.label != answer && other.key != ANSWER_STOP_KEY)
            .collect();
        match wrong.get(rng.int(wrong.len() as i32).max(0) as usize) {
            Some(miss) => miss.selection.clone(),
            None => option.selection.clone(),
        }
    };
    Some(ActionBody::Answer {
        choice_id: pending.id.clone(),
        selection: vec![selection],
    })
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
        /// The finalists not chosen, best mean first (`runners_up_of`); empty off step 6's pick.
        runners_up: Vec<RunnerUp>,
    },
}

/// A finalist the decision did not choose: its line and its mean score. `submitted` plays the first
/// of them the true state accepts when it refuses the chosen action.
struct RunnerUp {
    line: Vec<ActionBody>,
    score: f64,
}

/// Every finalist but the chosen one, best mean first. Stable, so equal means keep the finalists'
/// order.
fn runners_up_of(finalists: &[Scored<'_>], means: &[f64], chosen_index: usize) -> Vec<RunnerUp> {
    let mut out: Vec<RunnerUp> = finalists
        .iter()
        .zip(means)
        .enumerate()
        .filter(|(index, _)| *index != chosen_index)
        .map(|(_, (entry, mean))| RunnerUp {
            line: entry.line.actions.clone(),
            score: *mean,
        })
        .collect();
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out
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

    // 3, R1184002: a question of a sequence, worked out from its statement, at once (R1563005: right
    // most of the time).
    if let Some(answer) = worked_answer_action(&public, seat, rng) {
        return Plan::Immediate(answer, DecisionReason::Prompt);
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
                runners_up: Vec::new(),
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
            runners_up: Vec::new(),
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
            runners_up: Vec::new(),
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
            runners_up: Vec::new(),
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
        runners_up: runners_up_of(&finalists, &means, chosen_index),
    }
}

/// The body a decision submits: the chosen one when the reducer accepts it on the true state; else
/// the first action of the first runner-up it accepts (the finalists not chosen, best mean first),
/// with the search's reason and that runner-up's line and mean (#1575); and only when none is
/// accepted, the first of the match's replacements (`replacements_for`: endTurn, then an answer,
/// then any other legal action) as a fallback. A chosen action can be legal on every determinization
/// and still refuse on the true state — a face-down aura trap taxes the hand the determinization could
/// not see (R403) — and a submitted refusal is recorded against the AI and replaced anyway, so the
/// same replacement is better submitted directly: the trajectory is the same one the match would
/// have made of it, minus the rejection.
fn submitted(state: &GameState, seat: PlayerId, decision: Decision, runners_up: &[RunnerUp]) -> Decision {
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
            reduce_unpiloted(state, &Action::new(action.clone(), seat, "submitted".to_string()))
                .error
                .is_none()
        }))
        .unwrap_or(false)
    };
    if trial(&decision.action) {
        return decision;
    }
    for runner_up in runners_up {
        if let Some(first) = runner_up.line.first()
            && trial(first)
        {
            return Decision {
                action: first.clone(),
                line: runner_up.line.clone(),
                stats: SearchStats {
                    score: runner_up.score,
                    ..decision.stats
                },
                ..decision
            };
        }
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

    let stats = |counter: &CountingNodeCounter, score: f64, stopped_by: Option<StoppedBy>| SearchStats {
        nodes: counter.used(),
        determinizations: scratch.determinizations,
        lines: scratch.lines,
        sim_errors: counter.sim_errors(),
        sim_panics: counter.sim_panics(),
        stopped_by: stopped_by.unwrap_or_else(|| counter.stopped_by()),
        score,
    };

    match outcome {
        Ok(Plan::Immediate(action, reason)) => Some(submitted(state, seat, immediate(action, reason), &[])),
        Ok(Plan::Searched {
            action,
            reason,
            line,
            score,
            stopped_by,
            runners_up,
        }) => Some(submitted(
            state,
            seat,
            Decision {
                action,
                reason,
                line,
                stats: stats(&counter, score, stopped_by),
                first_sim_panic: counter.first_sim_panic(),
            },
            &runners_up,
        )),
        Err(payload) => {
            // The plan's own panic: a failed simulation like the others (once in `sim_errors`, as
            // before #1429), and a panic.
            note_sim_panic(&counter, payload.as_ref());
            let action = fallback_action(&scratch.candidates);
            Some(submitted(
                state,
                seat,
                Decision {
                    line: vec![action.clone()],
                    action,
                    reason: DecisionReason::Fallback,
                    stats: stats(&counter, 0.0, None),
                    first_sim_panic: counter.first_sim_panic(),
                },
                &[],
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::{Scenario, json, scenario};

    use super::*;
    use crate::config::AI_BUDGET;
    use crate::simulate::create_node_counter;

    const AI: PlayerId = PlayerId::P1;
    const LOBBYIST: &str = "classic-068"; // C #68: (3)+ Cost cards cost (1) more.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const VANILLA: &str = "core-008"; // (1) Unit 4/4

    /// The AI (p1) at 3 mana with a (3) and a (1) Unit in hand. With `lobbyist`, C #68 stands on the
    /// other side's field and the (3) costs 4: the hidden tax R403's face-down aura was, planted where
    /// the search cannot see it by deciding on the board without it. It is the literal's last card, so
    /// every other id is the same on both boards.
    fn board(lobbyist: bool) -> Scenario {
        jackioh_cards::register_all();
        let field: Vec<&str> = if lobbyist { vec![LOBBYIST] } else { Vec::new() };
        scenario(json!({
            "p1": { "hand": [MENACE, VANILLA], "mana": 3 },
            "p2": { "field": field },
        }))
    }

    /// What `decide` would submit, its runners-up beside it, as planned on `state`.
    fn planned(state: &GameState) -> (Decision, Vec<RunnerUp>) {
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let mut scratch = Scratch {
            candidates: Vec::new(),
            determinizations: 0,
            lines: 0,
        };
        let mut rng = Rng::new("decide-1575", 0);
        let Plan::Searched {
            action,
            reason,
            line,
            score,
            runners_up,
            ..
        } = plan(
            state,
            AI,
            AI_BUDGET,
            &AI_STYLE_EXACT,
            &mut rng,
            &counter,
            &mut scratch,
        )
        else {
            panic!("the search decides");
        };
        let decision = Decision {
            action,
            reason,
            line,
            stats: SearchStats {
                score,
                ..quiet_stats()
            },
            first_sim_panic: None,
        };
        (decision, runners_up)
    }

    fn plays(action: Option<&ActionBody>, card: &CardInstance) -> bool {
        matches!(action, Some(ActionBody::Play { instance_id, .. }) if *instance_id == card.id)
    }

    /// #1575: the search picks the (3) Unit, the true state refuses it, and the runner-up line (the
    /// (1) Unit) is submitted in its place, as the search's own decision, not endTurn.
    #[test]
    fn a_refused_choice_submits_the_runner_up_lines_first_action_not_end_turn() {
        let believed = board(false);
        let truth = board(true);
        let (decision, runners_up) = planned(believed.state());
        assert!(
            plays(Some(&decision.action), believed.card(MENACE)),
            "{:?}",
            decision.action
        );
        assert!(
            runners_up.windows(2).all(|pair| pair[0].score >= pair[1].score),
            "best mean first"
        );
        let runner_up = runners_up.first().expect("a runner-up");
        assert!(
            plays(runner_up.line.first(), believed.card(VANILLA)),
            "{:?}",
            runner_up.line
        );

        let sent = submitted(truth.state(), AI, decision, &runners_up);
        assert_eq!(Some(&sent.action), runner_up.line.first());
        assert_ne!(sent.action, ActionBody::EndTurn);
        assert_eq!(sent.reason, DecisionReason::Search);
        assert_eq!(sent.line, runner_up.line);
        assert_eq!(sent.stats.score, runner_up.score);
    }

    /// With no runner-up the true state accepts, the refusal falls back as before: endTurn first.
    #[test]
    fn a_refused_choice_with_no_runner_up_falls_back_to_end_turn() {
        let believed = board(false);
        let truth = board(true);
        let (decision, _) = planned(believed.state());
        let sent = submitted(truth.state(), AI, decision, &[]);
        assert_eq!(sent.action, ActionBody::EndTurn);
        assert_eq!(sent.reason, DecisionReason::Fallback);
    }

    /// An accepted choice is submitted as decided, its runners-up unread.
    #[test]
    fn an_accepted_choice_is_submitted_as_decided() {
        let believed = board(false);
        let (decision, runners_up) = planned(believed.state());
        let sent = submitted(believed.state(), AI, decision.clone(), &runners_up);
        assert_eq!(sent, decision);
    }
}
