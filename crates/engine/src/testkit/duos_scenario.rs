//! `duos_scenario` (#936 S4): a begun Duos game for a test, built out of the wrapper's own functions
//! (`duos_create`, `duos_begin`, `duos_reduce`) as `scenario()` is out of the engine's. Every seat
//! keeps its whole opening hand, so a test starts at round 1 with A1 and A2 to act. The decks are
//! dealt from the registered catalog, the testkit's override included (`register_catalog`).

use crate::duos::{DuosState, duos_begin, duos_create, duos_reduce};
use crate::setup::mulligan_prompt_for;
use crate::wire::{
    ActionBody, DUOS_SEATS, DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosTeamRules, PerTeam,
};

/// A begun Duos game from four decks (A1, A2, B1, B2), every mulligan kept, round 1 open; `teams`
/// as `DuosCreateOptions.teams`. Panics naming the seat if a mulligan is refused.
pub fn duos_scenario(seed: &str, decks: DuosDecks, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    let created = duos_create(&DuosCreateOptions {
        seed: seed.to_string(),
        decks,
        teams,
        ..DuosCreateOptions::default()
    });
    let mut state = duos_begin(&created).state;
    for seat in DUOS_SEATS {
        let Some(prompt) = mulligan_prompt_for(&state.duels[seat.duel()], seat.player()) else {
            continue;
        };
        let keep: Vec<String> = prompt.options.iter().map(|option| option.key.clone()).collect();
        let action = DuosAction {
            seat,
            nonce: format!("duos-scenario-mulligan-{seat}"),
            body: DuosActionBody::Duel {
                body: ActionBody::Mulligan { keep },
            },
        };
        let result = duos_reduce(&state, &action);
        if let Some(error) = result.error {
            panic!("duos_scenario: {seat}'s mulligan was refused: {error}");
        }
        state = result.state;
    }
    state
}
