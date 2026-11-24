//! Audit of the engine's Duos mode (`crates/engine/src/duos/`, `duos_life.rs`). Temporary.

use jackioh_engine::config::DUOS_HUMAN_TEAM;
use jackioh_engine::duos::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::{A1, A2, B1, B2};

const VANILLA: &str = "core-008"; // Mr. Vanilla
const BALTHASAR: &str = "stinkyscape-018-2";

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

/// What `duos_create` puts on a duel's side (`duos_seat`): its team's own HP, unspent, on a full pool.
fn duos_side(s: &mut Scenario, player: PlayerId, own: i32) {
    let side = &mut s.state_mut().players[player];
    side.hero.health = own;
    side.auto_end_turn = Some(false);
    side.duos = Some(DuosLife {
        pool: DUOS_HUMAN_TEAM.pool_health,
        pool_cap: DUOS_HUMAN_TEAM.pool_health,
        spent: false,
    });
}

fn balthasar_votes(own: i32, duos: bool) -> usize {
    jackioh_cards::register_all();
    let library = vec![VANILLA; 4];
    let mut s = scenario(json!({
        "active": "p2",
        "p1": { "hand": [VANILLA], "backrow": [BALTHASAR], "health": own, "library": library },
        "p2": { "hand": [VANILLA], "library": library },
    }));
    if duos {
        duos_side(&mut s, P1, own);
        duos_side(&mut s, P2, DUOS_HUMAN_TEAM.own_health);
    }
    s.end_turn();
    assert_eq!(s.state().active, P1);
    s.last_events()
        .iter()
        .filter(|event| matches!(event, GameEvent::VoteCast { def_id, .. } if def_id == BALTHASAR))
        .count()
}

/// §13.2: "Each duel hero starts at its seat's own HP ... instead of at 30 health"; §13.4: a seat starts
/// with its team's `own_health` (20). BALTHASAR: "Vote to remove you if your hero has less than half its
/// starting health" (R1866002: "the health that hero started the game with"). Half of 20 is 10, so a
/// Duos hero on 14 of its 20 own HP has not lost half and BALTHASAR casts no vote.
#[test]
fn balthasar_reads_a_duos_heros_starting_health_as_its_own_hp() {
    // The control: outside Duos the hero started on 30, and 14 is below half.
    assert_eq!(balthasar_votes(14, false), 1, "1v1: 14 of 30 is below half");
    // In Duos, 9 of 20 is below half.
    assert_eq!(balthasar_votes(9, true), 1, "Duos: 9 of 20 is below half");
    // In Duos, 14 of 20 is not.
    assert_eq!(balthasar_votes(14, true), 0, "Duos: 14 of 20 is above half");
}

#[test]
fn smoke() {
    let state = begun_with("audit-smoke", None);
    assert_eq!(state.round, 1);
    let _ = (A1, A2, B1, B2, VANILLA);
}
