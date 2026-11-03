//! A hero that starts the game with Armor (SPEC §4.4 step 2, §6.1, §9.9; R2922000): the handicap's
//! `heroArmor`, which a dungeon run's fight sets for its human (R2922001). The Armor is the hero's own,
//! as any it gains later: it takes its amount off every hit and is never spent. It is absent from every
//! game that does not set it above 0, which hashes as before the field existed.
//!
//! Fixtures: the vanilla catalog (`fx-1` up), and effects run through `make_context` and settled, as
//! `hero_floor.rs` runs them.

use jackioh_engine::effects::damage;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::subsystems::glitch::reset_match;
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{SettleOptions, settle};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{act, setup_catalog};

const ARMORED: PlayerId = PlayerId::P2;
const OTHER: PlayerId = PlayerId::P1;

fn handicaps(p1: Option<Handicap>, p2: Option<Handicap>) -> PerPlayerOpt<Handicap> {
    PerPlayerOpt { p1, p2 }
}

fn size(handicap: Option<Handicap>) -> i32 {
    handicap.map_or(DECK_SIZE, |h| h.deck_size)
}

fn created(seed: &str, p1: Option<Handicap>, p2: Option<Handicap>) -> GameState {
    setup_catalog();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (vanilla_deck(size(p1), 1), vanilla_deck(size(p2), 1)),
        handicaps: Some(handicaps(p1, p2)),
        ..Default::default()
    })
}

/// Both mulligans kept whole: turn 1, p1's main phase.
fn started(seed: &str, p1: Option<Handicap>, p2: Option<Handicap>) -> GameState {
    let mut state = begin_game(&created(seed, p1, p2)).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: player,
            },
        );
    }
    state
}

/// Apply effects as `controller`'s, then settle.
fn run_settled(state: &mut GameState, effects: &[Effect], controller: PlayerId) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            apply_effects(effects, &mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
}

fn hit(to: &str, amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": to }, "amount": amount })))
}

fn health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn armor(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.armor
}

fn armored(armor: i32) -> Handicap {
    Handicap {
        hero_armor: Some(armor),
        ..HUMAN_HANDICAP
    }
}

#[test]
fn r2922000_a_seat_whose_handicap_sets_hero_armor_starts_with_it_and_stores_it() {
    for tier in [
        HUMAN_HANDICAP,
        AI_DIFFICULTY.easy,
        AI_DIFFICULTY.medium,
        AI_DIFFICULTY.hard,
        AI_TUTORIAL,
        TUTORIAL_HUMAN,
    ] {
        assert_eq!(tier.hero_armor, None, "no tier or lesson sets it: {tier:?}");
    }

    let state = created("r2922000-start", None, Some(armored(3)));
    assert_eq!(armor(&state, ARMORED), 3);
    assert_eq!(armor(&state, OTHER), 0);
    assert_eq!(health(&state, ARMORED), HERO_HEALTH, "Armor is no health");
    assert_eq!(state.players[ARMORED].handicap, Some(armored(3)));
    assert_eq!(state.players[OTHER].handicap, None);
    let json = serde_json::to_value(state.players[ARMORED].handicap).expect("serializes");
    assert_eq!(json.get("heroArmor"), Some(&json!(3)), "{json}");

    let state = started("r2922000-start", None, Some(armored(3)));
    assert_eq!(
        armor(&state, ARMORED),
        3,
        "the opening deal and mulligans keep it"
    );
}

#[test]
fn r2922000_the_armor_cuts_every_hit_and_is_not_spent() {
    let mut state = started("r2922000-hit", None, Some(armored(2)));
    run_settled(&mut state, &[hit("enemyHero", 5)], OTHER);
    assert_eq!(health(&state, ARMORED), HERO_HEALTH - 3, "5 less the Armor's 2");
    assert_eq!(armor(&state, ARMORED), 2, "the Armor is not spent");
    run_settled(&mut state, &[hit("enemyHero", 2)], OTHER);
    assert_eq!(
        health(&state, ARMORED),
        HERO_HEALTH - 3,
        "a hit no bigger than the Armor deals none"
    );
    assert_eq!(armor(&state, ARMORED), 2);

    run_settled(&mut state, &[hit("enemyHero", 5)], ARMORED);
    assert_eq!(health(&state, OTHER), HERO_HEALTH - 5, "the other hero has none");
}

#[test]
fn r2922000_armor_0_or_none_stores_nothing_and_hashes_as_the_plain_game() {
    let plain = created("r2922000-plain", None, None);
    let zero = created("r2922000-plain", Some(armored(0)), Some(armored(0)));
    for player in [PlayerId::P1, PlayerId::P2] {
        assert_eq!(zero.players[player].handicap, None, "Armor 0 stores nothing");
        assert_eq!(armor(&zero, player), 0);
    }
    assert_eq!(hash_state(&zero), hash_state(&plain));
    assert_eq!(
        hash_state(&begin_game(&zero).state),
        hash_state(&begin_game(&plain).state)
    );

    let medium = Handicap {
        hero_armor: Some(0),
        ..AI_DIFFICULTY.medium
    };
    let state = created("r2922000-tier", None, Some(medium));
    assert_eq!(state.players.p2.handicap, Some(AI_DIFFICULTY.medium));
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard, AI_TUTORIAL] {
        let state = created("r2922000-tier", None, Some(tier));
        let json = serde_json::to_value(state.players.p2.handicap).expect("serializes");
        assert!(json.get("heroArmor").is_none(), "{json}");
    }
}

#[test]
fn r2922000_a_glitch_reset_starts_the_hero_with_it_again() {
    // R676: the reset rebuilds the game from the stored handicap, so the hero has its starting Armor
    // again and no Armor it gained since.
    let mut state = started("r2922000-glitch", None, Some(armored(3)));
    state.players[ARMORED].hero.armor = 7;
    {
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        reset_match(&mut sink);
    }
    assert_eq!(state.resets, Some(1));
    assert_eq!(armor(&state, ARMORED), 3);
    assert_eq!(armor(&state, OTHER), 0);
    assert_eq!(state.players[ARMORED].handicap, Some(armored(3)));
}

#[test]
fn r2922000_a_negative_hero_armor_is_refused() {
    let refused = state::validate_handicap(&armored(-1), "p2").expect_err("refused");
    let message = refused.to_string();
    assert!(message.contains("heroArmor"), "{message}");
    assert!(message.contains("R2922000"), "{message}");
    for armor in [0, 5] {
        assert!(state::validate_handicap(&armored(armor), "p2").is_ok(), "{armor}");
    }
}

/// A full game, every seat on the random policy, then folded from its inputs.
fn played_and_folded(seed: &str, p1: Option<Handicap>, p2: Option<Handicap>) -> (GameState, FoldResult) {
    let mut state = begin_game(&created(seed, p1, p2)).state;
    let mut policy = Rng::new(&format!("{seed}-policy"), 0);
    let mut log: Vec<Action> = Vec::new();
    while state.result.is_none() {
        assert!(log.len() < 5000, "{seed} ends");
        let player = seat_to_act(&state).expect("a seat to act");
        let body = subsystems::ai_policy::choose_action(&state, player, &mut policy).expect("legal");
        let action = Action::new(body, player, format!("rp{}", log.len()));
        let result = reduce(&state, &action);
        assert!(result.error.is_none(), "{:?}", result.error);
        log.push(action);
        state = result.state;
    }
    let folded = fold(&FoldArgs {
        seed: seed.to_string(),
        decks: (vanilla_deck(size(p1), 1), vanilla_deck(size(p2), 1)),
        log,
        handicaps: Some(handicaps(p1, p2)),
        ..Default::default()
    });
    (state, folded)
}

#[test]
fn r2922000_a_game_with_hero_armor_folds_back_to_its_hash() {
    let seed = "r2922000-replay";
    let (live, folded) = played_and_folded(seed, Some(armored(2)), Some(armored(1)));
    assert!(folded.errors.is_empty(), "{seed}: {:?}", folded.errors);
    assert_eq!(
        hash_state(&folded.state),
        hash_state(&live),
        "the fold reproduces the game"
    );
    assert_eq!(folded.state.players.p1.handicap, Some(armored(2)));
    assert_eq!(folded.state.players.p2.handicap, Some(armored(1)));
}
