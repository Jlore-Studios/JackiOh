//! What each Duos seat may send (#936 B2, B4, B6, B9, B10, B19) and why the reducer refuses the rest.
//! `duos_legal_actions` and `duos_reduce` ask the same functions, so a greyed-out button and a
//! refusal cannot disagree (§9.3). A seat's duel actions are its duel's own `legal_actions` filtered by
//! the round: a prompt's answer and its mulligan always (B2, B6); a main-phase action only as its duel's
//! round seat before it ends (B4); never a draw offer, the duel's concede or the setting R82 reads (B9,
//! B10). The wrapper applies R82 itself (§13.3).

use std::ops::ControlFlow;

use crate::query::{hand_cap_of, room_in};
use crate::reduce::{each_legal_action, legal_actions};
use crate::state::EngineError;
use crate::wire::{ActionBody, ActionType, DUOS_SEATS, DuosActionBody, DuosSeat, Phase};
use crate::zones::OffFieldZone;

use super::state::DuosState;

/// R82's "nothing left but ending the turn" (B9); R82 never counts an emote as something to do.
const NOTHING_LEFT: &[ActionType] = &[
    ActionType::EndTurn,
    ActionType::Concede,
    ActionType::OfferDraw,
    ActionType::AnswerDraw,
    ActionType::Emote,
];

/// Every action `seat` may send now, in order: its duel actions, its passes (hand order), then concede
/// unless its own offer stands (B11).
pub fn duos_legal_actions(state: &DuosState, seat: DuosSeat) -> Vec<DuosActionBody> {
    if state.result.is_some() {
        return Vec::new();
    }
    let game = &state.duels[seat.duel()];
    let mut out: Vec<DuosActionBody> = legal_actions(game, seat.player())
        .into_iter()
        .filter(|body| offered(state, seat, body))
        .map(|body| DuosActionBody::Duel { body })
        .collect();
    if may_take_turn_actions(state, seat).is_ok() {
        for card in &game.players[seat.player()].hand {
            if why_pass_refused(state, seat, &card.id).is_ok() {
                out.push(DuosActionBody::Pass {
                    instance_id: card.id.clone(),
                });
            }
        }
    }
    if state.concede_offer[seat.team()] != Some(seat) {
        out.push(DuosActionBody::Concede);
    }
    out
}

/// The seats whose legal actions hold more than concede, in seat order: who the game waits on.
pub fn duos_seats_to_act(state: &DuosState) -> Vec<DuosSeat> {
    DUOS_SEATS
        .into_iter()
        .filter(|seat| {
            duos_legal_actions(state, *seat)
                .iter()
                .any(|action| *action != DuosActionBody::Concede)
        })
        .collect()
}

/// Whether Duos offers `seat` this duel action; the R79 clock actions are the server's to send.
fn offered(state: &DuosState, seat: DuosSeat, body: &ActionBody) -> bool {
    body.action_type() != ActionType::CeilingReached && why_duel_refused(state, seat, body).is_ok()
}

/// Why Duos refuses `body` from `seat` before the duel's own `reduce` judges it.
pub(super) fn why_duel_refused(
    state: &DuosState,
    seat: DuosSeat,
    body: &ActionBody,
) -> Result<(), EngineError> {
    match body.action_type() {
        // B10, B11: a seat concedes with its team.
        ActionType::Concede => Err(EngineError::new(
            "concede with your team: send concede, and the game ends when your teammate does too",
        )),
        ActionType::OfferDraw | ActionType::AnswerDraw => {
            Err(EngineError::new("draw offers are not part of Duos"))
        }
        // B9: the wrapper ends a round seat's turn when nothing is left, so the duel's setting stays off.
        ActionType::SetAutoEndTurn => Err(EngineError::new(
            "in Duos the round ends a turn with nothing left to do",
        )),
        // B37, B38: the clocks act through the seats' actions; a team never loses to a disconnect.
        ActionType::Timeout | ActionType::DisconnectExpired => Err(EngineError::new(
            "in Duos the clocks act through the seats' own actions",
        )),
        // R79's wall-clock ceiling is the server's to send; the duel's draw ends the game (`ceiling`).
        ActionType::CeilingReached => Ok(()),
        // B2, B6: a mulligan and a prompt's answer are the duel's to judge, at any time.
        ActionType::Mulligan | ActionType::Answer => Ok(()),
        // B5: End turn only marks the seat ended, so what the duel would check is checked here.
        ActionType::EndTurn => {
            may_take_turn_actions(state, seat)?;
            may_act_in_main_phase(state, seat)
        }
        _ => may_take_turn_actions(state, seat),
    }
}

/// B2, B4, B6: a main-phase action needs the round begun and `seat` its duel's round seat, not ended.
fn may_take_turn_actions(state: &DuosState, seat: DuosSeat) -> Result<(), EngineError> {
    if state.result.is_some() {
        return Err(EngineError::new("the game is over"));
    }
    if state.round == 0 {
        return Err(EngineError::new(
            "the round has not begun: every seat's mulligan comes first",
        ));
    }
    let duel = seat.duel();
    if state.round_seats[duel] != seat {
        return Err(EngineError::new("it is not your turn"));
    }
    if state.ended[duel] {
        return Err(EngineError::new(
            "you have ended your turn this round: it ends with your teammate's",
        ));
    }
    Ok(())
}

/// What the duel's own reducer would check of the actions Duos handles itself (End turn, a pass).
fn may_act_in_main_phase(state: &DuosState, seat: DuosSeat) -> Result<(), EngineError> {
    let game = &state.duels[seat.duel()];
    if game.result.is_some() {
        return Err(EngineError::new("the game is over"));
    }
    if game.pending.is_some() {
        return Err(EngineError::new("a prompt is open: answer it first"));
    }
    if game.active != seat.player() || game.phase != Phase::Main {
        return Err(EngineError::new("it is not your turn"));
    }
    Ok(())
}

/// B19: a pass needs a round seat in its main phase, holding the card, with its team's pass cost in
/// mana, under its team's pass cap this round, and a teammate's hand below its cap.
pub(super) fn why_pass_refused(
    state: &DuosState,
    seat: DuosSeat,
    instance_id: &str,
) -> Result<(), EngineError> {
    may_take_turn_actions(state, seat)?;
    may_act_in_main_phase(state, seat)?;
    let duel = seat.duel();
    let side = &state.duels[duel].players[seat.player()];
    if !side.hand.iter().any(|card| card.id == instance_id) {
        return Err(EngineError::new(format!("no card {instance_id} in your hand")));
    }
    let rules = state.teams[seat.team()].rules;
    let (cost, mana) = (rules.pass_cost, side.mana.current);
    if mana < cost {
        return Err(EngineError::new(format!(
            "a pass costs {cost} mana and you have {mana}"
        )));
    }
    let passes = rules.pass_cap;
    if state.passes[duel] >= passes {
        return Err(EngineError::new(format!(
            "you have made all {passes} of this round's passes"
        )));
    }
    let mate = seat.teammate();
    let theirs = &state.duels[mate.duel()];
    if room_in(theirs, mate.player(), OffFieldZone::Hand) == 0 {
        let cap = hand_cap_of(theirs, mate.player());
        return Err(EngineError::new(format!(
            "your teammate's hand is full ({cap} cards)"
        )));
    }
    Ok(())
}

/// B9, R82: whether the wrapper ends `duel`'s round seat now, its duel offering it only `NOTHING_LEFT`;
/// a card it could still pass keeps it in the round (§13.3).
pub(super) fn auto_end_due(state: &DuosState, duel: usize) -> bool {
    if state.result.is_some() || state.round == 0 || state.ended[duel] {
        return false;
    }
    let seat = state.round_seats[duel];
    if may_act_in_main_phase(state, seat).is_err() {
        return false;
    }
    let game = &state.duels[duel];
    let may_pass = game.players[seat.player()]
        .hand
        .iter()
        .any(|card| why_pass_refused(state, seat, &card.id).is_ok());
    if may_pass {
        return false;
    }
    // As the 1v1 `auto_end_due` asks it: the walk stops at the first action that is something to do.
    let walk = each_legal_action(game, seat.player(), &mut |action| {
        if NOTHING_LEFT.contains(&action.action_type()) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    });
    walk.is_continue()
}
