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

/// A player who always takes the first node, wins every fight at 20 HP, takes every artifact, opens
/// the first pack and keeps its first cards, extracts the first card at a mini-boss and nothing at the
/// final boss, leaves shops, heals at campfires, walks away from events and takes the first blessing.
fn autoplay(state: &RunState) -> RunAction {
    match &state.phase {
        RunPhase::Map => RunAction::Go { col: 0 },
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
fn the_map_holds_sixteen_fights_three_of_them_bosses_each_after_a_campfire() {
    let content = content();
    let state = start(&content);
    assert_eq!(state.map.iter().filter(|row| row[0].kind.is_fight()).count(), 16);
    let bosses: Vec<usize> = (0..state.map.len())
        .filter(|r| state.map[*r][0].kind.is_boss())
        .collect();
    assert_eq!(bosses.len(), 3);
    assert_eq!(state.map[bosses[2]][0].kind, RunNodeKind::Boss);
    assert_eq!(
        state.map[bosses[2]][0].artifact.as_deref(),
        Some("temp-final-relic")
    );
    for r in bosses {
        assert_eq!(state.map[r - 1][0].kind, RunNodeKind::Campfire);
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
    assert_eq!(end.stats.fights_won, 16);
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
