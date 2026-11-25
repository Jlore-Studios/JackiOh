//! The Assault on the real catalog: the placeholder's shape, the set read by name, and every
//! level's fight on the engine ([[R4579001]], [[R4579002]]).

use super::*;
use crate::assault::content::content_file;
use crate::assault::day::MONTH_DAYS;
use crate::config::ASSAULT_LEVELS;
use jackioh_engine::catalog::{deckable, registered_catalog};
use jackioh_engine::config::{HUMAN_HANDICAP, Handicap, TURN_CAP_PLAYER_TURNS};
use jackioh_engine::reduce::{begin_game, legal_actions, reduce, seat_to_act};
use jackioh_engine::replay::{self, ReplayInput};
use jackioh_engine::state::{
    CreateGameOptions, SetupKind, create_game, create_recorded_game, validate_setup,
};
use jackioh_engine::wire::{Action, ActionBody, PerPlayerOpt, PlayerId, Rarity};

/// The first `n` `deckable` ids of the catalog that ships, in catalog order.
fn first_deckable(n: usize) -> Vec<String> {
    registered_catalog()
        .values()
        .filter(|def| deckable(def))
        .take(n)
        .map(|def| def.id.clone())
        .collect()
}

/// Every date of 2027's first week.
fn week() -> Vec<String> {
    (1..=7).map(|day| format!("2027-01-{day:02}")).collect()
}

/// Every date of 2027, from `MONTH_DAYS`.
fn year() -> Vec<String> {
    let mut days = Vec::new();
    for (month, &length) in MONTH_DAYS.iter().enumerate() {
        for day in 1..=length {
            days.push(format!("2027-{:02}-{day:02}", month + 1));
        }
    }
    assert_eq!(days.len(), 365, "2027 holds 365 days");
    days
}

/// One level's fight as `createGame` options, with the human's deck at P1 and the marks as P2's.
fn fight_options(
    human: &[String],
    level: &AssaultLevel,
    seed: &str,
) -> (CreateGameOptions, PerPlayerOpt<Handicap>) {
    let handicaps = PerPlayerOpt {
        p1: Some(Handicap {
            deck_size: human.len() as i32,
            ..HUMAN_HANDICAP
        }),
        p2: Some(level.handicap),
    };
    let options = CreateGameOptions {
        seed: seed.into(),
        decks: (human.to_vec(), level.deck.clone()),
        handicaps: Some(handicaps.clone()),
        deck_marks: Some(PerPlayerOpt {
            p1: None,
            p2: Some(level.marks.clone()),
        }),
        ..Default::default()
    };
    (options, handicaps)
}

/// Past both seats' keep-all mulligans, with the mulligans logged.
fn keep_all(state: &jackioh_engine::state::GameState) -> (jackioh_engine::state::GameState, Vec<Action>) {
    let mut state = state.clone();
    let mut log = Vec::new();
    for _ in 0..2 {
        let seat = seat_to_act(&state).expect("a seat mulligans");
        assert!(
            legal_actions(&state, seat)
                .iter()
                .any(|body| matches!(body, ActionBody::Mulligan { .. })),
            "a mulligan is legal"
        );
        let keep = state.players[seat]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let action = Action::new(
            ActionBody::Mulligan { keep },
            seat,
            format!("mulligan-{}", log.len()),
        );
        let result = reduce(&state, &action);
        assert_eq!(result.error, None, "{action:?}");
        log.push(action);
        state = result.state;
    }
    (state, log)
}

#[test]
fn the_placeholder_holds_every_deckable_card_and_twelve_stand_ins() {
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let content = assault_placeholder_content(catalog);

    // The cards are the catalog's deckable ids, in order.
    let expected: Vec<&str> = catalog
        .values()
        .filter(|def| deckable(def))
        .map(|def| def.id.as_str())
        .collect();
    let ids: Vec<&str> = content.cards.iter().map(|card| card.id.as_str()).collect();
    assert_eq!(ids, expected);

    // Twelve stand-ins, three per rank, with their cards flagged and of their doctrine.
    assert_eq!(content.commanders.len(), 12);
    for rank in 1..=4 {
        assert_eq!(
            content
                .commanders
                .iter()
                .filter(|commander| commander.rank == rank)
                .count(),
            3,
            "three rank-{rank} stand-ins"
        );
    }
    for commander in &content.commanders {
        assert_eq!(commander.cards.len(), 2);
        for id in &commander.cards {
            let card = content
                .cards
                .iter()
                .find(|card| &card.id == id)
                .unwrap_or_else(|| panic!("{id} is a card of the content"));
            assert!(card.commander, "{id} is flagged");
            assert_eq!(card.doctrine, commander.doctrine, "{id}");
        }
    }

    // Every group holds every rarity.
    let groups: Vec<Option<&str>> = std::iter::once(None)
        .chain(content.doctrines.iter().map(|row| Some(row.id.as_str())))
        .collect();
    for group in groups {
        for rarity in [
            Rarity::Common,
            Rarity::Rare,
            Rarity::Epic,
            Rarity::Legendary,
            Rarity::Mythic,
        ] {
            assert!(
                content.cards.iter().any(|card| {
                    card.doctrine.as_deref() == group && card.rarity == rarity && !card.commander
                }),
                "group {group:?} holds {rarity:?}"
            );
        }
    }
}

#[test]
fn the_catalog_content_reads_the_set_by_name_and_assault_json() {
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let content = assault_catalog_content(catalog);
    // Every card is of `ASSAULT_SET`, C1's set.
    for card in &content.cards {
        assert_eq!(catalog[&card.id].set, crate::config::ASSAULT_SET, "{}", card.id);
    }
    // The doctrines are the file's eight.
    assert_eq!(content.doctrines, content_file().doctrines);
    assert_eq!(content.doctrines.len(), 8);
}

#[test]
fn create_game_takes_every_level_of_a_year_on_the_placeholder() {
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let content = assault_placeholder_content(catalog);
    let human = first_deckable(25);
    assert_eq!(human.len(), 25);
    for date in year() {
        let day = assault_day(&date, &content).unwrap();
        assert_eq!(day.levels.len(), ASSAULT_LEVELS as usize, "{date}");
        for level in &day.levels {
            let (options, _) = fight_options(&human, level, &format!("{}-L{}-A1", day.seed, level.level));
            validate_setup(&options, catalog, SetupKind::New)
                .unwrap_or_else(|error| panic!("{date} level {}: {error}", level.level));
        }
    }
}

#[test]
fn every_level_of_a_week_folds_back_to_its_hash() {
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let content = assault_placeholder_content(catalog);
    let human = first_deckable(25);
    for date in week() {
        let day = assault_day(&date, &content).unwrap();
        for level in &day.levels {
            let seed = format!("{}-L{}-A1", day.seed, level.level);
            let (options, handicaps) = fight_options(&human, level, &seed);
            let begun = begin_game(&create_game(&options)).state;
            let (state, log) = keep_all(&begun);
            let left = replay::hash_state(&state);
            let input = ReplayInput {
                seed: seed.clone(),
                decks: options.decks.clone(),
                log,
                handicaps: Some(handicaps),
                deck_marks: options.deck_marks.clone(),
                ..Default::default()
            };
            validate_setup(&input_to_options(&input), catalog, SetupKind::Recorded)
                .expect("the fold's setup is valid");
            let folded = replay::fold(&input);
            assert!(folded.errors.is_empty(), "{date} level {}", level.level);
            assert_eq!(
                replay::hash_state(&folded.state),
                left,
                "{date} level {} folds back to its hash",
                level.level
            );
        }
    }
}

/// A replay's setup as `createGame` options.
fn input_to_options(input: &ReplayInput) -> CreateGameOptions {
    CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        handicaps: input.handicaps.clone(),
        deck_marks: input.deck_marks.clone(),
        ..Default::default()
    }
}

#[test]
fn r4579002_a_draw_at_the_turn_cap_reports_a_loss_and_a_concession_a_win() {
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let _ = catalog;
    let five = first_deckable(5);
    assert_eq!(five.len(), 5);
    let health = |hero_health| Handicap {
        deck_size: 5,
        hero_health: Some(hero_health),
        ..HUMAN_HANDICAP
    };
    let handicaps = PerPlayerOpt {
        p1: Some(health(10_000)),
        p2: Some(health(10_000)),
    };
    let options = CreateGameOptions {
        seed: "assault-draw-cap".into(),
        decks: (five.clone(), five.clone()),
        handicaps: Some(handicaps.clone()),
        ..Default::default()
    };
    let begun = begin_game(&create_recorded_game(&options)).state;
    let (mut state, mut log) = keep_all(&begun);
    let mut nonce = log.len();
    while state.result.is_none() {
        let seat = seat_to_act(&state).expect("a seat ends its turn");
        let action = Action::new(ActionBody::EndTurn, seat, format!("n{nonce}"));
        nonce += 1;
        let result = reduce(&state, &action);
        assert_eq!(result.error, None, "{action:?}");
        log.push(action);
        state = result.state;
        assert!(nonce < 500, "both seats end their turns");
    }
    assert_eq!(state.turn, TURN_CAP_PLAYER_TURNS);
    let game = ReplayInput {
        seed: options.seed.clone(),
        decks: options.decks.clone(),
        log,
        handicaps: Some(handicaps),
        ..Default::default()
    };
    for human in [PlayerId::P1, PlayerId::P2] {
        assert_eq!(
            assault_fight_report(&game, human),
            Ok(AssaultAction::FightOver {
                won: false,
                turns: TURN_CAP_PLAYER_TURNS
            }),
            "{human:?} reports a loss"
        );
    }

    // When P2 concedes, P1 reports a win.
    let begun = begin_game(&create_recorded_game(&options)).state;
    let (mut state, mut log) = keep_all(&begun);
    let mut nonce = log.len();
    while state.result.is_none() {
        let seat = seat_to_act(&state).expect("a seat acts");
        let body = if seat == PlayerId::P2
            && !legal_actions(&state, seat)
                .iter()
                .any(|body| matches!(body, ActionBody::Mulligan { .. }))
        {
            ActionBody::Concede
        } else {
            ActionBody::EndTurn
        };
        let action = Action::new(body, seat, format!("c{nonce}"));
        nonce += 1;
        let result = reduce(&state, &action);
        assert_eq!(result.error, None, "{action:?}");
        log.push(action);
        state = result.state;
        assert!(nonce < 20, "P1 ends one turn, then P2 concedes");
    }
    let conceded = ReplayInput {
        seed: options.seed.clone(),
        decks: options.decks.clone(),
        log,
        handicaps: options.handicaps.clone(),
        ..Default::default()
    };
    assert!(
        matches!(
            assault_fight_report(&conceded, PlayerId::P1),
            Ok(AssaultAction::FightOver { won: true, .. })
        ),
        "P1 reports a win"
    );
}
