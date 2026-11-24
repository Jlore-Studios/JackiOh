//! Audit, area `fuzz`: invariants checked at every AI decision of seeded games, on the configs the
//! gates hold fixed (every card dealt, each set leaned on, handicaps, setup inputs, skill, style).
//!
//! The harness drives the loop `play_match` drives (`seat_to_act`, the controller, `reduce`) so that
//! it holds the true state at every decision.

#![allow(dead_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use indexmap::{IndexMap, IndexSet};
use jackioh_ai::{
    AI_BUDGET, AI_EVAL, AI_SKILL_MAX, AI_STYLE_EXACT, AiDeckOptions, AiOptions, AiStyle, Decision,
    DecisionReason, MatchSetup, NextSwing, NodeCounter, Prices, SearchBudget, build_ai_deck,
    create_node_counter, decide_styled, evaluate, find_lethal, greedy_action, lethal_hint, panic_message,
    plays_best, random_action, redact, skill_miss_action, unstoppable_lethal,
};
use jackioh_engine::config::AI_DIFFICULTY;
use jackioh_engine::view_for::view_for;
use jackioh_engine::{
    Action, ActionBody, ActionType, CardInstance, CreateGameOptions, DeckMark, FoldArgs, GameState, Handicap,
    LastBoardEntry, PLAYER_IDS, PerPlayer, PerPlayerOpt, Phase, PlayerId, Rng, SetName, begin_game,
    create_game, fold, hash_state, legal_actions, reduce, seat_to_act,
};
use serde_json::Value;

// ---------------------------------------------------------------------------------------------
// Specs
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Ctl {
    Ai {
        budget: SearchBudget,
        skill: f64,
        style: AiStyle,
    },
    Greedy,
    Random,
}

impl Ctl {
    fn full(budget: SearchBudget) -> Ctl {
        Ctl::Ai {
            budget,
            skill: AI_SKILL_MAX,
            style: AI_STYLE_EXACT,
        }
    }
    fn is_exact_ai(&self) -> bool {
        matches!(self, Ctl::Ai { skill, style, .. } if *skill >= AI_SKILL_MAX && *style == AI_STYLE_EXACT)
    }
}

#[derive(Clone, Debug)]
struct Spec {
    label: String,
    seed: String,
    decks: (Vec<String>, Vec<String>),
    handicaps: Option<PerPlayerOpt<Handicap>>,
    setup: MatchSetup,
    ctl: PerPlayer<Ctl>,
    max_actions: usize,
}

/// A tenth of the shipped budget's nodes, its shape otherwise but for two worlds.
const SMALL: SearchBudget = SearchBudget {
    nodes: 150,
    lethal_nodes: 40,
    determinizations: 2,
    ..AI_BUDGET
};

/// A quarter of it, on the shipped three worlds.
const MID: SearchBudget = SearchBudget {
    nodes: 300,
    lethal_nodes: 75,
    ..AI_BUDGET
};

fn deck(seed: &str, seat: PlayerId, handicap: &Handicap, lean: Option<SetName>) -> Vec<String> {
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
        handicap.deck_size,
        &AiDeckOptions {
            mana_cap: Some(handicap.mana_cap),
            banned: Some(Vec::new()),
            gated_sets: Some(Vec::new()),
            lean_set: lean,
            ..AiDeckOptions::default()
        },
    )
}

fn spec(
    label: &str,
    n: usize,
    handicaps: PerPlayer<Handicap>,
    lean: PerPlayer<Option<SetName>>,
    ctl: PerPlayer<Ctl>,
) -> Spec {
    let seed = format!("audit-fuzz:{label}:{n}");
    let decks = (
        deck(&seed, PlayerId::P1, &handicaps.p1, lean.p1),
        deck(&seed, PlayerId::P2, &handicaps.p2, lean.p2),
    );
    Spec {
        label: format!("{label}:{n}"),
        seed,
        decks,
        handicaps: Some(PerPlayerOpt {
            p1: Some(handicaps.p1),
            p2: Some(handicaps.p2),
        }),
        setup: MatchSetup::default(),
        ctl,
        max_actions: 3000,
    }
}

const LEANS: &[(&str, SetName)] = &[
    ("splus", SetName::StinkyscapePlus),
    ("tg", SetName::TrainingGrounds),
    ("stinky", SetName::Stinkyscape),
    ("mplus", SetName::MeditativePlus),
    ("med", SetName::Meditative),
    ("cplus", SetName::ClassicPlus),
    ("classic", SetName::Classic),
    ("core", SetName::Core),
];

/// The spec a family name and a game number give.
fn spec_of(family: &str, n: usize) -> Spec {
    let easy = AI_DIFFICULTY.easy;
    let both = |h: Handicap| PerPlayer::new(h, h);
    let odd = n % 2 == 1;
    if let Some(rest) = family.strip_prefix("lean-") {
        // lean-<set>[-mid|-rand|-greedy]
        let (name, mode) = match rest.split_once('-') {
            Some((name, mode)) => (name, mode),
            None => (rest, "ai"),
        };
        let set = LEANS
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, set)| *set)
            .expect("a known set");
        let budget = if mode == "mid" { MID } else { SMALL };
        let other = match mode {
            "rand" => Ctl::Random,
            "greedy" => Ctl::Greedy,
            _ => Ctl::full(budget),
        };
        let ctl = if odd {
            PerPlayer::new(Ctl::full(budget), other)
        } else {
            PerPlayer::new(other, Ctl::full(budget))
        };
        return spec(family, n, both(easy), PerPlayer::new(Some(set), Some(set)), ctl);
    }
    match family {
        // Every card, no lean: AI against AI.
        "all" => spec(
            family,
            n,
            both(easy),
            PerPlayer::new(None, None),
            PerPlayer::new(Ctl::full(SMALL), Ctl::full(SMALL)),
        ),
        "all-mid" => spec(
            family,
            n,
            both(easy),
            PerPlayer::new(None, None),
            PerPlayer::new(Ctl::full(MID), Ctl::full(MID)),
        ),
        // The Hard and Medium tiers against Easy, the tier alternating seats.
        "hard" | "medium" => {
            let tier = if family == "hard" {
                AI_DIFFICULTY.hard
            } else {
                AI_DIFFICULTY.medium
            };
            let handicaps = if odd {
                PerPlayer::new(tier, easy)
            } else {
                PerPlayer::new(easy, tier)
            };
            let lean = LEANS[n % LEANS.len()].1;
            spec(
                family,
                n,
                handicaps,
                PerPlayer::new(Some(lean), Some(lean)),
                PerPlayer::new(Ctl::full(SMALL), Ctl::full(SMALL)),
            )
        }
        // A skill between the ends and a style that strays, against the exact AI.
        "style" => {
            let stray = Ctl::Ai {
                budget: SMALL,
                skill: 0.6,
                style: AiStyle {
                    temperature: 3.0,
                    missed_lethal_chance: 0.5,
                },
            };
            let ctl = if odd {
                PerPlayer::new(stray, Ctl::full(SMALL))
            } else {
                PerPlayer::new(Ctl::full(SMALL), stray)
            };
            let lean = LEANS[n % LEANS.len()].1;
            spec(family, n, both(easy), PerPlayer::new(Some(lean), Some(lean)), ctl)
        }
        // Every setup input: both decks dealt, last boards, Glitch boards, deck marks.
        "setup" => {
            let lean = LEANS[n % LEANS.len()].1;
            let mut out = spec(
                family,
                n,
                both(easy),
                PerPlayer::new(Some(lean), Some(lean)),
                PerPlayer::new(Ctl::full(SMALL), Ctl::full(SMALL)),
            );
            let board = |deck: &[String]| -> Vec<LastBoardEntry> {
                deck.iter()
                    .take(4)
                    .enumerate()
                    .map(|(at, def_id)| LastBoardEntry {
                        def_id: def_id.clone(),
                        radiant: at % 2 == 0,
                    })
                    .collect()
            };
            let marks = |from: usize| -> Vec<DeckMark> {
                (0..8)
                    .map(|k| DeckMark {
                        index: (from + 3 * k) % 25,
                        radiant: k % 2 == 0,
                        upgrades: (k % 3) as i32,
                    })
                    .collect()
            };
            out.setup = MatchSetup {
                dealt: Some(match n % 3 {
                    0 => vec![PlayerId::P1, PlayerId::P2],
                    1 => vec![PlayerId::P1],
                    _ => vec![PlayerId::P2],
                }),
                last_boards: Some((board(&out.decks.1), board(&out.decks.0))),
                glitch_boards: Some((board(&out.decks.0), board(&out.decks.1))),
                deck_marks: Some(PerPlayerOpt {
                    p1: Some(marks(0)),
                    p2: Some(marks(1)),
                }),
            };
            out
        }
        other => panic!("unknown family {other}"),
    }
}

// ---------------------------------------------------------------------------------------------
// One game
// ---------------------------------------------------------------------------------------------

/// One AI decision of a game: where it was taken and what the controller's stream was before it.
struct Rec {
    at: usize,
    seat: PlayerId,
    before: Rng,
    budget: SearchBudget,
    style: AiStyle,
    decision: Decision,
    cursor: u32,
}

struct Played {
    s0: GameState,
    log: Vec<Action>,
    recs: Vec<Rec>,
    /// Each deck card's first state index outside its owner's hand and library.
    left_hidden: PerPlayer<IndexMap<String, usize>>,
    /// Each deck card's first state index outside its owner's library.
    left_library: PerPlayer<IndexMap<String, usize>>,
    deck_ids: PerPlayer<Vec<String>>,
    end: GameState,
}

#[derive(Default, Debug)]
struct Tally {
    games: usize,
    finished: usize,
    actions: usize,
    decisions: usize,
    searched: usize,
    misses: usize,
    sim_errors: usize,
    fallbacks_budget: usize,
    determinism_checks: usize,
    lethal_claims: usize,
    lethal_claims_kept: usize,
    hints: usize,
    hints_true: usize,
    unstoppable: usize,
    unstoppable_true: usize,
    twin_windows: usize,
    twin_windows_diverged: usize,
    twin_steps: usize,
    twin_decisions: usize,
    twin_changed_cards: usize,
    evaluations: usize,
    max_nodes_over: usize,
    bump_checks: usize,
}

struct Flags {
    /// Decide again every this many searched decisions (0: never).
    determinism_every: usize,
    hints: bool,
    unstoppable: bool,
    twins: bool,
    window: usize,
    /// Ids minted unseen before each searched decision is taken again (0: never).
    bump: usize,
}

struct Claim {
    seat: PlayerId,
    turn: i32,
    at: usize,
    line: Vec<ActionBody>,
    trace: Vec<String>,
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn note_exposure(
    state: &GameState,
    at: usize,
    deck_ids: &PerPlayer<Vec<String>>,
    left_hidden: &mut PerPlayer<IndexMap<String, usize>>,
    left_library: &mut PerPlayer<IndexMap<String, usize>>,
) {
    for player in PLAYER_IDS {
        let side = &state.players[player];
        let library: IndexSet<&str> = side.library.iter().map(|card| card.id.as_str()).collect();
        let hand: IndexSet<&str> = side.hand.iter().map(|card| card.id.as_str()).collect();
        for id in &deck_ids[player] {
            let in_library = library.contains(id.as_str());
            if !in_library && !left_library[player].contains_key(id) {
                left_library[player].insert(id.clone(), at);
            }
            if !in_library && !hand.contains(id.as_str()) && !left_hidden[player].contains_key(id) {
                left_hidden[player].insert(id.clone(), at);
            }
        }
    }
}

fn short(value: &impl serde::Serialize) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    if text.len() > 300 {
        format!("{}…", &text[..300])
    } else {
        text
    }
}

fn replacement(state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    let legal = legal_actions(state, seat);
    legal
        .iter()
        .find(|action| matches!(action, ActionBody::EndTurn))
        .or_else(|| {
            legal.iter().find(|action| {
                matches!(action.action_type(), ActionType::Answer | ActionType::Mulligan)
            })
        })
        .or_else(|| {
            legal.iter().find(|action| {
                !matches!(
                    action.action_type(),
                    ActionType::Concede | ActionType::OfferDraw | ActionType::Emote
                )
            })
        })
        .cloned()
}

fn create(spec: &Spec) -> GameState {
    create_game(&CreateGameOptions {
        seed: spec.seed.clone(),
        decks: spec.decks.clone(),
        handicaps: spec.handicaps.clone(),
        dealt: spec.setup.dealt.clone(),
        last_boards: spec.setup.last_boards.clone(),
        glitch_boards: spec.setup.glitch_boards.clone(),
        deck_marks: spec.setup.deck_marks.clone(),
        ..CreateGameOptions::default()
    })
}

/// Both seats played by the AI at the shipped budget from `state` until `stop` or the game's end.
fn side_game(
    state: &GameState,
    tag: &str,
    stop: &dyn Fn(&GameState) -> bool,
    trace: &mut Vec<String>,
) -> GameState {
    let mut current = state.clone();
    let mut rngs = PerPlayer::new(Rng::new(&format!("{tag}:p1"), 0), Rng::new(&format!("{tag}:p2"), 0));
    for step in 0..120 {
        if current.result.is_some() || stop(&current) {
            break;
        }
        let seat = seat_to_act(&current).unwrap_or(current.active);
        let mut options = AiOptions::with_budget(rngs[seat].clone(), AI_BUDGET);
        let Some(decision) = decide_styled(&current, seat, &mut options, &AI_STYLE_EXACT) else {
            trace.push(format!("{}: no decision", seat.as_str()));
            break;
        };
        rngs[seat] = options.rng.clone();
        trace.push(format!(
            "{}:{}:{}",
            seat.as_str(),
            decision.reason.as_str(),
            short(&decision.action)
        ));
        let result = reduce(&current, &Action::new(decision.action.clone(), seat, format!("{tag}{step}")));
        if result.error.is_some() {
            trace.push(format!("refused: {:?}", result.error));
            break;
        }
        current = result.state;
    }
    current
}

/// What the hint's own solver found on its own worlds, and how that line fares on the true state.
fn hint_detail(state: &GameState, seat: PlayerId) -> String {
    use jackioh_ai::{
        DeterminizeOptions, LETHAL_HINT_DETERMINIZATIONS, LETHAL_HINT_NODES, LETHAL_HINT_SEED, det_ground,
        determinize_on,
    };
    let public = redact(state, seat);
    let ground = det_ground(&public, seat);
    let mut rng = Rng::new(LETHAL_HINT_SEED, 0);
    let worlds: Vec<GameState> = (0..LETHAL_HINT_DETERMINIZATIONS)
        .map(|_| determinize_on(&public, seat, &mut rng, DeterminizeOptions::default(), &ground))
        .collect();
    let counter = create_node_counter(LETHAL_HINT_NODES, None);
    let line = find_lethal(&worlds, seat, &counter, LETHAL_HINT_NODES, Prices::NONE);
    let mut out = format!("hint line ({} nodes): {}", counter.used(), short(&line));
    let Some(line) = line else {
        return out;
    };
    let mut current = state.clone();
    for (step, action) in line.iter().enumerate() {
        if current.result.is_some() {
            break;
        }
        let result = reduce(&current, &Action::new(action.clone(), seat, format!("hint-line{step}")));
        if let Some(error) = result.error {
            out.push_str(&format!(" | true state refuses step {step}: {error}"));
            break;
        }
        let kinds: Vec<String> = result
            .events
            .iter()
            .filter_map(|event| {
                serde_json::to_value(event)
                    .ok()
                    .and_then(|value| value.get("type").and_then(Value::as_str).map(str::to_string))
            })
            .collect();
        out.push_str(&format!(" | step {step}: {}", kinds.join(",")));
        current = result.state;
    }
    out.push_str(&format!(" | on the true state: {:?}", current.result));
    let opp = seat.opponent();
    let backrow: Vec<String> = state.players[opp]
        .backrow
        .iter()
        .flatten()
        .map(|card| format!("{}:{}:faceUp={:?}", card.id, card.def_id, card.face_up))
        .collect();
    out.push_str(&format!(" | opp backrow {backrow:?} opp life {:?}", state.players[opp].hero));
    let mut options = AiOptions::with_budget(Rng::new("hint:p1", 0), AI_BUDGET);
    if seat == PlayerId::P2 {
        options.rng = Rng::new("hint:p2", 0);
    }
    if let Some(decision) = decide_styled(state, seat, &mut options, &AI_STYLE_EXACT) {
        out.push_str(&format!(" | decide: {} {:?}", decision.reason.as_str(), decision.stats));
    }
    out
}

fn won_by(state: &GameState, seat: PlayerId) -> bool {
    state
        .result
        .as_ref()
        .is_some_and(|result| result.winner.player() == Some(seat))
}

fn play(spec: &Spec, flags: &Flags, tally: &mut Tally, problems: &mut Vec<String>) -> Played {
    let created = create(spec);
    let deck_ids = PerPlayer::new(
        ids_of(&created.players.p1.library),
        ids_of(&created.players.p2.library),
    );
    let s0 = begin_game(&created).state;
    let mut state = s0.clone();
    let mut rngs = PerPlayer::new(
        Rng::new(&format!("{}:ctl:p1", spec.seed), 0),
        Rng::new(&format!("{}:ctl:p2", spec.seed), 0),
    );
    let mut log: Vec<Action> = Vec::new();
    let mut recs: Vec<Rec> = Vec::new();
    let mut left_hidden = PerPlayer::new(IndexMap::new(), IndexMap::new());
    let mut left_library = PerPlayer::new(IndexMap::new(), IndexMap::new());
    let mut claim: Option<Claim> = None;
    let mut turn_seen: Option<(i32, PlayerId)> = None;
    let mut flagged: Option<(PlayerId, i32, usize)> = None;
    let mut searched_here = 0usize;
    let here = |at: usize, state: &GameState| format!("{} seed={} at={} turn={}", spec.label, spec.seed, at, state.turn);

    loop {
        let at = log.len();
        note_exposure(&state, at, &deck_ids, &mut left_hidden, &mut left_library);
        if state.result.is_some() || at >= spec.max_actions {
            break;
        }
        let seat = seat_to_act(&state).unwrap_or(state.active);

        // Property 7: the evaluation is a finite number on every state met.
        for player in PLAYER_IDS {
            for next in [NextSwing::Enemy, NextSwing::Seat] {
                tally.evaluations += 1;
                match catch_unwind(AssertUnwindSafe(|| evaluate(&state, player, next, &AI_EVAL))) {
                    Ok(score) if score.is_finite() => {}
                    Ok(score) => problems.push(format!("[P7 evaluate] {} {score} for {}", here(at, &state), player.as_str())),
                    Err(payload) => problems.push(format!(
                        "[P7 evaluate panicked] {} for {}: {}",
                        here(at, &state),
                        player.as_str(),
                        panic_message(payload.as_ref())
                    )),
                }
            }
        }

        // The start of a seat's main phase: the hint and the concession check.
        let own_main = state.pending.is_none() && state.active == seat && state.phase == Phase::Main;
        if own_main && turn_seen != Some((state.turn, seat)) {
            turn_seen = Some((state.turn, seat));
            if let Some((doomed, turn, flagged_at)) = flagged
                && doomed == seat
                && state.turn > turn
            {
                problems.push(format!(
                    "[P4c unstoppable-real] {}: flagged at {} (turn {}) yet {} starts another turn alive; controllers {:?}",
                    here(at, &state),
                    flagged_at,
                    turn,
                    seat.as_str(),
                    spec.ctl
                ));
                flagged = None;
            }
            if flags.hints {
                tally.hints += 1;
                if lethal_hint(&state, seat, None) {
                    tally.hints_true += 1;
                    let turn = state.turn;
                    let mut trace = Vec::new();
                    let end = side_game(
                        &state,
                        "hint",
                        &|current: &GameState| {
                            current.turn != turn || (current.pending.is_none() && current.active != seat)
                        },
                        &mut trace,
                    );
                    if !won_by(&end, seat) {
                        problems.push(format!(
                            "[P4b hint] {}: lethal_hint true for {} yet the AI at AI_BUDGET does not win this turn: {:?} || {}",
                            here(at, &state),
                            seat.as_str(),
                            trace,
                            hint_detail(&state, seat)
                        ));
                    }
                }
            }
            if flags.unstoppable {
                tally.unstoppable += 1;
                if unstoppable_lethal(&state, seat, None) {
                    tally.unstoppable_true += 1;
                    flagged = Some((seat, state.turn, at));
                    let opp = seat.opponent();
                    let mut trace = Vec::new();
                    // The seat's turn by the AI at the shipped budget, to the opponent's main phase.
                    let handed = side_game(
                        &state,
                        "doom",
                        &|current: &GameState| {
                            current.pending.is_none() && current.active == opp && current.phase == Phase::Main
                        },
                        &mut trace,
                    );
                    if handed.result.is_some() {
                        if !won_by(&handed, opp) {
                            problems.push(format!(
                                "[P4c unstoppable] {}: flagged for {} yet its own turn ends the game otherwise: {:?} {:?}",
                                here(at, &state),
                                seat.as_str(),
                                handed.result,
                                trace
                            ));
                        }
                    } else {
                        let limit = 6000;
                        let counter = create_node_counter(limit, None);
                        let found = find_lethal(std::slice::from_ref(&handed), opp, &counter, limit, Prices::NONE);
                        if found.is_none() {
                            let turn = handed.turn;
                            let mut reply = Vec::new();
                            let end = side_game(
                                &handed,
                                "doom-reply",
                                &|current: &GameState| current.turn != turn,
                                &mut reply,
                            );
                            problems.push(format!(
                                "[P4c unstoppable] {}: flagged for {} yet after the AI's turn {:?} the solver finds no lethal for {} on the true state in {} of {} nodes; the AI's reply {} it: {:?}",
                                here(at, &state),
                                seat.as_str(),
                                trace,
                                opp.as_str(),
                                counter.used(),
                                limit,
                                if won_by(&end, opp) { "still wins" } else { "does not win" },
                                reply
                            ));
                        }
                    }
                }
            }
        }

        let ctl = spec.ctl[seat];
        let mut reason: Option<DecisionReason> = None;
        let chosen: Option<ActionBody> = match ctl {
            Ctl::Random => random_action(&state, seat, &mut rngs[seat]),
            Ctl::Greedy => greedy_action(&state, seat, &mut rngs[seat]),
            Ctl::Ai { budget, skill, style } => {
                if !plays_best(skill, &mut rngs[seat]) {
                    tally.misses += 1;
                    let rng = &mut rngs[seat];
                    match catch_unwind(AssertUnwindSafe(|| skill_miss_action(&state, seat, rng))) {
                        Ok(action) => action,
                        Err(payload) => {
                            problems.push(format!(
                                "[P5 skill miss panicked] {}: {}",
                                here(at, &state),
                                panic_message(payload.as_ref())
                            ));
                            None
                        }
                    }
                } else {
                    let before = rngs[seat].clone();
                    let mut options = AiOptions::with_budget(before.clone(), budget);
                    let outcome =
                        catch_unwind(AssertUnwindSafe(|| decide_styled(&state, seat, &mut options, &style)));
                    match outcome {
                        Err(payload) => {
                            problems.push(format!(
                                "[P5 decide panicked] {}: {}",
                                here(at, &state),
                                panic_message(payload.as_ref())
                            ));
                            None
                        }
                        Ok(None) => {
                            problems.push(format!("[P5 decide none] {}", here(at, &state)));
                            None
                        }
                        Ok(Some(decision)) => {
                            let cursor = options.rng.cursor();
                            rngs[seat] = options.rng.clone();
                            tally.decisions += 1;
                            tally.sim_errors += decision.stats.sim_errors;
                            reason = Some(decision.reason);
                            // Property 3: the nodes a decision reports against its budget.
                            if decision.stats.nodes > budget.nodes {
                                problems.push(format!(
                                    "[P3 budget] {}: {} nodes of {}: {:?}",
                                    here(at, &state),
                                    decision.stats.nodes,
                                    budget.nodes,
                                    decision.stats
                                ));
                            }
                            if !decision.stats.score.is_finite() {
                                problems.push(format!(
                                    "[P7 score] {}: {:?}",
                                    here(at, &state),
                                    decision.stats
                                ));
                            }
                            // Property 5.
                            if decision.stats.sim_panics > 0 {
                                problems.push(format!(
                                    "[P5 sim panic] {}: {} panics, first {:?}; action {}",
                                    here(at, &state),
                                    decision.stats.sim_panics,
                                    decision.first_sim_panic,
                                    short(&decision.action)
                                ));
                            }
                            if decision.reason == DecisionReason::Fallback {
                                let by_budget = decision.stats.stopped_by == jackioh_ai::StoppedBy::Budget;
                                if by_budget && decision.stats.sim_panics == 0 {
                                    tally.fallbacks_budget += 1;
                                } else {
                                    problems.push(format!(
                                        "[P5 fallback] {}: {:?} action {}",
                                        here(at, &state),
                                        decision.stats,
                                        short(&decision.action)
                                    ));
                                }
                            }
                            // Property 6: redact is a fixed point.
                            let once = redact(&state, seat);
                            let twice = redact(&once, seat);
                            if once != twice {
                                let diff = diff_values(
                                    &serde_json::to_value(&once).unwrap_or_default(),
                                    &serde_json::to_value(&twice).unwrap_or_default(),
                                );
                                problems.push(format!(
                                    "[P6 redact twice] {} seat {}: {:?}",
                                    here(at, &state),
                                    seat.as_str(),
                                    diff
                                ));
                            }
                            // Property 2: the same state, stream and budget give the same decision.
                            if decision.stats.nodes > 0 {
                                tally.searched += 1;
                                searched_here += 1;
                                if flags.determinism_every > 0 && searched_here % flags.determinism_every == 0 {
                                    tally.determinism_checks += 1;
                                    let mut again = AiOptions::with_budget(before.clone(), budget);
                                    let second = decide_styled(&state, seat, &mut again, &style);
                                    if second.as_ref() != Some(&decision) || again.rng.cursor() != cursor {
                                        problems.push(format!(
                                            "[P2 determinism] {}: first {} {:?} cursor {}, second {} cursor {}",
                                            here(at, &state),
                                            short(&decision.action),
                                            decision.stats,
                                            cursor,
                                            short(&second),
                                            again.rng.cursor()
                                        ));
                                    }
                                }
                            }
                            // The redacted state keeps `next_id`: the same state with two more ids minted
                            // where the seat cannot see (the other seat drew S+ #17 Smoothie).
                            if flags.bump > 0 && decision.stats.nodes > 0 {
                                tally.bump_checks += 1;
                                let mut bumped = state.clone();
                                bumped.next_id += flags.bump as u32;
                                let mut again = AiOptions::with_budget(before.clone(), budget);
                                let second = decide_styled(&bumped, seat, &mut again, &style);
                                if second.as_ref() != Some(&decision) || again.rng.cursor() != cursor {
                                    problems.push(format!(
                                        "[P1 nextId] {}: next_id {} +{}: first {} {:?} cursor {}, second {} cursor {}",
                                        here(at, &state),
                                        state.next_id,
                                        flags.bump,
                                        short(&decision.action),
                                        decision.stats,
                                        cursor,
                                        short(&second),
                                        again.rng.cursor()
                                    ));
                                }
                            }
                            // Property 4: a lethal the solver proved.
                            if decision.reason == DecisionReason::Lethal && ctl.is_exact_ai() {
                                let fresh = !claim
                                    .as_ref()
                                    .is_some_and(|open| open.seat == seat && open.turn == state.turn);
                                if fresh {
                                    tally.lethal_claims += 1;
                                    claim = Some(Claim {
                                        seat,
                                        turn: state.turn,
                                        at,
                                        line: decision.line.clone(),
                                        trace: Vec::new(),
                                    });
                                }
                            }
                            let action = decision.action.clone();
                            recs.push(Rec {
                                at,
                                seat,
                                before,
                                budget,
                                style,
                                decision,
                                cursor,
                            });
                            Some(action)
                        }
                    }
                }
            }
        };

        let Some(chosen) = chosen.or_else(|| {
            problems.push(format!(
                "[P5 no action] {}: controller {:?} gave none",
                here(at, &state),
                ctl
            ));
            replacement(&state, seat)
        }) else {
            break;
        };

        let mut action = Action::new(chosen.clone(), seat, format!("m{at}"));
        let mut result = match catch_unwind(AssertUnwindSafe(|| reduce(&state, &action))) {
            Ok(result) => result,
            Err(payload) => {
                problems.push(format!(
                    "[P5 reduce panicked] {}: {} on {}",
                    here(at, &state),
                    panic_message(payload.as_ref()),
                    short(&chosen)
                ));
                break;
            }
        };
        if let Some(error) = result.error.clone() {
            problems.push(format!(
                "[P5 refused] {}: {:?} ({:?}) sent {} refused: {}",
                here(at, &state),
                ctl,
                reason,
                short(&chosen),
                error
            ));
            let Some(other) = replacement(&state, seat) else {
                break;
            };
            action = Action::new(other, seat, format!("m{at}"));
            result = reduce(&state, &action);
            if result.error.is_some() {
                break;
            }
        }
        if let Some(open) = claim.as_mut() {
            let kinds: Vec<String> = result
                .events
                .iter()
                .filter_map(|event| {
                    serde_json::to_value(event)
                        .ok()
                        .and_then(|value| value.get("type").and_then(Value::as_str).map(str::to_string))
                })
                .collect();
            open.trace.push(format!(
                "{}:{}:{} -> {}",
                seat.as_str(),
                reason.map(|reason| reason.as_str()).unwrap_or("-"),
                short(&action.body),
                kinds.join(",")
            ));
        }
        log.push(action);
        state = result.state;
        tally.actions += 1;

        if let Some(open) = &claim {
            let over = state.result.is_some();
            let passed = state.turn != open.turn || (state.pending.is_none() && state.active != open.seat);
            if over || passed {
                if won_by(&state, open.seat) {
                    tally.lethal_claims_kept += 1;
                } else {
                    problems.push(format!(
                        "[P4a lethal] {}: {} claimed lethal at {} (turn {}) with line {} yet {}: {:?}",
                        here(log.len(), &state),
                        open.seat.as_str(),
                        open.at,
                        open.turn,
                        short(&open.line),
                        if over { format!("the game ended {:?}", state.result) } else { "the turn passed".to_string() },
                        open.trace
                    ));
                }
                claim = None;
            }
        }
    }

    tally.games += 1;
    if state.result.is_some() {
        tally.finished += 1;
    } else {
        problems.push(format!(
            "[P5 no result] {} seed={}: {} actions, turn {}",
            spec.label,
            spec.seed,
            log.len(),
            state.turn
        ));
    }
    // Property 5: the log folds to the game's hash.
    let folded = fold(&FoldArgs {
        seed: spec.seed.clone(),
        decks: spec.decks.clone(),
        log: log.clone(),
        handicaps: spec.handicaps.clone(),
        dealt: spec.setup.dealt.clone(),
        last_boards: spec.setup.last_boards.clone(),
        glitch_boards: spec.setup.glitch_boards.clone(),
        deck_marks: spec.setup.deck_marks.clone(),
        ..FoldArgs::default()
    });
    if !folded.errors.is_empty() || hash_state(&folded.state) != hash_state(&state) {
        problems.push(format!(
            "[P5 fold] {} seed={}: {} errors, hash {}",
            spec.label,
            spec.seed,
            folded.errors.len(),
            hash_state(&folded.state) == hash_state(&state)
        ));
    }

    Played {
        s0,
        log,
        recs,
        left_hidden,
        left_library,
        deck_ids,
        end: state,
    }
}

// ---------------------------------------------------------------------------------------------
// Property 1: the twin worlds
// ---------------------------------------------------------------------------------------------

/// The paths at which two JSON values differ, at most `MAX_DIFFS` of them.
const MAX_DIFFS: usize = 12;

fn diff_values(a: &Value, b: &Value) -> Vec<String> {
    let mut out = Vec::new();
    diff_into(a, b, String::new(), &mut out);
    out
}

fn diff_into(a: &Value, b: &Value, path: String, out: &mut Vec<String>) {
    if out.len() >= MAX_DIFFS || a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: IndexSet<&String> = x.keys().chain(y.keys()).collect();
            for key in keys {
                let next = format!("{path}.{key}");
                match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => diff_into(p, q, next, out),
                    (p, q) => {
                        if out.len() < MAX_DIFFS {
                            out.push(format!("{next}: {} vs {}", short(&p), short(&q)));
                        }
                    }
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (at, (p, q)) in x.iter().zip(y).enumerate() {
                diff_into(p, q, format!("{path}[{at}]"), out);
            }
        }
        _ => out.push(format!("{path}: {} vs {}", short(a), short(b))),
    }
}

fn card_mut<'a>(state: &'a mut GameState, owner: PlayerId, id: &str) -> Option<&'a mut CardInstance> {
    let side = &mut state.players[owner];
    side.hand
        .iter_mut()
        .chain(side.library.iter_mut())
        .find(|card| card.id == id)
}

/// Permutes what the cards `ids` of `owner`'s hand and library are, each id staying where it lies.
/// Answers how many ids now name another definition or face.
fn permute_contents(state: &mut GameState, owner: PlayerId, ids: &[String], rng: &mut Rng) -> usize {
    let side = &state.players[owner];
    let cards: Vec<CardInstance> = side
        .hand
        .iter()
        .chain(side.library.iter())
        .filter(|card| ids.contains(&card.id))
        .cloned()
        .collect();
    let shuffled = rng.shuffle(&cards);
    let mut changed = 0;
    for (target, source) in cards.iter().zip(shuffled) {
        if target.def_id != source.def_id || target.radiant != source.radiant {
            changed += 1;
        }
        if let Some(slot) = card_mut(state, owner, &target.id) {
            let mut next = source.clone();
            next.id = target.id.clone();
            next.zone = target.zone.clone();
            *slot = next;
        }
    }
    changed
}

/// Permutes where the cards `ids` lie in `owner`'s library, among the places they hold.
fn permute_positions(state: &mut GameState, owner: PlayerId, ids: &[String], rng: &mut Rng) {
    let library = &mut state.players[owner].library;
    let places: Vec<usize> = library
        .iter()
        .enumerate()
        .filter(|(_, card)| ids.contains(&card.id))
        .map(|(at, _)| at)
        .collect();
    let cards: Vec<CardInstance> = places.iter().map(|&at| library[at].clone()).collect();
    let shuffled = rng.shuffle(&cards);
    for (&at, card) in places.iter().zip(shuffled) {
        library[at] = card;
    }
}

/// The deck cards whose first index outside the hidden zone is past `through`.
fn kept_through(deck: &[String], left: &IndexMap<String, usize>, through: usize) -> Vec<String> {
    deck.iter()
        .filter(|id| left.get(*id).is_none_or(|&at| at > through))
        .cloned()
        .collect()
}

/// The world `seat` cannot tell from the game's own through state `through`: the opponent's cards
/// that stayed in its hand and library until then name each other's definitions, and both
/// libraries' cards that stayed there lie in another order. `light`: one pair of the opponent's.
fn twin_of(played: &Played, seat: PlayerId, through: usize, rng: &mut Rng, light: bool) -> (GameState, usize) {
    let opp = seat.opponent();
    let mut twin = played.s0.clone();
    let mut hidden = kept_through(&played.deck_ids[opp], &played.left_hidden[opp], through);
    if light {
        hidden = rng.shuffle(&hidden).into_iter().take(2).collect();
    }
    let changed = permute_contents(&mut twin, opp, &hidden, rng);
    if !light {
        let theirs = kept_through(&played.deck_ids[opp], &played.left_library[opp], through);
        permute_positions(&mut twin, opp, &theirs, rng);
        let own = kept_through(&played.deck_ids[seat], &played.left_library[seat], through);
        permute_positions(&mut twin, seat, &own, rng);
    }
    (twin, changed)
}

/// Replays the game's log on `seat`'s twin worlds, one per window of `flags.window` states, and at
/// each state of the window asserts the redaction and (at the seat's decisions) the decision equal.
fn check_twins(
    spec: &Spec,
    played: &Played,
    seat: PlayerId,
    flags: &Flags,
    tally: &mut Tally,
    problems: &mut Vec<String>,
) {
    // The game's own states and the seat's views of them.
    let mut states: Vec<GameState> = vec![played.s0.clone()];
    for action in &played.log {
        let next = reduce(states.last().expect("a state"), action).state;
        states.push(next);
    }
    let last_decision = played
        .recs
        .iter()
        .filter(|rec| rec.seat == seat)
        .map(|rec| rec.at)
        .max();
    let Some(last_decision) = last_decision else {
        return;
    };
    let mut rng = Rng::new(&format!("{}:twin:{}", spec.seed, seat.as_str()), 0);
    let mut lo = 0usize;
    while lo <= last_decision {
        let hi = (lo + flags.window - 1).min(last_decision);
        let mut from = lo;
        for attempt in 0..3 {
            let light = attempt > 0;
            let (mut twin, changed) = twin_of(played, seat, hi, &mut rng, light);
            tally.twin_windows += 1;
            tally.twin_changed_cards += changed;
            let mut diverged: Option<(usize, String)> = None;
            for at in 0..=hi {
                let own = &states[at];
                if view_for(own, seat) != view_for(&twin, seat) {
                    let diff = diff_values(
                        &serde_json::to_value(view_for(own, seat)).unwrap_or_default(),
                        &serde_json::to_value(view_for(&twin, seat)).unwrap_or_default(),
                    );
                    diverged = Some((at, format!("{:?}", diff.first())));
                    break;
                }
                if at >= from {
                    tally.twin_steps += 1;
                    let a = redact(own, seat);
                    let b = redact(&twin, seat);
                    let same = a == b;
                    if !same {
                        let diff = diff_values(
                            &serde_json::to_value(&a).unwrap_or_default(),
                            &serde_json::to_value(&b).unwrap_or_default(),
                        );
                        problems.push(format!(
                            "[P1 redact] {} seed={} seat={} at={} turn={} window={}..={} light={}: {:?}",
                            spec.label,
                            spec.seed,
                            seat.as_str(),
                            at,
                            own.turn,
                            lo,
                            hi,
                            light,
                            diff
                        ));
                    }
                    for rec in played.recs.iter().filter(|rec| rec.at == at && rec.seat == seat) {
                        tally.twin_decisions += 1;
                        let mut options = AiOptions::with_budget(rec.before.clone(), rec.budget);
                        let other = catch_unwind(AssertUnwindSafe(|| {
                            decide_styled(&twin, seat, &mut options, &rec.style)
                        }))
                        .unwrap_or(None);
                        if other.as_ref() != Some(&rec.decision) || options.rng.cursor() != rec.cursor {
                            problems.push(format!(
                                "[P1 decision] {} seed={} seat={} at={} turn={} window={}..={} light={} redact_same={}: own {} {:?} cursor {} | twin {} cursor {}",
                                spec.label,
                                spec.seed,
                                seat.as_str(),
                                at,
                                own.turn,
                                lo,
                                hi,
                                light,
                                same,
                                short(&rec.decision.action),
                                rec.decision.stats,
                                rec.cursor,
                                short(&other),
                                options.rng.cursor()
                            ));
                        }
                    }
                }
                if at == hi {
                    break;
                }
                let stepped = reduce(&twin, &played.log[at]);
                if let Some(error) = stepped.error {
                    diverged = Some((at + 1, format!("refused: {error}")));
                    break;
                }
                if std::env::var("AF_VERBOSE").is_ok()
                    && twin.next_id == own.next_id
                    && stepped.state.next_id != states[at + 1].next_id
                {
                    let mine = reduce(own, &played.log[at]);
                    println!(
                        "    nextId parts at {} (turn {}) seat {} on {}: own {} twin {}; events {:?}",
                        at,
                        own.turn,
                        seat.as_str(),
                        short(&played.log[at]),
                        states[at + 1].next_id,
                        stepped.state.next_id,
                        diff_values(
                            &serde_json::to_value(&mine.events).unwrap_or_default(),
                            &serde_json::to_value(&stepped.events).unwrap_or_default()
                        )
                    );
                }
                twin = stepped.state;
            }
            match diverged {
                None => break,
                Some((at, why)) => {
                    tally.twin_windows_diverged += 1;
                    if std::env::var("AF_VERBOSE").is_ok() {
                        println!(
                            "    twin diverged: {} seat={} window={}..={} light={} at={}: {}",
                            spec.label,
                            seat.as_str(),
                            lo,
                            hi,
                            light,
                            at,
                            why
                        );
                    }
                    // What was checked before the divergence stands; the rest is tried again.
                    from = from.max(at);
                    if from > hi {
                        break;
                    }
                }
            }
        }
        lo = hi + 1;
    }
}

// ---------------------------------------------------------------------------------------------
// The runs
// ---------------------------------------------------------------------------------------------

fn run(families: &[&str], from: usize, count: usize, flags: &Flags) -> (Tally, Vec<String>) {
    jackioh_cards::register_all();
    let mut tally = Tally::default();
    let mut problems: Vec<String> = Vec::new();
    for family in families {
        for n in from..from + count {
            let spec = spec_of(family, n);
            let started = Instant::now();
            let before = problems.len();
            let played = play(&spec, flags, &mut tally, &mut problems);
            let played_in = started.elapsed();
            if flags.twins {
                for seat in PLAYER_IDS {
                    if matches!(spec.ctl[seat], Ctl::Ai { .. }) {
                        check_twins(&spec, &played, seat, flags, &mut tally, &mut problems);
                    }
                }
            }
            println!(
                "game {}: result {:?} turn {} actions {} decisions {} played in {:.1}s, checked in {:.1}s, {} problems",
                spec.label,
                played.end.result.map(|result| result.winner),
                played.end.turn,
                played.log.len(),
                played.recs.len(),
                played_in.as_secs_f64(),
                (started.elapsed() - played_in).as_secs_f64(),
                problems.len() - before
            );
            for problem in &problems[before..] {
                println!("  {problem}");
            }
        }
    }
    println!("{tally:?}");
    (tally, problems)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn explore(families: &str) {
    let families: Vec<&str> = families.split(',').filter(|name| !name.is_empty()).collect();
    let flags = Flags {
        determinism_every: env_usize("AF_DET", 4),
        hints: env_usize("AF_HINTS", 1) == 1,
        unstoppable: env_usize("AF_UNSTOPPABLE", 1) == 1,
        twins: env_usize("AF_TWINS", 1) == 1,
        window: env_usize("AF_WINDOW", 30),
        bump: env_usize("AF_BUMP", 0),
    };
    let (_, problems) = run(&families, env_usize("AF_FROM", 1), env_usize("AF_N", 1), &flags);
    println!("{} problems", problems.len());
}

#[test]
fn explore_a() {
    if let Ok(families) = std::env::var("AF_A") {
        explore(&families);
    }
}

#[test]
fn explore_b() {
    if let Ok(families) = std::env::var("AF_B") {
        explore(&families);
    }
}

// ---------------------------------------------------------------------------------------------
// The opponent's unseen Smoothie
// ---------------------------------------------------------------------------------------------

/// S+ #17 Smoothie: "When added to your hand, select two random fruits for it."
const SMOOTHIE: &str = "stinkyscapeplus-017";

/// Both seats keep their whole hand, then p1 ends its first turn: p2's first main phase.
fn to_p2s_first_turn(start: &GameState) -> Option<GameState> {
    let mut state = start.clone();
    for step in 0..6 {
        if state.result.is_some() {
            return None;
        }
        let seat = seat_to_act(&state).unwrap_or(state.active);
        let own_main = state.pending.is_none() && state.active == seat && state.phase == Phase::Main;
        if own_main && seat == PlayerId::P2 {
            return Some(state);
        }
        let body = if own_main {
            ActionBody::EndTurn
        } else if state.pending.is_none() {
            ActionBody::Mulligan {
                keep: ids_of(&state.players[seat].hand),
            }
        } else {
            return None;
        };
        let result = reduce(&state, &Action::new(body, seat, format!("s{step}")));
        if result.error.is_some() {
            return None;
        }
        state = result.state;
    }
    None
}

/// Two games of `seed` that differ in one card of p1's deck, the one p1 draws on its first turn (or
/// holds from the deal): the deck's own card in the first, a Smoothie in the second.
fn smoothie_worlds(seed: &str) -> Option<(GameState, GameState, String)> {
    let easy = AI_DIFFICULTY.easy;
    let decks = (
        deck(seed, PlayerId::P1, &easy, None),
        deck(seed, PlayerId::P2, &easy, None),
    );
    if decks.0.iter().chain(decks.1.iter()).any(|id| id == SMOOTHIE) {
        return None;
    }
    let options = |decks: (Vec<String>, Vec<String>)| CreateGameOptions {
        seed: seed.to_string(),
        decks,
        ..CreateGameOptions::default()
    };
    let created = create_game(&options(decks.clone()));
    let plain_start = begin_game(&created).state;
    let plain = to_p2s_first_turn(&plain_start)?;
    // The card p1 drew since the deal, else the first it holds.
    let dealt: Vec<String> = ids_of(&plain_start.players.p1.hand);
    let hand = &plain.players.p1.hand;
    let slot_id = hand
        .iter()
        .find(|card| !dealt.contains(&card.id))
        .or_else(|| hand.first())?
        .id
        .clone();
    let slot = created.players.p1.library.iter().position(|card| card.id == slot_id)?;
    let mut other = decks.clone();
    other.0[slot] = SMOOTHIE.to_string();
    let smoothie = to_p2s_first_turn(&begin_game(&create_game(&options(other))).state)?;
    Some((plain, smoothie, slot_id))
}

/// Looks for a seed on which p2's decision differs between the two worlds.
#[test]
fn scan_smoothie() {
    let Ok(count) = std::env::var("AF_SCAN") else {
        return;
    };
    jackioh_cards::register_all();
    let count: usize = count.parse().unwrap_or(10);
    let from = env_usize("AF_FROM", 1);
    for n in from..from + count {
        let seed = format!("audit-fuzz:smoothie:{n}");
        let Some((mut plain, mut smoothie, slot_id)) = smoothie_worlds(&seed) else {
            println!("{seed}: skipped");
            continue;
        };
        let seat = PlayerId::P2;
        let mut rng = Rng::new(&format!("{seed}:ctl:p2"), 0);
        for step in 0..8 {
            if plain.result.is_some() || seat_to_act(&plain) != Some(seat) {
                break;
            }
            let same_view = view_for(&plain, seat) == view_for(&smoothie, seat);
            let diff = diff_values(
                &serde_json::to_value(redact(&plain, seat)).unwrap_or_default(),
                &serde_json::to_value(redact(&smoothie, seat)).unwrap_or_default(),
            );
            let mut a = AiOptions::with_budget(rng.clone(), AI_BUDGET);
            let mut b = AiOptions::with_budget(rng.clone(), AI_BUDGET);
            let first = decide_styled(&plain, seat, &mut a, &AI_STYLE_EXACT).expect("a decision");
            let second = decide_styled(&smoothie, seat, &mut b, &AI_STYLE_EXACT).expect("a decision");
            println!(
                "{seed} step {step} slot {slot_id}: same view {same_view}, next_id {} vs {}, redact diff {:?}; {} {} ({} nodes, {:.3}) vs {} {} ({} nodes, {:.3}){}",
                plain.next_id,
                smoothie.next_id,
                diff,
                first.reason.as_str(),
                short(&first.action),
                first.stats.nodes,
                first.stats.score,
                second.reason.as_str(),
                short(&second.action),
                second.stats.nodes,
                second.stats.score,
                if first.action != second.action { "  <== ACTION DIFFERS" } else { "" }
            );
            if !same_view || first.action != second.action {
                break;
            }
            rng = a.rng.clone();
            let action = Action::new(first.action.clone(), seat, format!("d{step}"));
            let (x, y) = (reduce(&plain, &action), reduce(&smoothie, &action));
            if x.error.is_some() || y.error.is_some() {
                break;
            }
            plain = x.state;
            smoothie = y.state;
        }
    }
}

/// The game of `spec`, with the card `seat`'s opponent was dealt as `id` a Smoothie instead.
fn with_smoothie(spec: &Spec, opp: PlayerId, id: &str) -> Option<GameState> {
    let created = create(spec);
    let slot = created.players[opp].library.iter().position(|card| card.id == id)?;
    let mut other = spec.clone();
    let deck = if opp == PlayerId::P1 {
        &mut other.decks.0
    } else {
        &mut other.decks.1
    };
    if deck.iter().any(|def_id| def_id == SMOOTHIE) {
        return None;
    }
    deck[slot] = SMOOTHIE.to_string();
    Some(begin_game(&create(&other)).state)
}

/// Plays whole games, then replays each with one card of the opponent's deck a Smoothie, and decides
/// again wherever the seat's view is the same and the id counter is not.
#[test]
fn scan_smoothie_games() {
    let Ok(families) = std::env::var("AF_SCAN2") else {
        return;
    };
    jackioh_cards::register_all();
    let flags = Flags {
        determinism_every: 0,
        hints: false,
        unstoppable: false,
        twins: false,
        window: 30,
        bump: 0,
    };
    let from = env_usize("AF_FROM", 1);
    let (mut pairs, mut stats_differ, mut actions_differ) = (0usize, 0usize, 0usize);
    for family in families.split(',') {
        for n in from..from + env_usize("AF_N", 1) {
            let spec = spec_of(family, n);
            let mut tally = Tally::default();
            let mut problems = Vec::new();
            let played = play(&spec, &flags, &mut tally, &mut problems);
            let mut states: Vec<GameState> = vec![played.s0.clone()];
            for action in &played.log {
                let next = reduce(states.last().expect("a state"), action).state;
                states.push(next);
            }
            for seat in PLAYER_IDS {
                let opp = seat.opponent();
                for id in &played.deck_ids[opp] {
                    let Some(mut world) = with_smoothie(&spec, opp, id) else {
                        continue;
                    };
                    for at in 0..played.log.len() {
                        let own = &states[at];
                        if view_for(own, seat) != view_for(&world, seat) {
                            break;
                        }
                        if own.next_id != world.next_id {
                            for rec in played.recs.iter().filter(|rec| rec.at == at && rec.seat == seat) {
                                pairs += 1;
                                let mut options = AiOptions::with_budget(rec.before.clone(), rec.budget);
                                let Some(other) = decide_styled(&world, seat, &mut options, &rec.style) else {
                                    continue;
                                };
                                if other.stats != rec.decision.stats {
                                    stats_differ += 1;
                                }
                                if other.action != rec.decision.action {
                                    actions_differ += 1;
                                    println!(
                                        "ACTION DIFFERS {} seat {} card {} at {} turn {} next_id {} vs {}: {} {:?} | {} {:?}",
                                        spec.label,
                                        seat.as_str(),
                                        id,
                                        at,
                                        own.turn,
                                        own.next_id,
                                        world.next_id,
                                        short(&rec.decision.action),
                                        rec.decision.stats,
                                        short(&other.action),
                                        other.stats
                                    );
                                }
                            }
                        }
                        let stepped = reduce(&world, &played.log[at]);
                        if stepped.error.is_some() {
                            break;
                        }
                        world = stepped.state;
                    }
                }
            }
            println!(
                "{}: {} actions; so far {pairs} pairs, {stats_differ} with other stats, {actions_differ} with another action",
                spec.label,
                played.log.len()
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Finding 1: tests
// ---------------------------------------------------------------------------------------------

const SMOOTHIE_SEED: &str = "audit-fuzz:smoothie:9";

#[test]
fn f1a_redact_is_the_same_for_two_worlds_the_seat_cannot_tell_apart() {
    jackioh_cards::register_all();
    let (plain, smoothie, _) = smoothie_worlds(SMOOTHIE_SEED).expect("the two worlds");
    let seat = PlayerId::P2;
    assert_eq!(view_for(&plain, seat), view_for(&smoothie, seat), "PREMISE: one view");
    let diff = diff_values(
        &serde_json::to_value(redact(&plain, seat)).expect("serializes"),
        &serde_json::to_value(redact(&smoothie, seat)).expect("serializes"),
    );
    assert_eq!(diff, Vec::<String>::new());
}

#[test]
fn f1b_decide_is_the_same_for_two_worlds_the_seat_cannot_tell_apart() {
    jackioh_cards::register_all();
    let (plain, smoothie, _) = smoothie_worlds(SMOOTHIE_SEED).expect("the two worlds");
    let seat = PlayerId::P2;
    assert_eq!(view_for(&plain, seat), view_for(&smoothie, seat), "PREMISE: one view");
    let decided = |state: &GameState| {
        let mut options = AiOptions::with_budget(Rng::new(&format!("{SMOOTHIE_SEED}:ctl:p2"), 0), AI_BUDGET);
        let decision = decide_styled(state, seat, &mut options, &AI_STYLE_EXACT).expect("a decision");
        (decision.action, decision.stats, options.rng.cursor())
    };
    assert_eq!(decided(&plain), decided(&smoothie));
}

#[test]
fn f1c_two_ids_minted_unseen_do_not_change_the_action() {
    use jackioh_ai::{MatchConfig, MatchHooks, SeatController, play_match};
    jackioh_cards::register_all();
    let spec = spec_of("hard", 4);
    let ai = SeatController::Ai {
        budget: Some(SMALL),
        skill: None,
    };
    let config = MatchConfig {
        seed: spec.seed.clone(),
        decks: spec.decks.clone(),
        handicaps: spec.handicaps.clone(),
        controllers: PerPlayer::new(ai, ai),
        max_actions: Some(61),
        setup: MatchSetup::default(),
    };
    let mut last: Option<(GameState, PlayerId, Rng)> = None;
    let record = {
        let mut hooks = MatchHooks {
            time_decision: Some(Box::new(|state, seat, rng, run| {
                last = Some((state.clone(), seat, rng.clone()));
                run();
            })),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks)
    };
    assert_eq!(record.log.len(), 61, "PREMISE: the game reaches its 61st action");
    let (state, seat, rng) = last.expect("a decision");
    let decided = |state: &GameState| {
        let mut options = AiOptions::with_budget(rng.clone(), SMALL);
        decide_styled(state, seat, &mut options, &AI_STYLE_EXACT)
            .expect("a decision")
            .action
    };
    // What the opponent drawing a Smoothie unseen does to the state `redact` hands the search.
    let mut minted = state.clone();
    minted.next_id += 2;
    println!("turn {} seat {} next_id {}", state.turn, seat.as_str(), state.next_id);
    assert_eq!(decided(&state), decided(&minted));
}
