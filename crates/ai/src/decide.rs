//! The AI's one entry point (SPEC §9.9, SURFACE §9). The order below is the contract:
//!
//! 1. ai_to_act / redact — the only two reads of the true state (R185).
//! 2. an unanswered draw offer is declined at once (R188).
//! 3. the mulligan keeps the cheap cards; a question of a sequence, or C+ #42 KY's Test's Easy problem
//!    (#2929), is answered with no search (R1184002).
//! 4. a single candidate is played with no search, listed on a throwaway determinization (the seat's own
//!    legal actions never depend on the hidden cards, so `options.rng` is untouched).
//! 5. the exact lethal solver, verified on every determinization and trimmed (R1965000), unless a style
//!    passes over it (R1452). On a state whose budget was scaled down, its walk runs on to the caller's own
//!    allowance while it stands at a position that already reads as lethal (R4788000).
//! 6. the beam on determinization 0, its best lines scored after the opponent's reply and its best first
//!    actions re-scored on the other determinizations; the best mean wins, a finalist that won on
//!    determinization 0 alone taking its mean on the others (R4608000). A host's wrap-up stops the beam and
//!    skips R3696000's payoff line, so that a clock still leaves the reply to the shortlist and the other
//!    determinizations (R3498000).
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
    AI_MULLIGAN, AI_MULLIGAN_CURVE, AI_REPLY, AI_SEARCH, AI_STYLE_EXACT, AI_WON_SCORE_SHARE,
    AI_WRAP_UP_SHARE,
};
use crate::determinize::{DetGround, DeterminizeOptions, det_ground, determinize_on};
use crate::evaluate::{
    NextSwing, Prices, activation_prices, attack_hook_prices, awaken_prices, death_prices, evaluate_priced,
    evolve_prices, prompt_activation_prices, turn_hook_prices,
};
use crate::lethal::{find_lethal_reaching, trim_lethal};
use crate::match_::replacements_for;
use crate::mulligan::mulligan_keep_curved;
use crate::observe::{HIDDEN_DEF_ID, ai_to_act, redact, unanswered_draw_offer};
use crate::reply::{hidden_card_ids, reply_score};
use crate::search::{Line, ReplayFill, beam_search, open_in_main, replay_line};
use crate::simulate::{
    CountingNodeCounter, LineStatus, create_beam_counter, create_node_counter, create_sub_counter,
    line_status, note_sim_panic, simulate,
};
use crate::types::{
    AiOptions, AiStyle, Decision, DecisionReason, NodeCounter, SearchBudget, SearchStats, StoppedBy,
};

/// R3498000: the milliseconds into a host's `limit_ms` at which it fires the wrap-up
/// (`AiOptions.wrap_up`), `AI_WRAP_UP_SHARE` of it, rounded; 0 for a limit of 0 or below. The one place
/// the share is applied, so that every host wraps up alike.
pub fn wrap_up_after_ms(limit_ms: i64) -> i64 {
    (limit_ms.max(0) as f64 * AI_WRAP_UP_SHARE).round() as i64
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

/// R4595001: the candidate whose one `reduce` on `det` leaves `seat` the best `evaluate_priced`
/// position, its prompts left open, the first on a tie; `None` when every candidate is refused.
fn best_single_step(
    det: &GameState,
    seat: PlayerId,
    candidates: &[ActionBody],
    prices: &Prices,
) -> Option<(ActionBody, f64)> {
    let mut best: Option<(ActionBody, f64)> = None;
    for (index, action) in candidates.iter().enumerate() {
        let stepped = catch_unwind(AssertUnwindSafe(|| {
            reduce_unpiloted(det, &Action::new(action.clone(), seat, format!("single:{index}")))
        }));
        let Ok(stepped) = stepped else { continue };
        if stepped.error.is_some() {
            continue;
        }
        let score = evaluate_priced(&stepped.state, seat, NextSwing::Enemy, &AI_EVAL, prices);
        if best.as_ref().is_none_or(|(_, top)| score > *top) {
            best = Some((action.clone(), score));
        }
    }
    best
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

/// The budget a decision of `fat` width spends (AI_SEARCH.fat_*). At or under fat_reference, the
/// caller's own, the lethal solver's allowance at most its `nodes`. Past it, `nodes` at most
/// `fat_budget_nodes` (R2575000), in inverse proportion to the width, floored at fat_min_nodes; the
/// lethal solver keeps its share of it, floored at fat_min_lethal. R4802000: neither floor raises a
/// figure above the caller's own, so a solver the caller gave nothing keeps nothing, and the share's
/// product saturates, so a field at any value overflows nothing. The search's shape is unchanged.
/// R4788000: `plan` lets the solver's walk reach past that share while it is closing on a lethal.
fn scaled_budget(budget: SearchBudget, fat: usize) -> SearchBudget {
    let reference = AI_SEARCH.fat_reference.max(1) as usize;
    if fat <= reference {
        return SearchBudget {
            lethal_nodes: budget.lethal_nodes.min(budget.nodes),
            ..budget
        };
    }
    let from = budget.nodes.min(AI_SEARCH.fat_budget_nodes.max(0) as usize);
    let nodes = (from * reference / fat)
        .max(AI_SEARCH.fat_min_nodes.max(0) as usize)
        .min(budget.nodes);
    let lethal_nodes = (budget.lethal_nodes.saturating_mul(nodes) / budget.nodes.max(1))
        .max(AI_SEARCH.fat_min_lethal.max(0) as usize)
        .min(budget.lethal_nodes)
        .min(nodes);
    SearchBudget {
        nodes,
        lethal_nodes,
        ..budget
    }
}

/// R4788000: the lethal solver's reach on a decision `scaled_budget` gave `scaled` of the caller's `budget`,
/// and the searched section with room for it. On a state scaled down, its walk may go on to the allowance
/// the caller gave, at most its `nodes` (R4802000), while it stands at a position that already reads as
/// lethal (`find_lethal_reaching`); a state that is not fat keeps that allowance, so it has no reach, and
/// a caller that gives the solver nothing gives it no reach either. The section is the scaled `nodes` and
/// what the reach adds, never past the caller's `nodes`.
fn lethal_reach(budget: SearchBudget, scaled: SearchBudget) -> (usize, usize) {
    let reach = if AI_SEARCH.lethal_reach && scaled.lethal_nodes > 0 {
        budget.lethal_nodes.min(budget.nodes).max(scaled.lethal_nodes)
    } else {
        scaled.lethal_nodes
    };
    let section = scaled
        .nodes
        .saturating_add(reach - scaled.lethal_nodes)
        .min(budget.nodes);
    (reach, section)
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

/// R4608000: each finalist's mean from its row of scores, determinization 0's first. A row whose
/// first score is a won game (above `AI_WON_SCORE_SHARE` of `AI_EVAL.win`) and whose other scores, at least
/// one, are none won takes the mean of those others; every other row the mean of all its scores.
pub fn finalist_means(rows: &[Vec<f64>]) -> Vec<f64> {
    let won = AI_EVAL.win * AI_WON_SCORE_SHARE;
    rows.iter()
        .map(|row| match row.split_first() {
            Some((first, others))
                if *first > won && !others.is_empty() && others.iter().all(|score| *score <= won) =>
            {
                mean_of(others)
            }
            _ => mean_of(row),
        })
        .collect()
}

/// The mean of `scores`, added first to last; 0 for none.
fn mean_of(scores: &[f64]) -> f64 {
    match scores.split_first() {
        None => 0.0,
        Some((first, rest)) => rest.iter().fold(*first, |total, score| total + score) / scores.len() as f64,
    }
}

/// What the guarded body decided, before the stats are read off the counter: they are the counter's,
/// with this score and, when given, this stop reason.
struct Plan {
    action: ActionBody,
    reason: DecisionReason,
    line: Vec<ActionBody>,
    score: f64,
    stopped_by: Option<StoppedBy>,
    /// The finalists not chosen, best mean first (`runners_up_of`); empty off step 6's pick.
    runners_up: Vec<RunnerUp>,
}

impl Plan {
    /// `action` alone for `reason`, scored 0, with this stop reason when given.
    fn single(action: ActionBody, reason: DecisionReason, stopped_by: Option<StoppedBy>) -> Plan {
        Plan {
            line: vec![action.clone()],
            action,
            reason,
            score: 0.0,
            stopped_by,
            runners_up: Vec::new(),
        }
    }

    /// Answered at once, before a node is taken or a world made, so the counter's stats are all zero;
    /// it reads as a search that ran out of moves, whatever the host's clock says.
    fn immediate(action: ActionBody, reason: DecisionReason) -> Plan {
        Plan::single(action, reason, Some(StoppedBy::Exhausted))
    }

    /// Step 7's plan: the fallback action alone.
    fn fallback(candidates: &[ActionBody], stopped_by: Option<StoppedBy>) -> Plan {
        Plan::single(fallback_action(candidates), DecisionReason::Fallback, stopped_by)
    }
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
#[derive(Default)]
struct Scratch {
    candidates: Vec<ActionBody>,
    determinizations: usize,
    lines: usize,
    /// R3498000: `SearchStats.replied`.
    replied: usize,
    /// R3498000: `SearchStats.worldsScored`.
    worlds_scored: usize,
    /// R4447000: `SearchStats.replaysCut`.
    replays_cut: usize,
}

/// R4070000: the world a decision's root lists its candidates and probes its prices on: `public`
/// determinized once on `AI_SEARCH.probe_seed`, never drawing from the decision's own `Rng`. `ground` is
/// `det_ground(public, seat)`.
pub fn probe_world(public: &GameState, seat: PlayerId, ground: &DetGround) -> GameState {
    determinize_on(
        public,
        seat,
        &mut Rng::new(AI_SEARCH.probe_seed, 0),
        DeterminizeOptions::default(),
        ground,
    )
}

/// R2424000, R2477000, R2564000, R2565000, R2603000, R2687000, R2688000, R3064000, R4648000: every Death
/// (a backrow card's and a hand Field Spell's among them), turn hook, Activate, attack hook, Dormant card's
/// awakening and Evolve card's evolution the seat can read, priced once on the probe world (`probe_world` of `public`) for the
/// whole decision from `counter`'s nodes; all are priced before any is passed, so no probe reads a price.
/// R4197000: and, at the seat's own prompt whose card can take a power, what each answer's power gives
/// (`prompt_activation_prices`).
pub fn decision_prices(probe: &GameState, seat: PlayerId, counter: &dyn NodeCounter) -> Prices {
    let deaths = death_prices(probe, seat, counter);
    let turn_hooks = turn_hook_prices(probe, seat, counter);
    let mut activations = activation_prices(probe, seat, counter);
    // R4197000: at the seat's own prompt that gives a card its power for the game (Heroic Power's Discover),
    // the power each answer leaves it; nothing at every other prompt and with no prompt open.
    activations.extend(prompt_activation_prices(probe, seat, counter));
    let attack_hooks = attack_hook_prices(probe, seat, counter);
    let awakenings = awaken_prices(probe, seat, counter);
    let evolutions = evolve_prices(probe, seat, counter);
    Prices {
        deaths,
        turn_hooks,
        activations,
        attack_hooks,
        awakenings,
        evolutions,
        pool_loss: None,
    }
}

/// R4797000: a sampled world as `decide` plays it: the opponent's automatic turn end is off (R82, R345),
/// as the concession check's worlds have it (`unstoppable.rs`), so the seat's End turn, or a play that ends
/// its turn, stops where the opponent's turn begins, the seat's turn log still holding the crystals it left
/// unspent (`passed_score`, `reply_score`), and the reply ends that turn itself. Without it, an opponent
/// with nothing to do ended its turn inside the seat's, whose next turn start rebuilt the log, and a line
/// that floated every crystal scored as one that spent them. With `AI_SEARCH.hold_opponent_turn` off, the
/// world as sampled.
pub fn search_world(mut world: GameState, seat: PlayerId) -> GameState {
    if AI_SEARCH.hold_opponent_turn {
        world.players[seat.opponent()].auto_end_turn = Some(false);
    }
    world
}

/// Steps 1–7 on the redacted copy; every number `stats` reads beside the counter goes in `scratch`.
/// `options` gives the budget, the wrap-up and the stream; its `should_stop` is the counter's.
/// `pool_loss` is what `decide_with_pool_loss` was given (R4188000), None for every 1v1 decision.
fn plan(
    state: &GameState,
    seat: PlayerId,
    style: &AiStyle,
    options: &mut AiOptions,
    counter: &dyn NodeCounter,
    scratch: &mut Scratch,
    pool_loss: Option<i32>,
) -> Plan {
    let budget = options.budget;
    let wrap_up = options.wrap_up;
    let rng = &mut options.rng;
    // 1. Everything below reads the redacted copy.
    let public = redact(state, seat);

    // 2. R188.
    if unanswered_draw_offer(&public, seat) {
        return Plan::immediate(
            ActionBody::AnswerDraw { accept: false },
            DecisionReason::DrawOffer,
        );
    }

    // 3. The mulligan: its own, at once, whether or not the other seat has answered (R265).
    if public.pending.is_none() && mulligan_prompt_for(&public, seat).is_some() {
        let keep = mulligan_keep_curved(&public, seat, AI_MULLIGAN.keep_max_cost, &AI_MULLIGAN_CURVE);
        return Plan::immediate(ActionBody::Mulligan { keep }, DecisionReason::Mulligan);
    }

    // 3, R1184002: a question of a sequence, or C+ #42 KY's Test's Easy problem (#2929), worked out from
    // its statement, at once (R1563005: right most of the time).
    if let Some(answer) = worked_answer_action(&public, seat, rng) {
        return Plan::immediate(answer, DecisionReason::Prompt);
    }

    // 4. Forced: a throwaway world lists the candidates without touching options.rng. Every
    // determinization of this decision starts from the same public state, so the read-only part
    // of the determinization (the pools, what each side has shown, each side's face-down
    // grants) is computed once (det_ground) and handed to each.
    let ground = det_ground(&public, seat);
    let probe = probe_world(&public, seat, &ground);
    scratch.candidates = candidate_actions(&probe, seat, Prices::NONE);
    if scratch.candidates.len() == 1
        && let Some(only) = scratch.candidates.first()
    {
        return Plan::immediate(only.clone(), DecisionReason::Forced);
    }

    // A decision's nodes cost more the wider the state is, so the searched part of the budget shrinks
    // where the state is fat (AI_SEARCH.fat_*, R186). The whole searched section (the lethal solver's
    // slice, the beam's, the replies') is drawn from one sub-counter of that size, so the replies
    // cannot spend what the beam was denied.
    let scaled = scaled_budget(budget, fat_width(&probe, scratch.candidates.len()));
    // R4788000: the lethal solver's reach (`lethal_reach`). `wide` is the searched section with room for
    // it; everything else draws from `narrow`, the scaled budget, or from `stage` below.
    let (reach, section) = lethal_reach(budget, scaled);
    let wide = create_sub_counter(counter, section);
    let narrow = create_sub_counter(&wide, scaled.nodes);
    let budget = scaled;
    let counter: &dyn NodeCounter = &narrow;

    // R2424000, R2477000, R2564000, R2688000, R3064000, R4648000: the decision's prices (`decision_prices`,
    // each Evolve card's evolution among them), from the searched section's nodes, passed to every step
    // below. R4188000: with the pool's coming loss `decide_with_pool_loss` was given.
    let prices = Prices {
        pool_loss: pool_loss.map(|loss| (seat, loss)),
        ..decision_prices(&probe, seat, counter)
    };

    // The determinizations' draws: with a style's own two (passing over lethal, picking a
    // finalist), the only draws decide ever takes from options.rng.
    let k = budget.determinizations.max(1);
    let mut dets: Vec<GameState> = Vec::new();
    for _ in 0..k {
        dets.push(search_world(
            determinize_on(&public, seat, rng, DeterminizeOptions::default(), &ground),
            seat,
        ));
    }
    scratch.determinizations = k;

    // 5. Lethal, verified on every determinization, unless the style passes over it (R1452): then the
    // search plays on kept off every line that wins this turn. Played, it is first trimmed to the
    // actions it needs (R1965000), else a lethal behind a free step that changes nothing would play that
    // step at every decision; a trim the counter stops short of a first action leaves no lethal to play.
    let mut passed_over: Option<ActionBody> = None;
    let before_lethal = counter.used();
    let found_lethal = find_lethal_reaching(&dets, seat, counter, budget.lethal_nodes, &wide, reach, &prices);
    // R4788000: what the walk spent past its scaled allowance is added to the searched section, so the beam
    // and the replies keep the nodes they had: with nothing spent past it, `stage` is `narrow`'s rest, node
    // for node.
    let extended = (counter.used() - before_lethal).saturating_sub(budget.lethal_nodes);
    let budget = SearchBudget {
        nodes: budget.nodes.saturating_add(extended),
        ..budget
    };
    let stage = create_sub_counter(&wide, budget.nodes.saturating_sub(wide.used()));
    let counter: &dyn NodeCounter = &stage;
    if let Some(lethal) = found_lethal
        && let Some(first_lethal) = lethal.first().cloned()
    {
        if passes_over_lethal(style, rng) {
            passed_over = Some(first_lethal);
        } else if let Some(trimmed) = trim_lethal(&dets, seat, lethal, counter, &prices)
            && let Some(first) = trimmed.first().cloned()
        {
            return Plan {
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
        return Plan::fallback(&scratch.candidates, None);
    };
    let root_turn = det0.turn;
    let remaining = budget.nodes.saturating_sub(counter.used());
    let finalist_count = budget.finalists.max(1);
    // What one finalist's scores on every other determinization keep back (R3696000's `keep`, below),
    // R4447000's fill slice included.
    let fill = ReplayFill::configured(budget.branching);
    // R4802000: saturating, so a budget field at its largest value ("no limit") overflows nothing; each
    // result is bounded by a `.min` or a counter's limit.
    let other_worlds = (k - 1).saturating_mul(
        budget
            .max_depth
            .saturating_add(1 + AI_REPLY.reserve_steps as usize)
            .saturating_add(fill.ceiling()),
    );
    let per_finalist =
        ((AI_SEARCH.lines_per_action * AI_REPLY.reserve_steps) as usize).saturating_add(other_worlds);
    let reserve = (remaining / 2).min(finalist_count.saturating_mul(per_finalist));
    // The beam's own slice of the counter. Its stop reason is read as the beam ends: nothing takes a
    // node through the slice afterwards. R3498000: the host's wrap-up stops this slice alone, so that the
    // reply and the other determinizations below still run until the hard stop. R4801000: it is first
    // polled once the beam has tried every move at its root (`end_root`), however many nodes those cost,
    // so a wrap-up that fired before the beam still leaves it a line, the root's End turn among them.
    // R4604014: and so is the slice's own limit, so a root move dearer than the slice still closes one.
    let before_beam = counter.used();
    let (found, beam_stopped_by) = {
        let beam_counter = create_beam_counter(counter, remaining - reserve, wrap_up);
        let found = beam_search(det0, seat, &beam_counter, budget, &prices);
        (found, beam_counter.stopped_by())
    };
    scratch.lines = found.len();

    if found.is_empty() {
        let stopped_by = if counter.stopped_by() != StoppedBy::Exhausted {
            counter.stopped_by()
        } else {
            beam_stopped_by
        };
        // R4595001: the beam spent nodes and no candidate's step fit them (a chain of expensive casts
        // each answer starts): each is stepped once, unanswered and uncharged, and the one that leaves the
        // best position is taken, a search of one ply rather than a fallback. A beam given no node at all
        // still falls back (B15).
        if counter.used() > before_beam
            && let Some((action, score)) = best_single_step(det0, seat, &scratch.candidates, &prices)
        {
            return Plan {
                score,
                ..Plan::single(action, DecisionReason::Search, Some(stopped_by))
            };
        }
        return Plan::fallback(&scratch.candidates, Some(stopped_by));
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
        finalist_count.saturating_mul(AI_SEARCH.lines_per_action as usize),
        det0,
        seat,
    );
    let hidden0 = hidden_card_ids(det0, seat);
    // R3498000: only a passed or yielded line is replied to; `reply_score` scores any other statically.
    let hands_over = |end: &GameState| {
        matches!(
            line_status(end, seat, root_turn),
            LineStatus::Passed | LineStatus::Yielded
        )
    };
    // R4791000: where the beam's slice refused a node (its budget or the host's wrap-up), a shortlisted line
    // it left open in the seat's main phase ends its turn here, on the decision's own nodes, and is
    // replied to as every other line is; a node refused there ends the replies as a refused reply does.
    let close_cut = AI_SEARCH.close_cut_lines && beam_stopped_by != StoppedBy::Exhausted;
    let mut replied: Vec<Scored<'_>> = Vec::new();
    let mut replied_count = 0;
    for &line in &shortlist {
        let closed;
        let end = if close_cut && open_in_main(&line.end, seat, root_turn) {
            match simulate(&line.end, seat, &ActionBody::EndTurn, counter, &prices) {
                None => break,
                Some(Ok(next)) => {
                    closed = next;
                    &closed
                }
                Some(Err(_)) => &line.end,
            }
        } else {
            &line.end
        };
        let Some(score) = reply_score(end, seat, root_turn, counter, &hidden0, &prices) else {
            break;
        };
        replied_count += usize::from(hands_over(end));
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
                finalist_count.saturating_mul(other_worlds),
                &hidden0,
                &prices,
            ) {
                Some(score) => {
                    replied_count += usize::from(hands_over(&line.end));
                    replied.push(Scored { line, score });
                }
                None => payoff_stop = Some(StoppedBy::Budget),
            }
        }
    }
    scratch.replied = replied_count;
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

    let mut rows: Vec<Vec<f64>> = finalists.iter().map(|entry| vec![entry.score]).collect();
    let mut scored_on: i32 = 1;
    for world in dets.iter().skip(1) {
        let hidden = hidden_card_ids(world, seat);
        let mut row: Vec<f64> = Vec::new();
        let mut cuts: usize = 0;
        for entry in &finalists {
            let Some(replay) = replay_line(
                world,
                seat,
                &entry.line.actions,
                counter,
                (!replied.is_empty()).then_some(&hidden),
                &prices,
                fill,
            ) else {
                break;
            };
            row.push(replay.score);
            cuts += usize::from(replay.cut);
        }
        // A world the counter could not finish is left out for every finalist alike.
        if row.len() < finalists.len() {
            break;
        }
        for (index, score) in row.into_iter().enumerate() {
            if let Some(scores) = rows.get_mut(index) {
                scores.push(score);
            }
        }
        scored_on += 1;
        scratch.replays_cut += cuts;
    }
    scratch.worlds_scored = usize::try_from(scored_on).unwrap_or(0);

    // R4608000: a finalist that won on determinization 0 alone is ranked by its mean on the others.
    let means = finalist_means(&rows);
    let chosen_index = pick_finalist(&means, style.temperature, rng);
    let chosen_mean = means.get(chosen_index).copied().unwrap_or(0.0);

    let chosen = finalists.get(chosen_index).map(|entry| entry.line);
    let Some((chosen, action)) =
        chosen.and_then(|line| line.actions.first().map(|action| (line, action.clone())))
    else {
        return Plan::fallback(&scratch.candidates, None);
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
    Plan {
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
    decide_in(state, seat, options, style, None)
}

/// R4188000: `decide` for a seat of a Duos duel, its threat judged against its life less `pool_loss`,
/// what its teammate's duel is about to take off the team's pool (`duos_pool_loss`).
pub fn decide_with_pool_loss(
    state: &GameState,
    seat: PlayerId,
    options: &mut AiOptions,
    pool_loss: i32,
) -> Option<Decision> {
    decide_in(state, seat, options, &AI_STYLE_EXACT, Some(pool_loss))
}

/// `decide_styled`, with the pool's coming loss `plan` prices the decision with (R4188000).
fn decide_in(
    state: &GameState,
    seat: PlayerId,
    options: &mut AiOptions,
    style: &AiStyle,
    pool_loss: Option<i32>,
) -> Option<Decision> {
    match catch_unwind(AssertUnwindSafe(|| ai_to_act(state, seat))) {
        Ok(true) => {}
        _ => return None,
    }

    let budget = options.budget;
    let counter = create_node_counter(budget.nodes, options.should_stop);
    let mut scratch = Scratch::default();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        plan(state, seat, style, options, &counter, &mut scratch, pool_loss)
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
        replays_cut: scratch.replays_cut,
        score,
    };

    // The plan's own panic is a failed simulation like the others (once in `sim_errors`, #1429), and a
    // panic: the fallback, with the stats the counter holds.
    let planned = outcome.unwrap_or_else(|payload| {
        note_sim_panic(&counter, payload.as_ref());
        Plan::fallback(&scratch.candidates, None)
    });
    let decision = Decision {
        action: planned.action,
        reason: planned.reason,
        line: planned.line,
        stats: stats(&counter, planned.score, planned.stopped_by),
        first_sim_panic: counter.first_sim_panic(),
    };
    Some(submitted(state, seat, decision, &planned.runners_up))
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

    /// R4802000: 150 nodes, 40 of them the lethal solver's.
    const SMALL_BUDGET: SearchBudget = SearchBudget {
        nodes: 150,
        lethal_nodes: 40,
        ..AI_BUDGET
    };

    #[test]
    fn r4802000_a_state_that_is_not_fat_keeps_the_callers_own_budget() {
        let reference = AI_SEARCH.fat_reference as usize;
        let small = SearchBudget {
            nodes: 30,
            lethal_nodes: 7,
            ..AI_BUDGET
        };
        let none = SearchBudget {
            lethal_nodes: 0,
            ..AI_BUDGET
        };
        for budget in [AI_BUDGET, SMALL_BUDGET, small, none] {
            assert_eq!(scaled_budget(budget, reference), budget);
        }
        let past = SearchBudget {
            nodes: 30,
            lethal_nodes: 100,
            ..AI_BUDGET
        };
        assert_eq!(scaled_budget(past, reference).lethal_nodes, 30);
    }

    #[test]
    fn r4802000_a_fat_states_floors_never_raise_the_callers_own_figures() {
        let fat = 2 * AI_SEARCH.fat_reference as usize;
        // The practice budget scales as it did.
        assert_eq!(
            scaled_budget(AI_BUDGET, fat),
            SearchBudget {
                nodes: 300,
                lethal_nodes: 75,
                ..AI_BUDGET
            }
        );
        assert_eq!(
            scaled_budget(AI_BUDGET, 1000),
            SearchBudget {
                nodes: 150,
                lethal_nodes: 60,
                ..AI_BUDGET
            }
        );
        // Below the floors, the caller's own figures.
        assert_eq!(scaled_budget(SMALL_BUDGET, fat), SMALL_BUDGET);
        let below = SearchBudget {
            nodes: 100,
            lethal_nodes: 30,
            ..AI_BUDGET
        };
        assert_eq!(scaled_budget(below, fat), below);
        let none = SearchBudget {
            lethal_nodes: 0,
            ..AI_BUDGET
        };
        assert_eq!(scaled_budget(none, fat).lethal_nodes, 0);
    }

    #[test]
    fn r4802000_a_budget_at_the_largest_values_scales_without_overflow() {
        let fat = 2 * AI_SEARCH.fat_reference as usize;
        let huge = SearchBudget {
            nodes: usize::MAX,
            lethal_nodes: usize::MAX,
            ..AI_BUDGET
        };
        let scaled = scaled_budget(huge, fat);
        assert_eq!(scaled.nodes, 300);
        assert!(scaled.lethal_nodes <= scaled.nodes, "{scaled:?}");
        assert_eq!(scaled_budget(huge, 1), huge);
    }

    #[test]
    fn r4788000_a_reach_never_passes_the_callers_own_figures() {
        let reference = AI_SEARCH.fat_reference as usize;
        // The practice budget at twice the reference: 300 nodes, 75 of them the solver's, which reaches
        // to the caller's 300 in a section of 525.
        let fat = 2 * reference;
        assert_eq!(lethal_reach(AI_BUDGET, scaled_budget(AI_BUDGET, fat)), (300, 525));
        let none = SearchBudget {
            lethal_nodes: 0,
            ..AI_BUDGET
        };
        let past = SearchBudget {
            nodes: 200,
            lethal_nodes: 400,
            ..AI_BUDGET
        };
        let huge = SearchBudget {
            nodes: usize::MAX,
            lethal_nodes: usize::MAX,
            ..AI_BUDGET
        };
        for budget in [AI_BUDGET, SMALL_BUDGET, none, past, huge] {
            for width in [reference, fat, 1000] {
                let scaled = scaled_budget(budget, width);
                let (reach, section) = lethal_reach(budget, scaled);
                assert!(reach >= scaled.lethal_nodes, "{budget:?} at {width}");
                assert!(
                    reach <= budget.lethal_nodes.min(budget.nodes),
                    "{budget:?} at {width}"
                );
                assert!(
                    section >= scaled.nodes && section <= budget.nodes,
                    "{budget:?} at {width}"
                );
                if width <= reference || budget.lethal_nodes == 0 {
                    assert_eq!(
                        (reach, section),
                        (scaled.lethal_nodes, scaled.nodes),
                        "{budget:?} at {width}"
                    );
                }
            }
        }
    }

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
        let mut scratch = Scratch::default();
        let mut options = AiOptions::with_budget(Rng::new("decide-1575", 0), AI_BUDGET);
        let planned = plan(
            state,
            AI,
            &AI_STYLE_EXACT,
            &mut options,
            &counter,
            &mut scratch,
            None,
        );
        let decision = Decision {
            action: planned.action,
            reason: planned.reason,
            line: planned.line,
            stats: SearchStats {
                score: planned.score,
                ..SearchStats::default()
            },
            first_sim_panic: None,
        };
        (decision, planned.runners_up)
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
                player: None,
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

    #[test]
    fn r4595001_an_unsearched_decision_takes_the_candidate_whose_one_step_evaluates_best() {
        let s = board(false);
        let state = s.state();
        let candidates = candidate_actions(state, AI, Prices::NONE);
        let menace = s.card(MENACE).id.clone();
        let (action, score) =
            best_single_step(state, AI, &candidates, Prices::NONE).expect("a step is accepted");
        assert!(
            matches!(&action, ActionBody::Play { instance_id, .. } if *instance_id == menace),
            "the 9/9 Taunt beats the 4/4 and passing: {action:?}"
        );
        // Its score is the best of every candidate's one step.
        let best = candidates
            .iter()
            .filter_map(|candidate| {
                let stepped =
                    reduce_unpiloted(state, &Action::new(candidate.clone(), AI, "check".to_string()));
                stepped
                    .error
                    .is_none()
                    .then(|| evaluate_priced(&stepped.state, AI, NextSwing::Enemy, &AI_EVAL, Prices::NONE))
            })
            .fold(f64::MIN, f64::max);
        assert_eq!(score, best);
        // Every candidate refused: nothing to take, so the decision falls back.
        let mut gone = action.clone();
        if let ActionBody::Play { instance_id, .. } = &mut gone {
            *instance_id = "no-such-card".to_string();
        }
        let refused = vec![gone];
        assert_eq!(best_single_step(state, AI, &refused, Prices::NONE), None);
    }
}
