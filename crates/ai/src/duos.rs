//! Duos co-op for the practice AI (#936 S5, B26–B32): the AI plays one seat of a `DuosState`, two
//! plain 1v1 duels side by side, by playing that seat's own duel with the 1v1 `decide`.
//!
//! A seat maps to its duel (`seat.duel()`) and to its player there (`seat.player()`), so every
//! mulligan, prompt and main-phase action is `decide`'s on `state.duels[seat.duel()]` (B26, B27),
//! sent as `DuosActionBody::Duel`. The wrapper adds one decision of its own: when `decide` would end
//! the turn, the seat first passes the cards `duos_passes` names, one per call, and ends the turn once
//! none is left (B27). Like `decide` it never concedes, so it never opens a team's concede offer.
//!
//! Knowledge (B29): `decide` reads its duel only through `redact` and its determinizations (R185).
//! `duos_passes` reads the seat's own hand and its own legal plays off the same redacted probe
//! `decide` lists its own actions on, and of the teammate only its own side: hand size, mana, open
//! zones. An opponent's hidden card never reaches a decision, so two states that differ only there
//! decide alike under the same options.
//!
//! The gate (B32): `duos_gate` plays the Easy AI team (`duos_decide` at AI_GATE_BUDGET) against a team
//! on §10.7's random policy over `duos_legal_actions` (`duos_random_action`), the AI on team A in odd
//! games and on team B in even ones, every seat dealt by `build_ai_deck` on its Easy handicap. An action
//! `duos_reduce` refuses, on either team, fails the gate (`DuosGateReport::passes`). Pure and seeded
//! like the rest of the crate (CLAUDE.md rule 4): a budget counts `reduce` calls, every stream is
//! seeded from the game's seed and its seat, and nothing reads a clock.

use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexSet;
use jackioh_engine::config::{
    AI_DIFFICULTY, AI_END_TURN_PROBABILITY, DUOS_HUMAN_TEAM, Difficulty, duos_ai_team,
};
use jackioh_engine::duos::{
    DuosState, duos_begin, duos_create, duos_legal_actions, duos_reduce, duos_seats_to_act,
};
use jackioh_engine::subsystems::AI_SKIPPED_ACTIONS;
use jackioh_engine::wire::{
    DUOS_SEATS, DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosOutcome,
    DuosSeat, PerTeam, Team,
};
use jackioh_engine::{
    Action, ActionBody, ActionType, CardInstance, CostOptions, GameState, PlayerId, PlayerState, Rng, Zone,
    effective_cost, hand_cap_of, legal_actions, max_mana_for, natural_mana_on, needs_zone, open_zones,
    reduce, refreshed_mana, row_for_card, spendable_mana,
};
use serde::{Deserialize, Serialize};

use crate::config::{
    AI_EVAL, AI_GATE_BUDGET, AI_SEARCH, DUOS_GATE_WIN_FLOOR, DUOS_PASS_MAX_FOLLOWUPS, DUOS_PASS_PROBE_SEED,
};
use crate::decide::decide;
use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::determinize::{DeterminizeOptions, determinize};
use crate::evaluate::{NextSwing, evaluate};
use crate::observe::redact;
use crate::types::AiOptions;

/// B32: the gate's frozen seed series. Game n of a run is `${series}:${n}`; its decks are dealt from
/// `${seed}:deck:${seat}` and each seat's controller draws from `${seed}:ctl:${seat}`.
pub const DUOS_GATE_SERIES: &str = "duos-gate:v1";

/// The most actions one gate game takes before it is scored unfinished: two duels, each with the
/// ceiling a 1v1 match runner gives one game (`match_.rs`'s AI_MATCH).
const DUOS_GATE_MAX_ACTIONS: usize = 6000;

// ---------------------------------------------------------------------------
// The AI seat
// ---------------------------------------------------------------------------

/// The action an AI-played seat takes next, or None when it owes nothing: `duos_legal_actions` holds
/// nothing but `Concede`, or nothing the AI would send. Its mulligan, its prompts and its own turn are
/// the 1v1 `decide`'s on its duel (B26, B27); when `decide` would end the turn the seat first makes
/// the passes `duos_passes` names, one per call, then ends the turn (B27). Deterministic for the same
/// state and options (B31): `options.rng` is `decide`'s only stream, and the caller reads its cursor
/// back afterwards.
pub fn duos_decide(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosActionBody> {
    if state.result.is_some() {
        return None;
    }
    let legal = duos_legal_actions(state, seat);
    if legal.iter().all(|body| matches!(body, DuosActionBody::Concede)) {
        return None;
    }
    let duel = state.duels.get(seat.duel())?;
    // `decide` has already tried its action on the duel's true state (decide.rs `submitted`); the
    // wrapper's own refusals are by kind (a seat that has ended, a draw offer), so an action of a
    // kind the wrapper offers is sent as decided. A mulligan's keep or an answer's selection may list
    // its ids in another order than `duos_legal_actions` does, and means the same.
    match decide(duel, seat.player(), options).map(|decision| decision.action) {
        Some(ActionBody::EndTurn) if offers(&legal, ActionType::EndTurn) => {
            Some(pass_or_end(state, seat, &legal))
        }
        Some(action) if offers(&legal, action.action_type()) => Some(DuosActionBody::Duel { body: action }),
        _ => fallback(state, seat, &legal),
    }
}

/// Whether the wrapper offers a duel action of this kind.
fn offers(legal: &[DuosActionBody], kind: ActionType) -> bool {
    legal
        .iter()
        .any(|body| matches!(body, DuosActionBody::Duel { body: inner } if inner.action_type() == kind))
}

/// B27: the first of `duos_passes`' cards the wrapper offers a `Pass` for, else the turn's end.
fn pass_or_end(state: &DuosState, seat: DuosSeat, legal: &[DuosActionBody]) -> DuosActionBody {
    duos_passes(state, seat)
        .into_iter()
        .find(|id| {
            legal
                .iter()
                .any(|body| matches!(body, DuosActionBody::Pass { instance_id } if instance_id == id))
        })
        .map(|instance_id| DuosActionBody::Pass { instance_id })
        .unwrap_or(DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        })
}

/// What the seat sends when `decide` answered nothing the wrapper offers: the turn's end (after its
/// passes) when offered, else the first answer or mulligan, else the first other duel action §10.7's
/// policy would take (`match_.rs`'s `replacements_for` order). None when only passes and the concede
/// are left: the seat waits (another seat's prompt in its duel blocks it).
fn fallback(state: &DuosState, seat: DuosSeat, legal: &[DuosActionBody]) -> Option<DuosActionBody> {
    if offers(legal, ActionType::EndTurn) {
        return Some(pass_or_end(state, seat, legal));
    }
    let duel_actions: Vec<&ActionBody> = legal
        .iter()
        .filter_map(|body| match body {
            DuosActionBody::Duel { body } => Some(body),
            _ => None,
        })
        .collect();
    let answer = duel_actions
        .iter()
        .find(|action| matches!(action.action_type(), ActionType::Answer | ActionType::Mulligan));
    let other = || {
        duel_actions
            .iter()
            .find(|action| !AI_SKIPPED_ACTIONS.contains(&action.action_type()))
    };
    answer.or_else(other).map(|action| DuosActionBody::Duel {
        body: ActionBody::clone(action),
    })
}

// ---------------------------------------------------------------------------
// Passing
// ---------------------------------------------------------------------------

/// The cards `seat` should pass now (at the end of its own turn), best first: ids of its hand cards
/// (B28). A card qualifies when the seat cannot play it this turn and its teammate can: the teammate
/// is a round seat that has not ended, can afford the card and has a zone for it. With the team's
/// `pass_cost` 0 a card also qualifies when the teammate can afford it next turn and the seat cannot.
/// The cards the teammate can play now come first, ranked by the teammate's best bounded follow-up
/// with each (R1127000), ties by costliest first and then in hand order; the cards it can only pay
/// for next turn keep costliest-first and hand order. The list never holds more than the passes left
/// this round, the teammate's free hand slots, or the passes the seat's mana pays for. Empty unless
/// `seat` is a round seat that has not ended.
///
/// B29: the seat's own hand, its plays and its next refresh are read off the redacted probe `decide`
/// lists its own actions on; of the teammate only its own side (hand size, mana, open zones), plus
/// the determinized probe R1127000's follow-up scoring replays each candidate card into. A
/// card's price is its own (`own_cost`), the part of it a pass keeps (B20).
pub fn duos_passes(state: &DuosState, seat: DuosSeat) -> Vec<String> {
    // `redact` and `determinize` run on the true duel, as in `decide`; a failure there is no pass.
    catch_unwind(AssertUnwindSafe(|| passes_of(state, seat))).unwrap_or_default()
}

fn passes_of(state: &DuosState, seat: DuosSeat) -> Vec<String> {
    if state.result.is_some() || !open_round_seat(state, seat) {
        return Vec::new();
    }
    let mate = seat.teammate();
    let (Some(duel), Some(mate_duel)) = (state.duels.get(seat.duel()), state.duels.get(mate.duel())) else {
        return Vec::new();
    };
    let rules = state.teams[seat.team()].rules;
    let player = seat.player();
    let mate_side = &mate_duel.players[mate.player()];

    // B19, B28: the passes left this round, the teammate's free hand slots and what the seat's mana
    // pays for, whichever is fewest.
    let made = state.passes.get(seat.duel()).copied().unwrap_or(0);
    let free_slots = hand_cap_of(mate_duel, mate.player()) - mate_side.hand.len() as i32;
    let paid_for = if rules.pass_cost > 0 {
        duel.players[player].mana.current / rules.pass_cost
    } else {
        i32::MAX
    };
    let room = (rules.pass_cap - made).min(free_slots).min(paid_for);
    let Ok(room) = usize::try_from(room) else {
        return Vec::new();
    };
    if room == 0 {
        return Vec::new();
    }

    let probe = own_probe(duel, player);
    let playable: IndexSet<String> = legal_actions(&probe, player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play { instance_id, .. } => Some(instance_id),
            _ => None,
        })
        .collect();
    let mate_acts = open_round_seat(state, mate);
    let mate_now = spendable_mana(mate_duel, mate.player());
    let mate_next = next_turn_mana(mate_duel, mate_side);
    let own_next = next_turn_mana(&probe, &probe.players[player]);

    // (cost, hand index, id): the plays the teammate can make this round, then (pass cost 0 only)
    // the cards it can pay for next turn and the seat cannot.
    let mut now: Vec<(i32, usize, String)> = Vec::new();
    let mut later: Vec<(i32, usize, String)> = Vec::new();
    for (index, card) in probe.players[player].hand.iter().enumerate() {
        let cost = own_cost(&probe, card);
        if mate_acts
            && !playable.contains(&card.id)
            && cost <= mate_now
            && has_zone_for(&probe, card, mate_duel, mate.player())
        {
            now.push((cost, index, card.id.clone()));
        } else if rules.pass_cost == 0 && cost <= mate_next && cost > own_next {
            later.push((cost, index, card.id.clone()));
        }
    }
    rank_now(&mut now, &probe, player, mate_duel, mate.player());
    best_first(&mut later);
    now.into_iter()
        .chain(later)
        .take(room)
        .map(|(_, _, id)| id)
        .collect()
}

/// B4, B19: the seat is its duel's round seat and has not ended its turn this round.
fn open_round_seat(state: &DuosState, seat: DuosSeat) -> bool {
    state.round > 0
        && state.round_seats.get(seat.duel()) == Some(&seat)
        && state.ended.get(seat.duel()) == Some(&false)
}

/// The seat's own duel as `decide`'s forced-move check sees it (decide.rs step 4): one determinization
/// of `redact` on the fixed probe seed, so the seat's own legal plays are read with no hidden card in
/// reach and without touching any caller's stream.
fn own_probe(duel: &GameState, player: PlayerId) -> GameState {
    determinize(
        &redact(duel, player),
        player,
        &mut Rng::new(AI_SEARCH.probe_seed, 0),
        DeterminizeOptions::default(),
    )
}

/// A card's own price (R65, R396): its override or printed cost with its cost modifier, floored, read
/// as a card out of a hand is read. No player's discount and no field aura reaches it, so it is the
/// part of the price a pass keeps (B20), and no face-down card's aura can move it.
fn own_cost(duel: &GameState, card: &CardInstance) -> i32 {
    let mut off_hand = card.clone();
    off_hand.zone = Zone::Library { player: card.owner };
    effective_cost(duel, &off_hand, CostOptions::default())
}

/// B28: the teammate has a zone for the card: a Spell needs none, any other card an open zone of its
/// row on the teammate's own side.
fn has_zone_for(card_state: &GameState, card: &CardInstance, mate_duel: &GameState, mate: PlayerId) -> bool {
    if !needs_zone(card_state, card) {
        return true;
    }
    let row = row_for_card(card_state, card);
    !open_zones(mate_duel, mate, row).is_empty()
}

/// What this side's next refresh gives (§2.3, R181): the turn's start counts the turn, then refreshes
/// (`turn.rs`), with the rider and a lost refresh as they stand and natural mana as the field stands.
fn next_turn_mana(duel: &GameState, side: &PlayerState) -> i32 {
    let mut next = side.clone();
    next.turns_started += 1;
    let max = max_mana_for(&next);
    refreshed_mana(&next, max, natural_mana_on(duel))
}

/// Costliest first, then in hand order.
fn best_first(cards: &mut [(i32, usize, String)]) {
    cards.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
}

/// R1127000: order the cards the teammate can play now by the teammate's best bounded follow-up
/// with each, highest first; ties by costliest first, then hand order. Scoring can never empty or
/// crash the pass list: a scoring panic falls back to the cost order.
fn rank_now(
    now: &mut [(i32, usize, String)],
    own_probe: &GameState,
    seat_player: PlayerId,
    mate_duel: &GameState,
    mate: PlayerId,
) {
    let scored = catch_unwind(AssertUnwindSafe(|| {
        now.iter()
            .map(|(cost, index, id)| {
                (
                    pass_score(own_probe, seat_player, id, mate_duel, mate),
                    *cost,
                    *index,
                    id.clone(),
                )
            })
            .collect::<Vec<_>>()
    }));
    let Ok(mut scored) = scored else {
        best_first(now);
        return;
    };
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(Ordering::Equal)
            .then(b.1.cmp(&a.1))
            .then(a.2.cmp(&b.2))
    });
    for (at, (_, cost, index, id)) in scored.into_iter().enumerate() {
        now[at] = (cost, index, id);
    }
}

/// R1127000: the teammate's best bounded follow-up with the passed card in hand: the maximum of
/// `evaluate` over at most `DUOS_PASS_MAX_FOLLOWUPS` legal `Play` actions (the passed card's own
/// plays first, then the rest in `legal_actions` order), each applied by one engine step to the
/// teammate's determinized probe with a copy of the card added to its hand, scored from the
/// teammate's seat; `evaluate` of the receiving probe when no follow-up applies. The probe is
/// built exactly like `own_probe`, on the fixed `DUOS_PASS_PROBE_SEED`, so no hidden opponent card
/// and no caller stream is ever read; a refused or panicking follow-up is skipped.
fn pass_score(
    own_probe: &GameState,
    seat_player: PlayerId,
    card_id: &str,
    mate_duel: &GameState,
    mate: PlayerId,
) -> f64 {
    let Some(passed) = own_probe.players[seat_player]
        .hand
        .iter()
        .find(|card| card.id == card_id)
        .cloned()
    else {
        return f64::NEG_INFINITY;
    };
    let mate_probe = determinize(
        &redact(mate_duel, mate),
        mate,
        &mut Rng::new(DUOS_PASS_PROBE_SEED, 0),
        DeterminizeOptions::default(),
    );
    let mut receiving = mate_probe;
    let mut given = passed;
    given.owner = mate;
    given.controller = mate;
    given.zone = Zone::Hand { player: mate };
    receiving.players[mate].hand.push(given.clone());
    let mut best: Option<f64> = None;
    // The passed card's own plays first, then the teammate's other plays in `legal_actions`
    // order: the card sits at the hand's end, so a plain prefix would cut its plays off behind a
    // full hand's and score every candidate alike. Still at most `DUOS_PASS_MAX_FOLLOWUPS` steps.
    let mut own_plays = Vec::new();
    let mut other_plays = Vec::new();
    for action in legal_actions(&receiving, mate) {
        match &action {
            ActionBody::Play { instance_id, .. } if *instance_id == given.id => {
                own_plays.push(action);
            }
            ActionBody::Play { .. } => other_plays.push(action),
            _ => {}
        }
    }
    let plays = own_plays
        .into_iter()
        .chain(other_plays)
        .take(DUOS_PASS_MAX_FOLLOWUPS);
    for (at, body) in plays.enumerate() {
        let applied = catch_unwind(AssertUnwindSafe(|| {
            reduce(
                &receiving,
                &Action::new(body, mate, format!("duos-pass-score:{at}")),
            )
        }));
        let Ok(applied) = applied else { continue };
        if applied.error.is_some() {
            continue;
        }
        let value = evaluate(&applied.state, mate, NextSwing::Enemy, &AI_EVAL);
        best = Some(best.map_or(value, |held: f64| held.max(value)));
    }
    best.unwrap_or_else(|| evaluate(&receiving, mate, NextSwing::Enemy, &AI_EVAL))
}

// ---------------------------------------------------------------------------
// The random policy and the gate
// ---------------------------------------------------------------------------

/// §10.7's random policy for one Duos seat, over `duos_legal_actions`: the concede and the duel
/// actions the policy never takes (`AI_SKIPPED_ACTIONS`) left out, the turn's end taken when nothing
/// else is on offer or with AI_END_TURN_PROBABILITY, otherwise a uniform pick among the rest, passes
/// included. No draw is taken when there is no choice. None when the seat has nothing to send.
pub fn duos_random_action(state: &DuosState, seat: DuosSeat, rng: &mut Rng) -> Option<DuosActionBody> {
    let mut end_turn: Option<DuosActionBody> = None;
    let mut others: Vec<DuosActionBody> = Vec::new();
    for body in duos_legal_actions(state, seat) {
        match policy_kind(&body) {
            PolicyKind::Skipped => {}
            PolicyKind::EndTurn => end_turn = Some(body),
            PolicyKind::Other => others.push(body),
        }
    }
    if others.is_empty() {
        return end_turn;
    }
    if end_turn.is_some() && rng.chance(AI_END_TURN_PROBABILITY) {
        return end_turn;
    }
    let at = rng.int(others.len() as i32);
    usize::try_from(at).ok().and_then(|at| others.into_iter().nth(at))
}

/// How §10.7's policy treats one offered action.
enum PolicyKind {
    Skipped,
    EndTurn,
    Other,
}

fn policy_kind(body: &DuosActionBody) -> PolicyKind {
    match body {
        DuosActionBody::Concede => PolicyKind::Skipped,
        DuosActionBody::Duel { body } if AI_SKIPPED_ACTIONS.contains(&body.action_type()) => {
            PolicyKind::Skipped
        }
        DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        } => PolicyKind::EndTurn,
        _ => PolicyKind::Other,
    }
}

/// Gate: the Easy AI team against a random-policy team (B32). `wins` counts the games the AI team won;
/// `draws` is reported beside them and counts for nothing; any `rejected` action fails the gate.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuosGateReport {
    pub games: usize,
    pub wins: usize,
    pub draws: usize,
    /// Actions `duos_reduce` refused, summed over the games, on either team (`DuosGateGame::rejected`).
    pub rejected: usize,
}

impl DuosGateReport {
    /// wins / games (0 for no games).
    pub fn rate(&self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            self.wins as f64 / self.games as f64
        }
    }

    /// B32: the AI team won at least DUOS_GATE_WIN_FLOOR of a run of at least one game, and no game had
    /// an action refused. A refusal is an AI bug or a gap between `duos_legal_actions` and
    /// `duos_reduce`, on either team, and fails the gate as a rejected action fails the 1v1 gates (B31).
    pub fn passes(&self) -> bool {
        self.games > 0 && self.rejected == 0 && self.wins as f64 >= DUOS_GATE_WIN_FLOOR * self.games as f64
    }
}

/// One gate game, as `duos_gate_game` played it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosGateGame {
    pub seed: String,
    /// The team the AI played (`duos_gate_subject`).
    pub subject: Team,
    pub won: bool,
    pub drawn: bool,
    /// The result's reason ("poolEmptied", "turnCap", …); None when the game reached
    /// DUOS_GATE_MAX_ACTIONS, or no seat owed an action before it ended.
    pub reason: Option<String>,
    /// The actions applied.
    pub actions: usize,
    /// Actions `duos_reduce` refused, each replaced by the random policy's.
    pub rejected: usize,
    /// Of `rejected`, the actions the AI team sent; the rest were the random-policy team's.
    pub ai_rejected: usize,
}

/// Game n (1-based): the AI is team A when n is odd and team B when n is even, so a run measures it
/// both going first (team A is P1 in both duels) and with The Coin.
pub fn duos_gate_subject(n: usize) -> Team {
    if n % 2 == 1 { Team::A } else { Team::B }
}

/// B32: games 1..=games of the series `seed`, the Easy AI team against the random-policy team.
pub fn duos_gate(games: usize, seed: &str) -> DuosGateReport {
    let played: Vec<DuosGateGame> = (1..=games).map(|n| duos_gate_game(n, seed)).collect();
    duos_gate_report(&played)
}

/// The report a run of these games makes.
pub fn duos_gate_report(played: &[DuosGateGame]) -> DuosGateReport {
    DuosGateReport {
        games: played.len(),
        wins: played.iter().filter(|game| game.won).count(),
        draws: played.iter().filter(|game| game.drawn).count(),
        rejected: played.iter().map(|game| game.rejected).sum(),
    }
}

/// Game n (1-based) of the series: seed `${series}:${n}`, every seat on AI_DIFFICULTY.easy with a deck
/// `build_ai_deck` deals for it, the AI team on `duos_ai_team(Easy)` and the other on
/// DUOS_HUMAN_TEAM. While a game is live, the first seat of `duos_seats_to_act` that sends something
/// acts: an AI seat through `duos_decide` at AI_GATE_BUDGET, a random-policy seat through
/// `duos_random_action`, each on its own stream. An action `duos_reduce` refuses is replaced by the
/// random policy's for that seat and counted in `rejected` (in `ai_rejected` too when the AI team sent
/// it). A game in which no listed seat sends anything stops there,
/// unfinished. A pure function of `n` and `series`.
pub fn duos_gate_game(n: usize, series: &str) -> DuosGateGame {
    let seed = format!("{series}:{n}");
    let subject = duos_gate_subject(n);
    let mut state = duos_begin(&duos_create(&duos_gate_options(&seed, subject))).state;
    let mut rngs: [Rng; 4] = DUOS_SEATS.map(|seat| Rng::new(&format!("{seed}:ctl:{seat}"), 0));

    let mut actions = 0usize;
    let mut rejected = 0usize;
    let mut ai_rejected = 0usize;
    while state.result.is_none() && actions < DUOS_GATE_MAX_ACTIONS {
        // The first listed seat that sends something acts; an AI seat that owes nothing (it waits on
        // another seat's prompt in its duel) lets the next one go.
        let mut picked: Option<(DuosSeat, usize, DuosActionBody)> = None;
        for seat in duos_seats_to_act(&state) {
            let slot = gate_slot(seat);
            let chosen = if seat.team() == subject {
                let mut options = AiOptions::with_budget(rngs[slot].clone(), AI_GATE_BUDGET);
                let decided = duos_decide(&state, seat, &mut options);
                rngs[slot] = options.rng;
                decided
            } else {
                duos_random_action(&state, seat, &mut rngs[slot])
            };
            if let Some(chosen) = chosen {
                picked = Some((seat, slot, chosen));
                break;
            }
        }
        let Some((seat, slot, chosen)) = picked else {
            break;
        };

        let nonce = format!("g{actions}");
        let reduced = duos_reduce(
            &state,
            &DuosAction {
                seat,
                nonce: nonce.clone(),
                body: chosen,
            },
        );
        let next = if reduced.error.is_none() {
            reduced.state
        } else {
            rejected += 1;
            if seat.team() == subject {
                ai_rejected += 1;
            }
            let Some(replacement) = duos_random_action(&state, seat, &mut rngs[slot]) else {
                break;
            };
            let retried = duos_reduce(
                &state,
                &DuosAction {
                    seat,
                    nonce,
                    body: replacement,
                },
            );
            if retried.error.is_some() {
                break;
            }
            retried.state
        };
        state = next;
        actions += 1;
    }

    let result = state.result.as_ref();
    DuosGateGame {
        seed,
        subject,
        won: result
            .is_some_and(|result| result.outcome == DuosOutcome::Win && result.winner == Some(subject)),
        drawn: result.is_some_and(|result| result.outcome == DuosOutcome::Draw),
        reason: result.map(|result| result.reason.clone()),
        actions,
        rejected,
        ai_rejected,
    }
}

/// A seat's place in DUOS_SEATS: the index of its controller's stream.
fn gate_slot(seat: DuosSeat) -> usize {
    DUOS_SEATS.iter().position(|each| *each == seat).unwrap_or(0)
}

/// A gate game's options: four Easy decks by the AI's deck rule (R186's ban included), Easy's handicap
/// on every seat (AI_DIFFICULTY.easy, §9.9), the AI team's rules by `duos_ai_team`.
fn duos_gate_options(seed: &str, subject: Team) -> DuosCreateOptions {
    let handicap = AI_DIFFICULTY.easy;
    let deck = |seat: DuosSeat| -> Vec<String> {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:deck:{seat}"), 0),
            handicap.deck_size,
            &AiDeckOptions {
                mana_cap: Some(handicap.mana_cap),
                ..AiDeckOptions::default()
            },
        )
    };
    let ai_rules = duos_ai_team(Difficulty::Easy);
    DuosCreateOptions {
        seed: seed.to_string(),
        decks: DuosDecks {
            a1: deck(DuosSeat::A1),
            a2: deck(DuosSeat::A2),
            b1: deck(DuosSeat::B1),
            b2: deck(DuosSeat::B2),
        },
        catalog: None,
        teams: Some(match subject {
            Team::A => PerTeam {
                a: ai_rules,
                b: DUOS_HUMAN_TEAM,
            },
            Team::B => PerTeam {
                a: DUOS_HUMAN_TEAM,
                b: ai_rules,
            },
        }),
        handicaps: Some(DuosHandicaps {
            a1: Some(handicap),
            a2: Some(handicap),
            b1: Some(handicap),
            b2: Some(handicap),
        }),
        dealt: None,
    }
}
