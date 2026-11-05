//! DR14 part B on the real catalog: the run's content, a fight's setup and a fight's report.

use super::*;
use crate::config::{RUN_DECK_MAX, RUN_LOOMING_PRESENCE, RUN_OVERWHELMING_PRESENCE, RUN_STARTER_CARDS};
use jackioh_engine::catalog::{deckable, registered_catalog};
use jackioh_engine::config::{AI_DIFFICULTY, HUMAN_HANDICAP, Handicap, LIBRARY_CAP};
use jackioh_engine::reduce::{begin_game, legal_actions, reduce, seat_to_act};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::state::{CreateGameOptions, SetupKind, create_recorded_game, validate_setup};
use jackioh_engine::wire::{
    Action, ActionBody, DeckMark, PerPlayerOpt, PlayerId, Rarity, SHIPPED_SETS, Selection,
};

pub(crate) fn catalog_content() -> RunContent {
    jackioh_cards::register_all();
    run_catalog_content(registered_catalog())
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
fn b5_catalog_content_pools_every_shipped_card_and_no_boss_card() {
    // B5: every non-token card of the sets that ship, as `deckable` reads it (so no card only a rule
    // makes, such as Dormant Starship, which `createGame` would refuse), and no boss card while the
    // Boss set has none.
    jackioh_cards::register_all();
    let catalog = registered_catalog();
    let content = run_catalog_content(catalog);

    let expected: Vec<&str> = catalog
        .values()
        .filter(|def| deckable(def))
        .map(|def| def.id.as_str())
        .collect();
    let ids: Vec<&str> = content.pool.iter().map(|card| card.id.as_str()).collect();
    assert_eq!(ids, expected);
    for set in SHIPPED_SETS {
        assert!(
            content.pool.iter().any(|card| card.set == set),
            "{set:?} is pooled"
        );
    }

    for card in &content.pool {
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
    // B10: only a finished game reports, and a setup `createGame` refuses is an error, not a panic: a
    // deck that names a custom card among them, since a run gives the game none.
    let mut unfinished = scripted_game();
    unfinished.log.pop();
    assert!(run_fight_report(&unfinished, PlayerId::P1).is_err());

    let mut refused = scripted_game();
    refused.decks.0.pop();
    let error = run_fight_report(&refused, PlayerId::P1).unwrap_err();
    assert!(error.contains("deck"), "{error}");

    let mut custom = scripted_game();
    custom.decks.0[0] = "custom-anything".to_string();
    let error = run_fight_report(&custom, PlayerId::P1).unwrap_err();
    assert!(error.contains("custom card"), "{error}");
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

/// A fight's setup as the `createGame` options the caller builds from it.
pub(crate) fn fight_options(setup: &RunFightSetup) -> CreateGameOptions {
    CreateGameOptions {
        seed: setup.seed.clone(),
        decks: (setup.human_deck.clone(), setup.enemy_deck.clone()),
        handicaps: Some(PerPlayerOpt {
            p1: Some(setup.human_handicap),
            p2: Some(setup.enemy_handicap),
        }),
        deck_marks: Some(PerPlayerOpt {
            p1: Some(setup.human_marks.clone()),
            p2: None,
        }),
        ..Default::default()
    }
}

/// The engine's verdict on a fight's setup, as `createGame` would take it.
pub(crate) fn engine_takes(setup: &RunFightSetup) -> Result<(), String> {
    let options = fight_options(setup);
    validate_setup(
        &options,
        registered_catalog(),
        &Default::default(),
        SetupKind::New,
    )
    .map_err(|e| e.to_string())
}

#[test]
fn r2701000_a_run_deck_stops_at_the_engines_library_cap_so_every_fight_starts() {
    // R2701000: a run deck holds at most `RUN_DECK_MAX`, the engine's `LIBRARY_CAP`, the most a
    // handicapped deck may hold; a card a full deck would gain is not gained.
    let content = catalog_content();
    assert_eq!(RUN_DECK_MAX, usize::try_from(LIBRARY_CAP).unwrap());
    let mut state = fight_at(&content, 0);
    let more: Vec<RunCard> = content
        .pool
        .iter()
        .filter(|c| !is_boss_card(c) && !in_deck(&state, &c.id))
        .take(RUN_DECK_MAX + 1 - state.deck.len())
        .map(|c| RunCard {
            id: c.id.clone(),
            radiant: false,
            buffs: 0,
        })
        .collect();
    state.deck.extend(more);
    assert_eq!(state.deck.len(), RUN_DECK_MAX + 1);
    // Without the cap a fight could not start: the engine refuses a deck past its library cap.
    let error = engine_takes(&run_fight_setup(&state, &content).unwrap()).unwrap_err();
    assert!(error.contains("deckSize"), "{error}");
    let extra = state.deck.pop().unwrap().id;
    assert_eq!(engine_takes(&run_fight_setup(&state, &content).unwrap()), Ok(()));

    // A pack keeps only what the deck has room for: none when full, fewer when nearly full.
    let reward = |state: &RunState| {
        let mut at = state.clone();
        at.phase = RunPhase::Reward {
            artifact: None,
            packs: vec![RunPackKind::Random],
        };
        run_reduce(&at, &content, &RunAction::OpenPack { index: 0 }).unwrap()
    };
    let full = reward(&state);
    assert!(matches!(&full.phase, RunPhase::Pack { cards, keep: 0 } if !cards.is_empty()));
    let kept = run_reduce(&full, &content, &RunAction::Keep { cards: Vec::new() }).unwrap();
    assert_eq!(kept.deck.len(), RUN_DECK_MAX);
    let mut nearly = state.clone();
    nearly.deck.truncate(RUN_DECK_MAX - 2);
    assert!(matches!(reward(&nearly).phase, RunPhase::Pack { keep: 2, .. }));

    // A shop refuses a card to a full deck, and takes it once there is room.
    let mut shop = state.clone();
    shop.coins = 500;
    shop.phase = RunPhase::Shop {
        stock: vec![RunShopItem {
            good: RunShopGood::Card { id: extra },
            price: 1,
            sold: false,
        }],
    };
    let refusal = run_reduce(&shop, &content, &RunAction::Buy { index: 0 }).unwrap_err();
    assert!(refusal.contains("full"), "{refusal}");
    assert_eq!(
        run_reduce(
            &with_deck_of(&shop, &nearly),
            &content,
            &RunAction::Buy { index: 0 }
        )
        .unwrap()
        .deck
        .len(),
        RUN_DECK_MAX - 1
    );

    // An event's card is not gained by a full deck, while the rest of the option is paid.
    let event = content
        .events
        .iter()
        .find_map(|e| e.options.iter().position(|o| o.card).map(|i| (e.id.clone(), i)))
        .expect("an event gives a card");
    let mut at_event = state.clone();
    at_event.coins = 500;
    at_event.phase = RunPhase::Event { id: event.0 };
    let chose = run_reduce(&at_event, &content, &RunAction::Choose { index: event.1 }).unwrap();
    assert_eq!(chose.deck, state.deck);
    let gained = run_reduce(
        &with_deck_of(&at_event, &nearly),
        &content,
        &RunAction::Choose { index: event.1 },
    )
    .unwrap();
    assert_eq!(gained.deck.len(), RUN_DECK_MAX - 1);
}

/// `at` holding `deck`'s cards.
fn with_deck_of(at: &RunState, deck: &RunState) -> RunState {
    RunState {
        deck: deck.deck.clone(),
        ..at.clone()
    }
}

/// `fight_at`'s first fight, its enemy one of known health and deck carrying `artifacts`.
fn known_fight(content: &RunContent, artifacts: &[&str]) -> RunState {
    let mut state = fight_at(content, 0);
    let (row, col) = (state.row.unwrap(), state.col);
    state.map[row][col].enemy = Some(RunEnemy {
        id: "test-enemy".into(),
        name: "Test Enemy".into(),
        hero_health: 20,
        deck: content.pool[..7].iter().map(|c| c.id.clone()).collect(),
        artifacts: artifacts.iter().map(|id| (*id).to_string()).collect(),
    });
    state
}

/// `state` holding exactly `artifacts`, each at its level.
fn holding(state: &RunState, artifacts: &[(&str, i32)]) -> RunState {
    RunState {
        artifacts: artifacts
            .iter()
            .map(|(id, level)| RunArtifact {
                id: (*id).to_string(),
                level: *level,
            })
            .collect(),
        ..state.clone()
    }
}

/// `state` under exactly `curses`.
fn cursed(state: &RunState, curses: &[&str]) -> RunState {
    RunState {
        curses: curses.iter().map(|id| (*id).to_string()).collect(),
        ..state.clone()
    }
}

/// The plain enemy handicap of `known_fight`, under `tier`.
fn known_enemy(tier: Handicap) -> Handicap {
    Handicap {
        deck_size: 7,
        hero_health: Some(20),
        ..tier
    }
}

#[test]
fn r2922001_a_run_with_no_curse_and_no_combat_artifact_gets_the_setup_it_got_before() {
    // R2922001: with nothing that changes a fight, the setup is the one before curses and Combat
    // artifacts acted, field for field. An Economic artifact, a blessing and an enemy's own artifact
    // other than Presence change no fight.
    let content = catalog_content();
    let state = known_fight(&content, &[]);
    assert!(state.curses.is_empty() && state.artifacts.is_empty());
    let enemy = run_enemy(&state).unwrap().clone();
    let human_deck: Vec<String> = state.deck.iter().map(|card| card.id.clone()).collect();
    let before = RunFightSetup {
        seed: format!("{}-0-0", state.seed),
        human_handicap: Handicap {
            deck_size: 10,
            hero_health: Some(state.hp),
            ..HUMAN_HANDICAP
        },
        human_deck,
        human_marks: Vec::new(),
        enemy_deck: enemy.deck.clone(),
        enemy_handicap: known_enemy(AI_DIFFICULTY.easy),
        enemy,
    };
    assert_eq!(run_fight_setup(&state, &content), Some(before.clone()));

    let greedy = holding(&state, &[("temp-greedy", 2)]);
    assert_eq!(run_fight_setup(&greedy, &content), Some(before.clone()));
    let mut blessed = state.clone();
    blessed.blessings = vec!["temp-blessing-vitality".into()];
    assert_eq!(run_fight_setup(&blessed, &content), Some(before.clone()));
    let crowned = known_fight(&content, &["temp-crown"]);
    let setup = run_fight_setup(&crowned, &content).unwrap();
    assert_eq!(setup.human_handicap, before.human_handicap);
    assert_eq!(setup.enemy_handicap, before.enemy_handicap);

    let json = serde_json::to_value(&setup).unwrap();
    assert!(json["humanHandicap"].get("heroArmor").is_none(), "{json}");
    assert!(json["enemyHandicap"].get("heroArmor").is_none(), "{json}");
}

#[test]
fn r2922001_each_curse_strengthens_the_enemy_by_its_effect_and_all_three_stack() {
    // R2922001: each curse taken changes only its own fields of the enemy's handicap, once; the
    // human's is untouched.
    let content = catalog_content();
    let state = known_fight(&content, &[]);
    let plain = run_fight_setup(&state, &content).unwrap();
    let easy = known_enemy(AI_DIFFICULTY.easy);
    let health = Handicap {
        hero_health: Some(25),
        ..easy
    };
    let cards = Handicap {
        extra_opening_cards: easy.extra_opening_cards + 1,
        ..easy
    };
    let mana = Handicap {
        mana_bonus: easy.mana_bonus + 1,
        mana_cap: easy.mana_cap + 1,
        ..easy
    };
    let all = Handicap {
        hero_health: Some(25),
        extra_opening_cards: easy.extra_opening_cards + 1,
        mana_bonus: easy.mana_bonus + 1,
        mana_cap: easy.mana_cap + 1,
        ..easy
    };
    let cases = [
        (vec!["temp-curse-1"], health),
        (vec!["temp-curse-2"], cards),
        (vec!["temp-curse-3"], mana),
        (vec!["temp-curse-1", "temp-curse-2", "temp-curse-3"], all),
    ];
    for (curses, expected) in cases {
        let setup = run_fight_setup(&cursed(&state, &curses), &content).unwrap();
        assert_eq!(setup.enemy_handicap, expected, "{curses:?}");
        assert_eq!(setup.human_handicap, plain.human_handicap, "{curses:?}");
    }

    // After Presence: the crystal joins the tier's bonus and raises its cap.
    for (presence, bonus, cap) in [(RUN_LOOMING_PRESENCE, 2, 6), (RUN_OVERWHELMING_PRESENCE, 2, 8)] {
        let state = cursed(&known_fight(&content, &[presence]), &["temp-curse-3"]);
        let setup = run_fight_setup(&state, &content).unwrap();
        assert_eq!(
            (setup.enemy_handicap.mana_bonus, setup.enemy_handicap.mana_cap),
            (bonus, cap),
            "{presence}"
        );
    }
}

#[test]
fn r2922001_each_combat_artifact_helps_the_human_by_its_effect_times_its_level() {
    // R2922001: a Combat artifact held changes the human's handicap once per level; the enemy's is
    // untouched.
    let content = catalog_content();
    let state = known_fight(&content, &[]);
    let plain = run_fight_setup(&state, &content).unwrap();
    for level in 1..=RUN_ARTIFACT_LEVEL_MAX {
        let human = |id: &str| {
            let setup = run_fight_setup(&holding(&state, &[(id, level)]), &content).unwrap();
            assert_eq!(setup.enemy_handicap, plain.enemy_handicap, "{id} at {level}");
            setup.human_handicap
        };
        assert_eq!(
            human("temp-thick-skin"),
            Handicap {
                hero_armor: Some(level),
                ..plain.human_handicap
            }
        );
        assert_eq!(
            human("temp-quick-start"),
            Handicap {
                extra_opening_cards: level,
                ..plain.human_handicap
            }
        );
        assert_eq!(
            human("temp-crown"),
            Handicap {
                hero_armor: Some(2 * level),
                ..plain.human_handicap
            }
        );
        assert_eq!(
            human("temp-final-relic"),
            Handicap {
                extra_opening_cards: 2 * level,
                ..plain.human_handicap
            }
        );
    }
    let both = holding(&state, &[("temp-thick-skin", 2), ("temp-crown", 1)]);
    assert_eq!(
        run_fight_setup(&both, &content)
            .unwrap()
            .human_handicap
            .hero_armor,
        Some(4)
    );
}

#[test]
fn r2922001_only_known_curses_and_held_combat_artifacts_count_and_never_below_none() {
    // R2922001: no other kind's fight effect is read, an unknown curse is none, and a negative amount
    // counts as none.
    let mut content = catalog_content();
    let set = |content: &mut RunContent, id: &str, effect: RunFightEffect| {
        if let Some(def) = content.passives.iter_mut().find(|p| p.id == id) {
            def.fight_effect = Some(effect);
        }
        if let Some(def) = content.curses.iter_mut().find(|c| c.id == id) {
            def.fight_effect = Some(effect);
        }
    };
    set(&mut content, "temp-greedy", RunFightEffect::OwnArmor { n: 4 });
    set(
        &mut content,
        "temp-thick-skin",
        RunFightEffect::OwnArmor { n: -2 },
    );
    set(
        &mut content,
        "temp-curse-1",
        RunFightEffect::EnemyHealth { n: -5 },
    );
    let state = known_fight(&content, &[]);
    let plain = run_fight_setup(&state, &content).unwrap();

    let greedy = holding(&state, &[("temp-greedy", 1)]);
    assert_eq!(run_fight_setup(&greedy, &content), Some(plain.clone()));
    let unknown = cursed(&state, &["no-such-curse"]);
    assert_eq!(run_fight_setup(&unknown, &content), Some(plain.clone()));

    let thin = holding(&state, &[("temp-thick-skin", 2)]);
    assert_eq!(
        run_fight_setup(&thin, &content)
            .unwrap()
            .human_handicap
            .hero_armor,
        None
    );
    let weak = cursed(&state, &["temp-curse-1"]);
    assert_eq!(
        run_fight_setup(&weak, &content)
            .unwrap()
            .enemy_handicap
            .hero_health,
        Some(20)
    );
}

#[test]
fn r2922001_thick_skins_armor_reaches_the_fights_hero_and_every_cursed_setup_starts() {
    // R2922001, R2922000: the human's starting Armor is on its hero when the game is created, and a
    // setup with every curse and every Combat artifact at its highest level is one the engine takes.
    let content = catalog_content();
    let state = holding(&fight_at(&content, 0), &[("temp-thick-skin", 2)]);
    let setup = run_fight_setup(&state, &content).unwrap();
    let game = create_recorded_game(&fight_options(&setup));
    assert_eq!(game.players[PlayerId::P1].hero.armor, 2);
    assert_eq!(game.players[PlayerId::P2].hero.armor, 0);
    assert_eq!(
        game.players[PlayerId::P1].handicap.and_then(|h| h.hero_armor),
        Some(2)
    );

    let every: Vec<&str> = content.curses.iter().map(|c| c.id.as_str()).collect();
    let combat: Vec<(&str, i32)> = content
        .passives
        .iter()
        .filter(|p| p.kind == RunPassiveKind::Combat)
        .map(|p| (p.id.as_str(), RUN_ARTIFACT_LEVEL_MAX))
        .collect();
    let all = cursed(&holding(&fight_at(&content, 0), &combat), &every);
    assert_eq!(engine_takes(&run_fight_setup(&all, &content).unwrap()), Ok(()));
}
