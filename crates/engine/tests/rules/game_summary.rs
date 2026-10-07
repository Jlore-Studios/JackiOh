//! Port of `packages/engine/test/game-summary.test.ts`.
//!
//! SPEC §9.11, R376: `summarizeGame` reads a finished game's record off `(seed, decks, log)`. Each
//! figure is checked against an oracle the summary does not use: the hands the state holds once the
//! mulligans resolve, the hand card each logged `play` names, and the drawn cards the hand holds once
//! the step that drew them is done. The fixture Hinder is cast on draw (§2.4) and the fixture Going
//! Long is a Quickdraw card (R225), so a cast and a Quickdraw deal are both in these games, and a full
//! hand burns a draw (R4). packages/cards test/game-summary.test.ts repeats the oracles over real cards.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{new_game, play_random_game};

fn p1_deck() -> Vec<String> {
    let mut deck = vec!["fx-hinder".to_string(), "fx-going-long".to_string()];
    deck.extend(vanilla_deck(DECK_SIZE - 2, 1));
    deck
}

fn p2_deck() -> Vec<String> {
    vanilla_deck(DECK_SIZE, 21)
}

fn decks() -> (Vec<String>, Vec<String>) {
    (p1_deck(), p2_deck())
}

fn fold_args(seed: &str, decks: (Vec<String>, Vec<String>), log: Vec<Action>) -> FoldArgs {
    FoldArgs {
        seed: seed.to_string(),
        decks,
        log,
        ..Default::default()
    }
}

fn hand(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.def_id.clone())
        .collect()
}

fn sorted(ids: &[String]) -> Vec<String> {
    let mut copy = ids.to_vec();
    copy.sort();
    copy
}

/// JS `list.splice(list.lastIndexOf(card), 1)`, kept as written: a card that is not there is -1,
/// which takes the last entry.
fn splice_last_index_of(list: &mut Vec<String>, card: &str) {
    match list.iter().rposition(|each| each == card) {
        Some(at) => {
            list.remove(at);
        }
        None => {
            list.pop();
        }
    }
}

/// JS `events.slice(from)`: a negative start counts from the end.
fn slice_from(events: &[GameEvent], from: isize) -> &[GameEvent] {
    let len = events.len() as isize;
    let start = if from < 0 {
        (len + from).max(0)
    } else {
        from.min(len)
    };
    &events[start as usize..]
}

struct Step<'a> {
    events: &'a [GameEvent],
    after: &'a GameState,
}

/// The cards a step drew into each hand from event `from` on, read off the state the step left rather
/// than the order of its events: each card a `drawn` names that its seat holds once the step is done.
/// A card burned on a full hand (R4) or cast on its draw (§2.4) is in no hand, and no fixture card in
/// these decks moves a card out of a hand in the step that drew it, so that hand settles it.
fn draws_kept(step: &Step<'_>, from: isize) -> PerPlayer<Vec<String>> {
    let mut drawn: PerPlayer<Vec<String>> = PerPlayer::new(vec![], vec![]);
    for event in slice_from(step.events, from) {
        let GameEvent::Drawn {
            player,
            instance_id,
            def_id,
            ..
        } = event
        else {
            continue;
        };
        if step.after.players[*player]
            .hand
            .iter()
            .any(|card| card.id == *instance_id)
        {
            drawn[*player].push(def_id.clone());
        }
    }
    drawn
}

/// Where the draws of the game start: the first `turnStarted` (§2.1), or -1 in a step before it.
fn turn_start_in(events: &[GameEvent]) -> isize {
    events
        .iter()
        .position(|event| matches!(event, GameEvent::TurnStarted { .. }))
        .map_or(-1, |at| at as isize)
}

struct Recorded {
    #[allow(dead_code)]
    before: GameState,
    after: GameState,
    events: Vec<GameEvent>,
}

/// Steps a hand-written game, keeping the log and every step's events.
struct Stepper {
    state: GameState,
    log: Vec<Action>,
    steps: Vec<Recorded>,
}

impl Stepper {
    fn act(&mut self, player: PlayerId, body: ActionBody) {
        let action = Action::new(body, player, format!("s{}", self.log.len()));
        let result = reduce(&self.state, &action);
        if let Some(error) = &result.error {
            panic!("{} refused: {error}", action.action_type());
        }
        self.steps.push(Recorded {
            before: self.state.clone(),
            after: result.state.clone(),
            events: result.events.clone(),
        });
        self.log.push(action);
        self.state = result.state;
    }
}

fn stepper(seed: &str, decks: (Vec<String>, Vec<String>)) -> Stepper {
    let begun = begin_game(&new_game(seed, Some(decks)));
    Stepper {
        state: begun.state,
        log: Vec::new(),
        steps: Vec::new(),
    }
}

fn step_of(recorded: &Recorded) -> Step<'_> {
    Step {
        events: &recorded.events,
        after: &recorded.after,
    }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

mod summarize_game_s9_11 {
    use super::*;

    #[test]
    fn r376_records_each_seats_deck_opening_hand_draws_and_plays_and_how_the_game_ended() {
        let mut game = stepper("summary-hand-written", decks());
        let dealt = game.state.clone();
        let p1_dealt = dealt.players.p1.hand.clone();
        // R225: the Quickdraw card is always dealt. p1 sends back one other card; p2 keeps its hand.
        assert!(hand(&dealt, PlayerId::P1).contains(&"fx-going-long".to_string()));
        let returned = p1_dealt
            .iter()
            .find(|card| card.def_id != "fx-going-long")
            .expect("p1 was dealt nothing but the Quickdraw card")
            .clone();
        game.act(
            PlayerId::P1,
            ActionBody::Mulligan {
                keep: p1_dealt
                    .iter()
                    .filter(|card| card.id != returned.id)
                    .map(|card| card.id.clone())
                    .collect(),
            },
        );
        game.act(
            PlayerId::P2,
            ActionBody::Mulligan {
                keep: ids(&dealt.players.p2.hand),
            },
        );

        // The step that resolved both mulligans also began turn 1 and drew its card.
        let resolved = game.steps.get(1).expect("no resolving step");
        assert_eq!(resolved.after.turn, 1);
        let turn_one_draws = draws_kept(&step_of(resolved), turn_start_in(&resolved.events));
        let resolved_after = resolved.after.clone();

        let play = legal_actions(&game.state, PlayerId::P1)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { .. }))
            .expect("p1 has no play on turn 1");
        let ActionBody::Play {
            instance_id: played_id,
            ..
        } = &play
        else {
            unreachable!("a play")
        };
        let played_def = game
            .state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.id == *played_id)
            .map(|card| card.def_id.clone());
        game.act(PlayerId::P1, play.clone());
        game.act(PlayerId::P1, ActionBody::EndTurn);
        game.act(PlayerId::P2, ActionBody::EndTurn);
        game.act(PlayerId::P1, ActionBody::Concede);

        let summary = summarize_game(&fold_args("summary-hand-written", decks(), game.log.clone()))
            .expect("no summary of a finished game");

        assert_eq!(summary.first, PlayerId::P1);
        assert_eq!(summary.winner, Winner::P2);
        assert_eq!(summary.reason, GameOverReason::Concede);
        assert_eq!(summary.turns, game.state.turn);
        assert_eq!(summary.seats.p1.deck, p1_deck());
        assert_eq!(summary.seats.p2.deck, p2_deck());

        // The hand once the mulligans resolved: what the step left, less the first turn's draw.
        let mut p1_opening = hand(&resolved_after, PlayerId::P1);
        for card in &turn_one_draws.p1 {
            splice_last_index_of(&mut p1_opening, card);
        }
        assert_eq!(sorted(&summary.seats.p1.opening), sorted(&p1_opening));
        assert!(summary.seats.p1.opening.contains(&"fx-going-long".to_string()));
        assert!(!summary.seats.p1.opening.contains(&returned.def_id));
        assert_eq!(summary.seats.p1.opening.len(), hand(&dealt, PlayerId::P1).len());
        assert_eq!(summary.seats.p2.opening, hand(&dealt, PlayerId::P2));

        assert_eq!(
            summary
                .seats
                .p1
                .played
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<_>>(),
            vec![played_def]
        );
        assert_eq!(summary.seats.p2.played, Vec::<String>::new());
        // Turn 1's draw, then p2's on turn 2 and p1's on turn 3, each drawn by the endTurn before it.
        let (Some(end_one), Some(end_two)) = (game.steps.get(3), game.steps.get(4)) else {
            panic!("the turns did not end")
        };
        let mut p1_drawn = turn_one_draws.p1.clone();
        p1_drawn.extend(draws_kept(&step_of(end_two), 0).p1);
        assert_eq!(summary.seats.p1.drawn, p1_drawn);
        assert_eq!(summary.seats.p2.drawn, draws_kept(&step_of(end_one), 0).p2);
        assert_eq!(summary.seats.p1.drawn.len(), 2);
        assert_eq!(summary.seats.p2.drawn.len(), 1);
    }

    #[test]
    fn r376_counts_a_play_from_hand_and_never_a_cast_against_the_hand_card_each_play_names() {
        let mut casts = 0;
        let mut checked = 0;
        for n in 1..=20 {
            let seed = format!("summary-random-{n}");
            let live = play_random_game(&seed, Some(decks()));
            let summary = summarize_game(&fold_args(&seed, live.decks.clone(), live.log.clone()))
                .unwrap_or_else(|| panic!("{seed} has no summary"));

            // The oracles: the hand card each logged `play` named, read off the state it was played from,
            // and each step's draws that its seat's hand holds once the step is done.
            let mut state = begin_game(&new_game(&seed, Some(live.decks.clone()))).state;
            let mut plays: PerPlayer<Vec<String>> = PerPlayer::new(vec![], vec![]);
            let mut drawn: PerPlayer<Vec<String>> = PerPlayer::new(vec![], vec![]);
            let mut opening: Option<PerPlayer<Vec<String>>> = None;
            let mut begun = false;
            for action in &live.log {
                if let ActionBody::Play { instance_id, .. } = &action.body {
                    let card = state.players[action.player_id]
                        .hand
                        .iter()
                        .find(|instance| instance.id == *instance_id);
                    if let Some(card) = card {
                        plays[action.player_id].push(card.def_id.clone());
                    }
                }
                let result = reduce(&state, action);
                casts += result
                    .events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == "fx-hinder"))
                    .count();
                // The draws of the game start at the first `turnStarted`, in the step that resolves the
                // mulligans. When that step leaves p1 in its first turn, p2's hand is its opening hand, and
                // p1's is its opening hand plus turn 1's draws.
                let from = if begun { 0 } else { turn_start_in(&result.events) };
                if from >= 0 {
                    let kept = draws_kept(
                        &Step {
                            events: &result.events,
                            after: &result.state,
                        },
                        from,
                    );
                    if !begun
                        && state.phase == Phase::Mulligan
                        && result.state.phase != Phase::Mulligan
                        && result.state.turn == 1
                    {
                        let mut p1 = hand(&result.state, PlayerId::P1);
                        for card in &kept.p1 {
                            splice_last_index_of(&mut p1, card);
                        }
                        opening = Some(PerPlayer::new(p1, hand(&result.state, PlayerId::P2)));
                    }
                    begun = true;
                    drawn.p1.extend(kept.p1.iter().cloned());
                    drawn.p2.extend(kept.p2.iter().cloned());
                }
                state = result.state;
            }

            assert_eq!(summary.seats.p1.played, plays.p1, "{seed}");
            assert_eq!(summary.seats.p2.played, plays.p2, "{seed}");
            assert_eq!(summary.seats.p1.drawn, drawn.p1, "{seed}");
            assert_eq!(summary.seats.p2.drawn, drawn.p2, "{seed}");
            assert_eq!(
                Some(summary.winner),
                live.state.result.as_ref().map(|result| result.winner),
                "{seed}"
            );
            if let Some(opening) = &opening {
                checked += 1;
                assert_eq!(sorted(&summary.seats.p1.opening), sorted(&opening.p1), "{seed}");
                assert_eq!(sorted(&summary.seats.p2.opening), sorted(&opening.p2), "{seed}");
            }
            // Every card drawn into a hand came out of that seat's library: its own deck.
            for player in PLAYER_IDS {
                let deck: IndexSet<String> = if player == PlayerId::P1 {
                    live.decks.0.iter().cloned().collect()
                } else {
                    live.decks.1.iter().cloned().collect()
                };
                for card in &summary.seats[player].drawn {
                    assert!(deck.contains(card), "{seed} {player} {card}");
                }
            }
        }
        // The property was exercised: Hinder was cast in some game, and most games opened on turn 1.
        assert!(casts > 0);
        assert!(checked > 10);
    }

    #[test]
    fn r376_leaves_out_of_a_seats_draws_a_card_burned_on_a_full_hand_or_cast_on_its_draw() {
        // On this seed the deal leaves Hinder in p1's library, so a later draw casts it (§2.4), and both
        // seats keep their hands and only end their turns, so both hands fill to HAND_CAP and every draw
        // past it burns (R4) until the game ends.
        let seed = "summary-burn-and-cast-1";
        let mut game = stepper(seed, decks());
        assert!(
            game.state
                .players
                .p1
                .library
                .iter()
                .any(|card| card.def_id == "fx-hinder")
        );
        let keep = ids(&game.state.players.p1.hand);
        game.act(PlayerId::P1, ActionBody::Mulligan { keep });
        let keep = ids(&game.state.players.p2.hand);
        game.act(PlayerId::P2, ActionBody::Mulligan { keep });
        let mut turn = 0;
        while game.state.result.is_none() {
            assert!(turn <= 2 * TURN_CAP_PLAYER_TURNS, "the game did not end");
            turn += 1;
            let active = game.state.active;
            game.act(active, ActionBody::EndTurn);
        }

        let summary = summarize_game(&fold_args(seed, decks(), game.log.clone()))
            .expect("no summary of a finished game");

        let mut kept: PerPlayer<Vec<String>> = PerPlayer::new(vec![], vec![]);
        let mut draws: PerPlayer<usize> = PerPlayer::new(0, 0);
        let mut burned: PerPlayer<usize> = PerPlayer::new(0, 0);
        let mut casts = 0;
        let mut begun = false;
        for recorded in &game.steps {
            let from = if begun { 0 } else { turn_start_in(&recorded.events) };
            if from < 0 {
                continue;
            }
            begun = true;
            let events = slice_from(&recorded.events, from);
            let kept_here = draws_kept(&step_of(recorded), from);
            kept.p1.extend(kept_here.p1.iter().cloned());
            kept.p2.extend(kept_here.p2.iter().cloned());
            for event in events {
                match event {
                    GameEvent::Drawn { player, .. } => draws[*player] += 1,
                    GameEvent::Burned { owner, .. } => burned[*owner] += 1,
                    GameEvent::CardPlayed { .. } => casts += 1,
                    _ => {}
                }
            }
        }

        // Both kinds of lost draw happened: a Hinder cast on its draw, and a draw burned in each seat.
        assert!(casts > 0);
        assert!(burned.p1 > 0);
        assert!(burned.p2 > 0);
        // Nothing was played from a hand, and only what reached a hand is a draw.
        assert_eq!(summary.seats.p1.played, Vec::<String>::new());
        assert_eq!(summary.seats.p2.played, Vec::<String>::new());
        assert_eq!(summary.seats.p1.drawn, kept.p1);
        assert_eq!(summary.seats.p2.drawn, kept.p2);
        assert_eq!(summary.seats.p1.drawn.len(), draws.p1 - burned.p1 - casts);
        assert_eq!(summary.seats.p2.drawn.len(), draws.p2 - burned.p2);
        assert!(!summary.seats.p1.drawn.contains(&"fx-hinder".to_string()));
    }

    #[test]
    fn r376_makes_no_summary_of_a_game_without_a_result() {
        let mut game = stepper("summary-unfinished", decks());
        game.act(PlayerId::P1, ActionBody::Mulligan { keep: vec![] });
        assert!(summarize_game(&fold_args("summary-unfinished", decks(), game.log.clone())).is_none());
    }

    #[test]
    fn r376_leaves_the_opening_hands_empty_when_the_game_ended_before_a_turn_began() {
        let mut game = stepper("summary-early-concede", decks());
        game.act(PlayerId::P2, ActionBody::Concede);
        let summary = summarize_game(&fold_args("summary-early-concede", decks(), game.log.clone()));
        assert_eq!(
            serde_json::to_value(&summary).expect("a summary serialises"),
            json!({
                "first": "p1",
                "winner": "p1",
                "reason": "concede",
                "turns": 0,
                "seats": {
                    "p1": { "deck": p1_deck(), "opening": [], "drawn": [], "played": [] },
                    "p2": { "deck": p2_deck(), "opening": [], "drawn": [], "played": [] },
                },
            })
        );
    }

    #[test]
    fn skips_a_logged_action_the_engine_refuses_as_the_fold_does() {
        let live = play_random_game("summary-refused", Some(decks()));
        let refused = Action::new(ActionBody::EndTurn, PlayerId::P2, "refused-0");
        let mut log = vec![refused];
        log.extend(live.log.iter().cloned());
        assert_eq!(
            fold(&fold_args("summary-refused", live.decks.clone(), log.clone()))
                .errors
                .len(),
            1
        );
        assert_eq!(
            summarize_game(&fold_args("summary-refused", live.decks.clone(), log)),
            summarize_game(&fold_args(
                "summary-refused",
                live.decks.clone(),
                live.log.clone()
            ))
        );
    }
}
