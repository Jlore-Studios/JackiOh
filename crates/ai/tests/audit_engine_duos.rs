//! Audit of the engine's Duos mode (`crates/engine/src/duos/`, `duos_life.rs`): the tests that prove
//! the audit's reported finding. Each asserts the ruled behaviour and fails on the code as it stands.
//!
//! Finding: Stinkyscape #18.2 BALTHASAR's "less than half its starting health" reads a Duos hero's
//! starting health as 30 (`query::starting_health_of`: the handicap's `heroHealth`, else HERO_HEALTH),
//! while a Duos hero starts at its team's own HP, 20 (§13.2, §13.4).

use jackioh_engine::config::DUOS_HUMAN_TEAM;
use jackioh_engine::duos::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::A1;

const VANILLA: &str = "core-008"; // Mr. Vanilla
const BALTHASAR: &str = "stinkyscape-018-2";

/// What `duos_create` puts on a duel's side (`duos::state::duos_seat`): R82 off, its own HP, unspent,
/// on its team's full pool.
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

/// BALTHASAR's votes at the start of its controller's turn, with that hero on `own` health; `duos`
/// makes the game a Duos duel, both sides holding the humans' Duos life.
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

/// §13.2: "Each duel hero starts at its seat's own HP, and each team at its pool (§13.4), instead of at
/// 30 health"; §13.4: "A seat starts with its team's `own_health` as its duel hero's health" (20).
/// BALTHASAR: "Start of turn: Vote to remove you if your hero has less than half its starting health";
/// R1866002: it "compares its controller's hero health with the health that hero started the game
/// with". Half of 20 is 10, so a Duos hero on 14 of its 20 own HP has not lost half: no vote.
#[test]
fn balthasar_reads_a_duos_heros_starting_health_as_its_own_hp() {
    // The controls: outside Duos the hero started on 30 and 14 is below half; in Duos 9 of 20 is.
    assert_eq!(balthasar_votes(14, false), 1, "1v1: 14 of 30 is below half");
    assert_eq!(balthasar_votes(9, true), 1, "Duos: 9 of 20 is below half");
    // The finding: in Duos 14 of 20 is above half, and BALTHASAR votes all the same.
    assert_eq!(balthasar_votes(14, true), 0, "Duos: 14 of 20 is above half");
}

fn deck() -> Vec<String> {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 22, 23, 24, 25, 26, 28,
    ]
    .iter()
    .map(|n| format!("core-{n:03}"))
    .collect()
}

/// The same through the real wrapper: a game `duos_create` made, A1 on 14 of its 20 own HP with its
/// team's pool whole (34 of its 40 life left), a BALTHASAR in its backrow. Both teams end a round, and
/// round 3 opens on A1's turn, whose start is BALTHASAR's.
#[test]
fn balthasar_in_a_real_duos_game_does_not_vote_at_14_of_20_own_hp() {
    jackioh_cards::register_all();
    let decks = DuosDecks {
        a1: deck(),
        a2: deck(),
        b1: deck(),
        b2: deck(),
    };
    let mut state = duos_scenario("audit-balthasar", decks, None);
    assert_eq!(state.round, 1);
    assert_eq!(
        state.duels[0].players[P1].hero.health,
        DUOS_HUMAN_TEAM.own_health,
        "§13.4: the hero starts on 20"
    );
    // Quiet seats: empty hands, Vanilla libraries.
    for seat in DUOS_SEATS {
        let player = seat.player();
        let game = &mut state.duels[seat.duel()];
        game.players[player].hand.clear();
        game.players[player].library.clear();
        for _ in 0..10 {
            let card = new_instance(game, VANILLA, player, Zone::Library { player });
            game.players[player].library.push(card);
        }
    }
    let game = &mut state.duels[A1.duel()];
    // The first backrow zone that takes it.
    let placed = (0..BACKROW_ZONES).any(|lane| {
        let (player, row) = (P1, Row::Backrow);
        let mut brain = new_instance(game, BALTHASAR, player, Zone::Field { player, row, lane });
        place_on_field(game, &mut brain, ZoneSlot { player, row, lane }, PlaceOnFieldOptions::default())
    });
    assert!(placed, "BALTHASAR is in A1's backrow");
    game.players[P1].hero.health = 14;

    // Every seat ends its turn until round 3 opens on A1's turn; the votes cast in duel 0 on the way.
    let mut votes = 0;
    let mut steps = 0;
    while state.round < 3 {
        let seat = duos_seats_to_act(&state)[0];
        let result = duos_reduce(
            &state,
            &DuosAction {
                seat,
                nonce: format!("audit-end-{steps}"),
                body: DuosActionBody::Duel {
                    body: ActionBody::EndTurn,
                },
            },
        );
        assert_eq!(result.error, None, "{seat} ends its turn");
        votes += result
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    DuosEvent::Duel { duel: 0, event: GameEvent::VoteCast { def_id, .. } } if def_id == BALTHASAR
                )
            })
            .count();
        state = result.state;
        steps += 1;
        assert!(steps < 8, "four End turns reach round 3");
    }
    assert_eq!((state.round, state.round_seats[0]), (3, A1), "round 3 opened on A1's turn");
    let side = &state.duels[0].players[P1];
    assert_eq!(
        (side.hero.health, side.duos.map(|life| (life.pool, life.spent))),
        (14, Some((20, false))),
        "A1 still holds 14 of its 20 own HP and a whole pool"
    );
    // §13.2, R1866002: 14 is not less than half of the 20 the hero started the game with.
    assert_eq!(votes, 0, "BALTHASAR voted against a hero on 14 of its 20 starting health");
}
