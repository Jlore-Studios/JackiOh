//! Issue #1843's two endings (SPEC §2.5, R1843000–R1843003): a player removed from the game by a full
//! tally of votes, and a player who flees. Proved on fixture scripts (`fixtures/endings.rs`), so the
//! engine owns both before any Stinkyscape card exists.

use jackioh_engine::effects::{FleeArgs, flee, vote_out, win_game};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{SettleOptions, settle};

use crate::rules::fixtures::endings::*;
use crate::rules::fixtures::harness::round_trip;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// A library on each side, so no turn here meets fatigue.
const LIBRARY: [&str; 4] = ["fx-1", "fx-2", "fx-3", "fx-4"];

fn table(p1: Value, p2: Value) -> Scenario {
    register_endings_fixtures();
    let (mut p1, mut p2) = (p1, p2);
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!(LIBRARY);
        }
    }
    scenario(json!({ "seed": "endings", "p1": p1, "p2": p2 }))
}

fn brains() -> Value {
    json!([melchior.id, balthasar.id, casper.id])
}

/// Each run's effects as a resolving card applies them, with no check between them, then one settle:
/// the board settles and the state check's game-end point runs.
fn run(state: &mut GameState, runs: &[(Option<CardInstance>, PlayerId, Vec<Effect>)]) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        for (card, controller, effects) in runs {
            let mut ctx = make_context(
                &mut sink,
                card.as_ref(),
                HookOptions {
                    controller: Some(*controller),
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

/// The `voteCast` events against `player`: (card definition, tally after it, tally that removes).
fn cast(events: &[GameEvent], player: PlayerId) -> Vec<(String, i32, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::VoteCast {
                player: against,
                def_id,
                votes,
                needed,
                ..
            } if *against == player => Some((def_id.clone(), *votes, *needed)),
            _ => None,
        })
        .collect()
}

fn defs(ballots: &[Ballot]) -> Vec<String> {
    ballots.iter().map(|ballot| ballot.def_id.clone()).collect()
}

fn result_of(s: &Scenario) -> (Winner, GameOverReason) {
    let result = s.state().result.expect("the game is over");
    (result.winner, result.reason)
}

/// The `fled` events, as (the fleer, what it left the other player).
fn flights(events: &[GameEvent]) -> Vec<(PlayerId, FleeOpponent)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Fled { player, opponent } => Some((*player, *opponent)),
            _ => None,
        })
        .collect()
}

fn tail(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().rev().take(2).map(GameEvent::event_type).collect()
}

mod r1843000_the_vote {
    use super::*;

    #[test]
    fn r1843000_three_cards_voting_at_their_controllers_start_of_turn_remove_them() {
        // p1 has more Units (MELCHIOR), p2 is under half its starting life (BALTHASAR), CASPER always votes.
        let mut s = table(
            json!({ "field": ["fx-1", "fx-2"], "hand": ["fx-3"] }),
            json!({ "backrow": brains(), "health": 10, "hand": ["fx-4"] }),
        );
        let voters: Vec<String> = (1..=3)
            .map(|lane| s.backrow(P2, lane).expect("a Brain").id)
            .collect();
        s.end_turn();
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::VotedOut));
        let events = s.last_events().to_vec();
        assert_eq!(
            cast(&events, P2),
            vec![
                (melchior.id.clone(), 1, VOTE_TALLY_SIZE),
                (balthasar.id.clone(), 2, VOTE_TALLY_SIZE),
                (casper.id.clone(), 3, VOTE_TALLY_SIZE),
            ]
        );
        // The three agree, and then the game ends: `votedOut` just before `gameOver`.
        assert_eq!(
            tail(&events),
            vec![GameEventType::GameOver, GameEventType::VotedOut]
        );
        let GameEvent::VotedOut { player, ballots } = &events[events.len() - 2] else {
            panic!("votedOut comes just before gameOver");
        };
        assert_eq!(*player, P2);
        assert_eq!(
            defs(ballots),
            vec![melchior.id.clone(), balthasar.id.clone(), casper.id.clone()]
        );
        let ids: Vec<String> = ballots.iter().map(|ballot| ballot.instance_id.clone()).collect();
        assert_eq!(ids, voters);
        // Removed at the start-of-turn triggers, before the turn's draw (R62).
        assert_eq!(s.hand(P2).len(), 1);

        // Both seats read the agreement and the ending (CLAUDE.md rule 7).
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let shown = view.result.expect("the view shows the result");
            assert_eq!(
                (shown.winner, shown.reason),
                (Winner::P1, GameOverReason::VotedOut)
            );
            assert!(
                view.events.iter().any(|event| matches!(
                    event,
                    GameEvent::VotedOut { player, ballots } if *player == P2 && ballots.len() == 3
                )),
                "{viewer} reads the agreement"
            );
            assert_eq!(cast(&view.events, P2).len(), 3, "{viewer} reads each vote");
        }
        let result = s.state().result.expect("over");
        assert_eq!(outcome_for(&result, P1), SeatOutcome::Win);
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Loss);
    }

    #[test]
    fn r1843000_a_vote_counts_only_when_its_condition_holds_and_the_tally_is_per_turn() {
        // At full health BALTHASAR does not vote: two of three, and the game goes on.
        let mut s = table(
            json!({ "field": ["fx-1", "fx-2"], "hand": ["fx-3"] }),
            json!({ "field": ["fx-5"], "backrow": brains(), "hand": ["fx-4"] }),
        );
        s.end_turn();
        assert!(s.state().result.is_none(), "two votes remove nobody");
        assert_eq!(
            cast(s.last_events(), P2),
            vec![
                (melchior.id.clone(), 1, VOTE_TALLY_SIZE),
                (casper.id.clone(), 2, VOTE_TALLY_SIZE)
            ]
        );
        let tally = s.state().players.p2.votes.clone().expect("the votes are kept");
        assert_eq!(tally.turn, s.state().turn);
        assert_eq!(defs(&tally.ballots), vec![melchior.id.clone(), casper.id.clone()]);

        // p2's next start of turn counts from nothing again: 1 and 2, never 3 and 4.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(
            s.state().result.is_none(),
            "a tally never carries over to a later turn"
        );
        assert_eq!(
            cast(s.last_events(), P2),
            vec![
                (melchior.id.clone(), 1, VOTE_TALLY_SIZE),
                (casper.id.clone(), 2, VOTE_TALLY_SIZE)
            ]
        );
        let tally = s.state().players.p2.votes.clone().expect("the votes are kept");
        assert_eq!(tally.turn, s.state().turn);
        assert_eq!(tally.ballots.len(), 2);
        assert!(!voted_out(s.state(), P2));
    }

    #[test]
    fn r1843000_a_card_definition_votes_once_a_turn() {
        // Two MELCHIORs and a CASPER are two votes, not three: the tally counts each Brain once.
        let mut s = table(
            json!({ "field": ["fx-1", "fx-2"], "hand": ["fx-3"] }),
            json!({ "backrow": [melchior.id, melchior.id, casper.id], "hand": ["fx-4"] }),
        );
        s.end_turn();
        assert!(s.state().result.is_none());
        assert_eq!(
            cast(s.last_events(), P2),
            vec![
                (melchior.id.clone(), 1, VOTE_TALLY_SIZE),
                (casper.id.clone(), 2, VOTE_TALLY_SIZE)
            ]
        );
        // A second vote from one card in a turn adds and says nothing.
        let mut state = s.state().clone();
        let again = s.backrow(P2, 3).expect("CASPER");
        let events = run(
            &mut state,
            &[(Some(again), P2, vec![vote_out(Default::default())])],
        );
        assert!(cast(&events, P2).is_empty());
        assert_eq!(state.players.p2.votes, s.state().players.p2.votes);
    }

    #[test]
    fn r1843000_a_vote_once_the_game_is_over_counts_nothing() {
        // DOOM, left of the Brains, ends the game first at that start of turn (R68's lane order): no vote.
        let mut s = table(
            json!({ "field": ["fx-1", "fx-2"], "hand": ["fx-3"] }),
            json!({ "backrow": [doom.id, melchior.id, balthasar.id, casper.id], "health": 10, "hand": ["fx-4"] }),
        );
        s.end_turn();
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::HeroDeath));
        assert!(cast(s.last_events(), P2).is_empty());
        assert_eq!(s.state().players.p2.votes, None);

        // And the verb, run on the finished game, records and announces nothing.
        let mut state = s.state().clone();
        let brain = s.backrow(P2, 2).expect("MELCHIOR");
        let events = run(
            &mut state,
            &[(Some(brain), P2, vec![vote_out(Default::default())])],
        );
        assert!(cast(&events, P2).is_empty());
        assert_eq!(state.players.p2.votes, None);
        assert_eq!(state.result, s.state().result);
    }

    #[test]
    fn r1843000_a_face_down_voter_is_hidden_from_the_other_seat() {
        let mut s = table(
            json!({ "field": ["fx-1", "fx-2"], "hand": ["fx-3"] }),
            json!({ "field": ["fx-5"], "backrow": [masked.id, casper.id], "hand": ["fx-4"] }),
        );
        let trap = s.backrow(P2, 1).expect("the face-down voter");
        s.end_turn();
        assert!(s.state().result.is_none());
        assert_eq!(cast(s.last_events(), P2).len(), 2);
        let mine = cast(&s.view(P2).events, P2);
        assert_eq!(mine[0].0, masked.id, "its controller reads the face-down voter");
        let theirs: Vec<GameEvent> = s
            .view(P1)
            .events
            .into_iter()
            .filter(|event| event.event_type() == GameEventType::VoteCast)
            .collect();
        let GameEvent::VoteCast {
            instance_id,
            def_id,
            votes,
            ..
        } = &theirs[0]
        else {
            panic!("the other seat reads the vote");
        };
        assert_eq!(
            (instance_id.as_str(), def_id.as_str(), *votes),
            (HIDDEN_ID, HIDDEN_ID, 1)
        );
        assert_ne!(instance_id, &trap.id);
        let GameEvent::VoteCast { def_id, .. } = &theirs[1] else {
            panic!("the second vote");
        };
        assert_eq!(def_id, &casper.id, "a face-up voter reads openly");
    }

    #[test]
    fn r1843000_the_new_state_is_absent_until_used() {
        // D3: a player who never voted nor fled serializes without either field, so no shipped game's
        // state or hash moves.
        let fresh = serde_json::to_value(create_player_state()).expect("a player serializes");
        assert!(fresh.get("votes").is_none());
        assert!(fresh.get("fled").is_none());
        let s = table(json!({ "field": ["fx-1"] }), json!({ "field": ["fx-2"] }));
        let state = serde_json::to_value(s.state()).expect("a state serializes");
        for side in ["p1", "p2"] {
            assert!(state["players"][side].get("votes").is_none());
            assert!(state["players"][side].get("fled").is_none());
        }
        // Every ending older than #1843 reads the same for both seats.
        for reason in GAME_OVER_REASONS.iter().copied() {
            if LOSER_DRAWS_REASONS.contains(&reason) {
                continue;
            }
            assert_eq!(
                seat_outcome(Winner::P1, reason, P2),
                SeatOutcome::Loss,
                "{reason}"
            );
            assert_eq!(seat_outcome(Winner::P1, reason, P1), SeatOutcome::Win, "{reason}");
            assert_eq!(
                seat_outcome(Winner::Draw, reason, P1),
                SeatOutcome::Draw,
                "{reason}"
            );
        }
    }
}

mod r1843001_removed_from_the_game {
    use super::*;

    /// Every Brain on both sides votes to remove its own controller, in one effect list.
    fn all_vote(s: &Scenario) -> Vec<(Option<CardInstance>, PlayerId, Vec<Effect>)> {
        [P1, P2]
            .into_iter()
            .flat_map(|player| (1..=3).map(move |lane| (player, lane)).collect::<Vec<_>>())
            .map(|(player, lane)| {
                (
                    s.backrow(player, lane),
                    player,
                    vec![vote_out(Default::default())],
                )
            })
            .collect()
    }

    #[test]
    fn r1843001_two_players_voted_out_at_one_check_draw() {
        let s = table(json!({ "backrow": brains() }), json!({ "backrow": brains() }));
        let mut state = s.state().clone();
        let events = run(&mut state, &all_vote(&s));
        let result = state.result.expect("both tallies are full");
        assert_eq!(
            (result.winner, result.reason),
            (Winner::Draw, GameOverReason::BothVotedOut)
        );
        let removed: Vec<PlayerId> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::VotedOut { player, .. } => Some(*player),
                _ => None,
            })
            .collect();
        assert_eq!(removed, vec![P1, P2]);
        assert_eq!(
            events.last().map(GameEvent::event_type),
            Some(GameEventType::GameOver)
        );
        assert_eq!(outcome_for(&result, P1), SeatOutcome::Draw);
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Draw);
    }

    #[test]
    fn r1843001_a_removal_is_read_before_a_held_win_and_after_a_hero_at_0() {
        let s = table(json!({ "field": ["fx-1"] }), json!({ "backrow": brains() }));
        let p2_votes = |s: &Scenario| -> Vec<(Option<CardInstance>, PlayerId, Vec<Effect>)> {
            (1..=3)
                .map(|lane| (s.backrow(P2, lane), P2, vec![vote_out(Default::default())]))
                .collect()
        };

        // p2's tally fills in a list that also wins p2 the game: the removal comes first.
        let mut state = s.state().clone();
        let mut runs = p2_votes(&s);
        runs.push((None, P2, vec![win_game(json_as(json!({})))]));
        run(&mut state, &runs);
        let result = state.result.expect("removed");
        assert_eq!(
            (result.winner, result.reason),
            (Winner::P1, GameOverReason::VotedOut)
        );

        // A hero at 0 still loses first (R850): p1 at 0 loses though p2's tally is full.
        let mut state = s.state().clone();
        state.players.p1.hero.health = 0;
        let events = run(&mut state, &p2_votes(&s));
        let result = state.result.expect("a hero at 0 ends the game");
        assert_eq!(
            (result.winner, result.reason),
            (Winner::P2, GameOverReason::HeroDeath)
        );
        assert!(
            !events
                .iter()
                .any(|event| event.event_type() == GameEventType::VotedOut)
        );
    }
}

mod r1843002_fleeing {
    use super::*;

    #[test]
    fn r1843002_fleeing_wins_and_leaves_the_opponent_a_draw() {
        let mut s = table(json!({ "hand": [escape.id] }), json!({ "field": ["fx-1"] }));
        s.play(escape.id.as_str(), json!({}));
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::Fled));
        let events = s.last_events().to_vec();
        assert_eq!(flights(&events), vec![(P1, FleeOpponent::Draw)]);
        assert_eq!(tail(&events), vec![GameEventType::GameOver, GameEventType::Fled]);
        let result = s.state().result.expect("over");
        assert_eq!(outcome_for(&result, P1), SeatOutcome::Win);
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Draw);
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            assert_eq!(view.result.map(|shown| shown.reason), Some(GameOverReason::Fled));
            assert_eq!(
                flights(&view.events),
                vec![(P1, FleeOpponent::Draw)],
                "{viewer} reads the escape"
            );
        }
    }

    #[test]
    fn r1843002_fleeing_with_the_hero_at_0_loses_first() {
        // It kills its caster, then flees: the hero check runs first (R850); the held flight ends nothing.
        let mut s = table(
            json!({ "hand": [doomed_escape.id] }),
            json!({ "field": ["fx-1"] }),
        );
        s.play(doomed_escape.id.as_str(), json!({}));
        assert_eq!(result_of(&s), (Winner::P2, GameOverReason::HeroDeath));
        assert_eq!(
            s.state().players.p1.fled,
            Some(FleeOpponent::Draw),
            "the flight was held"
        );
        assert!(flights(s.last_events()).is_empty());
    }

    #[test]
    fn r1843002_the_other_hero_at_0_is_a_loss_for_them_not_a_draw() {
        // The list kills the enemy hero, then flees: the hero check comes first, so the other player loses.
        let mut s = table(json!({ "hand": [parting_shot.id] }), json!({ "field": ["fx-1"] }));
        s.play(parting_shot.id.as_str(), json!({}));
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::HeroDeath));
        assert!(flights(s.last_events()).is_empty());
        let result = s.state().result.expect("over");
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Loss);
    }

    #[test]
    fn r1843002_another_win_of_the_fleers_comes_before_the_flight() {
        let mut s = table(
            json!({ "hand": [summit_escape.id] }),
            json!({ "field": ["fx-1"] }),
        );
        s.play(summit_escape.id.as_str(), json!({}));
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::WonByEffect));
        assert!(flights(s.last_events()).is_empty());
    }

    #[test]
    fn r1843002_two_fleers_draw() {
        let mut s = table(json!({ "hand": [exodus.id] }), json!({ "field": ["fx-1"] }));
        s.play(exodus.id.as_str(), json!({}));
        assert_eq!(result_of(&s), (Winner::Draw, GameOverReason::Fled));
        assert_eq!(
            flights(s.last_events()),
            vec![(P1, FleeOpponent::Draw), (P2, FleeOpponent::Draw)]
        );
        let result = s.state().result.expect("over");
        assert_eq!(outcome_for(&result, P1), SeatOutcome::Draw);
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Draw);
    }

    #[test]
    fn r1843002_a_fleer_and_another_winner_draw() {
        let s = table(json!({}), json!({}));
        let mut state = s.state().clone();
        let events = run(
            &mut state,
            &[
                (None, P1, vec![flee(Default::default())]),
                (None, P2, vec![win_game(json_as(json!({})))]),
            ],
        );
        let result = state.result.expect("two winners");
        assert_eq!(
            (result.winner, result.reason),
            (Winner::Draw, GameOverReason::AltWin)
        );
        assert!(flights(&events).is_empty());
    }

    #[test]
    fn r1843002_a_flight_once_the_game_is_over_does_nothing() {
        let mut s = table(json!({ "hand": [parting_shot.id] }), json!({ "field": ["fx-1"] }));
        s.play(parting_shot.id.as_str(), json!({}));
        let mut state = s.state().clone();
        let events = run(
            &mut state,
            &[(
                None,
                P2,
                vec![flee(FleeArgs {
                    player: None,
                    opponent: Some(FleeOpponent::Loss),
                })],
            )],
        );
        assert_eq!(state.players.p2.fled, None);
        assert_eq!(state.result, s.state().result);
        assert!(flights(&events).is_empty());
    }
}

mod r1843003_the_radiant_choice {
    use super::*;

    /// p1 plays Edge of the Universe's Radiant face and is asked what to leave p2.
    fn asked() -> Scenario {
        let mut s = table(
            json!({ "hand": [{ "def": escape.id, "radiant": true }] }),
            json!({ "field": ["fx-1"] }),
        );
        s.play(escape.id.as_str(), json!({}));
        s
    }

    fn answers(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(state, player)
            .into_iter()
            .filter(|body| matches!(body, ActionBody::Answer { .. }))
            .collect()
    }

    #[test]
    fn r1843003_the_fleer_is_asked_and_legal_actions_and_the_view_agree() {
        let s = asked();
        let state = s.state();
        assert!(state.result.is_none(), "the flight waits for the choice");
        let pending = state.pending.as_ref().expect("a prompt is open");
        assert_eq!((pending.player_id, pending.kind), (P1, PromptKind::Mode));
        let keys: Vec<&str> = pending.options.iter().map(|option| option.key.as_str()).collect();
        assert_eq!(keys, vec!["mode:draw", "mode:loss"]);
        let offered = answers(state, P1);
        assert_eq!(offered.len(), 2);
        assert!(answers(state, P2).is_empty(), "the other seat answers nothing");
        // The reducer refuses the other seat's answer, as `legal_actions` offers it none.
        let refused = reduce(
            state,
            &Action::new(offered[0].clone(), P2, "p2-answers".to_string()),
        );
        assert!(
            refused.error.is_some(),
            "the other seat may not answer the fleer's question"
        );
        assert!(
            matches!(s.view(P2).pending, Some(PendingView::Elsewhere(_))),
            "the other seat sees only that a prompt is open"
        );
        assert!(matches!(s.view(P1).pending, Some(PendingView::ForYou(_))));
        // Paused on the question, the state survives JSON (§9.3).
        assert_eq!(&round_trip(state), state);
    }

    #[test]
    fn r1843003_the_fleer_chooses_a_loss_or_a_draw_for_the_opponent() {
        let mut s = asked();
        s.answer(json!("mode:loss"));
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::FledDecisive));
        assert_eq!(flights(s.last_events()), vec![(P1, FleeOpponent::Loss)]);
        let result = s.state().result.expect("over");
        assert_eq!(outcome_for(&result, P2), SeatOutcome::Loss);

        let mut s = asked();
        s.answer(json!("mode:draw"));
        assert_eq!(result_of(&s), (Winner::P1, GameOverReason::Fled));
        assert_eq!(flights(s.last_events()), vec![(P1, FleeOpponent::Draw)]);
    }

    #[test]
    fn r1843003_the_timeout_answers_the_choice() {
        let mut s = asked();
        s.act(ActionBody::Timeout, P1, "the fleer's clock runs out");
        let (winner, reason) = result_of(&s);
        assert_eq!(winner, Winner::P1);
        assert!(
            matches!(reason, GameOverReason::Fled | GameOverReason::FledDecisive),
            "{reason}"
        );
    }

    #[test]
    fn r1843003_the_random_policy_answers_the_choice() {
        let s = asked();
        let state = s.state().clone();
        let mut rng = Rng::new("endings-policy", 0);
        let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
        assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
        let next = reduce(&state, &Action::new(body, P1, "policy-answer".to_string()));
        assert!(next.error.is_none(), "{:?}", next.error);
        let result = next.state.result.expect("the answer ends the game");
        assert_eq!(result.winner, Winner::P1);
        assert!(matches!(
            result.reason,
            GameOverReason::Fled | GameOverReason::FledDecisive
        ));
    }
}
