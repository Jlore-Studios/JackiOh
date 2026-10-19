//! `duos_begin` and `duos_reduce` (#936 S4): one Duos step, then the settle that follows every step.
//!
//! A step is one of three things. A duel action goes to its duel's own `reduce`, as the seat's player,
//! under the action's nonce — except End turn, which only marks its seat ended (B5). A pass moves a
//! hand card to the teammate's duel (B20). A concede opens, or joins, the team's offer (B11).
//!
//! Then the settle, in order, until nothing moves (`DUOS_SETTLE_CAP` is its backstop):
//!   1. the judge: a team's pool at 0 or less, or a duel's own ending, ends the game (B16);
//!   2. round 1 opens once all four mulligans have resolved and both duels' first turns have begun (B2),
//!      on those first turns: one an effect has already ended counts its seat as ended (B7);
//!   3. a round seat with nothing left but End turn, and no pass it may make, is ended (B9, R82's rule);
//!   4. once both round seats have ended, End turn reaches duel 0, then duel 1 (B5) — each under a
//!      nonce of its own (`round_end_nonce`), so a fold replays it exactly — and only then is the
//!      game judged: the round's close is part of the step that closed it, so pools both End turns
//!      emptied are emptied in the same step (B16, §13.4);
//!   5. once both duels' turns have moved on, the next round opens with each duel's new active player
//!      as its round seat (B5, B8) — a turn an effect ended (R456) has moved already (B7).
//!
//! After every change to a duel the pools are made equal again (B15, `duel_changed`).
//!
//! The wrapper's own nonces (the settle after `duos_begin`, each round's End turns) are minted in a
//! namespace of their own, `DUOS_NONCE_PREFIX`, which no seat's action may carry, so no action can
//! take a nonce the round's close is about to use and turn that End turn into a duel's silent
//! replay (R270's stall). A duel that finds the round's nonce already applied refuses the End turn
//! rather than taking it as done.

use crate::catalog::fused_id_specs;
use crate::config::{DUOS_SETTLE_CAP, SETUP_TURN};
use crate::mana::mana_event;
use crate::reduce::{begin_game, reduce};
use crate::resolve::RETURN_PRICE_KEY;
use crate::state::{CardInstance, EngineError, GameState};
use crate::subsystems::hero_power::POWER_KEY;
use crate::wire::{
    Action, ActionBody, DUOS_DUELS, DUOS_REASON_CEILING, DUOS_REASON_CONCEDE, DUOS_REASON_DUEL_RESULT,
    DUOS_REASON_POOL_EMPTIED, DUOS_REASON_TURN_CAP, DUOS_REASON_VOIDED, DuosAction, DuosActionBody,
    DuosEvent, DuosOutcome, DuosResult, DuosSeat, GameEvent, GameOverReason, PerTeam, Phase, PlayerId, Team,
    Zone,
};

use super::legal::{auto_end_due, why_duel_refused, why_pass_refused};
use super::state::{DuosBegunTurn, DuosReduceResult, DuosState, duos_seat};

/// The namespace of the wrapper's own nonces: a seat's action whose nonce starts with it is refused,
/// so the nonces the round's close reaches the duels under are never a seat's (R270's rule, as the
/// server keeps `srv-` for its own).
pub const DUOS_NONCE_PREFIX: &str = "duos:";

/// The nonce the settle runs under after `duos_begin`, which has no action of its own.
const BEGIN_NONCE: &str = "duos:begin";

/// Deal both duels and open all four mulligans (B1, B2): each duel's `begin_game`, duel 0 first.
/// Refused once either duel has begun.
pub fn duos_begin(state: &DuosState) -> DuosReduceResult {
    let begun = state.result.is_some()
        || state.duels.iter().any(|game| {
            game.phase != Phase::Setup
                || game.pending.is_some()
                || game.mulligan.is_some()
                || !game.mulliganed.is_empty()
        });
    if begun {
        return DuosReduceResult::refused(state, EngineError::new("the game has already begun"));
    }
    let mut next = state.clone();
    let mut events: Vec<DuosEvent> = Vec::new();
    for duel in 0..DUOS_DUELS {
        let dealt = begin_game(&next.duels[duel]);
        next.duels[duel] = dealt.state;
        duel_changed(&mut next, duel, &dealt.events);
        push_duel_events(&mut events, duel, dealt.events);
    }
    settle(&mut next, BEGIN_NONCE, &mut events);
    DuosReduceResult {
        state: next,
        events,
        error: None,
    }
}

/// Apply one Duos action. A refused one comes back with the input state, no events and the reason.
pub fn duos_reduce(state: &DuosState, action: &DuosAction) -> DuosReduceResult {
    let mut next = state.clone();
    let mut events: Vec<DuosEvent> = Vec::new();
    if let Err(error) = apply(&mut next, action, &mut events) {
        return DuosReduceResult::refused(state, error);
    }
    settle(&mut next, &action.nonce, &mut events);
    DuosReduceResult {
        state: next,
        events,
        error: None,
    }
}

fn apply(state: &mut DuosState, action: &DuosAction, events: &mut Vec<DuosEvent>) -> Result<(), EngineError> {
    if state.result.is_some() {
        return Err(EngineError::new("the game is over"));
    }
    if action.nonce.starts_with(DUOS_NONCE_PREFIX) {
        return Err(EngineError::new(format!(
            r#"a nonce starting "{DUOS_NONCE_PREFIX}" is the round's own"#
        )));
    }
    let seat = action.seat;
    match &action.body {
        DuosActionBody::Duel { body } => {
            why_duel_refused(state, seat, body)?;
            let duel = seat.duel();
            if *body == ActionBody::EndTurn {
                // B5: nothing reaches the duel until the other round seat has ended too.
                state.ended[duel] = true;
                return Ok(());
            }
            let duel_action = Action::new(body.clone(), seat.player(), action.nonce.clone());
            let result = reduce(&state.duels[duel], &duel_action);
            if let Some(error) = result.error {
                return Err(EngineError::new(error));
            }
            state.duels[duel] = result.state;
            duel_changed(state, duel, &result.events);
            push_duel_events(events, duel, result.events);
            Ok(())
        }
        DuosActionBody::Pass { instance_id } => {
            why_pass_refused(state, seat, instance_id)?;
            pass(state, seat, instance_id, events);
            Ok(())
        }
        DuosActionBody::Concede => concede(state, seat, events),
    }
}

/// B11: the first concede opens the team's offer, the teammate's concede ends the game for the other
/// team, and the offerer's own second concede is refused (its offer already stands). Each team has
/// an offer of its own, so the other team's concede neither takes nor clears it.
fn concede(state: &mut DuosState, seat: DuosSeat, events: &mut Vec<DuosEvent>) -> Result<(), EngineError> {
    let team = seat.team();
    match state.concede_offer[team] {
        Some(offer) if offer == seat => Err(EngineError::new(
            "your offer to concede stands: the game ends once your teammate concedes too",
        )),
        Some(_) => {
            end(state, events, won_by(team.other(), DUOS_REASON_CONCEDE));
            Ok(())
        }
        None => {
            state.concede_offer[team] = Some(seat);
            Ok(())
        }
    }
}

/// B20: pay the pass cost, take the card out of the passer's hand and put it at the right end of the
/// teammate's under the next instance id of the teammate's duel, as `passed_card` makes it. Not a
/// draw, a play or a discard: nothing in either duel hears it, and only the mana it paid is reported
/// there. Its stay in the passer's hand ends as any removal from a hand ends one (R1140).
fn pass(state: &mut DuosState, seat: DuosSeat, instance_id: &str, events: &mut Vec<DuosEvent>) {
    let from = seat.duel();
    let mate = seat.teammate();
    let to = mate.duel();
    let cost = state.teams[seat.team()].rules.pass_cost;

    let side = &mut state.duels[from].players[seat.player()];
    let Some(at) = side.hand.iter().position(|card| card.id == instance_id) else {
        return;
    };
    let card = side.hand.remove(at);
    if cost != 0 {
        side.mana.current -= cost;
        let paid = mana_event(seat.player(), side);
        push_duel_events(events, from, vec![paid]);
    }
    crate::zones::forget_hand_watch(&mut state.duels[from], instance_id);

    carry_match_defs(state, from, to, &card.def_id);

    let target: &mut GameState = &mut state.duels[to];
    let new_instance_id = format!("c{}", target.next_id);
    target.next_id += 1;
    let receiver = mate.player();
    let moved = passed_card(card, new_instance_id.clone(), receiver);
    target.players[receiver].hand.push(moved);

    state.passes[from] += 1;
    events.push(DuosEvent::Passed {
        from: seat,
        to: mate,
        instance_id: instance_id.to_string(),
        new_instance_id,
    });
}

/// R1168000: the memory entries a passed card keeps — its own return price (R153) and Heroic Power's
/// rolled power (R43), which are the card itself. Anything else a card remembers in a hand names
/// something of the duel it left (a Fortify Mind's secret, R862; a Journey Complete's journey,
/// R1246), and those ids name other things in the teammate's duel (§13.1), so it stays behind.
const PASSED_MEMORY: &[&str] = &[RETURN_PRICE_KEY, POWER_KEY];

/// B20, R1168000: `card` as a pass puts it in `receiver`'s hand under `id`: owned and controlled by
/// the receiver, everything on it kept (buffs, cost modifiers, face) but the memory that names the
/// duel it left. The AI's pass model makes its copy here too, so the two cannot drift.
pub fn passed_card(card: CardInstance, id: String, receiver: PlayerId) -> CardInstance {
    let mut moved = card;
    moved.id = id;
    moved.owner = receiver;
    moved.controller = receiver;
    moved.zone = Zone::Hand { player: receiver };
    moved
        .memory
        .retain(|key, _| PASSED_MEMORY.contains(&key.as_str()));
    moved
}

/// A card whose definition lives in its duel alone — a fused or crafted card's (R77, R102) — takes it
/// to the duel it is passed into, and with it every match-made definition it is made of: a Fuse's
/// ingredients may be fused or crafted cards themselves, and the card's readers (its params, its
/// element) read them through it.
fn carry_match_defs(state: &mut DuosState, from: usize, to: usize, def_id: &str) {
    let mut queue: Vec<String> = vec![def_id.to_string()];
    while let Some(id) = queue.pop() {
        let source = &state.duels[from];
        let Some(def) = source.transient_defs.get(&id).cloned() else {
            continue;
        };
        if let Some(specs) = fused_id_specs(Some(source), &id) {
            queue.extend(specs.into_iter().map(|spec| spec.def_id));
        }
        state.duels[to].transient_defs.entry(id).or_insert(def);
    }
}

/// After every step, until nothing moves: judge, open round 1, end the seats with nothing left, close
/// the round, open the next (see the header).
fn settle(state: &mut DuosState, nonce: &str, events: &mut Vec<DuosEvent>) {
    for _pass in 0..DUOS_SETTLE_CAP {
        if judge(state, events) {
            return;
        }
        if state.round == 0 {
            if !first_round_due(state) {
                return;
            }
            open_round(state, events);
            continue;
        }

        let mut moved = false;
        for duel in 0..DUOS_DUELS {
            if auto_end_due(state, duel) {
                state.ended[duel] = true;
                moved = true;
            }
        }
        if state.ended.iter().all(|ended| *ended) {
            // Both End turns, then one judge (B16): what either did to the pools is the same step's.
            for duel in 0..DUOS_DUELS {
                if !state.end_applied[duel] && end_round_turn(state, duel, nonce, events) {
                    moved = true;
                }
            }
            if judge(state, events) {
                return;
            }
            if round_closed(state) {
                open_round(state, events);
                continue;
            }
        }
        if !moved {
            return;
        }
    }
}

/// B2: both duels have resolved their mulligans and begun their first turn.
fn first_round_due(state: &DuosState) -> bool {
    state
        .duels
        .iter()
        .all(|game| game.mulligan.is_none() && game.turn != SETUP_TURN)
}

/// B5: both round seats have ended, End turn has reached both duels, and both turns have moved on (an
/// End turn a prompt paused has not, until the prompt is answered).
fn round_closed(state: &DuosState) -> bool {
    state.ended.iter().all(|ended| *ended)
        && state.end_applied.iter().all(|applied| *applied)
        && (0..DUOS_DUELS).all(|duel| turn_moved(state, duel))
}

/// Whether duel `duel` has left the round's turn: its turn has moved on, or a Glitch reset (R676)
/// dealt it again, which is no turn of the round's however its new count runs (R1168002).
fn turn_moved(state: &DuosState, duel: usize) -> bool {
    state.round_turns[duel] == SETUP_TURN || state.duels[duel].turn != state.round_turns[duel]
}

/// B2, B5, B8, B22: the next round — each duel's round seat is the seat of the turn the duel has
/// begun past the last round's (`take_round_turn`; an extra turn, R846, keeps it on the same team),
/// nobody has ended or passed, and each team's open concede offer lapses (B11).
///
/// Round 1 is both duels' first turns, team A's (B2). A duel whose first turn an effect already ended
/// while the other duel's mulligans were still open (R456: a Tommy Tempo drawn at its start) opens
/// round 1 on that turn all the same, its seat counted as ended, and so does any later round whose
/// turn ended as it began (R1168001): opening a round on the duel's new active player instead would
/// leave that duel a turn ahead of the other for the rest of the game.
fn open_round(state: &mut DuosState, events: &mut Vec<DuosEvent>) {
    state.round += 1;
    for duel in 0..DUOS_DUELS {
        state.passes[duel] = 0;
        if take_round_turn(state, duel) {
            continue;
        }
        // No turn begun since the last round's: a duel a Glitch reset (R676) sent back to its setup,
        // whose first turn joins the round as it begins (`duel_changed`).
        let game = &state.duels[duel];
        state.round_seats[duel] = DuosSeat::of(duel, game.active);
        state.round_turns[duel] = game.turn;
        state.ended[duel] = false;
        state.end_applied[duel] = false;
    }
    state.concede_offer = PerTeam::new(None, None);
    events.push(DuosEvent::RoundStarted { round: state.round });
}

/// R1168001: give duel `duel` the round's turn — the first turn it has begun past the last round's,
/// whose seat is the round seat. One the duel has already moved past (it ended as it began, R456)
/// counts as ended with End turn already reached (B7), so the duel's new active player waits for the
/// round to close and each duel's turn moves once a round (§13.3). Whether there was one.
fn take_round_turn(state: &mut DuosState, duel: usize) -> bool {
    let after = state.round_turns[duel];
    let begun = &mut state.begun[duel];
    let Some(at) = begun.iter().position(|begun| begun.turn > after) else {
        return false;
    };
    let taken = begun[at];
    begun.drain(..=at);
    let over = state.duels[duel].turn > taken.turn;
    state.round_seats[duel] = DuosSeat::of(duel, taken.player);
    state.round_turns[duel] = taken.turn;
    state.ended[duel] = over;
    state.end_applied[duel] = over;
    true
}

/// R1168001: queue the turns duel `duel` began in `events`, in order. A turn numbered at or below
/// one already queued means its count started over (a Glitch reset, R676): what was queued goes.
fn record_begun(state: &mut DuosState, duel: usize, events: &[GameEvent]) {
    for event in events {
        let GameEvent::TurnStarted { player, turn, .. } = event else {
            continue;
        };
        let begun = &mut state.begun[duel];
        if begun.last().is_some_and(|last| last.turn >= *turn) {
            begun.clear();
        }
        begun.push(DuosBegunTurn {
            turn: *turn,
            player: *player,
        });
    }
}

/// B5: End turn reaches `duel` at the round's close, as its active player. Whether it did: a duel
/// that refuses it (it cannot while its round seat has ended with no prompt open) is tried again
/// after the next step. So is one that has applied the round's nonce already: its `reduce` would
/// answer that replay with the state unchanged and no error, and taking that as the End turn would
/// leave the duel's turn where it is and the round closed for ever.
fn end_round_turn(state: &mut DuosState, duel: usize, nonce: &str, events: &mut Vec<DuosEvent>) -> bool {
    let game = &state.duels[duel];
    let nonce = round_end_nonce(nonce, state.round, duel);
    if game.applied.iter().any(|applied| applied.nonce == nonce) {
        return false;
    }
    let action = Action::new(ActionBody::EndTurn, game.active, nonce);
    let result = reduce(game, &action);
    if result.error.is_some() {
        return false;
    }
    state.end_applied[duel] = true;
    state.duels[duel] = result.state;
    duel_changed(state, duel, &result.events);
    push_duel_events(events, duel, result.events);
    true
}

/// The nonce the wrapper's own End turn reaches a duel under: in the wrapper's namespace, with the
/// round, the duel and the closing action's nonce, so no seat's action carries it, no two of a
/// game's are alike and a fold replays them exactly.
fn round_end_nonce(nonce: &str, round: i32, duel: usize) -> String {
    format!("{DUOS_NONCE_PREFIX}end:r{round}:d{duel}:{nonce}")
}

/// What follows any change to duel `duel`, whose step produced `events` (B7, B15, R1168001).
fn duel_changed(state: &mut DuosState, duel: usize, events: &[GameEvent]) {
    sync_pools(state, duel);
    record_begun(state, duel, events);
    if state.round == 0 {
        return;
    }
    if state.duels[duel].turn < state.round_turns[duel] {
        // R1168002: a Glitch reset (R676) dealt the duel again and its turn count started over. The
        // round's turn there is over, and no turn it began before the reset is one a round takes.
        state.begun[duel].clear();
        state.round_turns[duel] = SETUP_TURN;
        state.ended[duel] = true;
        state.end_applied[duel] = true;
        return;
    }
    if state.end_applied[duel] || state.duels[duel].turn == state.round_turns[duel] {
        return;
    }
    if state.round_turns[duel] == SETUP_TURN {
        // A duel a Glitch reset sent back to its setup joins the round as its first turn begins.
        take_round_turn(state, duel);
        return;
    }
    // B7: the turn moved without the round closing — an effect ended it (R456) — so the round seat
    // has ended, and End turn has reached the duel already.
    state.ended[duel] = true;
    state.end_applied[duel] = true;
}

/// B15: each team's pool as duel `from` left it, copied to the team's seat in every other duel, and
/// to that hero's health while it is spent (a spent hero's health reads the pool, B13). A seat that
/// has lost its Duos life — a Glitch reset deals a duel again (R676) — is made a Duos seat again: its
/// own HP back at the start, unspent, on its team's pool, and R82 off (`duos_seat`).
fn sync_pools(state: &mut DuosState, from: usize) {
    for &team in Team::ALL {
        let player = team.player();
        if let Some(life) = state.duels[from].players[player].duos {
            state.teams[team].pool = life.pool;
        }
        let pool = state.teams[team].pool;
        let rules = state.teams[team].rules;
        for (duel, game) in state.duels.iter_mut().enumerate() {
            let side = &mut game.players[player];
            match side.duos.as_mut() {
                Some(life) if duel != from => {
                    life.pool = pool;
                    if life.spent {
                        side.hero.health = pool;
                    }
                }
                Some(_) => {}
                None => duos_seat(side, &rules, pool),
            }
        }
    }
}

/// B16: end the game if it is over. Whether it is.
fn judge(state: &mut DuosState, events: &mut Vec<DuosEvent>) -> bool {
    if state.result.is_some() {
        return true;
    }
    let Some(result) = result_now(state) else {
        return false;
    };
    end(state, events, result);
    true
}

/// B16: a team whose pool is at 0 or less loses (both at once: a draw); otherwise a duel's own ending
/// ends the game with its team's result — `turnCap` for R2's cap, `ceiling` for R79's, `voided` for a
/// Glitch's void (R679, R1168002), `duelResult` for any other. Two duels that ended at once and
/// disagree are a draw.
fn result_now(state: &DuosState) -> Option<DuosResult> {
    let emptied: Vec<Team> = Team::ALL
        .iter()
        .copied()
        .filter(|team| state.teams[*team].pool <= 0)
        .collect();
    match emptied.as_slice() {
        [] => {}
        [loser] => return Some(won_by(loser.other(), DUOS_REASON_POOL_EMPTIED)),
        _ => return Some(drawn(DUOS_REASON_POOL_EMPTIED)),
    }
    let mut found: Option<DuosResult> = None;
    for game in &state.duels {
        let Some(ended) = game.result else {
            continue;
        };
        let reason = match ended.reason {
            GameOverReason::TurnCap => DUOS_REASON_TURN_CAP,
            GameOverReason::MatchCeiling => DUOS_REASON_CEILING,
            GameOverReason::Voided => DUOS_REASON_VOIDED,
            _ => DUOS_REASON_DUEL_RESULT,
        };
        let this = match ended.winner.player() {
            Some(player) => won_by(Team::of(player), reason),
            None => drawn(reason),
        };
        found = Some(match found {
            None => this,
            Some(first) if first.winner == this.winner => first,
            Some(first) => drawn(&first.reason),
        });
    }
    found
}

fn won_by(team: Team, reason: &str) -> DuosResult {
    DuosResult {
        outcome: DuosOutcome::Win,
        winner: Some(team),
        reason: reason.to_string(),
    }
}

fn drawn(reason: &str) -> DuosResult {
    DuosResult {
        outcome: DuosOutcome::Draw,
        winner: None,
        reason: reason.to_string(),
    }
}

fn end(state: &mut DuosState, events: &mut Vec<DuosEvent>, result: DuosResult) {
    events.push(DuosEvent::Ended {
        result: result.clone(),
    });
    state.result = Some(result);
}

fn push_duel_events(events: &mut Vec<DuosEvent>, duel: usize, from_duel: Vec<GameEvent>) {
    let duel = duel as u8;
    events.extend(from_duel.into_iter().map(|event| DuosEvent::Duel { duel, event }));
}
