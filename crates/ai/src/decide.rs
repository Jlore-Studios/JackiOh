//! The AI's one entry point (SPEC §9.9, SURFACE §9). The order below is the contract:
//!
//! 1. ai_to_act / redact — the only two reads of the true state (R185).
//! 2. an unanswered draw offer is declined at once (R188).
//! 3. the mulligan keeps the cheap cards; a question of a sequence, or C+ #42 KY's Test's Easy problem
//!    (#2929), is answered with no search (R1184002).
//! 4. a single candidate is played with no search, listed on a throwaway determinization (the seat's own
//!    legal actions never depend on the hidden cards, so `options.rng` is untouched).
//! 5. the exact lethal solver, verified on every determinization and trimmed (R1965000), unless a style
//!    passes over it (R1452).
//! 6. the beam on determinization 0, its best lines scored after the opponent's reply and its best first
//!    actions re-scored on the other determinizations; the best mean wins. A host's wrap-up stops the
//!    beam and skips R3696000's payoff line, so that a clock still leaves the reply to the shortlist and
//!    the other determinizations (R3498000).
//! 7. a fallback when nothing could be scored.
//!
//! The chosen action is tried on the true state first (`submitted`, #1575). Only it is played, so the AI
//! re-plans after every action. `decide` never panics out: a failure becomes a fallback counted in
//! simErrors, a panic (an engine invariant, SURFACE §4.4.9) in simPanics too (#1429).

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::subsystems::ky_competition::worked_answer;
use jackioh_engine::{
    ANSWER_STOP_KEY, Action, ActionBody, CardInstance, GameState, PLAYER_IDS, PlayerId, PromptKind,
    PromptOption, Rng, active_units_of, mulligan_prompt_for, reduce_unpiloted,
};

use crate::candidates::candidate_actions;
use crate::config::{
    AI_EVAL, AI_KY_COMPETITION_ACCURACY, AI_KY_COMPETITION_RADIANT_ACCURACY, AI_KY_TEST_ACCURACY,
    AI_MULLIGAN, AI_MULLIGAN_CURVE, AI_REPLY, AI_SEARCH, AI_STYLE_EXACT, AI_WRAP_UP_SHARE,
};
use crate::determinize::{DeterminizeOptions, det_ground, determinize_on};
use crate::evaluate::{
    Prices, activation_prices, attack_hook_prices, awaken_prices, death_prices, holds_starship,
    turn_hook_prices,
};
use crate::learned::{LEARNED, unknown_root};
use crate::lethal::{find_lethal, trim_lethal};
use crate::match_::replacements_for;
use crate::mulligan::mulligan_keep_curved;
use crate::observe::{HIDDEN_DEF_ID, ai_to_act, redact, unanswered_draw_offer};
use crate::reply::{hidden_card_ids, reply_score};
use crate::search::{Line, beam_search, score_line};
use crate::simulate::{
    CountingNodeCounter, LineStatus, create_node_counter, create_sub_counter, create_wrap_up_counter,
    note_sim_panic,
};
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
        replied: 0,
        worlds_scored: 0,
        score: 0.0,
    }
}

/// R3498000: the milliseconds into a host's `limit_ms` at which it fires the wrap-up
/// (`AiOptions.wrap_up`), `AI_WRAP_UP_SHARE` of it, rounded; 0 for a limit of 0 or below. The one place
/// the share is applied, so that every host wraps up alike.
pub fn wrap_up_after_ms(limit_ms: i64) -> i64 {
    (limit_ms.max(0) as f64 * AI_WRAP_UP_SHARE).round() as i64
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
/// Competition), or C+ #42 KY's Test's Easy problem (#2929), worked out from the redacted prompt's
/// statement (`worked_answer`), so the key is never read (R465) and no search spends the budget on a
/// guess, past the turn clock (R79). It never stops. R1563005: right `AI_KY_COMPETITION_ACCURACY` of the
/// time (the Radiant face's own when Radiant; `AI_KY_TEST_ACCURACY` with no streak), else wrong, drawn
/// from its own stream. `None` for any other prompt, or a statement it cannot work out, which the search
/// then answers.
fn worked_answer_action(public: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    let pending = public.pending.as_ref()?;
    if pending.player_id != seat || pending.kind != PromptKind::Answer {
        return None;
    }
    let answer = worked_answer(&pending.prompt)?.to_string();
    let option = pending.options.iter().find(|option| option.label == answer)?;
    let accuracy = match pending.streak {
        None => AI_KY_TEST_ACCURACY,
        Some(_) if pending.resume.radiant => AI_KY_COMPETITION_RADIANT_ACCURACY,
        Some(_) => AI_KY_COMPETITION_ACCURACY,
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

/// R3696000: whether `line` hands the turn over (`Passed` or `Yielded`, the lines `reply_score`
/// replies to) with a delayed entry of `seat`'s own waiting at its end that `root` did not hold: a
/// payoff still to come (a countdown, a next-turn reward), which the static score that ranks the lines
/// does not see.
fn leaves_own_delayed(line: &Line, root: &GameState, seat: PlayerId) -> bool {
    matches!(line.status, LineStatus::Passed | LineStatus::Yielded)
        && line
            .end
            .delayed
            .iter()
            .any(|entry| entry.owner == seat && !root.delayed.iter().any(|held| held.id == entry.id))
}

/// Lines in order, at most `perAction` for any one first action, `count` in all, and beside them the
/// payoff line (R3696000): when none of them `leaves_own_delayed`, the best line that does, past `count`
/// and the per-action cap, which `payoff_score` replies to last.
fn shortlist_lines<'a>(
    found: &'a [Line],
    count: usize,
    root: &GameState,
    seat: PlayerId,
) -> (Vec<&'a Line>, Option<&'a Line>) {
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
    let waits = |line: &&Line| !line.actions.is_empty() && leaves_own_delayed(line, root, seat);
    let payoff = if out.iter().any(waits) {
        None
    } else {
        found.iter().find(|line| waits(line))
    };
    (out, payoff)
}

/// R3696000: the payoff line's score after the opponent's reply, drawn only on the nodes `counter` holds
/// past `keep` (what the finalists' scores on every other determinization keep back), so it never takes
/// one of theirs; `None` when those run out first.
fn payoff_score(
    line: &Line,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    keep: usize,
    hidden: &IndexSet<String>,
    prices: &Prices,
) -> Option<f64> {
    let spare = create_sub_counter(
        counter,
        counter
            .limit()
            .saturating_sub(counter.used())
            .saturating_sub(keep),
    );
    reply_score(&line.end, seat, root_turn, &spare, hidden, prices)
}

/// One line and the score it was given.
#[derive(Clone, Copy)]
struct Scored<'a> {
    line: &'a Line,
    score: f64,
}

/// The best-scoring line for each distinct first action, best first, at most `count` of them. A play
/// that differs from a better one only in its lane and scores the same is left out (#1797): the beam
/// keeps a play's lanes apart so that the reply can tell them apart, and where it did not, the lane
/// would only take the place of another first action.
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
    // The map keeps first-seen order.
    let mut entries: Vec<Scored<'a>> = best.into_values().collect();
    // Stable, so equal scores keep the beam's order.
    entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<Scored<'a>> = Vec::new();
    for entry in entries {
        if out.len() >= count {
            break;
        }
        let first = entry.line.actions.first().map(lane_free);
        let another_lane = out
            .iter()
            .any(|held| held.score == entry.score && held.line.actions.first().map(lane_free) == first);
        if !another_lane {
            out.push(entry);
        }
    }
    out
}

/// `action` with the lane of the zone it plays into left out (#1797).
fn lane_free(action: &ActionBody) -> ActionBody {
    let mut action = action.clone();
    if let ActionBody::Play { zone: Some(zone), .. } = &mut action {
        zone.lane = 0;
    }
    action
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

/// The budget a decision of `fat` width spends (AI_SEARCH.fat_*): `nodes`, at most `fat_budget_nodes`
/// (R2575000), in inverse proportion to the width past fat_reference, floored at fat_min_nodes; the
/// lethal solver keeps its share of it, floored at fat_min_lethal. The search's shape is unchanged.
fn scaled_budget(budget: SearchBudget, fat: usize) -> SearchBudget {
    let reference = AI_SEARCH.fat_reference.max(1) as usize;
    let nodes = if fat <= reference {
        budget.nodes
    } else {
        let from = budget.nodes.min(AI_SEARCH.fat_budget_nodes.max(0) as usize);
        (from * reference / fat).max(AI_SEARCH.fat_min_nodes.max(0) as usize)
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
/// or below, or with fewer than two finalists, the first best, with no draw; otherwise one draw on a
/// softmax over the means.
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

/// What the guarded body leaves behind for the fallback when it panics.
struct Scratch {
    candidates: Vec<ActionBody>,
    determinizations: usize,
    lines: usize,
    /// R3498000: `SearchStats.replied`.
    replied: usize,
    /// R3498000: `SearchStats.worldsScored`.
    worlds_scored: usize,
}

/// Steps 1–7 on the redacted copy; every number `stats` reads beside the counter goes in `scratch`.
/// `options` gives the budget, the wrap-up and the stream; its `should_stop` is the counter's.
fn plan(
    state: &GameState,
    seat: PlayerId,
    style: &AiStyle,
    options: &mut AiOptions,
    counter: &dyn NodeCounter,
    scratch: &mut Scratch,
) -> Plan {
    let budget = options.budget;
    let wrap_up = options.wrap_up;
    let rng = &mut options.rng;
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

    // 3, R1184002: a question of a sequence, or C+ #42 KY's Test's Easy problem (#2929), worked out from
    // its statement, at once (R1563005: right most of the time).
    if let Some(answer) = worked_answer_action(&public, seat, rng) {
        return Plan::Immediate(answer, DecisionReason::Prompt);
    }

    // 4. Forced: a throwaway world lists the candidates without touching options.rng. Every
    // determinization of this decision starts from the same public state, so the read-only part
    // of the determinization (the pools, what each side has shown, each side's face-down
    // grants) is computed once (det_ground) and handed to each.
    let ground = det_ground(&public, seat);
    let probe = determinize_on(
        &public,
        seat,
        &mut Rng::new(AI_SEARCH.probe_seed, 0),
        DeterminizeOptions::default(),
        &ground,
    );
    scratch.candidates = candidate_actions(&probe, seat, Prices::NONE);
    if scratch.candidates.len() == 1
        && let Some(only) = scratch.candidates.first()
    {
        return Plan::Immediate(only.clone(), DecisionReason::Forced);
    }

    // A decision's nodes cost more the wider the state is, so the searched part of the budget shrinks
    // where the state is fat (AI_SEARCH.fat_*, R186). The whole searched section (the lethal solver's
    // slice, the beam's, the replies') is drawn from one sub-counter of that size, so the replies
    // cannot spend what the beam was denied.
    let budget = scaled_budget(budget, fat_width(&probe, scratch.candidates.len()));
    let stage = create_sub_counter(counter, budget.nodes);
    let counter: &dyn NodeCounter = &stage;

    // R2424000, R2477000, R2564000, R2565000, R2603000, R2687000, R2688000, R3064000: every Death (a
    // backrow card's and a hand Field Spell's among them), turn hook, Activate, attack hook and Dormant
    // card's awakening the seat can read, priced once on the probe world for the whole decision from the
    // searched section's nodes; all are priced before any is passed, so no probe reads a price, and every
    // step below is passed them (`Prices`).
    let deaths = death_prices(&probe, seat, counter);
    let turn_hooks = turn_hook_prices(&probe, seat, counter);
    let activations = activation_prices(&probe, seat, counter);
    let attack_hooks = attack_hook_prices(&probe, seat, counter);
    let awakenings = awaken_prices(&probe, seat, counter);
    let prices = Prices {
        deaths,
        turn_hooks,
        activations,
        attack_hooks,
        awakenings,
        // R3038003: read off the redacted root, so no hidden card is in it, and carried with the prices
        // so that every line of the decision, on every determinization, is scored by the same eval.
        starship_root: holds_starship(&public, seat),
        // R3479000: read off the redacted root too, against the network the search scores with.
        unknown_root: unknown_root(&public, seat, &LEARNED),
    };

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

    // 5. Lethal, verified on every determinization, unless the style passes over it (R1452): then the
    // search plays on kept off every line that wins this turn. Played, it is first trimmed to the
    // actions it needs (R1965000), else a lethal behind a free step that changes nothing would play that
    // step at every decision; a trim the counter stops short of a first action leaves no lethal to play.
    let mut passed_over: Option<ActionBody> = None;
    if let Some(lethal) = find_lethal(&dets, seat, counter, budget.lethal_nodes, &prices)
        && let Some(first_lethal) = lethal.first().cloned()
    {
        if passes_over_lethal(style, rng) {
            passed_over = Some(first_lethal);
        } else if let Some(trimmed) = trim_lethal(&dets, seat, lethal, counter, &prices)
            && let Some(first) = trimmed.first().cloned()
        {
            return Plan::Searched {
                action: first,
                reason: DecisionReason::Lethal,
                line: trimmed,
                score: AI_EVAL.win - f64::from(public.turn),
                stopped_by: None,
                runners_up: Vec::new(),
            };
        }
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
    // What one finalist's scores on every other determinization keep back (R3696000's `keep`, below).
    let other_worlds = (k - 1) * (budget.max_depth + 1 + AI_REPLY.reserve_steps as usize);
    let per_finalist = (AI_SEARCH.lines_per_action * AI_REPLY.reserve_steps) as usize + other_worlds;
    let reserve = (remaining / 2).min(finalist_count * per_finalist);
    // The beam's own slice of the counter. Its stop reason is read as the beam ends: nothing takes a
    // node through the slice afterwards. R3498000: the host's wrap-up stops this slice alone, so that the
    // reply and the other determinizations below still run until the hard stop. It is first polled once
    // the root has been expanded (root_branching and endTurn): a wrap-up that fired before the beam would
    // otherwise leave it no line and the decision would fall back.
    let (found, beam_stopped_by) = {
        let beam_counter =
            create_wrap_up_counter(counter, remaining - reserve, wrap_up, budget.root_branching + 1);
        let found = beam_search(det0, seat, &beam_counter, budget, &prices);
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
    // for any one first action, then the best one leaving a payoff still to come, R3696000, on the nodes
    // the other determinizations do not keep back) are scored after the opponent's reply on
    // determinization 0; each first action keeps its best line; the best `finalists` first actions are
    // scored again on every other determinization, and the best mean wins.
    let (shortlist, payoff) = shortlist_lines(
        &found,
        finalist_count * AI_SEARCH.lines_per_action as usize,
        det0,
        seat,
    );
    let hidden0 = hidden_card_ids(det0, seat);
    let mut replied: Vec<Scored<'_>> = Vec::new();
    for &line in &shortlist {
        let Some(score) = reply_score(&line.end, seat, root_turn, counter, &hidden0, &prices) else {
            break;
        };
        replied.push(Scored { line, score });
    }
    // R3696000: the payoff line last, once every shortlisted line is replied to: skipped once the host's
    // clock has passed its wrap-up (R3498000), else replied to on the nodes past what the other worlds
    // keep back. Either stop, when it comes, is the decision's own.
    let mut payoff_stop: Option<StoppedBy> = None;
    if let Some(line) = payoff
        && replied.len() == shortlist.len()
    {
        if wrap_up.is_some_and(|wrapped| wrapped()) {
            payoff_stop = Some(StoppedBy::Clock);
        } else {
            match payoff_score(
                line,
                seat,
                root_turn,
                counter,
                finalist_count * other_worlds,
                &hidden0,
                &prices,
            ) {
                Some(score) => replied.push(Scored { line, score }),
                None => payoff_stop = Some(StoppedBy::Budget),
            }
        }
    }
    // R3498000: only a passed or yielded line is replied to; `reply_score` scores any other statically.
    scratch.replied = replied
        .iter()
        .filter(|entry| matches!(entry.line.status, LineStatus::Passed | LineStatus::Yielded))
        .count();
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
                &prices,
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
    scratch.worlds_scored = usize::try_from(scored_on).unwrap_or(0);

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
        payoff_stop.unwrap_or(beam_stopped_by)
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

/// The body a decision submits: the chosen one when the reducer accepts it on the true state, else the
/// first action of the first runner-up it accepts, best mean first (#1575), else the match's first
/// replacement (`replacements_for`). A chosen action legal on every determinization can refuse on the
/// true state (a face-down aura trap taxes the unseen hand, R403), and a submitted refusal is recorded
/// against the AI, so the replacement is submitted directly.
fn submitted(state: &GameState, seat: PlayerId, decision: Decision, runners_up: &[RunnerUp]) -> Decision {
    if holds_placeholder(state) {
        // The worker and arena hand `decide` the redacted state, whose `ai:hidden` placeholders the
        // reducer cannot resolve: only a true state proves a refusal, so the match's own replacement
        // stands in for one.
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
        replied: 0,
        worlds_scored: 0,
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        plan(state, seat, style, options, &counter, &mut scratch)
    }));

    let stats = |counter: &CountingNodeCounter, score: f64, stopped_by: Option<StoppedBy>| SearchStats {
        nodes: counter.used(),
        determinizations: scratch.determinizations,
        lines: scratch.lines,
        sim_errors: counter.sim_errors(),
        sim_panics: counter.sim_panics(),
        stopped_by: stopped_by.unwrap_or_else(|| counter.stopped_by()),
        replied: scratch.replied,
        worlds_scored: scratch.worlds_scored,
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
            // The plan's own panic: a failed simulation like the others (once in `sim_errors`, #1429),
            // and a panic.
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
    use jackioh_engine::{DelayedAt, DelayedEffect, Phase, Resume, Row, ZoneChoice};

    use super::*;
    use crate::config::AI_BUDGET;
    use crate::simulate::{LineStatus, create_node_counter, simulate};

    const AI: PlayerId = PlayerId::P1;
    const HUMAN: PlayerId = PlayerId::P2;
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
            replied: 0,
            worlds_scored: 0,
        };
        let mut options = AiOptions::with_budget(Rng::new("decide-1575", 0), AI_BUDGET);
        let Plan::Searched {
            action,
            reason,
            line,
            score,
            runners_up,
            ..
        } = plan(state, AI, &AI_STYLE_EXACT, &mut options, &counter, &mut scratch)
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

    /// #1797: of the (3) Unit's lanes 1 and 2, which score alike, only lane 1 is a finalist; its lane
    /// 3, which scores otherwise, is one of its own, and so the third is the (1) Unit's play.
    #[test]
    fn lanes_of_one_play_that_score_alike_are_one_finalist() {
        let scenario = board(false);
        let state = scenario.state();
        let play = |def_id: &str, lane: i32| ActionBody::Play {
            instance_id: scenario.card(def_id).id.clone(),
            zone: Some(ZoneChoice {
                row: Row::Units,
                lane,
            }),
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: None,
        };
        let line = |action: ActionBody| Line {
            actions: vec![action, ActionBody::EndTurn],
            score: 0.0,
            status: LineStatus::Passed,
            end: state.clone(),
        };
        let lines = [
            line(play(MENACE, 1)),
            line(play(MENACE, 2)),
            line(play(MENACE, 3)),
            line(play(VANILLA, 1)),
        ];
        let scored: Vec<Scored<'_>> = lines
            .iter()
            .zip([5.0, 5.0, 6.0, 4.0])
            .map(|(line, score)| Scored { line, score })
            .collect();
        let firsts: Vec<ActionBody> = best_per_first_action(&scored, 3)
            .iter()
            .filter_map(|entry| entry.line.actions.first().cloned())
            .collect();
        assert_eq!(firsts, vec![play(MENACE, 3), play(MENACE, 1), play(VANILLA, 1)]);
    }

    /// A delayed entry `id` of `owner`'s, due at the start of its next turn: a payoff still to come.
    fn waiting(id: &str, owner: PlayerId) -> DelayedEffect {
        DelayedEffect {
            id: id.to_string(),
            seq: 0,
            owner,
            at: DelayedAt {
                phase: Phase::Start,
                player: owner,
            },
            not_before: None,
            resume: Resume {
                def_id: MENACE.to_string(),
                hook: "resume".to_string(),
                step: "reward".to_string(),
                radiant: false,
                instance_id: None,
                data: IndexMap::new(),
            },
            watch: None,
            hand_watch: None,
            due_own_turn: None,
            label: None,
        }
    }

    /// One line per `firsts` entry, best first, each a switch of unit `u<first>` then the end of
    /// turn, scored 10 less its index and ending on `root` with the entry of each `waits` row at its
    /// index waiting, the row's status its own (else `Passed`).
    fn lines_of(
        root: &GameState,
        firsts: &[u32],
        waits: &[(usize, &str, PlayerId, LineStatus)],
    ) -> Vec<Line> {
        firsts
            .iter()
            .enumerate()
            .map(|(at, first)| {
                let mut end = root.clone();
                let mut status = LineStatus::Passed;
                for (index, id, owner, row_status) in waits {
                    if *index == at {
                        end.delayed.push(waiting(id, *owner));
                        status = *row_status;
                    }
                }
                Line {
                    actions: vec![
                        ActionBody::SwitchPosition {
                            instance_id: format!("u{first}"),
                        },
                        ActionBody::EndTurn,
                    ],
                    score: 10.0 - at as f64,
                    status,
                    end,
                }
            })
            .collect()
    }

    /// The shortlist's scores, the payoff line's last.
    fn scores((out, payoff): &(Vec<&Line>, Option<&Line>)) -> Vec<f64> {
        out.iter()
            .copied()
            .chain(*payoff)
            .map(|line| line.score)
            .collect()
    }

    /// Eight lines' first actions: two places for each of the first three, so the plain ranking of
    /// six places takes the first six lines and cuts lines 6 and 7.
    const SIX_AND_ONE: [u32; 8] = [1, 1, 2, 2, 3, 3, 4, 5];

    #[test]
    fn r3696000_the_best_delayed_payoff_line_keeps_a_place_after_the_six_that_outscore_it() {
        let root = board(false).state().clone();
        let lines = lines_of(
            &root,
            &SIX_AND_ONE,
            &[
                (6, "d9", AI, LineStatus::Passed),
                (7, "d10", AI, LineStatus::Passed),
            ],
        );
        let out = shortlist_lines(&lines, 6, &root, AI);
        assert_eq!(scores(&out), vec![10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0]);
    }

    #[test]
    fn r3696000_no_place_is_held_back_without_a_delayed_payoff_line() {
        let root = board(false).state().clone();
        let lines = lines_of(&root, &SIX_AND_ONE, &[]);
        let out = shortlist_lines(&lines, 6, &root, AI);
        assert_eq!(scores(&out), vec![10.0, 9.0, 8.0, 7.0, 6.0, 5.0]);
    }

    #[test]
    fn r3696000_no_place_is_held_back_when_a_delayed_payoff_line_made_the_cut() {
        let root = board(false).state().clone();
        let lines = lines_of(
            &root,
            &SIX_AND_ONE,
            &[
                (1, "d9", AI, LineStatus::Passed),
                (6, "d10", AI, LineStatus::Passed),
            ],
        );
        let out = shortlist_lines(&lines, 6, &root, AI);
        assert_eq!(scores(&out), vec![10.0, 9.0, 8.0, 7.0, 6.0, 5.0]);
    }

    /// Every line's end holds `d1`, which the root held too; line 6 leaves the other seat's entry and
    /// line 7, until it yields, is never replied to. None is a payoff of the AI's still to come.
    #[test]
    fn r3696000_an_entry_the_root_held_the_other_seat_owns_or_on_a_line_never_replied_to_is_no_payoff() {
        let mut root = board(false).state().clone();
        root.delayed.push(waiting("d1", AI));
        let mut lines = lines_of(
            &root,
            &SIX_AND_ONE,
            &[
                (6, "d2", HUMAN, LineStatus::Passed),
                (7, "d3", AI, LineStatus::Over),
            ],
        );
        assert_eq!(scores(&shortlist_lines(&lines, 6, &root, AI)).len(), 6);
        lines[7].status = LineStatus::Yielded;
        let out = shortlist_lines(&lines, 6, &root, AI);
        assert_eq!(scores(&out), vec![10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 3.0]);
    }

    /// R3696000: `payoff_score` with exactly `keep` nodes left takes none of them and leaves the counter
    /// running for the other worlds; with more left it is the line's `reply_score`.
    #[test]
    fn r3696000_the_payoff_line_is_replied_to_on_no_node_the_other_worlds_keep() {
        let root = board(true).state().clone();
        let Some(Ok(end)) = simulate(
            &root,
            AI,
            &ActionBody::EndTurn,
            &create_node_counter(AI_BUDGET.nodes, None),
            Prices::NONE,
        ) else {
            panic!("the AI ends its turn");
        };
        let line = Line {
            actions: vec![ActionBody::EndTurn],
            score: 0.0,
            status: LineStatus::Passed,
            end,
        };
        let hidden = hidden_card_ids(&root, AI);
        let keep = 40;
        let tight = create_node_counter(AI_BUDGET.nodes, None);
        while tight.limit() - tight.used() > keep {
            assert!(tight.take());
        }
        assert_eq!(
            payoff_score(&line, AI, root.turn, &tight, keep, &hidden, Prices::NONE),
            None
        );
        assert_eq!(tight.used(), AI_BUDGET.nodes - keep);
        assert_eq!(tight.stopped_by(), StoppedBy::Exhausted);
        assert!(tight.take(), "the kept nodes are still the other worlds'");
        let plain = reply_score(
            &line.end,
            AI,
            root.turn,
            &create_node_counter(AI_BUDGET.nodes, None),
            &hidden,
            Prices::NONE,
        );
        assert!(plain.is_some(), "PREMISE: the reply scores the line");
        let ample = create_node_counter(AI_BUDGET.nodes, None);
        assert_eq!(
            payoff_score(&line, AI, root.turn, &ample, keep, &hidden, Prices::NONE),
            plain
        );
        assert!(ample.used() > 0);
    }
}
