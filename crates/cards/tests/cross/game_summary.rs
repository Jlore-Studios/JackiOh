//! SPEC §9.11, R376: the engine's `summarize_game` over real cards. The engine's
//! tests/rules/game_summary.rs proves it with fixtures; this file plays real games with SPEC §10.7's
//! policy, as the fuzz gate does, and checks the record against oracles it does not use:
//!
//!  - the opening hands are the hands the state holds once the mulligans resolve (p1's less its
//!    first turn's draw), and The Coin is in the second seat's;
//!  - the plays are the hand cards the logged `play` actions named, less those countered in their
//!    announce window (§10.5 step 3a, R448), which were never played, and each as the card it resolves
//!    as when step 3 replaced it (C #23 Devil's Pact, R449): these games counter and replace some. #96
//!    My Pawn plays out a turn for its owner's opponent inside the action of the attack that sprang it,
//!    with no `play` in the log, so a game in which it fired may hold more plays than the log names,
//!    never fewer and never out of order;
//!  - so a cast (#21 Hinder or #90.1 CN-Virus cast on its draw, §2.4), which emits `cardPlayed` with
//!    no `play` behind it, is never counted as one: these games cast some, and the plays still match;
//!  - the draws are every card drawn from the first turn on, less those burned on a full hand (R4) and
//!    those cast on their draw, which never reached the hand: these games burn and cast some.
//!
//! Port of `packages/cards/test/game-summary.test.ts` (part 5).

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::testkit::*;

const GAMES: i32 = 40;
const MAX_ACTIONS: usize = 3000;
const MY_PAWN: &str = "core-096";

/// Every catalog id that is not a Token, sorted.
fn pool() -> Vec<String> {
    let mut ids: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .collect();
    ids.sort();
    ids
}

fn hand(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.def_id.clone())
        .collect()
}

fn sorted(ids: &[String]) -> Vec<String> {
    let mut out = ids.to_vec();
    out.sort();
    out
}

/// A `drawn` event, as the cards a step drew wait for their fate (TS `Extract<GameEvent, { type: "drawn" }>`).
struct Drawn {
    player: PlayerId,
    instance_id: String,
    def_id: String,
}

/// What `draws_kept` read off one step.
struct DrawsKept {
    kept: PerPlayer<Vec<String>>,
    burns: i32,
    casts: i32,
}

/// The instance id of the events that decide a drawn card's fate (TS `"instanceId" in event`).
fn fate_instance_id(event: &GameEvent) -> Option<&str> {
    match event {
        GameEvent::AddedToHand { instance_id, .. }
        | GameEvent::Burned { instance_id, .. }
        | GameEvent::CardPlayed { instance_id, .. }
        | GameEvent::Countered { instance_id, .. }
        | GameEvent::Transformed { instance_id, .. } => Some(instance_id.as_str()),
        _ => None,
    }
}

/// The cards a step drew into each hand from event `from` on, by what became of each card a `drawn`
/// names, whatever came between: it entered the hand (an `addedToHand` names it), burned on a full
/// hand (a `burned` does, R4) or was cast on its draw (§2.4: its `cardPlayed`, or the `countered` or
/// `transformed` that took its place, R448, R449). Only the first reached the hand. A cast's choices
/// are asked before its announce, so a draw whose fate a prompt holds waits in `waiting` for a later
/// step's events.
fn draws_kept(events: &[GameEvent], from: usize, waiting: &mut Vec<Drawn>) -> DrawsKept {
    let mut kept: PerPlayer<Vec<String>> = PerPlayer::new(Vec::new(), Vec::new());
    let mut burns = 0;
    let mut casts = 0;
    for event in events.iter().skip(from) {
        if let GameEvent::Drawn {
            player,
            instance_id,
            def_id,
            ..
        } = event
        {
            waiting.push(Drawn {
                player: *player,
                instance_id: instance_id.clone(),
                def_id: def_id.clone(),
            });
            continue;
        }
        let Some(instance_id) = fate_instance_id(event) else {
            continue;
        };
        let Some(at) = waiting.iter().position(|drawn| drawn.instance_id == instance_id) else {
            continue;
        };
        let drawn = waiting.remove(at);
        match event {
            GameEvent::AddedToHand { .. } => kept[drawn.player].push(drawn.def_id),
            GameEvent::Burned { .. } => burns += 1,
            _ => casts += 1,
        }
    }
    DrawsKept { kept, burns, casts }
}

/// Whether `part` is `whole` with some entries left out, in order.
fn subsequence(part: &[String], whole: &[String]) -> bool {
    let mut at = 0;
    for id in whole {
        if at < part.len() && part[at] == *id {
            at += 1;
        }
    }
    at == part.len()
}

/// A play is open while its card waits in the resolving zone (§10.5), which a prompt can hold past
/// its action (R449's replacement's choices, C #4 Palantir's announce window). Before its announce,
/// a `transformed` naming it is step 3's replacement (R449), the card the play resolves as; after
/// it, a `countered` naming it takes the play back.
struct Play {
    id: String,
    def_id: String,
    open: bool,
    announced: bool,
}

/// The first open play, p1's then p2's, whose card is `instance_id`: its seat and its index.
fn open_play(plays: &PerPlayer<Vec<Play>>, instance_id: &str) -> Option<(PlayerId, usize)> {
    PLAYER_IDS.into_iter().find_map(|seat| {
        plays[seat]
            .iter()
            .position(|play| play.open && play.id == instance_id)
            .map(|at| (seat, at))
    })
}

mod summarize_game_over_real_cards_s9_11 {
    use super::*;

    // TS ran this with `{ timeout: 120_000 }`; cargo test has no per-test timeout.
    #[test]
    fn r376_reads_opening_hands_plays_and_casts_off_real_games_as_the_state_and_the_log_show_them() {
        register_all();
        let pool = pool();
        let size = DECK_SIZE as usize;
        let mut pawn_games = 0;
        let mut casts = 0;
        let mut burns = 0;
        let mut draw_casts = 0;
        let mut countered_plays = 0;
        let mut replaced_plays = 0;

        for n in 1..=GAMES {
            let seed = format!("summary-cards-{n}");
            let shuffled = create_rng(&format!("summary-cards-decks-{n}"), 0).shuffle(&pool);
            let decks: (Vec<String>, Vec<String>) =
                (shuffled[..size].to_vec(), shuffled[size..size * 2].to_vec());
            let mut policy = create_rng(&format!("summary-cards-policy-{n}"), 0);

            let mut state = begin_game(&create_game(&CreateGameOptions {
                seed: seed.clone(),
                decks: decks.clone(),
                ..CreateGameOptions::default()
            }))
            .state;
            let mut log: Vec<Action> = Vec::new();
            let mut plays: PerPlayer<Vec<Play>> = PerPlayer::new(Vec::new(), Vec::new());
            let mut drawn: PerPlayer<Vec<String>> = PerPlayer::new(Vec::new(), Vec::new());
            let mut opening: Option<PerPlayer<Vec<String>>> = None;
            let mut begun = false;
            let mut pawn = false;
            let mut cast_ids: Vec<String> = Vec::new();
            let mut unplaced: IndexSet<String> = IndexSet::new();
            let mut waiting: Vec<Drawn> = Vec::new();

            while state.result.is_none() && log.len() < MAX_ACTIONS {
                let player = seat_to_act(&state).unwrap_or_else(|| panic!("{seed}: no seat to act"));
                let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
                    panic!("{seed}: no action for {}", player.as_str());
                };
                let action = Action::new(chosen, player, format!("g{}", log.len()));
                if let ActionBody::Play { instance_id, .. } = &action.body
                    && let Some(card) = state.players[player]
                        .hand
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                {
                    plays[player].push(Play {
                        id: card.id.clone(),
                        def_id: card.def_id.clone(),
                        open: true,
                        announced: false,
                    });
                }
                let result = reduce(&state, &action);
                if let Some(error) = &result.error {
                    panic!("{seed}: {} refused: {error}", action.action_type());
                }
                let events: &[GameEvent] = &result.events;
                for event in events {
                    match event {
                        GameEvent::CardAnnounced { instance_id, .. } => {
                            if let Some((seat, at)) = open_play(&plays, instance_id) {
                                plays[seat][at].announced = true;
                            }
                        }
                        GameEvent::Transformed {
                            instance_id,
                            new_instance_id,
                            to_def_id,
                            ..
                        } => {
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                && !plays[seat][at].announced
                            {
                                plays[seat][at].id = new_instance_id.clone();
                                plays[seat][at].def_id = to_def_id.clone();
                                replaced_plays += 1;
                            }
                        }
                        GameEvent::Countered {
                            instance_id,
                            player: countered,
                            ..
                        } => {
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                && plays[seat][at].announced
                            {
                                // TS: `plays[event.player].splice(plays[event.player].indexOf(play), 1)`,
                                // which takes the last entry when the play is the other seat's.
                                let list = &mut plays[*countered];
                                if seat == *countered {
                                    list.remove(at);
                                } else {
                                    list.pop();
                                }
                                countered_plays += 1;
                            }
                        }
                        _ => {}
                    }
                }
                for seat in PLAYER_IDS {
                    let resolving: IndexSet<&str> = result.state.players[seat]
                        .resolving
                        .iter()
                        .map(|card| card.id.as_str())
                        .collect();
                    for play in plays[seat].iter_mut() {
                        play.open = play.open && resolving.contains(play.id.as_str());
                    }
                }
                if events
                    .iter()
                    .any(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == MY_PAWN))
                {
                    pawn = true;
                }
                // A cast on its draw is a `cardPlayed` naming a drawn card before any hand took it: its announce
                // comes between (R448), and its choices may be asked first, in a prompt of their own (R70).
                for event in events {
                    match event {
                        GameEvent::Drawn { instance_id, .. } => {
                            unplaced.insert(instance_id.clone());
                        }
                        GameEvent::AddedToHand { instance_id, .. } | GameEvent::Burned { instance_id, .. } => {
                            unplaced.shift_remove(instance_id);
                        }
                        GameEvent::CardPlayed {
                            instance_id, def_id, ..
                        } => {
                            if unplaced.shift_remove(instance_id) {
                                cast_ids.push(def_id.clone());
                            }
                        }
                        _ => {}
                    }
                }
                // The draws of the game start at the first `turnStarted`, in the step that resolves the
                // mulligans. When that step leaves p1 in its first turn, p2's hand is its opening hand, and
                // p1's is its opening hand plus turn 1's draws.
                let from = if begun {
                    Some(0)
                } else {
                    events
                        .iter()
                        .position(|event| matches!(event, GameEvent::TurnStarted { .. }))
                };
                if let Some(from) = from {
                    let step = draws_kept(events, from, &mut waiting);
                    if !begun
                        && state.phase == Phase::Mulligan
                        && result.state.phase != Phase::Mulligan
                        && result.state.turn == 1
                    {
                        let mut p1 = hand(&result.state, PlayerId::P1);
                        for card in &step.kept.p1 {
                            // TS: `p1.splice(p1.lastIndexOf(card), 1)`; -1 takes the last entry.
                            let at = p1
                                .iter()
                                .rposition(|id| id == card)
                                .or_else(|| p1.len().checked_sub(1));
                            if let Some(at) = at {
                                p1.remove(at);
                            }
                        }
                        opening = Some(PerPlayer::new(p1, hand(&result.state, PlayerId::P2)));
                    }
                    begun = true;
                    drawn.p1.extend(step.kept.p1);
                    drawn.p2.extend(step.kept.p2);
                    burns += step.burns;
                    draw_casts += step.casts;
                }
                log.push(action);
                state = result.state;
            }
            let Some(end) = state.result else {
                panic!("{seed} did not finish");
            };
            if let Some(card) = waiting.first() {
                panic!(
                    "{seed}: {} ({}) was drawn and then nothing became of it",
                    card.def_id, card.instance_id
                );
            }

            let summary = summarize_game(&FoldArgs {
                seed: seed.clone(),
                decks: decks.clone(),
                log: log.clone(),
                ..FoldArgs::default()
            });
            let Some(summary) = summary else {
                panic!("{seed} has no summary");
            };
            assert_eq!(summary.winner, end.winner, "{seed}");
            assert_eq!(summary.reason, end.reason, "{seed}");
            assert_eq!(summary.turns, state.turn, "{seed}");
            assert_eq!(summary.first, PlayerId::P1, "{seed}");

            if let Some(opening) = &opening {
                assert_eq!(sorted(&summary.seats.p1.opening), sorted(&opening.p1), "{seed}");
                assert_eq!(sorted(&summary.seats.p2.opening), sorted(&opening.p2), "{seed}");
            }
            // §2.1, R244: The Coin goes to the seat going second, after its mulligan.
            assert!(
                summary.seats.p2.opening.iter().any(|id| id == COIN_DEF_ID),
                "{seed}"
            );
            assert!(
                !summary.seats.p1.opening.iter().any(|id| id == COIN_DEF_ID),
                "{seed}"
            );

            for player in PLAYER_IDS {
                assert_eq!(
                    summary.seats[player].drawn,
                    drawn[player],
                    "{seed} {}",
                    player.as_str()
                );
                let played = &summary.seats[player].played;
                let logged: Vec<String> = plays[player].iter().map(|play| play.def_id.clone()).collect();
                if pawn {
                    assert!(subsequence(&logged, played), "{seed} {}", player.as_str());
                } else {
                    assert_eq!(*played, logged, "{seed} {}", player.as_str());
                }
            }
            if pawn {
                pawn_games += 1;
            }
            casts += cast_ids.len();
        }

        // The games cast cards on their draw, after the opening hands too, burned draws on a full hand,
        // countered plays and replaced others, and most of them were held to the exact match of plays.
        assert!(casts > 0);
        assert!(countered_plays > 0);
        assert!(replaced_plays > 0);
        assert!(draw_casts > 0);
        assert!(burns > 0);
        assert!(pawn_games < GAMES / 2);
    }
}
