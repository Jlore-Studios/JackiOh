//! #1697, SPEC §9.3: a self-play match carries the setup inputs `create_game` takes beyond its decks
//! and handicaps: dealt seats (R433), last boards (R417, R564) and Glitch boards (R678). Each reaches
//! the game `play_match` creates and the fold `MatchConfig::fold_args` hands `fold`, a config that sets
//! none is the game and the JSON it was before, and C+ #29 Portal to the Past, which reads the last
//! board, resolves in an AI game.

use jackioh_ai::{MatchConfig, MatchHooks, MatchRecord, MatchSetup, SeatController, play_match};
use jackioh_engine::testkit::{
    ActionBody, FoldArgs, GameState, LastBoardEntry, PLAYER_IDS, PerPlayer, PlayerId, fold, hash_state, json,
    legal_actions,
};

use super::support::{random_decks, register_cards};

/// C+ #29 Portal to the Past: Discovers from the caster's last board.
const PORTAL: &str = "classicplus-029";
/// Classic cards, which no Core deck holds, so one in a hand came from a board.
const P1_BOARD_CARD: &str = "classic-086";
const P2_BOARD_CARD: &str = "classic-041";
/// The deck slot each seat's Portal takes.
const PORTAL_SLOT: usize = 0;
/// How many seeds the Portal test tries looking for a game in which a Portal is played.
const PORTAL_SEEDS: i32 = 8;
/// Actions the all-inputs game plays: enough for the first turns, short enough to stay fast.
const SETUP_ACTIONS: usize = 12;

fn board(def_id: &str) -> Vec<LastBoardEntry> {
    vec![LastBoardEntry {
        def_id: def_id.to_string(),
        radiant: false,
    }]
}

fn board_card(seat: PlayerId) -> &'static str {
    match seat {
        PlayerId::P1 => P1_BOARD_CARD,
        PlayerId::P2 => P2_BOARD_CARD,
    }
}

fn random_vs_random(seed: &str, setup: MatchSetup) -> MatchConfig {
    MatchConfig {
        seed: seed.to_string(),
        decks: random_decks(seed),
        handicaps: None,
        controllers: PerPlayer::new(SeatController::Random, SeatController::Random),
        max_actions: None,
        setup,
    }
}

/// Game n of the Portal test: each seat's last board is one card of its own, and each deck holds a Portal.
fn portal_game(n: i32) -> MatchConfig {
    let seed = format!("match-setup-portal-{n}");
    let setup = MatchSetup {
        last_boards: Some((board(P1_BOARD_CARD), board(P2_BOARD_CARD))),
        ..MatchSetup::default()
    };
    let mut config = random_vs_random(&seed, setup);
    config.decks.0[PORTAL_SLOT] = PORTAL.to_string();
    config.decks.1[PORTAL_SLOT] = PORTAL.to_string();
    config
}

/// The play of a Portal `seat` holds, when it is legal.
fn portal_play(state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    legal_actions(state, seat).into_iter().find(|action| {
        let ActionBody::Play { instance_id, .. } = action else {
            return false;
        };
        state.players[seat]
            .hand
            .iter()
            .any(|card| card.id == *instance_id && card.def_id == PORTAL)
    })
}

/// Plays `config` with a Portal played whenever one can be, and says which seats then held their own
/// board's card at cost (0): Portal's answer, since no deck holds it.
fn play_forcing_portal(config: &MatchConfig) -> (MatchRecord, PerPlayer<bool>) {
    let mut resolved = PerPlayer::new(false, false);
    let record = {
        let mut hooks = MatchHooks {
            override_choice: Some(Box::new(|state, seat, action| {
                portal_play(state, seat).or(action)
            })),
            after_action: Some(Box::new(|_before, after, _seat, _action| {
                for seat in PLAYER_IDS {
                    if after.players[seat]
                        .hand
                        .iter()
                        .any(|card| card.def_id == board_card(seat) && card.cost_override == Some(0))
                    {
                        resolved[seat] = true;
                    }
                }
            })),
            ..MatchHooks::default()
        };
        play_match(config, &mut hooks)
    };
    (record, resolved)
}

#[test]
fn r417_a_config_with_last_boards_lets_portal_to_the_past_resolve_and_its_record_folds_only_with_them() {
    register_cards();
    let (config, record, _) = (1..=PORTAL_SEEDS)
        .map(|n| {
            let config = portal_game(n);
            let (record, resolved) = play_forcing_portal(&config);
            (config, record, resolved)
        })
        .find(|(_, _, resolved)| resolved.p1 || resolved.p2)
        .expect("a Portal resolves in one of the seeded games");
    assert!(record.rejected.is_empty(), "{:?}", record.rejected);
    assert!(record.thrown.is_empty(), "{:?}", record.thrown);

    let replayed = fold(&config.fold_args(record.log.clone()));
    assert!(replayed.errors.is_empty(), "{:?}", replayed.errors);
    assert_eq!(hash_state(&replayed.state), record.hash);

    // Without the boards it is not the same game: the Portal gives nothing (R129) and its answer is refused.
    let without = fold(&FoldArgs {
        last_boards: None,
        ..config.fold_args(record.log.clone())
    });
    assert!(!(without.errors.is_empty() && hash_state(&without.state) == record.hash));
}

#[test]
fn every_setup_input_reaches_the_game_and_its_fold() {
    register_cards();
    let mut config = random_vs_random(
        "match-setup-inputs",
        MatchSetup {
            dealt: Some(vec![PlayerId::P2]),
            last_boards: Some((board(P1_BOARD_CARD), board(P2_BOARD_CARD))),
            glitch_boards: Some((board(P2_BOARD_CARD), board(P1_BOARD_CARD))),
        },
    );
    config.max_actions = Some(SETUP_ACTIONS);
    let mut first: Option<GameState> = None;
    let record = {
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(|before, _after, _seat, _action| {
                if first.is_none() {
                    first = Some(before.clone());
                }
            })),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks)
    };
    let first = first.expect("the match took an action");

    assert_eq!(
        first.opening.as_ref().and_then(|opening| opening.dealt.clone()),
        Some(vec![PlayerId::P2])
    );
    assert_eq!(
        serde_json::to_value(&first.last_boards).expect("last boards serialize"),
        json!({
            "p1": [{ "defId": P1_BOARD_CARD, "radiant": false }],
            "p2": [{ "defId": P2_BOARD_CARD, "radiant": false }],
        })
    );
    assert_eq!(
        serde_json::to_value(&first.glitch_boards).expect("Glitch boards serialize"),
        json!({
            "p1": [{ "defId": P2_BOARD_CARD, "radiant": false }],
            "p2": [{ "defId": P1_BOARD_CARD, "radiant": false }],
        })
    );

    let replayed = fold(&config.fold_args(record.log.clone()));
    assert!(replayed.errors.is_empty(), "{:?}", replayed.errors);
    assert_eq!(hash_state(&replayed.state), record.hash);
    let bare = fold(&FoldArgs {
        dealt: None,
        last_boards: None,
        glitch_boards: None,
        ..config.fold_args(record.log.clone())
    });
    assert_ne!(hash_state(&bare.state), record.hash);

    // The inputs are written beside the config's other keys, and read back from them.
    let written = serde_json::to_value(&config).expect("a config serializes");
    for key in ["dealt", "lastBoards", "glitchBoards"] {
        assert!(written.get(key).is_some(), "{key}");
    }
    assert!(written.get("setup").is_none());
    assert_eq!(
        serde_json::from_value::<MatchConfig>(written).expect("a config deserializes"),
        config
    );
}

#[test]
fn a_config_without_setup_inputs_writes_and_plays_the_game_it_did_before() {
    register_cards();
    let config = random_vs_random("match-setup-plain", MatchSetup::default());
    let record = play_match(&config, &mut MatchHooks::default());

    let written = serde_json::to_value(&config).expect("a config serializes");
    let mut keys: Vec<&str> = written
        .as_object()
        .expect("a config is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["controllers", "decks", "seed"]);
    assert_eq!(
        serde_json::from_value::<MatchConfig>(written).expect("a config deserializes"),
        config
    );

    let args = config.fold_args(record.log.clone());
    assert_eq!(
        args,
        FoldArgs {
            seed: config.seed.clone(),
            decks: config.decks.clone(),
            log: record.log.clone(),
            ..FoldArgs::default()
        }
    );
    let replayed = fold(&args);
    assert!(replayed.errors.is_empty(), "{:?}", replayed.errors);
    assert_eq!(hash_state(&replayed.state), record.hash);
}
