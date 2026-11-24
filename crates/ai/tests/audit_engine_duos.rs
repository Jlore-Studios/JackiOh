//! Audit of the engine's Duos mode (`crates/engine/src/duos/`, `duos_life.rs`). Temporary.

use jackioh_engine::duos::*;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::{A1, A2, B1, B2};

const VANILLA: &str = "core-008"; // Mr. Vanilla

fn deck() -> Vec<String> {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 22, 23, 24, 25, 26, 28,
    ]
    .iter()
    .map(|n| format!("core-{n:03}"))
    .collect()
}

fn begun_with(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    jackioh_cards::register_all();
    let decks = DuosDecks {
        a1: deck(),
        a2: deck(),
        b1: deck(),
        b2: deck(),
    };
    duos_scenario(seed, decks, teams)
}

#[test]
fn smoke() {
    let state = begun_with("audit-smoke", None);
    assert_eq!(state.round, 1);
    let _ = (A1, A2, B1, B2, VANILLA);
}
