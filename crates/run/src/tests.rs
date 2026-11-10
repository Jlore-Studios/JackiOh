//! The run's rules on the placeholder content (`placeholder.json`).

use super::*;

fn content() -> RunContent {
    run_placeholder_content()
}

fn setup(content: &RunContent) -> RunSetup {
    RunSetup {
        seed: run_daily_seed("2026-11-01", 1),
        starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
        artifacts: vec!["temp-greedy".into()],
        curses: Vec::new(),
    }
}

fn start(content: &RunContent) -> RunState {
    run_create(&setup(content), &RunUnlocks::default(), content).unwrap()
}

fn act(state: &RunState, content: &RunContent, action: RunAction) -> RunState {
    run_reduce(state, content, &action).unwrap()
}

fn won(hp: i32) -> RunAction {
    RunAction::FightOver {
        won: true,
        hp,
        damage_dealt: 30,
        healing_done: 2,
        cards_drawn: 12,
        turns: 8,
    }
}

/// A player who always takes the first node it may step onto, wins every fight at 20 HP, takes every artifact, opens
/// the first pack and keeps its first cards, extracts the first card at a mini-boss and nothing at the
/// final boss, leaves shops, heals at campfires, walks away from events and takes the first blessing.
fn autoplay(state: &RunState) -> RunAction {
    match &state.phase {
        RunPhase::Map => RunAction::Go {
            col: run_next_cols(state)[0],
        },
        RunPhase::Fight => won(20),
        RunPhase::Reward {
            artifact: Some(_), ..
        } => RunAction::TakeArtifact { take: true },
        RunPhase::Reward { artifact: None, .. } => RunAction::OpenPack { index: 0 },
        RunPhase::Pack { cards, keep } => RunAction::Keep {
            cards: cards[..*keep].to_vec(),
        },
        RunPhase::ExtractCard => RunAction::Extract { index: Some(0) },
        RunPhase::ExtractArtifact => RunAction::Extract { index: None },
        RunPhase::Shop { .. } => RunAction::Leave,
        RunPhase::Campfire => RunAction::Heal,
        RunPhase::Event { .. } => RunAction::Choose { index: 2 },
        RunPhase::Shrine { .. } => RunAction::Bless { index: Some(0) },
        RunPhase::Won | RunPhase::Dead => unreachable!("the run is over"),
    }
}

fn play_out(content: &RunContent, setup: &RunSetup) -> (RunState, Vec<RunAction>) {
    let mut state = run_create(setup, &RunUnlocks::default(), content).unwrap();
    let mut log = Vec::new();
    while !matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
        let action = autoplay(&state);
        state = act(&state, content, action.clone());
        log.push(action);
    }
    (state, log)
}

fn first_of(content: &RunContent, rarity: PrintedRarity) -> String {
    content
        .pool
        .iter()
        .find(|c| c.rarity == rarity && !is_boss_card(c))
        .unwrap()
        .id
        .clone()
}

#[test]
fn a_seed_is_six_capital_letters_and_the_daily_one_is_the_same_for_everyone() {
    let seed = run_daily_seed("2026-11-01", 1);
    assert_eq!(seed.len(), RUN_SEED_LENGTH);
    assert!(seed_is_valid(&seed));
    assert_eq!(seed, run_daily_seed("2026-11-01", 1));
    assert_ne!(seed, run_daily_seed("2026-11-01", 2));
    assert_ne!(seed, run_daily_seed("2026-11-02", 1));
    assert!(!seed_is_valid("abcdef"));
    assert!(!seed_is_valid("ABCDE"));
}

#[test]
fn the_map_holds_three_acts_each_ending_at_a_lone_boss_with_its_drop() {
    let content = content();
    let state = start(&content);
    assert_eq!(state.map.len(), RUN_ACTS * (RUN_ACT_ROWS + 1));
    let bosses: Vec<usize> = (0..state.map.len())
        .filter(|r| state.map[*r].iter().any(|node| node.kind.is_boss()))
        .collect();
    let expected: Vec<usize> = (0..RUN_ACTS).map(|a| (a + 1) * (RUN_ACT_ROWS + 1) - 1).collect();
    assert_eq!(bosses, expected);
    let last = bosses[RUN_ACTS - 1];
    assert_eq!(state.map[last].len(), 1);
    assert_eq!(state.map[last][0].kind, RunNodeKind::Boss);
    assert_eq!(state.map[last][0].artifact.as_deref(), Some("temp-final-relic"));
    for r in bosses {
        assert_eq!(state.map[r].len(), 1, "row {r} holds its boss alone");
        let drop = state.map[r][0]
            .artifact
            .as_deref()
            .expect("a boss's drop is preseeded");
        assert!(passive_def(&content, drop).unwrap().tier >= Some(RUN_MINI_BOSS_DROP_TIER));
    }
    assert_eq!(state.map, start(&content).map, "one seed, one map");
}

#[test]
fn an_enemy_deck_holds_its_own_cards_and_no_other_boss_card() {
    let content = content();
    let state = start(&content);
    for node in state.map.iter().flatten() {
        let Some(enemy) = &node.enemy else { continue };
        assert_eq!(enemy.deck.len(), RUN_ENEMY_DECK_SIZE);
        let def = content.enemies.iter().find(|e| e.id == enemy.id);
        for id in &enemy.deck {
            let card = card_def(&content, id).unwrap();
            assert!(
                !is_boss_card(card) || def.is_some_and(|d| d.deck.contains(id)),
                "{id} in {}",
                enemy.name
            );
        }
    }
}

#[test]
fn a_starting_deck_fits_its_levels_slots() {
    let content = content();
    let mut setup = setup(&content);
    assert_eq!(run_check_setup(&setup, &RunUnlocks::default(), &content), Ok(()));

    setup.starter.pop();
    assert!(
        run_check_setup(&setup, &RunUnlocks::default(), &content)
            .unwrap_err()
            .contains("holds 10 cards")
    );

    // Level 1 has no Epic slot but its one Anything slot.
    let epic = first_of(&content, PrintedRarity::Epic);
    let mythic = first_of(&content, PrintedRarity::Mythic);
    setup.starter.push(epic.clone());
    let unlocks = RunUnlocks {
        cards: vec![epic.clone(), mythic.clone()],
        ..RunUnlocks::default()
    };
    assert_eq!(
        run_check_setup(&setup, &unlocks, &content),
        Ok(()),
        "the Anything slot takes it"
    );
    setup.starter[0] = mythic;
    let err = run_check_setup(&setup, &unlocks, &content).unwrap_err();
    assert!(err.contains("need a slot of Epic or better"), "{err}");
    let higher = RunUnlocks { level: 5, ..unlocks };
    assert_eq!(run_check_setup(&setup, &higher, &content), Ok(()));
}

#[test]
fn an_extracted_card_starts_a_campaign_run_only_when_owned_a_boss_card_excepted() {
    let content = content();
    let mut setup = setup(&content);
    let epic = first_of(&content, PrintedRarity::Epic);
    setup.starter[9] = epic.clone();
    assert!(
        run_check_setup(&setup, &RunUnlocks::default(), &content)
            .unwrap_err()
            .contains("not unlocked")
    );
    let sandbox = RunUnlocks {
        cards: vec![epic.clone()],
        ..RunUnlocks::default()
    };
    assert_eq!(run_check_setup(&setup, &sandbox, &content), Ok(()));
    let campaign = RunUnlocks {
        owned: Some(Vec::new()),
        ..sandbox.clone()
    };
    assert!(
        run_check_setup(&setup, &campaign, &content)
            .unwrap_err()
            .contains("does not own")
    );
    let owning = RunUnlocks {
        owned: Some(vec![epic]),
        ..sandbox
    };
    assert_eq!(run_check_setup(&setup, &owning, &content), Ok(()));

    setup.starter[9] = "pseudo-boss-1".into();
    let boss_card = RunUnlocks {
        cards: vec!["pseudo-boss-1".into()],
        owned: Some(Vec::new()),
        ..RunUnlocks::default()
    };
    assert_eq!(
        run_check_setup(&setup, &boss_card, &content),
        Ok(()),
        "a boss card needs no owning"
    );
}

#[test]
fn a_run_starts_with_at_most_three_artifacts_worth_at_most_eight_points() {
    let content = content();
    let mut setup = setup(&content);
    let all = RunUnlocks {
        artifacts: vec!["temp-war-chest".into(), "temp-crown".into(), "temp-coupon".into()],
        ..RunUnlocks::default()
    };
    // Gold 3 + Silver 2 + Bronze 1 = 6.
    setup.artifacts = vec!["temp-greedy".into(), "temp-wide-net".into(), "temp-picky".into()];
    assert_eq!(run_check_setup(&setup, &all, &content), Ok(()));
    setup.artifacts.push("temp-big-heart".into());
    assert!(
        run_check_setup(&setup, &all, &content)
            .unwrap_err()
            .contains("at most 3")
    );
    // Diamond 5 + Gold 3 + Silver 2 = 10.
    setup.artifacts = vec!["temp-crown".into(), "temp-greedy".into(), "temp-wide-net".into()];
    assert!(
        run_check_setup(&setup, &all, &content)
            .unwrap_err()
            .contains("10 points")
    );
    setup.artifacts = vec!["temp-crown".into(), "temp-greedy".into()];
    assert_eq!(run_check_setup(&setup, &all, &content), Ok(()));
    setup.artifacts = vec!["temp-crown".into()];
    assert!(
        run_check_setup(&setup, &RunUnlocks::default(), &content)
            .unwrap_err()
            .contains("not unlocked")
    );
    setup.artifacts = vec!["temp-final-relic".into()];
    let relic = RunUnlocks {
        artifacts: vec!["temp-final-relic".into()],
        ..RunUnlocks::default()
    };
    assert!(
        run_check_setup(&setup, &relic, &content)
            .unwrap_err()
            .contains("cannot start")
    );
}

#[test]
fn a_run_played_to_the_end_wins_and_its_file_folds_back_to_the_same_state() {
    let content = content();
    let setup = setup(&content);
    let (end, log) = play_out(&content, &setup);
    assert_eq!(end.phase, RunPhase::Won);
    assert!(
        (RUN_ROUTE_FIGHTS_MIN..=RUN_ROUTE_FIGHTS_MAX).contains(&end.stats.fights_won),
        "R3583001: {} fights on the route",
        end.stats.fights_won
    );
    assert_eq!(end.stats.mini_bosses_won, 2);
    assert_eq!(end.stats.bosses_won, 1);
    assert_eq!(end.extracted_cards.len(), 2, "one card at each mini-boss");
    assert!(
        end.artifacts.iter().any(|a| a.id == "temp-final-relic"),
        "the final boss's drop"
    );
    // One copy of a card a deck: the placeholder pool's 44 cards cap what 3 or 4 kept a fight would reach.
    assert!(end.deck.len() >= 30, "the deck grows: {}", end.deck.len());

    let file = RunFile {
        setup,
        unlocks: RunUnlocks::default(),
        log,
    };
    let json = serde_json::to_string(&file).unwrap();
    let back: RunFile = serde_json::from_str(&json).unwrap();
    assert_eq!(run_fold(&back, &content), Ok(end));
}

#[test]
fn the_final_boss_drop_cannot_be_extracted() {
    let content = content();
    let mut state = start(&content);
    state.artifacts.push(RunArtifact {
        id: "temp-final-relic".into(),
        level: 1,
    });
    state.phase = RunPhase::ExtractArtifact;
    let relic = state.artifacts.len() - 1;
    let err = run_reduce(&state, &content, &RunAction::Extract { index: Some(relic) }).unwrap_err();
    assert!(err.contains("cannot be extracted"));
    let done = act(&state, &content, RunAction::Extract { index: Some(0) });
    assert_eq!(done.extracted_artifacts, vec!["temp-greedy".to_string()]);
    assert_eq!(done.phase, RunPhase::Won);
}

#[test]
fn a_lost_fight_ends_the_run() {
    let content = content();
    let mut state = act(&start(&content), &content, RunAction::Go { col: 0 });
    let lost = RunAction::FightOver {
        won: false,
        hp: 0,
        damage_dealt: 0,
        healing_done: 0,
        cards_drawn: 0,
        turns: 0,
    };
    state = act(&state, &content, lost);
    assert_eq!(state.phase, RunPhase::Dead);
    assert!(run_reduce(&state, &content, &RunAction::Go { col: 0 }).is_err());
}

/// `state` played on by `autoplay` until it is back on the map.
fn back_on_the_map(state: &RunState, content: &RunContent) -> RunState {
    let mut state = state.clone();
    while state.phase != RunPhase::Map {
        let action = autoplay(&state);
        state = act(&state, content, action);
    }
    state
}

#[test]
fn r3583000_go_steps_only_onto_a_node_the_one_stood_on_leads_to() {
    let content = content();
    let state = start(&content);
    assert_eq!(
        run_next_cols(&state),
        (0..state.map[0].len()).collect::<Vec<_>>(),
        "any start before the first step"
    );
    let (mut accepted, mut refused) = (false, false);
    for col in 0..state.map[0].len() {
        let at = back_on_the_map(&act(&state, &content, RunAction::Go { col }), &content);
        assert_eq!((at.row, at.col), (Some(0), col));
        let next = &at.map[0][col].next;
        assert_eq!(&run_next_cols(&at), next);
        for to in 0..at.map[1].len() {
            match run_reduce(&at, &content, &RunAction::Go { col: to }) {
                Ok(after) => {
                    assert!(next.contains(&to), "({col}) -> {to} is no edge");
                    assert_eq!((after.row, after.col), (Some(1), to));
                    accepted = true;
                }
                Err(error) => {
                    assert!(!next.contains(&to), "({col}) -> {to} is an edge");
                    assert_eq!(error, "no path leads there from here");
                    refused = true;
                }
            }
        }
    }
    assert!(accepted && refused);
}

#[test]
fn r3583000_a_mini_boss_leads_to_every_node_of_the_next_acts_first_row() {
    let content = content();
    let state = start(&content);
    let row = mini_boss_rows(&state)[0];
    let at = back_on_the_map(&act(&fighting_on(&state, row), &content, won(20)), &content);
    let first: Vec<usize> = (0..at.map[row + 1].len()).collect();
    assert_eq!(first.len(), RUN_MAP_PATHS);
    assert_eq!(run_next_cols(&at), first);
    for col in first {
        let after = act(&at, &content, RunAction::Go { col });
        assert_eq!((after.row, after.col), (Some(row + 1), col));
    }
}

#[test]
fn r3583000_a_log_stepping_off_the_paths_does_not_fold() {
    let content = content();
    let mut state = start(&content);
    let mut log = Vec::new();
    while log.is_empty() || state.phase != RunPhase::Map {
        let action = autoplay(&state);
        state = act(&state, &content, action.clone());
        log.push(action);
    }
    let off = (0..state.map[1].len())
        .find(|col| !run_next_cols(&state).contains(col))
        .expect("a node of the second row the first step does not lead to");
    log.push(RunAction::Go { col: off });
    let step = log.len() - 1;
    let file = RunFile {
        setup: setup(&content),
        unlocks: RunUnlocks::default(),
        log,
    };
    assert_eq!(
        run_fold(&file, &content),
        Err(format!("step {step}: no path leads there from here"))
    );
}

/// The map's mini-boss rows, the first act's first.
fn mini_boss_rows(state: &RunState) -> Vec<usize> {
    (0..state.map.len())
        .filter(|r| state.map[*r][0].kind == RunNodeKind::MiniBoss)
        .collect()
}

/// `state` in the fight on column 0 of `row`, as the walk to it would leave it.
fn fighting_on(state: &RunState, row: usize) -> RunState {
    let mut state = state.clone();
    state.row = Some(row);
    state.col = 0;
    state.phase = RunPhase::Fight;
    state
}

fn hold(state: &mut RunState, id: &str) {
    state.artifacts.push(RunArtifact {
        id: id.into(),
        level: 1,
    });
}

/// The artifact a won fight's reward offers.
fn offered(state: &RunState) -> Option<String> {
    match &state.phase {
        RunPhase::Reward { artifact, .. } => artifact.clone(),
        other => panic!("not a reward: {other:?}"),
    }
}

/// Whether a relic is one a roll may deal at `RUN_MINI_BOSS_DROP_TIER` or better.
fn gold_or_better_player_relic(content: &RunContent, id: &str) -> bool {
    let def = passive_def(content, id).expect("an offered relic is in the content");
    matches!(def.kind, RunPassiveKind::Combat | RunPassiveKind::Economic)
        && !def.unobtainable
        && def.tier >= Some(RUN_MINI_BOSS_DROP_TIER)
}

#[test]
fn r3286000_a_won_mini_boss_whose_drop_is_held_offers_another_relic_of_gold_or_better() {
    let content = content();
    for n in 0..20 {
        let run_setup = RunSetup {
            seed: run_seed(&format!("r3286000-{n}")),
            ..setup(&content)
        };
        let mut state = run_create(&run_setup, &RunUnlocks::default(), &content).unwrap();
        let row = mini_boss_rows(&state)[0];
        let drop_id = state.map[row][0]
            .artifact
            .clone()
            .expect("a mini-boss is dealt a drop");
        hold(&mut state, &drop_id);
        let fight = fighting_on(&state, row);
        let reward = act(&fight, &content, won(20));
        let other = offered(&reward).expect("a relic is offered");
        assert_ne!(other, drop_id, "seed {n}");
        assert!(!held_ids(&state).contains(&other.as_str()), "seed {n}");
        assert!(gold_or_better_player_relic(&content, &other), "seed {n}");
        assert_eq!(
            offered(&act(&fight, &content, won(20))),
            Some(other),
            "one state, one offer"
        );
    }
}

#[test]
fn r3286000_two_mini_bosses_dealt_one_drop_offer_it_at_the_first_and_another_relic_at_the_second() {
    let content = content();
    let state = (0..200)
        .map(|n| RunSetup {
            seed: run_seed(&format!("r3286000-both-{n}")),
            ..setup(&content)
        })
        .map(|run_setup| run_create(&run_setup, &RunUnlocks::default(), &content).unwrap())
        .find(|s| {
            let rows = mini_boss_rows(s);
            s.map[rows[0]][0].artifact == s.map[rows[1]][0].artifact
        })
        .expect("a seed deals both mini-bosses one drop");
    let rows = mini_boss_rows(&state);
    let drop_id = state.map[rows[0]][0]
        .artifact
        .clone()
        .expect("a mini-boss is dealt a drop");

    let first = act(&fighting_on(&state, rows[0]), &content, won(20));
    assert_eq!(offered(&first), Some(drop_id.clone()));
    let taken = act(&first, &content, RunAction::TakeArtifact { take: true });
    assert!(held_ids(&taken).contains(&drop_id.as_str()));

    let second = act(&fighting_on(&taken, rows[1]), &content, won(20));
    let other = offered(&second).expect("a relic is offered at the second mini-boss");
    assert_ne!(other, drop_id);
    assert!(!held_ids(&taken).contains(&other.as_str()));
    assert!(gold_or_better_player_relic(&content, &other));
}

#[test]
fn r3286000_with_no_relic_left_to_offer_the_reward_has_none() {
    let content = content();
    let mut state = start(&content);
    let gold_or_better: Vec<String> = content
        .passives
        .iter()
        .filter(|p| gold_or_better_player_relic(&content, &p.id))
        .map(|p| p.id.clone())
        .collect();
    assert!(gold_or_better.len() >= 2, "{gold_or_better:?}");
    for id in &gold_or_better {
        if !held_ids(&state).contains(&id.as_str()) {
            hold(&mut state, id);
        }
    }
    let below_gold_is_open = content.passives.iter().any(|p| {
        matches!(p.kind, RunPassiveKind::Combat | RunPassiveKind::Economic)
            && !p.unobtainable
            && !held_ids(&state).contains(&p.id.as_str())
            && p.tier < Some(RUN_MINI_BOSS_DROP_TIER)
    });
    assert!(below_gold_is_open, "the floor, not an empty pool, leaves nothing");

    let rows = mini_boss_rows(&state);
    assert!(!rows.is_empty());
    for row in rows {
        let reward = act(&fighting_on(&state, row), &content, won(20));
        assert!(
            matches!(reward.phase, RunPhase::Reward { artifact: None, ref packs } if !packs.is_empty()),
            "{:?}",
            reward.phase
        );
    }
}

#[test]
fn the_artifact_comes_before_the_pack_and_greedy_keeps_one_more() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Reward {
        artifact: Some("temp-coupon".into()),
        packs: vec![RunPackKind::Random],
    };
    assert!(run_reduce(&state, &content, &RunAction::OpenPack { index: 0 }).is_err());
    state = act(&state, &content, RunAction::TakeArtifact { take: false });
    state = act(&state, &content, RunAction::OpenPack { index: 0 });
    let RunPhase::Pack { cards, keep } = state.phase.clone() else {
        panic!("a pack is open")
    };
    assert_eq!(keep, RUN_KEEP + 1, "the starting Greedy");
    assert_eq!(cards.len(), RUN_PACK_SIZE);
    assert!(
        cards
            .iter()
            .all(|id| !is_boss_card(card_def(&content, id).unwrap()))
    );
    assert!(
        run_reduce(
            &state,
            &content,
            &RunAction::Keep {
                cards: cards[..RUN_KEEP].to_vec()
            }
        )
        .is_err()
    );
    assert!(
        run_reduce(&state, &content, &RunAction::Keep { cards: Vec::new() }).is_err(),
        "no Skip passive"
    );
    let kept = act(
        &state,
        &content,
        RunAction::Keep {
            cards: cards[..keep].to_vec(),
        },
    );
    assert_eq!(kept.deck.len(), RUN_STARTER_SIZE + keep);
}

#[test]
fn a_campfire_heals_makes_radiant_buffs_or_removes_two() {
    let content = content();
    let mut state = start(&content);
    state.hp = 10;
    state.phase = RunPhase::Campfire;
    assert_eq!(
        act(&state, &content, RunAction::Heal).hp,
        10 + RUN_MAX_HP * RUN_CAMPFIRE_HEAL_PERCENT / PERCENT
    );
    assert!(act(&state, &content, RunAction::MakeRadiant { index: 0 }).deck[0].radiant);
    assert_eq!(
        act(&state, &content, RunAction::Buff { index: 0 }).deck[0].buffs,
        1
    );
    let removed = act(&state, &content, RunAction::RemoveCards { indices: vec![0, 3] });
    assert_eq!(removed.deck.len(), RUN_STARTER_SIZE - RUN_CAMPFIRE_REMOVE);
    assert!(!in_deck(&removed, &state.deck[0].id) && !in_deck(&removed, &state.deck[3].id));
    assert!(run_reduce(&state, &content, &RunAction::RemoveCards { indices: vec![1, 1] }).is_err());
}

#[test]
fn a_shop_refuses_what_the_run_cannot_pay() {
    let content = content();
    let mut state = start(&content);
    let mut rng = Rng::new("shop", 0);
    state.phase = RunPhase::Shop {
        stock: roll_shop(&state, &content, &mut rng),
    };
    assert!(
        run_reduce(&state, &content, &RunAction::Buy { index: 0 })
            .unwrap_err()
            .contains("Coins needed")
    );
    state.coins = 500;
    state.tickets = RUN_RADIANT_TICKETS;
    state = act(&state, &content, RunAction::Buy { index: 0 });
    assert_eq!(state.deck.len(), RUN_STARTER_SIZE + 1);
    assert!(
        run_reduce(&state, &content, &RunAction::Buy { index: 0 })
            .unwrap_err()
            .contains("sold")
    );
    state = act(&state, &content, RunAction::MakeRadiant { index: 0 });
    assert!(state.deck[0].radiant);
    assert!(
        run_reduce(&state, &content, &RunAction::Buff { index: 0 })
            .unwrap_err()
            .contains("Tickets needed")
    );
}

#[test]
fn the_shop_prices_are_what_each_service_spends_with_a_discount_held_and_none_for_a_top_level_relic() {
    let content = content();
    let mut state = start(&content);
    state.artifacts = vec![
        RunArtifact {
            id: "temp-coupon".into(),
            level: 2,
        },
        RunArtifact {
            id: "temp-greedy".into(),
            level: 1,
        },
        RunArtifact {
            id: "temp-big-heart".into(),
            level: RUN_ARTIFACT_LEVEL_MAX,
        },
    ];
    state.coins = 1_000;
    state.tickets = 1_000;
    state.phase = RunPhase::Shop {
        stock: roll_shop(&state, &content, &mut Rng::new("shop", 0)),
    };
    let prices = run_shop_prices(&state, &content);
    assert!(
        prices.remove_coins < RUN_REMOVE_PRICE,
        "the held coupon discounts"
    );

    // What an action takes from the purse, as (Coins, Tickets).
    let spent = |action: RunAction| {
        let after = act(&state, &content, action);
        (state.coins - after.coins, state.tickets - after.tickets)
    };
    assert_eq!(
        spent(RunAction::RemoveCard { index: 0 }),
        (prices.remove_coins, 0)
    );
    assert_eq!(
        spent(RunAction::MakeRadiant { index: 0 }),
        (0, prices.radiant_tickets)
    );
    assert_eq!(spent(RunAction::Buff { index: 0 }), (0, prices.buff_tickets));

    assert_eq!(
        prices.level_tickets,
        vec![Some(RUN_LEVEL_TICKETS * 2), Some(RUN_LEVEL_TICKETS), None]
    );
    for (index, price) in prices.level_tickets.iter().enumerate() {
        match price {
            Some(price) => assert_eq!(spent(RunAction::LevelArtifact { index }), (0, *price)),
            None => assert!(run_reduce(&state, &content, &RunAction::LevelArtifact { index }).is_err()),
        }
    }

    let RunPhase::Shop { stock } = &state.phase else {
        panic!("a shop is open")
    };
    assert_eq!(prices.stock_coins.len(), stock.len());
    for (index, coins) in prices.stock_coins.iter().enumerate() {
        let after = act(&state, &content, RunAction::Buy { index });
        assert_eq!(state.coins - after.coins, *coins, "stock item {index}");
    }
    assert!(
        prices
            .stock_coins
            .iter()
            .zip(stock)
            .any(|(coins, item)| *coins < item.price),
        "the discount reaches the stock"
    );

    let plain = start(&content);
    let outside = run_shop_prices(&plain, &content);
    assert!(outside.stock_coins.is_empty(), "no stock outside a shop");
    assert_eq!(outside.remove_coins, RUN_REMOVE_PRICE, "no discount relic held");
}

#[test]
fn bosses_and_curses_multiply_the_score() {
    let content = content();
    let mut state = start(&content);
    state.row = Some(0);
    state.stats.fights_won = 1;
    let base = run_score(&state, &content);
    assert_eq!(base, RUN_SCORE_ROW + i64::from(RUN_SCORE_FIGHT.fight));
    state.stats.fights_won = 2;
    state.stats.mini_bosses_won = 1;
    let one_mini = (RUN_SCORE_ROW + i64::from(RUN_SCORE_FIGHT.fight + RUN_SCORE_FIGHT.mini_boss))
        * RUN_SCORE_MINI_BOSS_PERCENT
        / i64::from(PERCENT);
    assert_eq!(run_score(&state, &content), one_mini);
    state.curses = vec!["temp-curse-3".into()];
    let cursed = one_mini * (i64::from(PERCENT) + 3 * RUN_SCORE_CURSE_POINT_PERCENT) / i64::from(PERCENT);
    assert_eq!(run_score(&state, &content), cursed);

    let plain = play_out(&content, &setup(&content)).0;
    let mut cursed_setup = setup(&content);
    cursed_setup.curses = vec!["temp-curse-2".into()];
    let cursed = play_out(&content, &cursed_setup).0;
    assert!(run_score(&cursed, &content) > run_score(&plain, &content));
    assert!(run_embers(run_score(&plain, &content)) > 0);
}

#[test]
fn r2922001_every_placeholder_curse_and_combat_artifact_has_a_fight_effect_its_text_states() {
    // R2922001: each placeholder curse and Combat artifact changes a fight, and its text says by how
    // much; no other passive has a fight effect, and none says it changes nothing.
    let content = content();
    let amount = |effect: RunFightEffect| match effect {
        RunFightEffect::EnemyHealth { n }
        | RunFightEffect::EnemyOpeningCards { n }
        | RunFightEffect::EnemyMana { n }
        | RunFightEffect::OwnOpeningCards { n }
        | RunFightEffect::OwnArmor { n } => n,
    };
    let combat = content
        .passives
        .iter()
        .filter(|p| p.kind == RunPassiveKind::Combat)
        .map(|p| (&p.id, &p.text, p.fight_effect));
    let curses = content.curses.iter().map(|c| (&c.id, &c.text, c.fight_effect));
    for (id, text, effect) in combat.chain(curses) {
        let effect = effect.unwrap_or_else(|| panic!("{id} has a fight effect"));
        let n = amount(effect);
        assert!(n > 0, "{id}");
        assert!(text.contains(&n.to_string()), "{id}: {text:?} states {n}");
        for nothing in ["no fight", "nothing in a fight", "do nothing"] {
            assert!(!text.contains(nothing), "{id}: {text:?}");
        }
    }
    for passive in content
        .passives
        .iter()
        .filter(|p| p.kind != RunPassiveKind::Combat)
    {
        assert_eq!(passive.fight_effect, None, "{}", passive.id);
    }

    let json =
        |id: &str| serde_json::to_value(content.passives.iter().find(|p| p.id == id).unwrap()).unwrap();
    assert!(json("temp-picky").get("fightEffect").is_none());
    assert_eq!(
        json("temp-thick-skin")["fightEffect"],
        serde_json::json!({"type": "ownArmor", "n": 1})
    );
}
