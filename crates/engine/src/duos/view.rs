//! `duos_view_for` (#936 B23): what one seat may see: its own duel as its 1v1 seat, its teammate's
//! as the teammate's own 1v1 view shows it, and both teams' life. Who has ended and how many passes
//! are left is the seat's own team's business: the enemy's read false and 0.

use crate::view_for::view_for;
use crate::wire::{DuosSeat, DuosSideLife, DuosTeamView, DuosView, Team};

use super::state::DuosState;

pub fn duos_view_for(state: &DuosState, seat: DuosSeat) -> DuosView {
    let mate = seat.teammate();
    DuosView {
        seat,
        own: view_for(&state.duels[seat.duel()], seat.player()),
        teammate: view_for(&state.duels[mate.duel()], mate.player()),
        team: team_view(state, seat.team(), true),
        enemy: team_view(state, seat.team().other(), false),
        round: state.round,
        round_seats: state.round_seats.clone(),
        concede_offered_by: state.concede_offer[seat.team()],
        result: state.result.clone(),
    }
}

/// One team's life, and for the viewer's own team (`own`) its round: B22's passes left, B63's ended.
fn team_view(state: &DuosState, team: Team, own: bool) -> DuosTeamView {
    let seats = team.seats();
    let rules = state.teams[team].rules;
    DuosTeamView {
        team,
        pool: state.teams[team].pool,
        pool_cap: rules.pool_health,
        sides: seats.iter().map(|seat| side_life(state, *seat)).collect(),
        ended: seats.iter().map(|seat| own && has_ended(state, *seat)).collect(),
        passes_left: seats
            .iter()
            .map(|seat| if own { passes_left(state, *seat) } else { 0 })
            .collect(),
        pass_cost: rules.pass_cost,
    }
}

/// B13: a seat's own HP, 0 once spent (its hero's health then reads the pool).
fn side_life(state: &DuosState, seat: DuosSeat) -> DuosSideLife {
    let side = &state.duels[seat.duel()].players[seat.player()];
    let spent = side.duos.is_some_and(|life| life.spent);
    DuosSideLife {
        own: if spent { 0 } else { side.hero.health },
        spent,
    }
}

/// B5, B7: `seat` is its duel's round seat and has ended its turn this round.
fn has_ended(state: &DuosState, seat: DuosSeat) -> bool {
    let duel = seat.duel();
    state.round > 0 && state.round_seats[duel] == seat && state.ended[duel]
}

/// B22: the passes `seat` may still make this round.
fn passes_left(state: &DuosState, seat: DuosSeat) -> i32 {
    let duel = seat.duel();
    let made = if state.round_seats[duel] == seat {
        state.passes[duel]
    } else {
        0
    };
    (state.teams[seat.team()].rules.pass_cap - made).max(0)
}
