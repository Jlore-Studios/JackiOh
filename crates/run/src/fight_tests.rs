//! DR14 part B on the real catalog: the run's content, a fight's setup and a fight's report.

use super::*;
use crate::config::{RUN_LOOMING_PRESENCE, RUN_OVERWHELMING_PRESENCE, RUN_STARTER_CARDS};
use jackioh_engine::catalog::{deckable, registered_catalog};
use jackioh_engine::config::{AI_DIFFICULTY, HUMAN_HANDICAP, Handicap};
use jackioh_engine::reduce::{begin_game, legal_actions, reduce, seat_to_act};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::state::{CreateGameOptions, create_recorded_game};
use jackioh_engine::wire::{
    Action, ActionBody, CustomCard, DeckMark, PerPlayerOpt, PlayerId, Rarity, SHIPPED_SETS, Selection,
};

fn catalog_content() -> RunContent {
    jackioh_cards::register_all();
    run_catalog_content(registered_catalog(), &[])
}

/// A run on the catalog's content, its starting deck the first `RUN_STARTER_SIZE` starting cards.
fn catalog_run(content: &RunContent) -> RunState {
    let setup = RunSetup {
        seed: run_daily_seed("2026-11-01", 1),
        starter: RUN_STARTER_CARDS[..RUN_STARTER_SIZE]
            .iter()
            .map(|id| (*id).to_string())
            .collect(),
        artifacts: Vec::new(),
        curses: Vec::new(),
    };
    run_create(&setup, &RunUnlocks::default(), content).unwrap()
}

/// `catalog_run` stepped onto node `col` of the first row, a fight row.
fn fight_at(content: &RunContent, col: usize) -> RunState {
    run_reduce(&catalog_run(content), content, &RunAction::Go { col }).unwrap()
}

#[test]
fn b5_catalog_content_pools_every_shipped_card_and_the_custom_ones_and_no_boss_card() {
    // B5: every non-token card of the sets that ship, as `deckable` reads it (so no card only a rule
    // makes, such as Dormant Starship, which `createGame` would refuse), then the custom cards given,
    // and no boss card while the Boss set has none.
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let custom: CustomCard = serde_json::from_str(
        r#"{"id":"custom-run-test","version":1,"frame":{"name":"Run Test","type":"Unit","cost":1,"rarity":"Rare","attack":1,"health":1}}"#,
    )
    .unwrap();
    let content = run_catalog_content(catalog, std::slice::from_ref(&custom));

    let mut expected: Vec<&str> = catalog
        .values()
        .filter(|def| deckable(def))
        .map(|def| def.id.as_str())
        .collect();
    expected.push("custom-run-test");
    let ids: Vec<&str> = content.pool.iter().map(|card| card.id.as_str()).collect();
    assert_eq!(ids, expected);
    for set in SHIPPED_SETS {
        assert!(
            content.pool.iter().any(|card| card.set == set),
            "{set:?} is pooled"
        );
    }

    for card in &content.pool[..content.pool.len() - 1] {
        let def = &catalog[&card.id];
        assert!(SHIPPED_SETS.contains(&card.set), "{} ships", card.id);
        assert!(!def.is_token(), "{} is no token", card.id);
        assert_eq!(card.name, def.name);
        assert_eq!(card.set, def.set);
        assert_eq!(card.card_type, def.type_);
        assert_eq!(card.rarity.as_str(), def.rarity.as_str());
        let tags: Vec<&str> = def.tags.iter().map(|tag| tag.as_str()).collect();
        assert_eq!(card.tags, tags);
    }
    let last = content.pool.last().unwrap();
    assert_eq!(
        last,
        &RunPoolCard {
            id: "custom-run-test".into(),
            name: "Run Test".into(),
            rarity: PrintedRarity::Rare,
            set: SetName::Custom,
            card_type: CardType::Unit,
            tags: Vec::new(),
        }
    );

    assert!(!content.pool.iter().any(is_boss_card), "no boss card");
    assert!(
        content.enemies.iter().all(|enemy| enemy.deck.is_empty()),
        "the placeholder bosses' own cards are pseudo cards, none in the pool"
    );

    let placeholder = run_placeholder_content();
    let enemy_ids = |c: &RunContent| c.enemies.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
    assert_eq!(enemy_ids(&content), enemy_ids(&placeholder));
    assert_eq!(content.passives, placeholder.passives);
    assert_eq!(content.events, placeholder.events);
    assert_eq!(content.curses, placeholder.curses);
    assert_eq!(
        content.starter_access,
        RUN_STARTER_CARDS.map(String::from).to_vec()
    );

    // The map deals each enemy a full deck from the pool alone.
    let content = catalog_content();
    for node in catalog_run(&content).map.iter().flatten() {
        if let Some(enemy) = &node.enemy {
            assert_eq!(enemy.deck.len(), RUN_ENEMY_DECK_SIZE, "{}", enemy.id);
            assert!(
                enemy
                    .deck
                    .iter()
                    .all(|id| content.pool.iter().any(|c| &c.id == id))
            );
        }
    }
}

#[test]
fn b6_the_starting_cards_are_twenty_training_grounds_commons_and_rares_of_the_catalog() {
    // B6
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    assert_eq!(RUN_STARTER_CARDS.len(), 20);
    for (i, id) in RUN_STARTER_CARDS.iter().enumerate() {
        assert!(!RUN_STARTER_CARDS[..i].contains(id), "{id} twice");
        let def = catalog
            .get(*id)
            .unwrap_or_else(|| panic!("{id} is in the catalog"));
        assert_eq!(def.set, SetName::TrainingGrounds, "{id}");
        assert!(matches!(def.rarity, Rarity::Common | Rarity::Rare), "{id}");
        assert!(deckable(def), "{id} may be in a deck");
    }
}

#[test]
fn b7_a_fights_setup_answers_only_in_a_fight_with_the_runs_hp_deck_and_marks() {
    // B7
    let content = catalog_content();
    assert_eq!(run_fight_setup(&catalog_run(&content), &content), None);

    let mut state = fight_at(&content, 0);
    assert_eq!(state.phase, RunPhase::Fight);
    state.hp = 17;
    state.deck[1].radiant = true;
    state.deck[3].buffs = 2;
    state.deck[4].radiant = true;
    state.deck[4].buffs = 1;
    let setup = run_fight_setup(&state, &content).unwrap();
    let ids: Vec<String> = state.deck.iter().map(|card| card.id.clone()).collect();
    assert_eq!(setup.human_deck, ids);
    assert_eq!(
        setup.human_handicap,
        Handicap {
            deck_size: 10,
            hero_health: Some(17),
            ..HUMAN_HANDICAP
        }
    );
    let mark = |index, radiant, upgrades| DeckMark {
        index,
        radiant,
        upgrades,
    };
    assert_eq!(
        setup.human_marks,
        vec![mark(1, true, 0), mark(3, false, 2), mark(4, true, 1)]
    );

    let after = run_reduce(
        &state,
        &content,
        &RunAction::FightOver {
            won: true,
            hp: 17,
            damage_dealt: 0,
            healing_done: 0,
            cards_drawn: 0,
            turns: 0,
        },
    )
    .unwrap();
    assert_eq!(run_fight_setup(&after, &content), None);
}

#[test]
fn b8_the_enemys_handicap_is_its_presences_tier_with_its_deck_and_health() {
    // B8
    let content = catalog_content();
    let mut state = fight_at(&content, 0);
    let dealt = run_enemy(&state).unwrap().clone();
    let setup = run_fight_setup(&state, &content).unwrap();
    assert_eq!(setup.enemy_deck.len(), RUN_ENEMY_DECK_SIZE);
    assert_eq!(setup.enemy_deck, dealt.deck);
    assert_eq!(setup.enemy_handicap.deck_size as usize, RUN_ENEMY_DECK_SIZE);
    assert_eq!(setup.enemy_handicap.hero_health, Some(dealt.hero_health));
    assert_eq!(setup.enemy, dealt);

    let deck: Vec<String> = content.pool[..7].iter().map(|c| c.id.clone()).collect();
    let tiers = [
        (Vec::new(), AI_DIFFICULTY.easy),
        (vec![RUN_LOOMING_PRESENCE.to_string()], AI_DIFFICULTY.medium),
        (
            vec![
                RUN_OVERWHELMING_PRESENCE.to_string(),
                "temp-final-relic".to_string(),
            ],
            AI_DIFFICULTY.hard,
        ),
    ];
    let (row, col) = (state.row.unwrap(), state.col);
    for (artifacts, tier) in tiers {
        let enemy = RunEnemy {
            id: "test-enemy".into(),
            name: "Test Enemy".into(),
            hero_health: 40,
            deck: deck.clone(),
            artifacts,
        };
        state.map[row][col].enemy = Some(enemy.clone());
        let setup = run_fight_setup(&state, &content).unwrap();
        assert_eq!(
            setup.enemy_handicap,
            Handicap {
                deck_size: 7,
                hero_health: Some(40),
                ..tier
            }
        );
        assert_eq!(setup.enemy_deck, deck);
        assert_eq!(setup.enemy, enemy);
    }
}

#[test]
fn b9_a_fights_seed_is_the_runs_seed_row_and_column() {
    // B9: one run meets one deal at each node.
    let content = catalog_content();
    let seed = run_daily_seed("2026-11-01", 1);
    let setup = run_fight_setup(&fight_at(&content, 0), &content).unwrap();
    assert_eq!(setup.seed, format!("{seed}-0-0"));
    assert_eq!(run_fight_setup(&fight_at(&content, 0), &content), Some(setup));
    let other = run_fight_setup(&fight_at(&content, 1), &content).unwrap();
    assert_eq!(other.seed, format!("{seed}-0-1"));
}

fn tg(n: &str) -> String {
    format!("traininggrounds-{n}")
}

/// A short game of known totals. P1 (20 health) holds True Strike, Stockpile, Mr. Vanilla, Spikey
/// Steven and Clobberer; P2 (10 health) five units. Both keep their opening hands. P1 casts Stockpile
/// when it can, else True Strike at P2's hero, else ends its turn; P2 only ends its turn, and concedes
/// on turn 4.
fn scripted_game() -> ReplayInput {
    jackioh_cards::register_all();
    let seed = "ABCDEF-0-1".to_string();
    let decks = (
        ["058", "042", "043", "001", "008"].map(tg).to_vec(),
        ["006", "007", "047", "054", "048"].map(tg).to_vec(),
    );
    let handicap = |hero_health| Handicap {
        deck_size: 5,
        hero_health: Some(hero_health),
        ..HUMAN_HANDICAP
    };
    let handicaps = PerPlayerOpt {
        p1: Some(handicap(20)),
        p2: Some(handicap(10)),
    };
    let options = CreateGameOptions {
        seed: seed.clone(),
        decks: decks.clone(),
        handicaps: Some(handicaps.clone()),
        ..Default::default()
    };
    let mut state = begin_game(&create_recorded_game(&options)).state;
    let mut log: Vec<Action> = Vec::new();
    while state.result.is_none() {
        assert!(log.len() < 20, "the scripted game ends");
        let seat = seat_to_act(&state).expect("a seat acts");
        let legal = legal_actions(&state, seat);
        let hand = &state.players[seat].hand;
        let play_of = |def_id: &str, targets: Option<Vec<Selection>>| {
            legal.iter().find(|body| match body {
                ActionBody::Play {
                    instance_id,
                    targets: given,
                    ..
                } => {
                    hand.iter().any(|c| &c.id == instance_id && c.def_id == def_id)
                        && (targets.is_none() || *given == targets)
                }
                _ => false,
            })
        };
        let body = if legal.iter().any(|b| matches!(b, ActionBody::Mulligan { .. })) {
            ActionBody::Mulligan {
                keep: hand.iter().map(|c| c.id.clone()).collect(),
            }
        } else if seat == PlayerId::P2 {
            if state.turn >= 4 {
                ActionBody::Concede
            } else {
                ActionBody::EndTurn
            }
        } else {
            play_of(&tg("042"), None)
                .or_else(|| play_of(&tg("058"), Some(vec![Selection::Hero { player: PlayerId::P2 }])))
                .cloned()
                .unwrap_or(ActionBody::EndTurn)
        };
        let action = Action::new(body, seat, format!("n{}", log.len()));
        let result = reduce(&state, &action);
        assert_eq!(result.error, None, "{action:?}");
        log.push(action);
        state = result.state;
    }
    ReplayInput {
        seed,
        decks,
        log,
        handicaps: Some(handicaps),
        ..Default::default()
    }
}

#[test]
fn b10_a_finished_game_reports_the_winner_health_damage_healing_draws_and_turns() {
    // B10. P1 ends at 19: 20, less 1 and 2 of fatigue, plus Stockpile's 2. It dealt 5: True Strike's 4
    // and P2's own 1 of fatigue, a damage event on the enemy's hero, while P1's fatigue is no damage it
    // dealt. It drew 2: turn 1's and Stockpile's first, its second a fatigue draw that draws no card.
    let game = scripted_game();
    assert_eq!(
        run_fight_report(&game, PlayerId::P1),
        Ok(RunAction::FightOver {
            won: true,
            hp: 19,
            damage_dealt: 5,
            healing_done: 2,
            cards_drawn: 2,
            turns: 4,
        })
    );
    // P2 conceded at 5 (10, less True Strike's 4 and 1 of fatigue), and P1's 3 of fatigue are on its
    // enemy's hero.
    assert_eq!(
        run_fight_report(&game, PlayerId::P2),
        Ok(RunAction::FightOver {
            won: false,
            hp: 5,
            damage_dealt: 3,
            healing_done: 0,
            cards_drawn: 1,
            turns: 4,
        })
    );
}

#[test]
fn b10_an_unfinished_game_and_a_setup_the_engine_refuses_are_refused() {
    // B10: only a finished game reports, and a setup `createGame` refuses is an error, not a panic.
    let mut unfinished = scripted_game();
    unfinished.log.pop();
    assert!(run_fight_report(&unfinished, PlayerId::P1).is_err());

    let mut refused = scripted_game();
    refused.decks.0.pop();
    let error = run_fight_report(&refused, PlayerId::P1).unwrap_err();
    assert!(error.contains("deck"), "{error}");
}

#[test]
fn b11_a_level_costs_its_embers_until_the_ladders_top_and_the_constants_are_the_configs() {
    // B11
    for level in 1..RUN_SLOT_LADDER.len() {
        assert_eq!(
            run_level_cost(level),
            Some(RUN_LEVEL_EMBERS[level - 1]),
            "level {level}"
        );
    }
    assert_eq!(run_level_cost(0), None);
    assert_eq!(run_level_cost(RUN_SLOT_LADDER.len()), None);
    assert_eq!(run_level_cost(RUN_SLOT_LADDER.len() + 1), None);

    let constants = run_constants();
    assert_eq!(
        constants,
        RunConstants {
            slot_ladder: RUN_SLOT_LADDER.to_vec(),
            starter_size: RUN_STARTER_SIZE,
            starter_artifacts_max: RUN_STARTER_ARTIFACTS_MAX,
            starter_artifact_points: RUN_STARTER_ARTIFACT_POINTS,
            tier_points: RUN_TIER_POINTS.to_vec(),
        }
    );
    let json = serde_json::to_value(&constants).unwrap();
    for key in [
        "slotLadder",
        "starterSize",
        "starterArtifactsMax",
        "starterArtifactPoints",
        "tierPoints",
    ] {
        assert!(json.get(key).is_some(), "{key}");
    }
}
