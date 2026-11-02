//! A seat that cannot lose (SPEC §2, §9.10; R2201000): the handicap's `heroFloor`, which a lesson's
//! human plays under (`TUTORIAL_HUMAN`). The floor holds against every way to lose (a hit is capped at
//! what the hero has above its floor), never stops the floored seat from winning, and is absent from
//! every game that does not set it.
//!
//! Fixtures: the vanilla catalog (`fx-1` up), and effects run through `make_context` and settled, as
//! `alt_wins.rs` runs them, so the state check's game-end point runs after each.

use jackioh_engine::effects::{alt_win, damage, draw, flee, lose_health, set_health, swap_health, win_game};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::state::VoteTally;
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{SettleOptions, settle};
use jackioh_engine::wire::Ballot;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{act, setup_catalog};

const FLOORED: PlayerId = PlayerId::P2;
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

/// A lesson's game: p2 is the floored human, p1 an unhandicapped opponent.
fn floored(seed: &str) -> GameState {
    started(seed, None, Some(TUTORIAL_HUMAN))
}

/// Apply effects as `controller`'s, then settle: the board settles and the game-end point runs.
fn run_settled(state: &mut GameState, effects: &[Effect], controller: PlayerId) -> Vec<GameEvent> {
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
    events
}

fn hit(to: &str, amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": to }, "amount": amount })))
}

fn health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

mod r2201000_a_seat_that_cannot_lose {
    use super::*;

    #[test]
    fn r2201000_tutorial_human_sets_the_floor_and_a_floored_seat_stores_it() {
        assert_eq!(TUTORIAL_HUMAN.hero_floor, Some(TUTORIAL_HERO_FLOOR));
        assert_eq!(TUTORIAL_HERO_FLOOR, 1);
        assert_eq!(AI_TUTORIAL.hero_floor, None, "the tutorial opponent can lose");
        for handicap in [HUMAN_HANDICAP, AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
            assert_eq!(handicap.hero_floor, None, "no tier sets a floor");
        }
        let state = floored("r2201000-stored");
        assert_eq!(state.players[FLOORED].handicap, Some(TUTORIAL_HUMAN));
        assert!(!state::can_lose(&state, FLOORED));
        assert!(state::can_lose(&state, OTHER));
        let json = serde_json::to_value(state.players[FLOORED].handicap).expect("serializes");
        assert_eq!(json["heroFloor"], json!(1));
    }

    #[test]
    fn r2201000_a_floored_hero_at_1_takes_100_damage_fatigue_life_loss_and_set_health_and_stays_at_1_with_the_game_going_on()
     {
        let mut state = floored("r2201000-every-loss");
        state.players[FLOORED].hero.health = 1;

        let events = run_settled(&mut state, &[hit("enemyHero", 100)], OTHER);
        assert_eq!(health(&state, FLOORED), 1, "100 damage");
        assert!(state.result.is_none(), "the game goes on after 100 damage");
        let landed: Vec<i32> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id, amount, ..
                } if target_id == "hero-p2" => Some(*amount),
                _ => None,
            })
            .collect();
        assert!(
            landed.iter().all(|amount| *amount == 0),
            "the hit is capped to 0: {landed:?}"
        );

        // Fatigue is an ordinary hit (R3): an empty library, five draws.
        state.players[FLOORED].library.clear();
        run_settled(
            &mut state,
            &[draw(json_as(json!({ "count": 5, "player": "enemy" })))],
            OTHER,
        );
        assert!(state.players[FLOORED].fatigue_count >= 5, "every draw fatigued");
        assert_eq!(health(&state, FLOORED), 1, "fatigue");
        assert!(state.result.is_none(), "the game goes on after fatigue");

        run_settled(
            &mut state,
            &[lose_health(json_as(json!({ "player": "enemy", "amount": 100 })))],
            OTHER,
        );
        assert_eq!(health(&state, FLOORED), 1, "life loss");
        assert!(state.result.is_none(), "the game goes on after life loss");

        run_settled(
            &mut state,
            &[set_health(json_as(
                json!({ "to": { "of": "enemyHero" }, "value": 0 }),
            ))],
            OTHER,
        );
        assert_eq!(health(&state, FLOORED), 1, "set health to 0");
        assert!(state.result.is_none(), "the game goes on after set health");

        // R73's swap with a hero at 0 or less: the floored side still reads its floor.
        state.players[OTHER].hero.health = -3;
        run_settled(&mut state, &[swap_health()], OTHER);
        assert_eq!(health(&state, FLOORED), 1, "the swap");
    }

    #[test]
    fn r2201000_a_floored_hero_above_its_floor_takes_what_it_has_above_it_and_the_hit_reports_what_landed() {
        let mut state = floored("r2201000-capped");
        state.players[FLOORED].hero.health = 5;
        let events = run_settled(&mut state, &[hit("enemyHero", 10)], OTHER);
        assert_eq!(health(&state, FLOORED), 1);
        assert!(
            events.iter().any(|event| matches!(
                event,
                GameEvent::Damage { target_id, amount: 4, .. } if target_id == "hero-p2"
            )),
            "the hit deals the 4 above the floor: {events:?}"
        );
        // A hit that does not reach the floor is untouched.
        state.players[FLOORED].hero.health = 20;
        run_settled(&mut state, &[hit("enemyHero", 3)], OTHER);
        assert_eq!(health(&state, FLOORED), 17);
    }

    #[test]
    fn r2201000_no_win_of_the_opponents_ends_the_game_against_a_floored_seat() {
        let mut state = floored("r2201000-win-by-effect");
        run_settled(&mut state, &[win_game(json_as(json!({})))], OTHER);
        assert!(state.result.is_none(), "an opponent's win by effect is not read");

        let mut state = floored("r2201000-alt-win");
        run_settled(
            &mut state,
            &[alt_win(json_as(json!({ "condition": "health", "threshold": 1 })))],
            OTHER,
        );
        assert!(state.result.is_none(), "an opponent's held win is not read");

        let mut state = floored("r2201000-vote");
        state.players[FLOORED].votes = Some(VoteTally {
            turn: state.turn,
            ballots: (0..VOTE_TALLY_SIZE)
                .map(|n| Ballot {
                    instance_id: format!("ballot-{n}"),
                    def_id: format!("fx-{}", n + 1),
                })
                .collect(),
        });
        run_settled(&mut state, &[], OTHER);
        assert!(state.result.is_none(), "a floored seat is never voted out");

        // A flight that would leave it a loss leaves it a draw.
        let mut state = floored("r2201000-flee");
        run_settled(&mut state, &[flee(json_as(json!({ "opponent": "loss" })))], OTHER);
        let result = state.result.expect("the opponent may still leave");
        assert_eq!(result.reason, GameOverReason::Fled);
        assert_eq!(
            game_over::seat_outcome(result.winner, result.reason, FLOORED),
            game_over::SeatOutcome::Draw,
            "the floored seat draws, never loses"
        );
    }

    #[test]
    fn r2201000_the_floored_seat_still_wins_and_its_opponent_still_loses() {
        let mut state = floored("r2201000-wins-by-damage");
        run_settled(&mut state, &[hit("enemyHero", 100)], FLOORED);
        let result = state.result.expect("the floored seat's lethal ends the game");
        assert_eq!(result.winner, Winner::from(FLOORED));
        assert_eq!(result.reason, GameOverReason::HeroDeath);

        let mut state = floored("r2201000-wins-by-effect");
        run_settled(&mut state, &[win_game(json_as(json!({})))], FLOORED);
        let result = state.result.expect("the floored seat's own win holds");
        assert_eq!(result.winner, Winner::from(FLOORED));
        assert_eq!(result.reason, GameOverReason::WonByEffect);

        // Both claim: the floored seat's win stands alone, the opponent's is not read.
        let mut state = floored("r2201000-both-win");
        state.players[OTHER].won_by_effect = Some(true);
        run_settled(&mut state, &[win_game(json_as(json!({})))], FLOORED);
        let result = state.result.expect("one winner");
        assert_eq!(result.winner, Winner::from(FLOORED), "not a draw");
    }

    #[test]
    fn r2201000_a_floor_below_1_or_above_the_heros_starting_health_is_refused() {
        for floor in [0, -1, HERO_HEALTH + 1] {
            let handicap = Handicap {
                hero_floor: Some(floor),
                ..HUMAN_HANDICAP
            };
            assert!(
                state::validate_handicap(&handicap, "p2").is_err(),
                "floor {floor}"
            );
        }
        for floor in [1, HERO_HEALTH] {
            let handicap = Handicap {
                hero_floor: Some(floor),
                ..HUMAN_HANDICAP
            };
            assert!(state::validate_handicap(&handicap, "p2").is_ok(), "floor {floor}");
        }
        let low = Handicap {
            hero_health: Some(5),
            hero_floor: Some(6),
            ..HUMAN_HANDICAP
        };
        assert!(
            state::validate_handicap(&low, "p2").is_err(),
            "above a heroHealth of 5"
        );
        assert!(state::validate_handicap(&TUTORIAL_HUMAN, "p2").is_ok());
    }

    #[test]
    fn r2201000_a_human_who_only_ends_turns_beats_the_tutorial_opponent_by_its_fatigue_before_the_turn_cap() {
        // p1 is the tutorial opponent (12 cards, 3 mana, 20 health) on the random policy (§10.7);
        // p2 the floored human, who does nothing but end its turns.
        let mut state = started("r2201000-fatigue-win", Some(AI_TUTORIAL), Some(TUTORIAL_HUMAN));
        let mut policy = Rng::new("r2201000-fatigue-policy", 0);
        let mut steps = 0;
        while state.result.is_none() {
            steps += 1;
            assert!(steps < 5000, "the game ends");
            let player = seat_to_act(&state).expect("a seat to act");
            let body = if player == FLOORED && state.pending.is_none() {
                ActionBody::EndTurn
            } else {
                subsystems::ai_policy::choose_action(&state, player, &mut policy).expect("a legal action")
            };
            let result = reduce(&state, &Action::new(body, player, format!("fw{steps}")));
            assert!(result.error.is_none(), "{:?}", result.error);
            state = result.state;
            assert!(health(&state, FLOORED) >= 1, "the floor holds every step");
        }
        let result = state.result.expect("over");
        assert_eq!(result.winner, Winner::from(FLOORED), "the human wins: {result:?}");
        assert_eq!(result.reason, GameOverReason::HeroDeath);
        assert!(
            state.turn < TURN_CAP_PLAYER_TURNS,
            "before the turn cap: turn {}",
            state.turn
        );
        assert!(
            state.players[OTHER].fatigue_count > 0,
            "by the opponent's fatigue"
        );
    }
}

mod r2201000_r2201001_hashes_and_replays {
    use super::*;

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
    fn r2201000_r2201001_the_floor_and_the_hold_replay_alike() {
        let held = Handicap {
            pool_hold: Some(true),
            ..TUTORIAL_HUMAN
        };
        for (n, p2) in [TUTORIAL_HUMAN, held].into_iter().enumerate() {
            let seed = format!("r2201000-replay-{n}");
            let (live, folded) = played_and_folded(&seed, Some(AI_TUTORIAL), Some(p2));
            assert!(folded.errors.is_empty(), "{seed}: {:?}", folded.errors);
            assert_eq!(
                hash_state(&folded.state),
                hash_state(&live),
                "{seed}: the fold reproduces the game"
            );
            assert_eq!(
                folded.state.players.p2.handicap,
                Some(p2),
                "{seed}: the fold stores the handicap"
            );
        }
    }

    #[test]
    fn r2201000_r2201001_neither_field_reaches_a_game_that_does_not_set_it() {
        let plain = created("r2201000-plain", None, None);
        let unset = Handicap {
            hero_floor: None,
            pool_hold: Some(false),
            ..HUMAN_HANDICAP
        };
        let with_unset = created("r2201000-plain", Some(unset), Some(unset));
        for player in [PlayerId::P1, PlayerId::P2] {
            assert!(
                with_unset.players[player].handicap.is_none(),
                "a hold of false stores nothing"
            );
        }
        assert_eq!(hash_state(&with_unset), hash_state(&plain));
        assert_eq!(
            hash_state(&begin_game(&with_unset).state),
            hash_state(&begin_game(&plain).state)
        );

        // The tiers store neither key.
        for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard, AI_TUTORIAL] {
            let state = created("r2201000-tier", None, Some(tier));
            let json = serde_json::to_value(state.players.p2.handicap).expect("serializes");
            assert!(json.get("heroFloor").is_none(), "{json}");
            assert!(json.get("poolHold").is_none(), "{json}");
        }
        assert!(!state::holds_pools(&plain));
    }
}
