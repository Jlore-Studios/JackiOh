//! R2660000: the ladder bots' concession check (SPEC §9.12): whether a lethal shows on the opponent's
//! board that nothing `seat` does this turn stops. Both turns are played through the engine: every
//! line of `seat`'s own turn, and for each, the opponent's lethal solver (§9.9's `find_lethal`) on
//! the turn it hands over. Like `lethal_hint` (R2102000) it reads the true state only through
//! `redact` (R185), and it counts only what shows: the opponent's hand and face-down cards are taken
//! out, so none of them can make the lethal nor stop the seat's answer to it. Every doubt answers
//! false: a cut walk, a failed step, a roll of the game's random numbers, a card drawn from a library
//! the seat cannot see into, a choice the seat would make in the opponent's turn, or a panic. So a bot
//! that asks it concedes only when the loss is certain, and plays on whenever it is not proved.

use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use indexmap::IndexSet;
use jackioh_engine::{
    Action, ActionBody, ActionType, CardInstance, GameState, Phase, PlayerId, Rng, legal_actions,
    reduce_unpiloted, subsystems,
};

use crate::config::{
    UNSTOPPABLE_LETHAL_DETERMINIZATIONS, UNSTOPPABLE_LETHAL_NODES, UNSTOPPABLE_LETHAL_REPLY_NODES,
    UNSTOPPABLE_LETHAL_SEED,
};
use crate::determinize::{DeterminizeOptions, det_ground, determinize_on};
use crate::evaluate::Prices;
use crate::lethal::find_lethal;
use crate::observe::{HIDDEN_DEF_ID, redact};
use crate::simulate::{LineStatus, create_node_counter, line_status, search_signature, simulate, unit_lanes};
use crate::types::NodeCounter;

/// R2660000: whether `seat`, at its own turn, faces a lethal showing on the opponent's board that nothing
/// it can do this turn stops. It fills `redact(state, seat)` minus the opponent's hand and face-down cards
/// (`showing_only`) as `UNSTOPPABLE_LETHAL_DETERMINIZATIONS` worlds drawn from `UNSTOPPABLE_LETHAL_SEED`'s
/// own stream, and holds only when every line of the seat's turn loses on every one (`doomed_on`), within
/// `UNSTOPPABLE_LETHAL_NODES` nodes in all. False outside the seat's own main phase, with a prompt open,
/// after the game is over, on a panic, and when the walk is cut short by the nodes or by `should_stop`.
pub fn unstoppable_lethal(state: &GameState, seat: PlayerId, should_stop: Option<&dyn Fn() -> bool>) -> bool {
    let own_main = state.active == seat && state.phase == Phase::Main;
    if state.result.is_some() || state.pending.is_some() || !own_main {
        return false;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let public = showing_only(redact(state, seat), seat);
        let ground = det_ground(&public, seat);
        let mut rng = Rng::new(UNSTOPPABLE_LETHAL_SEED, 0);
        let counter = create_node_counter(UNSTOPPABLE_LETHAL_NODES, should_stop);
        (0..UNSTOPPABLE_LETHAL_DETERMINIZATIONS).all(|_| {
            let world = determinize_on(&public, seat, &mut rng, DeterminizeOptions::default(), &ground);
            doomed_on(&world, seat, &counter)
        })
    }))
    .unwrap_or(false)
}

/// R2660000: the redacted state with only what shows on the opponent's side: no hand, and no face-down
/// card (`HIDDEN_DEF_ID`) in its backrow, beneath a backrow pile or waiting to be set, so no unseen trap
/// can stop the seat's answer and no unseen card can make the lethal. A pile whose top was hidden goes
/// with it. Both automatic turn ends (R345) are off: the opponent's turn never runs on into the seat's
/// own draw or fatigue, and only "end turn" hands the turn over (`step_from`), as at R82.
fn showing_only(mut public: GameState, seat: PlayerId) -> GameState {
    let hidden = |card: &CardInstance| card.def_id == HIDDEN_DEF_ID;
    public.players[seat].auto_end_turn = Some(false);
    let side = &mut public.players[seat.opponent()];
    side.hand.clear();
    side.auto_end_turn = Some(false);
    side.resolving.retain(|card| !hidden(card));
    for (lane, slot) in side.backrow.iter_mut().enumerate() {
        let top_hidden = slot.as_ref().is_some_and(hidden);
        if top_hidden {
            *slot = None;
        }
        if let Some(pile) = side.backrow_piles.as_mut().and_then(|piles| piles.get_mut(lane)) {
            if top_hidden {
                pile.clear();
            } else {
                pile.retain(|card| !hidden(card));
            }
        }
    }
    public
}

/// R2660000: whether every line `seat` can play this turn on `world` ends lost: by the end of its own
/// turn, or by a lethal the opponent's board then has (`board_lethal`). It tries every legal action but
/// the ones R84 skips and the mulligan, "end turn" first, a position once (`position_key`). A step that
/// fails, that the nodes cut, or that leaves anything to chance (`sure_step`) is an escape, and so is a
/// line that ends in anything but the opponent's win.
fn doomed_on(world: &GameState, seat: PlayerId, counter: &dyn NodeCounter) -> bool {
    let root_turn = world.turn;
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(position_key(world, seat));
    let mut stack: Vec<(Rc<GameState>, ActionBody)> = Vec::new();
    push_moves(&mut stack, Rc::new(world.clone()), seat);
    while let Some((state, action)) = stack.pop() {
        let from = step_from(&state, &action);
        let next = match simulate(&from, seat, &action, counter, Prices::NONE) {
            Some(Ok(next)) if sure_step(&from, &next, seat) => next,
            _ => return false,
        };
        match line_status(&next, seat, root_turn) {
            LineStatus::Over => {
                if !lost(&next, seat) {
                    return false;
                }
            }
            LineStatus::Passed | LineStatus::Yielded => {
                if !board_lethal(&next, seat, counter) {
                    return false;
                }
            }
            LineStatus::Open => {
                if visited.insert(position_key(&next, seat)) {
                    push_moves(&mut stack, Rc::new(next), seat);
                }
            }
        }
    }
    true
}

/// Pushes `seat`'s moves on `state` onto the walk's stack: every legal action but R84's skipped ones
/// (a concession, a draw offer or answer, an emote) and the mulligan, "end turn" pushed last so that
/// it is tried first.
fn push_moves(stack: &mut Vec<(Rc<GameState>, ActionBody)>, state: Rc<GameState>, seat: PlayerId) {
    let skipped = subsystems::AI_SKIPPED_ACTIONS;
    let mut moves: Vec<ActionBody> = legal_actions(&state, seat)
        .into_iter()
        .filter(|action| {
            let kind = action.action_type();
            !skipped.contains(&kind) && kind != ActionType::Mulligan
        })
        .collect();
    moves.sort_by_key(|action| matches!(action, ActionBody::EndTurn));
    for action in moves {
        stack.push((Rc::clone(&state), action));
    }
}

/// R2660000: the state `action` is played from: `state`, but for "end turn" without the Marketplace
/// (R1853000). Ending the turn is the seat's last use of it, and the opponent's turn start would roll it
/// new offers on the match's random numbers, which `sure_step` refuses, so no game with a Marketplace
/// could ever be proved. Taking it out takes away only the opponent's options.
fn step_from<'a>(state: &'a GameState, action: &ActionBody) -> Cow<'a, GameState> {
    if matches!(action, ActionBody::EndTurn) && state.marketplace.is_some() {
        let mut from = state.clone();
        from.marketplace = None;
        Cow::Owned(from)
    } else {
        Cow::Borrowed(state)
    }
}

/// A position of the walk: the search's own identity for it (`search_signature`) and the units'
/// lanes, which that leaves out, so a unit played into another lane is another position.
fn position_key(state: &GameState, seat: PlayerId) -> String {
    format!("{}|{}", search_signature(state, seat), unit_lanes(state))
}

/// R2660000: whether one step from `before` to `after` left nothing to chance: it drew no random
/// number from the game's stream, and took no card from `drawer`'s library or put none in it. The
/// walk sees one outcome of a roll, and one guessed order of a library; any other could be the
/// escape, so a step that depends on either proves nothing.
fn sure_step(before: &GameState, after: &GameState, drawer: PlayerId) -> bool {
    let ids = |state: &GameState| -> Vec<String> {
        state.players[drawer]
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect()
    };
    after.rng_cursor == before.rng_cursor && ids(before) == ids(after)
}

/// Whether the game on `state` is over and `seat`'s opponent won it.
fn lost(state: &GameState, seat: PlayerId) -> bool {
    state
        .result
        .as_ref()
        .is_some_and(|result| result.winner.player() == Some(seat.opponent()))
}

/// R2660000: whether `seat`'s opponent, on the turn `state` hands it, has a lethal on its board alone.
/// The card it just drew is dropped with its hand, and the line `find_lethal` proves within
/// `UNSTOPPABLE_LETHAL_REPLY_NODES` is played again step by step: it counts only when it draws no random
/// number and no card of the opponent's library (`sure_step`) and opens no prompt for `seat`, whose answer
/// the solver would only have guessed. A lethal that needs the turn to end (an end-of-turn trigger) is not
/// counted: the solver looks inside one turn, and that errs only towards playing on.
fn board_lethal(state: &GameState, seat: PlayerId, counter: &dyn NodeCounter) -> bool {
    let opp = seat.opponent();
    let mut leaf = state.clone();
    leaf.players[opp].hand.clear();
    let Some(line) = find_lethal(
        std::slice::from_ref(&leaf),
        opp,
        counter,
        UNSTOPPABLE_LETHAL_REPLY_NODES,
        Prices::NONE,
    ) else {
        return false;
    };
    let mut at = leaf;
    for (step, action) in line.into_iter().enumerate() {
        if at.result.is_some() {
            break;
        }
        if !counter.take() {
            return false;
        }
        let played = reduce_unpiloted(&at, &Action::new(action, opp, format!("unstoppable:{step}")));
        let asks_seat = played
            .state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.player_id == seat);
        if played.error.is_some() || asks_seat || !sure_step(&at, &played.state, opp) {
            return false;
        }
        at = played.state;
    }
    lost(&at, seat)
}
