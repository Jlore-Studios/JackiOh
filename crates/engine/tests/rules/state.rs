// Port of `packages/engine/test/state.test.ts`: `createGame` (M1-T1).

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{token_def, vanilla_catalog, vanilla_deck};

fn catalog() -> CardDefs {
    vanilla_catalog(40, 1)
}

fn deck_a() -> Vec<String> {
    vanilla_deck(DECK_SIZE, 1)
}

fn deck_b() -> Vec<String> {
    vanilla_deck(DECK_SIZE, 21)
}

fn game() -> GameState {
    create_game(&CreateGameOptions {
        seed: "state-test".into(),
        decks: (deck_a(), deck_b()),
        catalog: Some(catalog()),
        ..Default::default()
    })
}

/// `createGame` with these decks against the fixture catalog, which must refuse: TS threw, Rust
/// panics with the same message (`state.rs`'s `create_game`). Returns that message.
fn refusal(decks: (Vec<String>, Vec<String>)) -> String {
    let options = CreateGameOptions {
        seed: "s".into(),
        decks,
        catalog: Some(catalog()),
        ..Default::default()
    };
    let payload = catch_unwind(AssertUnwindSafe(|| create_game(&options)))
        .expect_err("createGame should refuse these decks");
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    payload
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .unwrap_or_default()
}

/// TS `toThrow(/first.*second/)`: `first` occurs, and `second` occurs after it.
fn matches_in_order(text: &str, first: &str, second: &str) -> bool {
    text.find(first)
        .is_some_and(|at| text[at + first.len()..].contains(second))
}

mod create_game_m1_t1 {
    use super::*;

    #[test]
    fn starts_two_players_at_full_health_with_empty_zones_turn_0_and_no_prompt() {
        let state = game();

        let seats: Vec<PlayerId> = state.players.iter().map(|(player, _)| player).collect();
        assert_eq!(seats, PLAYER_IDS.to_vec());
        assert_eq!(state.turn, 0);
        assert_eq!(state.phase, Phase::Setup);
        assert!(state.pending.is_none());
        assert!(state.result.is_none());
        assert_eq!(state.rng_cursor, 0);
        assert_eq!(
            serde_json::to_value(state.counters).unwrap(),
            json!({ "drawn": 0, "played": 0, "destroyed": 0, "exiled": 0 })
        );

        for player in PLAYER_IDS {
            let side = &state.players[player];
            assert_eq!(side.hero.health, HERO_HEALTH);
            assert!(side.hand.is_empty());
            assert!(side.graveyard.is_empty());
            assert!(side.exile.is_empty());
            assert_eq!(side.library.len() as i32, DECK_SIZE);
            assert_eq!(side.units.len() as i32, UNIT_ZONES);
            assert_eq!(side.backrow.len() as i32, BACKROW_ZONES);
            assert!(side.units.iter().all(|zone| zone.is_none()));
            assert!(side.backrow.iter().all(|zone| zone.is_none()));
            assert_eq!(side.locks.units, vec![false; UNIT_ZONES as usize]);
            assert_eq!(side.turns_started, 0);
        }
    }

    #[test]
    fn gives_every_card_a_unique_instance_owned_by_its_player_in_deck_order() {
        let state = game();
        let ids: Vec<String> = PLAYER_IDS
            .iter()
            .flat_map(|p| state.players[*p].library.iter().map(|c| c.id.clone()))
            .collect();
        let unique: IndexSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
        let p1_defs: Vec<String> = state.players.p1.library.iter().map(|c| c.def_id.clone()).collect();
        assert_eq!(p1_defs, deck_a());
        assert!(
            state
                .players
                .p1
                .library
                .iter()
                .all(|c| c.owner == PlayerId::P1 && c.controller == PlayerId::P1)
        );
        assert!(state.players.p2.library.iter().all(|c| c.owner == PlayerId::P2));
        assert_eq!(
            state.players.p1.library.first().map(|c| c.zone.clone()),
            Some(Zone::Library { player: PlayerId::P1 })
        );
    }

    #[test]
    fn is_deterministic_and_serializable() {
        let round_tripped: GameState = serde_json::from_value(serde_json::to_value(game()).unwrap()).unwrap();
        assert_eq!(round_tripped, game());
    }

    #[test]
    fn rejects_a_deck_that_is_not_exactly_20_cards_naming_the_rule() {
        let short = refusal((vanilla_deck(19, 1), deck_b()));
        assert!(short.contains("exactly 20 cards (§2.6 L2)"), "{short}");
        let long = refusal((deck_a(), vanilla_deck(21, 21)));
        assert!(long.contains("p2: deck must hold exactly 20"), "{long}");
    }

    #[test]
    fn rejects_duplicate_card_ids_naming_the_rule() {
        let mut with_duplicate = vanilla_deck(19, 1);
        with_duplicate.push("fx-1".into());
        let message = refusal((with_duplicate, deck_b()));
        assert!(matches_in_order(&message, "appears twice", "§2.6 L3"), "{message}");
    }

    #[test]
    fn rejects_a_token_tagged_card_naming_the_rule() {
        let token = token_def("rush", [Tag::Token]);
        let mut with_token = vanilla_deck(19, 1);
        with_token.push(token.id);
        let message = refusal((with_token, deck_b()));
        assert!(matches_in_order(&message, "is a Token card", "§2.6 L3"), "{message}");
    }

    #[test]
    fn rejects_a_card_that_is_not_in_the_catalog_naming_the_rule() {
        let mut with_ghost = vanilla_deck(19, 1);
        with_ghost.push("fx-does-not-exist".into());
        let message = refusal((with_ghost, deck_b()));
        assert!(message.contains("not in the catalog (§9.4 L6)"), "{message}");
    }
}
