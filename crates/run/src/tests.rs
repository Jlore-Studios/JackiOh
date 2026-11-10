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
/// final boss, leaves shops, rests at campfires, walks away from events and takes the first blessing.
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
        RunPhase::Campfire => RunAction::GainMaxHp,
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
    // R3584000: every won fight raised the max, and HP is refilled to it.
    let rests = log.iter().filter(|a| **a == RunAction::GainMaxHp).count() as i32;
    assert_eq!(
        end.max_hp_bonus,
        16 * RUN_WIN_MAX_HP + rests * RUN_CAMPFIRE_MAX_HP
    );
    assert_eq!(end.hp, run_max_hp(&end, &content));
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
fn an_old_heal_rest_reads_as_gain_max_hp_and_folds() {
    let content = content();
    let setup = setup(&content);
    let (end, log) = play_out(&content, &setup);
    assert!(log.contains(&RunAction::GainMaxHp));
    // New writes keep the new name: the alias reads only.
    assert_eq!(
        serde_json::to_value(RunAction::GainMaxHp).unwrap(),
        serde_json::json!({ "type": "gainMaxHp" })
    );
    let file = RunFile {
        setup,
        unlocks: RunUnlocks::default(),
        log,
    };
    // What the device kept before #3644: the campfire screen sent `{ type: "heal" }`.
    let json = serde_json::to_string(&file)
        .unwrap()
        .replace("\"gainMaxHp\"", "\"heal\"");
    let back: RunFile = serde_json::from_str(&json).unwrap();
    assert!(back.log.contains(&RunAction::GainMaxHp));
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
    // R3584000: a bonus from a win earlier changes nothing about a loss.
    state.max_hp_bonus = RUN_WIN_MAX_HP;
    state.hp = run_max_hp(&state, &content);
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
    assert_eq!(state.hp, 0);
    assert!(run_reduce(&state, &content, &RunAction::Go { col: 0 }).is_err());
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
fn a_campfire_gains_max_hp_makes_radiant_buffs_or_removes_two() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Campfire;
    // HP outside a fight always equals the max, so the rise lands full; a rise is not healing.
    let rested = act(&state, &content, RunAction::GainMaxHp);
    assert_eq!(rested.max_hp_bonus, RUN_CAMPFIRE_MAX_HP);
    assert_eq!(rested.hp, RUN_MAX_HP + RUN_CAMPFIRE_MAX_HP);
    assert_eq!(rested.hp, run_max_hp(&rested, &content));
    assert_eq!(rested.stats.healed, 0);
    assert_eq!(rested.phase, RunPhase::Map);
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
fn r3584000_a_won_fight_of_each_kind_raises_max_hp_and_refills() {
    // R3584000: a win of any kind adds `RUN_WIN_MAX_HP` for the rest of the run, then refills HP
    // to the new max, whatever the hero's final health was; the fight's damage is the max before
    // minus the final health, and the refill is not healing.
    let content = content();
    for kind in [
        RunNodeKind::Fight,
        RunNodeKind::Elite,
        RunNodeKind::Ghost,
        RunNodeKind::MiniBoss,
        RunNodeKind::Boss,
    ] {
        let mut state = start(&content);
        state.map[0][0].kind = kind;
        let max_before = run_max_hp(&state, &content);
        let reward = act(&fighting_on(&state, 0), &content, won(1));
        assert_eq!(reward.max_hp_bonus, RUN_WIN_MAX_HP, "{kind:?}");
        assert_eq!(reward.hp, max_before + RUN_WIN_MAX_HP, "{kind:?}");
        assert_eq!(reward.hp, run_max_hp(&reward, &content), "{kind:?}");
        assert_eq!(reward.stats.damage_taken, max_before - 1, "{kind:?}");
        assert_eq!(reward.stats.healed, 0, "{kind:?}");
    }
}

#[test]
fn r3584000_a_second_win_stacks_on_the_first() {
    // R3584000: each win adds its own `RUN_WIN_MAX_HP`.
    let content = content();
    let state = start(&content);
    let row: usize = (0..state.map.len())
        .filter(|r| state.map[*r][0].kind == RunNodeKind::Fight)
        .nth(1)
        .expect("a second fight row");
    let once = act(&fighting_on(&state, 0), &content, won(1));
    let twice = act(&fighting_on(&once, row), &content, won(1));
    assert_eq!(twice.max_hp_bonus, 2 * RUN_WIN_MAX_HP);
    assert_eq!(twice.hp, run_max_hp(&twice, &content));
    assert_eq!(twice.hp, RUN_MAX_HP + 2 * RUN_WIN_MAX_HP);
}

#[test]
fn r3584000_a_relic_or_blessing_max_hp_effect_stacks_on_the_bonus_once_per_level() {
    // R3584000: `run_max_hp` is `RUN_MAX_HP`, plus the run's own bonus, plus the max-HP effects
    // held; an artifact's counts once per level, a blessing's once.
    let content = content();
    let mut state = start(&content);
    state.max_hp_bonus = RUN_WIN_MAX_HP;
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP + RUN_WIN_MAX_HP);
    hold(&mut state, "temp-big-heart");
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP + RUN_WIN_MAX_HP + 5);
    state.artifacts[1].level = 2;
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP + RUN_WIN_MAX_HP + 10);
    state.blessings.push("temp-blessing-vitality".into());
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP + RUN_WIN_MAX_HP + 20);
}

#[test]
fn r3584001_an_event_option_trades_max_hp_and_hp_follows_it() {
    // R3584001: "Lose 5 max HP for 50 Coins" lowers the max by 5, and HP with it.
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Event {
        id: "temp-event-stranger".into(),
    };
    let max_before = run_max_hp(&state, &content);
    let after = act(&state, &content, RunAction::Choose { index: 1 });
    assert_eq!(after.max_hp_bonus, -5);
    assert_eq!(after.hp, max_before - 5);
    assert_eq!(after.hp, run_max_hp(&after, &content));
    assert_eq!(after.coins, 50);
    assert_eq!(after.stats.damage_taken, 5);
    assert_eq!(after.stats.healed, 0);
    assert_eq!(after.phase, RunPhase::Map);
}

#[test]
fn r3584001_an_event_refused_below_one_max_hp_leaves_the_state_unchanged() {
    // R3584001: an option that would leave max HP below 1 is refused, the state unchanged.
    let content = content();
    let mut state = start(&content);
    for _ in 0..5 {
        state.phase = RunPhase::Event {
            id: "temp-event-stranger".into(),
        };
        state = act(&state, &content, RunAction::Choose { index: 1 });
    }
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP - 25);
    assert_eq!(state.hp, RUN_MAX_HP - 25);
    state.phase = RunPhase::Event {
        id: "temp-event-stranger".into(),
    };
    let before = state.clone();
    let err = run_reduce(&state, &content, &RunAction::Choose { index: 1 }).unwrap_err();
    assert!(err.contains("not enough max HP"), "{err}");
    assert_eq!(state, before, "a refused option leaves the state unchanged");
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

// The mutation audit (#3513): each test below kills mutants `sh scripts/mutants.sh run` found
// surviving, by asserting the value the spec states.

/// §15.10: the score is `RUN_SCORE_ROW` per row reached, `RUN_SCORE_FIGHT` per win by its kind, and
/// each point of the fights' reports at its own weight; every term counts once.
#[test]
fn the_score_counts_every_row_win_and_report_point_at_its_own_weight() {
    let content = content();
    let mut state = start(&content);
    let rows = 5;
    let (normal, elites, ghosts) = (4, 2, 3);
    let (dealt, healed, drawn) = (7, 11, 13);
    state.row = Some(rows - 1);
    state.stats = RunStats {
        fights_won: normal + elites + ghosts,
        elites_won: elites,
        ghosts_won: ghosts,
        damage_dealt: dealt,
        healing_done: healed,
        cards_drawn: drawn,
        ..RunStats::default()
    };
    let win = |kind: RunNodeKind, n: i32| i64::from(RUN_SCORE_FIGHT.of(kind)) * i64::from(n);
    let expected = RUN_SCORE_ROW * i64::try_from(rows).unwrap()
        + win(RunNodeKind::Fight, normal)
        + win(RunNodeKind::Elite, elites)
        + win(RunNodeKind::Ghost, ghosts)
        + RUN_SCORE_DAMAGE * i64::from(dealt)
        + RUN_SCORE_HEALING * i64::from(healed)
        + RUN_SCORE_CARD_DRAWN * i64::from(drawn);
    assert_eq!(run_score(&state, &content), expected);
}

/// §15.10: beating the final boss counts its win, then multiplies the score by
/// `RUN_SCORE_FINAL_BOSS_PERCENT`.
#[test]
fn the_final_boss_counts_its_win_then_multiplies_the_score() {
    let content = content();
    let mut state = start(&content);
    state.row = Some(0);
    state.stats.fights_won = 2;
    state.stats.bosses_won = 1;
    let before = RUN_SCORE_ROW
        + i64::from(RUN_SCORE_FIGHT.of(RunNodeKind::Fight))
        + i64::from(RUN_SCORE_FIGHT.of(RunNodeKind::Boss));
    assert_eq!(
        run_score(&state, &content),
        before * RUN_SCORE_FINAL_BOSS_PERCENT / i64::from(PERCENT)
    );
}

/// §15.10: Embers are the score divided by `RUN_SCORE_PER_EMBER`, rounded down.
#[test]
fn embers_are_the_score_over_its_rate_rounded_down() {
    assert_eq!(run_embers(0), 0);
    assert_eq!(run_embers(RUN_SCORE_PER_EMBER - 1), 0);
    assert_eq!(run_embers(RUN_SCORE_PER_EMBER), 1);
    assert_eq!(run_embers(4 * RUN_SCORE_PER_EMBER - 1), 3);
}

/// §15.4: a relic's map effect counts once per level: a Max HP relic at level 2 adds its HP twice.
#[test]
fn a_max_hp_relic_adds_its_hp_once_per_level() {
    let content = content();
    let mut state = start(&content);
    state.artifacts.clear();
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP);
    let (id, n) = relic_with(&content, |e| match e {
        RunEffect::MaxHp { n } => Some(n),
        _ => None,
    });
    let level = 2;
    state.artifacts = vec![RunArtifact { id, level }];
    assert_eq!(run_max_hp(&state, &content), RUN_MAX_HP + n * level);
}

/// §15.3: a curse the content lacks is refused, and `run_create` refuses content that lacks a
/// normal, elite, mini-boss or final-boss enemy, or an event.
#[test]
fn an_unknown_curse_and_content_missing_an_enemy_tier_or_an_event_are_refused() {
    let content = content();
    let unlocks = RunUnlocks::default();
    let mut cursed = setup(&content);
    cursed.curses = vec![content.curses[0].id.clone()];
    assert!(run_check_setup(&cursed, &unlocks, &content).is_ok());
    cursed.curses = vec!["no-such-curse".into()];
    let refused = run_check_setup(&cursed, &unlocks, &content).unwrap_err();
    assert!(refused.contains("is no curse"), "{refused}");

    for tier in [
        RunNodeKind::Fight,
        RunNodeKind::Elite,
        RunNodeKind::MiniBoss,
        RunNodeKind::Boss,
    ] {
        let mut thin = content.clone();
        thin.enemies.retain(|e| e.tier != tier);
        let refused = run_create(&setup(&content), &unlocks, &thin).unwrap_err();
        assert!(refused.contains(&format!("no {tier:?} enemy")), "{refused}");
    }
    let mut eventless = content.clone();
    eventless.events.clear();
    let refused = run_create(&setup(&content), &unlocks, &eventless).unwrap_err();
    assert!(refused.contains("no event"), "{refused}");
}

/// §15.5: tiles are dealt by `RUN_TILE_WEIGHTS`: a roll under the first weight is the first item,
/// and each weight after it takes the next span of rolls.
#[test]
fn a_weighted_draw_gives_each_item_its_own_span_of_rolls() {
    let rows = [('a', 2), ('b', 3), ('c', 4)];
    let total: i32 = rows.iter().map(|(_, w)| w).sum();
    let mut rolled = Vec::new();
    for cursor in 0..1024 {
        let roll = Rng::new("weighted", cursor).int(total);
        let mut floor = 0;
        let expected = rows
            .iter()
            .find(|(_, w)| {
                floor += w;
                roll < floor
            })
            .unwrap()
            .0;
        assert_eq!(
            weighted(&rows, &mut Rng::new("weighted", cursor)),
            expected,
            "roll {roll}"
        );
        rolled.push(roll);
    }
    for roll in 0..total {
        assert!(rolled.contains(&roll), "roll {roll} came up");
    }
}

/// §15.5: fight and tile rows hold `RUN_ROW_WIDTH_MIN` to `RUN_ROW_WIDTH_MAX` nodes, and each width
/// comes up.
#[test]
fn a_row_holds_the_fewest_to_the_most_nodes() {
    let widths: Vec<i32> = (0..256)
        .map(|cursor| row_width(&mut Rng::new("width", cursor)))
        .collect();
    let range = RUN_ROW_WIDTH_MIN..=RUN_ROW_WIDTH_MAX;
    assert!(widths.iter().all(|w| range.contains(w)), "{widths:?}");
    for width in range {
        assert!(widths.contains(&width), "width {width} came up");
    }
}

/// §15.5: a fight node is an Elite when its first roll is under `RUN_ELITE_PERCENT`, else a Ghost
/// when its second is under `RUN_GHOST_PERCENT`, else a Fight: a roll at either percent is not under
/// it.
#[test]
fn a_fight_node_is_an_elite_or_a_ghost_on_a_roll_under_its_percent() {
    let (mut at_elite, mut at_ghost) = (false, false);
    for cursor in 0..4096 {
        let mut probe = Rng::new("kind", cursor);
        let (first, second) = (probe.int(PERCENT), probe.int(PERCENT));
        let expected = if first < RUN_ELITE_PERCENT {
            RunNodeKind::Elite
        } else if second < RUN_GHOST_PERCENT {
            RunNodeKind::Ghost
        } else {
            RunNodeKind::Fight
        };
        assert_eq!(
            fight_kind(&mut Rng::new("kind", cursor)),
            expected,
            "rolls {first}, {second}"
        );
        at_elite |= first == RUN_ELITE_PERCENT;
        at_ghost |= first > RUN_ELITE_PERCENT && second == RUN_GHOST_PERCENT;
    }
    assert!(at_elite && at_ghost, "a roll at each percent came up");
}

/// §15.7: a won fight offers a relic on a roll under `RUN_ARTIFACT_PERCENT` of its kind, and never on
/// a roll at it.
#[test]
fn a_fights_relic_comes_on_a_roll_under_its_percent_not_at_it() {
    let content = content();
    let state = start(&content);
    let percent = RUN_ARTIFACT_PERCENT.of(RunNodeKind::Fight);
    let cursor_at = |roll: i32| {
        (0..u32::MAX)
            .find(|&cursor| Rng::new("relic", cursor).int(PERCENT) == roll)
            .unwrap()
    };
    let offer = |cursor: u32| {
        roll_reward_artifact(
            &state,
            &content,
            RunNodeKind::Fight,
            None,
            &mut Rng::new("relic", cursor),
        )
    };
    assert_eq!(offer(cursor_at(percent)), None);
    assert!(offer(cursor_at(percent - 1)).is_some());
}

/// §15.8: a chest holds `RUN_CHEST_COINS` and `RUN_CHEST_TICKETS`; §15.10's stats count what it paid
/// and that it was opened.
#[test]
fn a_chest_pays_its_coins_and_tickets_and_the_stats_count_them() {
    let content = content();
    let mut state = start(&content);
    assert_eq!((state.coins, state.tickets), (0, 0));
    state.map[0] = vec![RunNode {
        kind: RunNodeKind::Chest,
        enemy: None,
        event: None,
        artifact: None,
    }];
    let opened = act(&state, &content, RunAction::Go { col: 0 });
    assert_eq!(
        (opened.coins, opened.tickets),
        (RUN_CHEST_COINS, RUN_CHEST_TICKETS)
    );
    assert_eq!(
        (opened.stats.coins_earned, opened.stats.tickets_earned),
        (RUN_CHEST_COINS, RUN_CHEST_TICKETS)
    );
    assert_eq!(opened.stats.chests_opened, 1);
}

/// §15.8: a shop's price is what it takes, so exactly that much is enough; §15.10's stats count the
/// Coins and Tickets spent.
#[test]
fn exactly_the_price_pays_and_the_stats_count_what_was_spent() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Shop {
        stock: roll_shop(&state, &content, &mut Rng::new("shop", 0)),
    };
    let price = run_shop_prices(&state, &content).stock_coins[0];
    assert!(price > 0);
    state.coins = price;
    let bought = act(&state, &content, RunAction::Buy { index: 0 });
    assert_eq!((bought.coins, bought.stats.coins_spent), (0, price));
    state.tickets = RUN_RADIANT_TICKETS;
    let radiant = act(&state, &content, RunAction::MakeRadiant { index: 0 });
    assert_eq!(
        (radiant.tickets, radiant.stats.tickets_spent),
        (0, RUN_RADIANT_TICKETS)
    );
}

/// The first relic (no blessing) whose effect `read` reads, and the amount it reads.
fn relic_with(content: &RunContent, read: impl Fn(RunEffect) -> Option<i32>) -> (String, i32) {
    content
        .passives
        .iter()
        .filter(|p| p.kind != RunPassiveKind::Blessing)
        .find_map(|p| p.effect.and_then(&read).map(|n| (p.id.clone(), n)))
        .expect("a relic with the effect")
}

/// §15.4: a rise of max HP heals by the rise; §15.10 counts the relic taken.
#[test]
fn gaining_a_max_hp_relic_heals_by_its_rise_and_counts_it() {
    let content = content();
    let mut state = start(&content);
    let (id, n) = relic_with(&content, |e| match e {
        RunEffect::MaxHp { n } => Some(n),
        _ => None,
    });
    let hurt = run_max_hp(&state, &content) - 2 * n;
    state.hp = hurt;
    gain_artifact(&mut state, &content, id);
    assert_eq!(state.hp, hurt + n);
    assert_eq!(state.stats.artifacts_taken, 1);
}

/// §15.8: each shop service makes its one change and §15.10 counts it; a card is never removed past
/// the deck's end, nor the last one.
#[test]
fn each_shop_service_makes_its_change_and_the_stats_count_it() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Shop {
        stock: roll_shop(&state, &content, &mut Rng::new("shop", 0)),
    };
    state.coins = 1_000;
    state.tickets = 1_000;
    let bought = act(&state, &content, RunAction::Buy { index: 0 });
    assert_eq!(bought.stats.cards_bought, 1);
    let removed = act(&state, &content, RunAction::RemoveCard { index: 0 });
    assert_eq!(
        (removed.deck.len(), removed.stats.cards_removed),
        (state.deck.len() - 1, 1)
    );
    let radiant = act(&state, &content, RunAction::MakeRadiant { index: 0 });
    assert_eq!(radiant.stats.cards_made_radiant, 1);
    let buffed = act(&state, &content, RunAction::Buff { index: 0 });
    assert_eq!(
        (buffed.deck[0].buffs, buffed.stats.cards_buffed),
        (state.deck[0].buffs + 1, 1)
    );
    let leveled = act(&state, &content, RunAction::LevelArtifact { index: 0 });
    assert_eq!(
        (leveled.artifacts[0].level, leveled.stats.artifacts_leveled),
        (state.artifacts[0].level + 1, 1)
    );
    let past_the_end = RunAction::RemoveCard {
        index: state.deck.len(),
    };
    assert!(run_reduce(&state, &content, &past_the_end).is_err());
    let mut last = state.clone();
    last.deck.truncate(1);
    assert!(run_reduce(&last, &content, &RunAction::RemoveCard { index: 0 }).is_err());
}

/// §15.8: a campfire gains `RUN_CAMPFIRE_MAX_HP` max HP, healing HP by the rise up to the new
/// max, or removes `RUN_CAMPFIRE_REMOVE` different cards, leaving at least one; §15.10 counts both.
#[test]
fn a_campfire_gains_its_max_hp_up_to_the_new_max_or_removes_its_cards_and_the_stats_count_them() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::Campfire;
    let max = run_max_hp(&state, &content);
    state.hp = 1;
    let rested = act(&state, &content, RunAction::GainMaxHp);
    assert_eq!(rested.max_hp_bonus, RUN_CAMPFIRE_MAX_HP);
    assert_eq!((rested.hp, rested.stats.healed), (1 + RUN_CAMPFIRE_MAX_HP, 0));
    state.hp = max - 1;
    let topped = act(&state, &content, RunAction::GainMaxHp);
    assert_eq!(
        (topped.hp, topped.stats.healed),
        (max - 1 + RUN_CAMPFIRE_MAX_HP, 0)
    );

    let first: Vec<usize> = (0..RUN_CAMPFIRE_REMOVE).collect();
    let removed = act(
        &state,
        &content,
        RunAction::RemoveCards {
            indices: first.clone(),
        },
    );
    assert_eq!(removed.deck.len(), state.deck.len() - RUN_CAMPFIRE_REMOVE);
    assert_eq!(
        removed.stats.cards_removed,
        i32::try_from(RUN_CAMPFIRE_REMOVE).unwrap()
    );
    let mut past_the_end = first.clone();
    *past_the_end.last_mut().unwrap() = state.deck.len();
    let refused = RunAction::RemoveCards {
        indices: past_the_end,
    };
    assert!(run_reduce(&state, &content, &refused).is_err());
    let mut small = state.clone();
    small.deck.truncate(RUN_CAMPFIRE_REMOVE);
    assert!(run_reduce(&small, &content, &RunAction::RemoveCards { indices: first }).is_err());
}

/// §15.8: an event's option moves max HP (and HP with it) and Coins and Tickets by what it says,
/// refused when it costs more than the run holds; §15.10 counts the HP lost and healed and the
/// Coins and Tickets spent and earned.
#[test]
fn an_event_option_moves_max_hp_and_pays_or_earns_exactly_what_it_says() {
    let mut content = content();
    let option = |max_hp: i32, coins: i32, tickets: i32| RunEventOption {
        text: String::new(),
        max_hp,
        coins,
        tickets,
        card: false,
        artifact: false,
    };
    let (lost, cost, fee) = (5, 30, 3);
    let (raised, earned, gained) = (7, 50, 2);
    content.events.push(RunEventDef {
        id: "audit-event".into(),
        text: String::new(),
        options: vec![option(-lost, -cost, -fee), option(raised, earned, gained)],
    });
    let mut state = start(&content);
    state.phase = RunPhase::Event {
        id: "audit-event".into(),
    };
    // HP outside a fight always equals the max, so the max-HP trade moves HP by the same amount.
    state.hp = run_max_hp(&state, &content);
    state.coins = cost;
    state.tickets = fee;

    let paid = act(&state, &content, RunAction::Choose { index: 0 });
    assert_eq!((paid.hp, paid.coins, paid.tickets), (state.hp - lost, 0, 0));
    assert_eq!(
        (
            paid.stats.damage_taken,
            paid.stats.coins_spent,
            paid.stats.tickets_spent
        ),
        (lost, cost, fee)
    );
    for short in [(cost - 1, fee), (cost, fee - 1)] {
        let mut poor = state.clone();
        (poor.coins, poor.tickets) = short;
        let refused = run_reduce(&poor, &content, &RunAction::Choose { index: 0 }).unwrap_err();
        assert!(refused.contains("not enough to pay"), "{short:?}: {refused}");
    }

    let rich = act(&state, &content, RunAction::Choose { index: 1 });
    assert_eq!(
        (rich.hp, rich.coins, rich.tickets),
        (state.hp + raised, cost + earned, fee + gained)
    );
    assert_eq!(
        (
            rich.stats.healed,
            rich.stats.coins_earned,
            rich.stats.tickets_earned
        ),
        (raised, earned, gained)
    );
}

/// §15.7: a pack keeps exactly its count, only of its own cards and each once; with a Skip relic it may
/// keep none, for the relic's Coins; §15.10 counts the cards kept and the packs skipped.
#[test]
fn a_pack_keeps_exactly_its_count_of_its_own_cards_or_none_for_a_skip_relics_coins() {
    let content = content();
    let mut state = act(&start(&content), &content, RunAction::Go { col: 0 });
    state = act(&state, &content, won(20));
    if matches!(
        state.phase,
        RunPhase::Reward {
            artifact: Some(_),
            ..
        }
    ) {
        state = act(&state, &content, RunAction::TakeArtifact { take: false });
    }
    state = act(&state, &content, RunAction::OpenPack { index: 0 });
    let RunPhase::Pack { cards, keep } = state.phase.clone() else {
        panic!("a pack is open")
    };
    assert!(keep >= 2, "keep {keep}");
    let kept = act(
        &state,
        &content,
        RunAction::Keep {
            cards: cards[..keep].to_vec(),
        },
    );
    assert_eq!(kept.deck.len(), state.deck.len() + keep);
    assert_eq!(kept.stats.cards_kept, i32::try_from(keep).unwrap());

    let mut stranger = cards[..keep].to_vec();
    stranger[0] = state.deck[0].id.clone();
    assert!(run_reduce(&state, &content, &RunAction::Keep { cards: stranger }).is_err());
    let mut twice = cards[..keep].to_vec();
    twice[1] = twice[0].clone();
    assert!(run_reduce(&state, &content, &RunAction::Keep { cards: twice }).is_err());

    let none = RunAction::Keep { cards: Vec::new() };
    assert!(run_reduce(&state, &content, &none).is_err());
    let (picky, coins) = relic_with(&content, |e| match e {
        RunEffect::Skip { coins } => Some(coins),
        _ => None,
    });
    state.artifacts.push(RunArtifact { id: picky, level: 1 });
    let skipped = act(&state, &content, none);
    assert_eq!(
        (skipped.coins, skipped.stats.packs_skipped),
        (state.coins + coins, 1)
    );
}

/// §15.7: after a mini-boss one card of the deck, or none, is extracted: never one past the deck's
/// end, never the last.
#[test]
fn extraction_takes_a_card_of_the_deck_but_never_past_its_end_or_the_last() {
    let content = content();
    let mut state = start(&content);
    state.phase = RunPhase::ExtractCard;
    let past_the_end = RunAction::Extract {
        index: Some(state.deck.len()),
    };
    assert!(run_reduce(&state, &content, &past_the_end).is_err());
    let extracted = act(&state, &content, RunAction::Extract { index: Some(0) });
    assert_eq!(extracted.extracted_cards, vec![state.deck[0].id.clone()]);
    state.deck.truncate(1);
    assert!(run_reduce(&state, &content, &RunAction::Extract { index: Some(0) }).is_err());
}

/// §15.7: a won fight pays `RUN_COINS` and `RUN_TICKETS` of its kind plus the per-fight relics'; §15.6
/// raises the max by `RUN_WIN_MAX_HP` and refills HP to it; §15.10 counts the HP lost, the report
/// and the win by its kind.
#[test]
fn a_won_fight_pays_by_its_kind_and_relics_and_counts_its_report() {
    let content = content();
    let (coin_relic, coins) = relic_with(&content, |e| match e {
        RunEffect::CoinsPerFight { n } => Some(n),
        _ => None,
    });
    let (ticket_relic, tickets) = relic_with(&content, |e| match e {
        RunEffect::TicketsPerFight { n } => Some(n),
        _ => None,
    });
    let (lost, dealt, healed, drawn, turns) = (4, 7, 3, 9, 6);
    for kind in [RunNodeKind::Elite, RunNodeKind::Ghost] {
        let mut state = start(&content);
        state.artifacts = vec![
            RunArtifact {
                id: coin_relic.clone(),
                level: 1,
            },
            RunArtifact {
                id: ticket_relic.clone(),
                level: 1,
            },
        ];
        let mut node = state.map[0][0].clone();
        node.kind = kind;
        node.artifact = None;
        state.map[0] = vec![node];
        let fighting = act(&state, &content, RunAction::Go { col: 0 });
        let report = RunAction::FightOver {
            won: true,
            hp: fighting.hp - lost,
            damage_dealt: dealt,
            healing_done: healed,
            cards_drawn: drawn,
            turns,
        };
        let after = act(&fighting, &content, report);
        // R3584000: a win raises the max by `RUN_WIN_MAX_HP` and refills HP to the new max.
        assert_eq!(after.max_hp_bonus, RUN_WIN_MAX_HP, "{kind:?}");
        assert_eq!(after.hp, run_max_hp(&after, &content), "{kind:?}");
        assert_eq!(
            (after.coins, after.tickets),
            (RUN_COINS.of(kind) + coins, RUN_TICKETS.of(kind) + tickets),
            "{kind:?}"
        );
        let s = &after.stats;
        assert_eq!(
            (
                s.damage_taken,
                s.damage_dealt,
                s.healing_done,
                s.cards_drawn,
                s.turns
            ),
            (lost, dealt, healed, drawn, turns),
            "{kind:?}"
        );
        let elite = i32::from(kind == RunNodeKind::Elite);
        assert_eq!((s.elites_won, s.ghosts_won), (elite, 1 - elite), "{kind:?}");
    }
}

/// §15.6: any other end drops HP to zero and ends the run; §15.10 counts the HP lost and the report,
/// a lost fight's included.
#[test]
fn a_lost_fight_counts_its_report_and_every_hp_left_as_lost() {
    let content = content();
    let fighting = act(&start(&content), &content, RunAction::Go { col: 0 });
    let (dealt, healed, drawn, turns) = (7, 3, 9, 6);
    let report = RunAction::FightOver {
        won: false,
        hp: 0,
        damage_dealt: dealt,
        healing_done: healed,
        cards_drawn: drawn,
        turns,
    };
    let lost = act(&fighting, &content, report);
    let s = &lost.stats;
    assert_eq!(
        (
            s.damage_taken,
            s.damage_dealt,
            s.healing_done,
            s.cards_drawn,
            s.turns
        ),
        (fighting.hp, dealt, healed, drawn, turns)
    );
    assert_eq!(lost.hp, 0);
    assert!(matches!(lost.phase, RunPhase::Dead));
}

/// §15.8: a shrine offers up to `RUN_SHRINE_OFFERS` blessings not held; §15.10 counts each node
/// visited, each event seen and each blessing taken.
#[test]
fn a_shrine_offers_only_blessings_not_held_and_the_stats_count_nodes_events_and_blessings() {
    let content = content();
    let mut state = start(&content);
    let blessings: Vec<String> = content
        .passives
        .iter()
        .filter(|p| p.kind == RunPassiveKind::Blessing)
        .map(|p| p.id.clone())
        .collect();
    state.blessings = vec![blessings[0].clone()];
    let tile = |kind: RunNodeKind, event: Option<String>| RunNode {
        kind,
        enemy: None,
        event,
        artifact: None,
    };
    state.map[0] = vec![tile(RunNodeKind::Shrine, None)];
    let shrine = act(&state, &content, RunAction::Go { col: 0 });
    let RunPhase::Shrine { offers } = &shrine.phase else {
        panic!("a shrine offers blessings")
    };
    assert_eq!(offers.len(), (blessings.len() - 1).min(RUN_SHRINE_OFFERS));
    assert!(
        offers
            .iter()
            .all(|id| blessings.contains(id) && *id != blessings[0]),
        "{offers:?}"
    );
    assert_eq!(shrine.stats.nodes_visited, 1);
    let blessed = act(&shrine, &content, RunAction::Bless { index: Some(0) });
    assert_eq!(blessed.stats.blessings_taken, 1);

    state.map[0] = vec![tile(RunNodeKind::Event, Some(content.events[0].id.clone()))];
    let event = act(&state, &content, RunAction::Go { col: 0 });
    assert_eq!(event.stats.events_seen, 1);
}

/// §15.8: a chest also holds a relic on a roll under `RUN_CHEST_ARTIFACT_PERCENT`, never on a roll at
/// it (stepping on the chest is its first roll).
#[test]
fn a_chest_holds_a_relic_on_a_roll_under_its_percent_not_at_it() {
    let content = content();
    let mut state = start(&content);
    state.map[0] = vec![RunNode {
        kind: RunNodeKind::Chest,
        enemy: None,
        event: None,
        artifact: None,
    }];
    let relics_at = |roll: i32| {
        let mut at = state.clone();
        at.rng_cursor = (0..u32::MAX)
            .find(|&cursor| Rng::new(&state.seed, cursor).int(PERCENT) == roll)
            .unwrap();
        act(&at, &content, RunAction::Go { col: 0 }).artifacts.len()
    };
    let held = state.artifacts.len();
    assert_eq!(relics_at(RUN_CHEST_ARTIFACT_PERCENT), held);
    assert_eq!(relics_at(RUN_CHEST_ARTIFACT_PERCENT - 1), held + 1);
}

/// §15.7: a pack shows `RUN_PACK_SIZE` cards plus the relics' more; the beaten enemy's pack holds only
/// its deck's cards the run lacks, and a set's, a card type's or a rarity's only theirs.
#[test]
fn a_pack_shows_its_size_plus_the_relics_and_only_cards_of_its_kind() {
    let content = content();
    let mut state = start(&content);
    state.artifacts.clear();
    let mut rng = Rng::new("pack", 0);
    assert_eq!(
        open_pack(&state, &content, &RunPackKind::Random, &mut rng).len(),
        RUN_PACK_SIZE
    );
    let (wide, more) = relic_with(&content, |e| match e {
        RunEffect::PackSize { n } => Some(n),
        _ => None,
    });
    state.artifacts = vec![RunArtifact { id: wide, level: 1 }];
    assert_eq!(
        open_pack(&state, &content, &RunPackKind::Random, &mut rng).len(),
        RUN_PACK_SIZE + usize::try_from(more).unwrap()
    );
    state.artifacts.clear();

    let def = |id: &String| content.pool.iter().find(|c| &c.id == id).unwrap();
    let sample = def(&open_pack(&state, &content, &RunPackKind::Random, &mut rng)[0]);
    for kind in [
        RunPackKind::Set { set: sample.set },
        RunPackKind::CardType {
            card_type: sample.card_type,
        },
        RunPackKind::Rarity {
            rarity: sample.rarity,
        },
    ] {
        let pack = open_pack(&state, &content, &kind, &mut rng);
        assert!(!pack.is_empty(), "{kind:?}");
        let fits = |card: &RunPoolCard| match &kind {
            RunPackKind::Set { set } => card.set == *set,
            RunPackKind::CardType { card_type } => card.card_type == *card_type,
            RunPackKind::Rarity { rarity } => card.rarity == *rarity,
            _ => unreachable!("a filtered pack"),
        };
        assert!(pack.iter().all(|id| fits(def(id))), "{kind:?}: {pack:?}");
    }

    state.row = Some(0);
    state.col = 0;
    let enemy = state.map[0][0].enemy.clone().expect("the first row is fights");
    let boss = open_pack(&state, &content, &RunPackKind::Boss, &mut rng);
    assert!(!boss.is_empty());
    assert!(
        boss.iter()
            .all(|id| enemy.deck.contains(id) && !in_deck(&state, id)),
        "{boss:?}"
    );
}
