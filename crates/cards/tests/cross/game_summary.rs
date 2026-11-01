//! SPEC §9.11, R376: engine `summarize_game` over real cards (SPEC §10.7 policy, §10.5, §2.4).
//! Oracles for opening hands (The Coin, R943, R2089000), plays (R448, R449, R1147000, R1183002,
//! #23, #96, #18), and draws/burns (R4, #21, #90).

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::catalog::deckable;
use jackioh_engine::testkit::*;

/// The decks deal from every set that ships (#1884, #2250), and a hundred games hold every kind of play
/// the test counts.
const GAMES: i32 = 100;
const MAX_ACTIONS: usize = 3000;
const MY_PAWN: &str = "core-096";

/// Every catalog id a deck may hold (no Token, and a set that ships, R1420), sorted.
fn pool() -> Vec<String> {
    let mut ids: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| deckable(def) && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .collect();
    ids.sort();
    ids
}

/// The two decks a game deals, `size` each, off a shuffle of `pool()`: its first and second `size`.
/// A Core card and its Legacy card are two cards a deck may hold together (R2511001).
fn deal(shuffled: &[String], size: usize) -> (Vec<String>, Vec<String>) {
    (shuffled[..size].to_vec(), shuffled[size..size * 2].to_vec())
}

/// The dealt cards of a hand: R2089000 leaves a Created card (R943) out of every opening hand.
fn hand(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .filter(|card| card.created != Some(true))
        .map(|card| card.def_id.clone())
        .collect()
}

fn sorted(ids: &[String]) -> Vec<String> {
    let mut out = ids.to_vec();
    out.sort();
    out
}

struct Drawn {
    player: PlayerId,
    instance_id: String,
    def_id: String,
    /// R943: a Created card shuffled in, which no `drawn` list holds (R2089000).
    created: bool,
}

struct DrawsKept {
    kept: PerPlayer<Vec<String>>,
    burns: i32,
    casts: i32,
}

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

/// an `addedToHand` names it (it reached the hand), a `burned` does (R4), or it was cast on its draw
/// (§2.4: its `cardPlayed`, or the `countered` or `transformed` that took its place, R448, R449). A
/// cast's choices are asked before its announce, so a draw a prompt holds waits in `waiting`. A
/// Created card reaching the hand is kept by no seat (R2089000).
fn draws_kept(
    events: &[GameEvent],
    from: usize,
    waiting: &mut Vec<Drawn>,
    created: &dyn Fn(&str) -> bool,
) -> DrawsKept {
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
                created: created(instance_id),
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
            GameEvent::AddedToHand { .. } if !drawn.created => kept[drawn.player].push(drawn.def_id),
            GameEvent::AddedToHand { .. } => {}
            GameEvent::Burned { .. } => burns += 1,
            _ => casts += 1,
        }
    }
    DrawsKept { kept, burns, casts }
}

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
    radiant: bool,
    /// R943: the card is Created, so its play is in `createdPlayed`, not `played` (R2089000).
    created: bool,
    placed: bool,
    open: bool,
    announced: bool,
}

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

    #[test]
    fn r376_r1147000_reads_opening_hands_plays_their_faces_and_casts_off_real_games_as_the_state_and_the_log_show_them()
     {
        register_all();
        let pool = pool();
        let size = DECK_SIZE as usize;
        let mut pawn_games = 0;
        let mut casts = 0;
        let mut burns = 0;
        let mut draw_casts = 0;
        let mut countered_plays = 0;
        let mut replaced_plays = 0;
        let mut radiant_plays = 0;
        let mut created_plays = 0;

        for n in 1..=GAMES {
            let seed = format!("summary-cards-{n}");
            let shuffled = create_rng(&format!("summary-cards-decks-{n}"), 0).shuffle(&pool);
            let decks: (Vec<String>, Vec<String>) = deal(&shuffled, size);
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
                        radiant: card.radiant,
                        created: card.created == Some(true),
                        placed: false,
                        open: true,
                        announced: false,
                    });
                }
                let result = reduce(&state, &action);
                if let Some(error) = &result.error {
                    panic!("{seed}: {} refused: {error}", action.action_type());
                }
                let events: &[GameEvent] = &result.events;
                // R943: whether a card is Created, as the step left it or found it. A card in neither
                // was made and gone within the step: the deal put every dealt card in the state.
                let created = |id: &str| {
                    find_instance(&result.state, id)
                        .or_else(|| find_instance(&state, id))
                        .is_none_or(|card| card.created == Some(true))
                };
                // The cards this step added to a hand, which the summary follows into that hand too; a
                // card a Counter sent there is its countering player's cast, never a play (R831001).
                let stolen_here: Vec<&str> = events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Countered { instance_id, .. } => Some(instance_id.as_str()),
                        _ => None,
                    })
                    .collect();
                let mut added_here: Vec<&str> = Vec::new();
                // The Radiant changes the step has made so far, for a card no play holds yet: one a hand
                // trigger casts later in the step is played on the face it has by then (R1147000).
                let mut radiant_here: IndexMap<String, bool> = IndexMap::new();
                for event in events {
                    match event {
                        GameEvent::AddedToHand { instance_id, .. }
                            if !stolen_here.contains(&instance_id.as_str()) =>
                        {
                            added_here.push(instance_id)
                        }
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
                                plays[seat][at].radiant =
                                    find_instance(&result.state, new_instance_id).is_some_and(|c| c.radiant);
                                // R943: the card a replacement puts in the play's place is Created.
                                plays[seat][at].created = true;
                                replaced_plays += 1;
                            }
                        }
                        GameEvent::RadiantSet { instance_id, .. } => {
                            radiant_here.insert(instance_id.clone(), true);
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                && !plays[seat][at].placed
                            {
                                plays[seat][at].radiant = true;
                            }
                        }
                        GameEvent::Deradianted { instance_id, .. } => {
                            radiant_here.insert(instance_id.clone(), false);
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                && !plays[seat][at].placed
                            {
                                plays[seat][at].radiant = false;
                            }
                        }
                        GameEvent::CardPlayed {
                            player: caster,
                            instance_id,
                            def_id,
                            former_id,
                            on_draw,
                            ..
                        } => {
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                .or_else(|| former_id.as_deref().and_then(|id| open_play(&plays, id)))
                            {
                                plays[seat][at].placed = true;
                            // R2179002: a card cast on its draw is no play, though it stood in the hand as
                            // the step began: a Trade shuffles it in and draws it straight back
                            // (summary-cards-28, #2280). R831001: nor is a card a Counter sent to a hand in
                            // this step, its countering player's cast, even when a second Counter sends it
                            // back to the hand it was played from (summary-cards-87, #2657).
                            } else if *on_draw != Some(true)
                                && !stolen_here.contains(&instance_id.as_str())
                                && let Some(card) = state.players[*caster]
                                    .hand
                                    .iter()
                                    .find(|card| card.id == *instance_id)
                                    .or_else(|| {
                                        added_here
                                            .contains(&instance_id.as_str())
                                            .then(|| find_instance(&result.state, instance_id))
                                            .flatten()
                                    })
                            {
                                // A card cast from the hand it stood in as the step began, or that the step
                                // added to it, counts as a play with no `play` logged: M+ #17 Dogpile's hand
                                // trigger (R1180005; summary-cards-35, -45, #1705, #2204). A Created one the
                                // step made Radiant is played Radiant (R2368000; summary-cards-54).
                                plays[*caster].push(Play {
                                    id: card.id.clone(),
                                    def_id: def_id.clone(),
                                    radiant: radiant_here.get(instance_id).copied().unwrap_or(card.radiant),
                                    created: card.created == Some(true),
                                    placed: true,
                                    open: false,
                                    announced: true,
                                });
                            }
                        }
                        GameEvent::GameOver { .. } => {
                            for seat in PLAYER_IDS {
                                plays[seat].retain(|play| !(play.open && !play.placed));
                            }
                        }
                        // A Death the play's own Tribute set off can discard the hand the card still stands
                        // in (§10.5 step 2), before any announce: the card was never played.
                        GameEvent::Discarded { instance_id, .. } => {
                            if let Some((seat, at)) = open_play(&plays, instance_id)
                                && !plays[seat][at].announced
                            {
                                plays[seat].remove(at);
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
                // M+ #18 Cognitive Offloading's autopilot (R1183002, #1187) plays the rest of a turn with no
                // `play` logged, as My Pawn's random turn does (summary-cards-26: M+ #10 stealing C #78), so
                // a game it engages in is held to My Pawn's check.
                if events.iter().any(|event| {
                    matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == MY_PAWN)
                        || matches!(event, GameEvent::AutopilotEngaged { .. })
                }) {
                    pawn = true;
                }
                // A cast on its draw is a `cardPlayed` naming a drawn card before any hand took it: its
                // announce comes between (R448), and its choices may be asked first, in a prompt of their own
                // (R70).
                for event in events {
                    match event {
                        GameEvent::Drawn { instance_id, .. } => {
                            unplaced.insert(instance_id.clone());
                        }
                        GameEvent::AddedToHand { instance_id, .. }
                        | GameEvent::Burned { instance_id, .. } => {
                            unplaced.shift_remove(instance_id);
                        }
                        GameEvent::CardPlayed {
                            instance_id, def_id, ..
                        } if unplaced.shift_remove(instance_id) => {
                            cast_ids.push(def_id.clone());
                        }
                        _ => {}
                    }
                }
                let from = if begun {
                    Some(0)
                } else {
                    events
                        .iter()
                        .position(|event| matches!(event, GameEvent::TurnStarted { .. }))
                };
                if let Some(from) = from {
                    let step = draws_kept(events, from, &mut waiting, &created);
                    if !begun
                        && state.phase == Phase::Mulligan
                        && result.state.phase != Phase::Mulligan
                        && result.state.turn == 1
                    {
                        let mut p1 = hand(&result.state, PlayerId::P1);
                        for card in &step.kept.p1 {
                            let at = p1
                                .iter()
                                .rposition(|id| id == card)
                                .or_else(|| p1.len().checked_sub(1));
                            if let Some(at) = at {
                                p1.remove(at);
                            }
                        }
                        // A card turn 1 took from a hand before the step ended was in the opening hand all
                        // the same: #21 Hinder, cast on its draw, discards one (§2.4). A discarded card the
                        // step did not draw goes back.
                        let mut p2 = hand(&result.state, PlayerId::P2);
                        let drawn_here: Vec<&String> = events[from..]
                            .iter()
                            .filter_map(|event| match event {
                                GameEvent::Drawn { instance_id, .. } => Some(instance_id),
                                _ => None,
                            })
                            .collect();
                        for event in &events[from..] {
                            if let GameEvent::Discarded {
                                instance_id,
                                def_id,
                                owner,
                            } = event
                                && !drawn_here.contains(&instance_id)
                                && !created(instance_id)
                            {
                                match owner {
                                    PlayerId::P1 => p1.push(def_id.clone()),
                                    PlayerId::P2 => p2.push(def_id.clone()),
                                }
                            }
                        }
                        // A card turn 1 added to a hand with no draw, and that the hand still holds, was in
                        // no opening hand either (summary-cards-36's p1 M #37 CN in a Bottle, cast on its
                        // draw, §2.4, adds a random card).
                        for event in &events[from..] {
                            if let GameEvent::AddedToHand {
                                player,
                                instance_id,
                                def_id,
                                ..
                            } = event
                                && !drawn_here.contains(&instance_id)
                                && !created(instance_id)
                                && result.state.players[*player]
                                    .hand
                                    .iter()
                                    .any(|card| card.id == *instance_id)
                            {
                                let held = match player {
                                    PlayerId::P1 => &mut p1,
                                    PlayerId::P2 => &mut p2,
                                };
                                if let Some(at) = held.iter().rposition(|id| id == def_id) {
                                    held.remove(at);
                                }
                            }
                        }
                        opening = Some(PerPlayer::new(p1, p2));
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
            // §2.1, R244: The Coin goes to the seat going second, after its mulligan. R2089000: it is
            // Created, so it is in no opening hand, and it is that seat's Created card, used or not.
            for player in PLAYER_IDS {
                assert!(
                    !summary.seats[player].opening.iter().any(|id| id == COIN_DEF_ID),
                    "{seed}"
                );
            }
            let p2 = &summary.seats.p2;
            assert!(
                p2.created_used
                    .iter()
                    .flatten()
                    .any(|used| used.card == COIN_DEF_ID)
                    || p2.created_unused.iter().flatten().any(|id| id == COIN_DEF_ID),
                "{seed}"
            );

            for player in PLAYER_IDS {
                assert_eq!(
                    summary.seats[player].drawn,
                    drawn[player],
                    "{seed} {}",
                    player.as_str()
                );
                let seat = &summary.seats[player];
                // R2089000: the dealt cards' plays in `played`, the Created cards' in `createdPlayed`.
                let (made, dealt): (Vec<&Play>, Vec<&Play>) =
                    plays[player].iter().partition(|play| play.created);
                let logged: Vec<String> = dealt.iter().map(|play| play.def_id.clone()).collect();
                let logged_made: Vec<String> = made.iter().map(|play| play.def_id.clone()).collect();
                let created_played: Vec<String> = seat
                    .created_played
                    .iter()
                    .flatten()
                    .map(|play| play.card.clone())
                    .collect();
                if pawn {
                    assert!(subsequence(&logged, &seat.played), "{seed} {}", player.as_str());
                    assert!(
                        subsequence(&logged_made, &created_played),
                        "{seed} {}",
                        player.as_str()
                    );
                } else {
                    assert_eq!(seat.played, logged, "{seed} {}", player.as_str());
                    let expected_radiant: Vec<bool> = dealt.iter().map(|play| play.radiant).collect();
                    assert_eq!(
                        seat.played_radiant.clone().unwrap_or_default(),
                        expected_radiant,
                        "{seed} {}",
                        player.as_str()
                    );
                    radiant_plays += expected_radiant.iter().filter(|&&r| r).count();
                    assert_eq!(created_played, logged_made, "{seed} {}", player.as_str());
                    let made_radiant: Vec<bool> = seat
                        .created_played
                        .iter()
                        .flatten()
                        .map(|play| play.radiant)
                        .collect();
                    assert_eq!(
                        made_radiant,
                        made.iter().map(|play| play.radiant).collect::<Vec<bool>>(),
                        "{seed} {}",
                        player.as_str()
                    );
                }
                created_plays += made.len();
            }
            if pawn {
                pawn_games += 1;
            }
            casts += cast_ids.len();
        }

        assert!(casts > 0);
        assert!(countered_plays > 0);
        assert!(replaced_plays > 0);
        assert!(draw_casts > 0);
        assert!(burns > 0);
        assert!(radiant_plays > 0);
        assert!(created_plays > 0);
        assert!(pawn_games < GAMES / 2);
    }
}

mod r831001_a_stolen_cast_is_no_play {
    use super::*;

    const BOOK: &str = "classic-024"; // (1) Spell, Book: Draw 3.
    const PALANTIR: &str = "classic-004"; // (2) Field Spell: steals the opponent's (1) Cost Spell and casts it.

    fn act(state: &GameState, log: &mut Vec<Action>, body: Value) -> GameState {
        let mut action = body;
        action["nonce"] = json!(format!("palantir-summary-{}", log.len()));
        let action: Action = json_as(action);
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        result.state
    }

    fn first_of(state: &GameState, player: PlayerId, def_id: &str) -> Option<String> {
        state.players[player]
            .hand
            .iter()
            .find(|card| card.def_id == def_id)
            .map(|card| card.id.clone())
    }

    fn end_turn(state: GameState, log: &mut Vec<Action>, player: PlayerId) -> GameState {
        // R82: a turn with nothing left to do has ended by itself.
        if state.active != player || state.result.is_some() {
            return state;
        }
        act(&state, log, json!({ "type": "endTurn", "playerId": player }))
    }

    fn game(seed: &str, decks: &(Vec<String>, Vec<String>)) -> Option<(Vec<Action>, String)> {
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        first_of(&state, PlayerId::P2, PALANTIR)?;
        let mut log: Vec<Action> = Vec::new();
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act(
                &state,
                &mut log,
                json!({ "type": "mulligan", "keep": keep, "playerId": player }),
            );
        }
        state = end_turn(state, &mut log, PlayerId::P1);
        let coin = first_of(&state, PlayerId::P2, COIN_DEF_ID)?;
        state = act(
            &state,
            &mut log,
            json!({ "type": "play", "instanceId": coin, "playerId": "p2" }),
        );
        let palantir = first_of(&state, PlayerId::P2, PALANTIR)?;
        state = act(
            &state,
            &mut log,
            json!({ "type": "play", "instanceId": palantir, "zone": { "row": "backrow", "lane": 1 }, "playerId": "p2" }),
        );
        state = end_turn(state, &mut log, PlayerId::P2);
        let book = first_of(&state, PlayerId::P1, BOOK)?;
        state = act(
            &state,
            &mut log,
            json!({ "type": "play", "instanceId": book, "playerId": "p1" }),
        );
        assert!(
            state.players.p2.graveyard.iter().any(|card| card.id == book),
            "{seed}"
        );
        state = act(&state, &mut log, json!({ "type": "concede", "playerId": "p1" }));
        assert!(state.result.is_some());
        Some((log, book))
    }

    #[test]
    fn r376_r831001_the_spell_palantir_steals_and_casts_is_neither_seat_s_play() {
        register_all();
        let units: Vec<String> = pool()
            .into_iter()
            .filter(|id| {
                CATALOG.get(id).is_some_and(|def| {
                    def.type_ == CardType::Unit && matches!(def.cost, CardCost::Fixed(cost) if cost >= 3)
                })
            })
            .take(DECK_SIZE as usize - 1)
            .collect();
        let deck = |first: &str| -> Vec<String> {
            std::iter::once(first.to_string())
                .chain(units.iter().cloned())
                .collect()
        };
        let decks = (deck(BOOK), deck(PALANTIR));
        let found = (0..300).find_map(|n| {
            let seed = format!("summary-palantir-{n}");
            game(&seed, &decks).map(|(log, _)| (seed, log))
        });
        let Some((seed, log)) = found else {
            panic!("a seed below 300 deals the Book and Palantir in time");
        };

        let Some(summary) = summarize_game(&FoldArgs {
            seed,
            decks,
            log,
            ..FoldArgs::default()
        }) else {
            panic!("a finished game has a summary");
        };
        // p1's play was countered, so it is none (R448); p2 cast it, which is no play either.
        assert!(summary.seats.p1.played.is_empty());
        // R2089000: The Coin is Created, so p2's play of it is its Created card's use, not a deck play.
        assert_eq!(summary.seats.p2.played, [PALANTIR]);
        let cards = |uses: &Option<Vec<CreatedUse>>| -> Vec<String> {
            uses.iter().flatten().map(|used| used.card.clone()).collect()
        };
        assert_eq!(cards(&summary.seats.p2.created_used), [COIN_DEF_ID]);
        assert_eq!(cards(&summary.seats.p2.created_played), [COIN_DEF_ID]);
        assert_eq!(summary.seats.p1.created_used, None);
    }
}

mod r1256000_per_seat_counts {
    use super::*;

    const SHEEPISH: &str = "core-041"; // (1) Trap: set face-down, fires when the opponent plays a Unit.
    const VANILLA: &str = "core-008"; // (1) 4/4 Unit with no text.
    const GLOWY: &str = "core-026"; // (3) Spell: make a card in your hand Radiant.

    fn fatigue_pool() -> Vec<String> {
        let mut ids: Vec<String> = CATALOG
            .iter()
            .filter(|(_, def)| {
                deckable(def)
                    && def.type_ == CardType::Unit
                    && matches!(def.set, SetName::Core | SetName::Classic)
                    && !def.base.text.to_lowercase().contains("cast on draw")
                    && !def.radiant.text.to_lowercase().contains("cast on draw")
                    && !def.base.text.to_lowercase().contains("quickdraw")
                    && !def.radiant.text.to_lowercase().contains("quickdraw")
            })
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    }

    fn units_except(first: &str) -> Vec<String> {
        pool()
            .into_iter()
            .filter(|id| id != first && CATALOG.get(id).is_some_and(|def| def.type_ == CardType::Unit))
            .take(DECK_SIZE as usize - 1)
            .collect()
    }

    fn act(state: &GameState, log: &mut Vec<Action>, body: Value) -> GameState {
        let mut action = body;
        action["nonce"] = json!(format!("summary-counts-{}", log.len()));
        let action: Action = json_as(action);
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        result.state
    }

    fn act_body(state: &GameState, log: &mut Vec<Action>, body: ActionBody, player: PlayerId) -> GameState {
        let action = Action::new(body, player, format!("summary-counts-{}", log.len()));
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        result.state
    }

    fn mulligans(state: GameState, log: &mut Vec<Action>) -> GameState {
        let mut state = state;
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act(
                &state,
                log,
                json!({ "type": "mulligan", "keep": keep, "playerId": player }),
            );
        }
        state
    }

    fn end_turn(state: GameState, log: &mut Vec<Action>, player: PlayerId) -> GameState {
        // R82: a turn with nothing left to do has ended by itself.
        if state.active != player || state.result.is_some() {
            return state;
        }
        act(&state, log, json!({ "type": "endTurn", "playerId": player }))
    }

    fn hand_id(state: &GameState, player: PlayerId, def_id: &str) -> Option<String> {
        state.players[player]
            .hand
            .iter()
            .find(|card| card.def_id == def_id)
            .map(|card| card.id.clone())
    }

    fn play_offer(state: &GameState, player: PlayerId, instance_id: &str) -> Option<ActionBody> {
        legal_actions(state, player)
            .into_iter()
            .find(|body| matches!(body, ActionBody::Play { instance_id: id, .. } if id == instance_id))
    }

    fn summarize(seed: String, decks: (Vec<String>, Vec<String>), log: Vec<Action>) -> GameSummary {
        summarize_game(&FoldArgs {
            seed,
            decks,
            log,
            ..FoldArgs::default()
        })
        .unwrap_or_else(|| panic!("a finished game has a summary"))
    }

    fn zeros(seat: &SeatSummary) {
        assert_eq!(seat.fatigue_draws, Some(0));
        assert_eq!(seat.fatigue_damage, Some(0));
        assert_eq!(seat.traps_set, Some(0));
        assert_eq!(seat.traps_fired, Some(0));
        assert_eq!(seat.position_switches, Some(0));
        assert_eq!(seat.radiant_upgrades, Some(0));
    }

    #[test]
    fn r1256000_fatigue_draws_and_damage_count_per_seat() {
        register_all();
        let pool = fatigue_pool();
        let size = DECK_SIZE as usize;
        assert!(pool.len() >= 2 * size, "need two distinct decks");
        let shuffled = create_rng("summary-fatigue-decks", 0).shuffle(&pool);
        let decks: (Vec<String>, Vec<String>) =
            (shuffled[..size].to_vec(), shuffled[size..size * 2].to_vec());
        let seed = "summary-fatigue".to_string();

        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.clone(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        state = mulligans(state, &mut log);
        while state.result.is_none() && log.len() < MAX_ACTIONS {
            let player = seat_to_act(&state).expect("a seat to act");
            state = end_turn(state, &mut log, player);
        }
        let end = state.result.expect("an endTurn-only game fatigues out");
        // p1 opens on three cards and p2 on four (R2368000), so p1's library holds one card more and p2
        // takes the eighth hit (1+…+8 = 36 at 30 HP) first, on its 29th turn, before p1's 30th.
        assert_eq!(end.winner, Winner::P1);
        assert_eq!(end.reason, GameOverReason::HeroDeath);

        let summary = summarize(seed, decks, log);
        assert_eq!(summary.seats.p1.fatigue_draws, Some(7));
        assert_eq!(summary.seats.p1.fatigue_damage, Some(28));
        assert_eq!(summary.seats.p2.fatigue_draws, Some(8));
        assert_eq!(summary.seats.p2.fatigue_damage, Some(36));
        for seat in [&summary.seats.p1, &summary.seats.p2] {
            assert_eq!(seat.traps_set, Some(0));
            assert_eq!(seat.traps_fired, Some(0));
            assert_eq!(seat.position_switches, Some(0));
            assert_eq!(seat.radiant_upgrades, Some(0));
        }
    }

    fn trap_game(seed: &str, decks: &(Vec<String>, Vec<String>)) -> Option<Vec<Action>> {
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        hand_id(&state, PlayerId::P1, SHEEPISH)?;
        hand_id(&state, PlayerId::P2, VANILLA)?;
        let mut log: Vec<Action> = Vec::new();
        state = mulligans(state, &mut log);
        let sheepish = hand_id(&state, PlayerId::P1, SHEEPISH)?;
        state = act_body(
            &state,
            &mut log,
            play_offer(&state, PlayerId::P1, &sheepish)?,
            PlayerId::P1,
        );
        state = end_turn(state, &mut log, PlayerId::P1);
        let vanilla = hand_id(&state, PlayerId::P2, VANILLA)?;
        state = act_body(
            &state,
            &mut log,
            play_offer(&state, PlayerId::P2, &vanilla)?,
            PlayerId::P2,
        );
        state = act(&state, &mut log, json!({ "type": "concede", "playerId": "p1" }));
        assert!(state.result.is_some());
        Some(log)
    }

    #[test]
    fn r1256000_a_trap_set_and_fired_counts_for_its_controller() {
        register_all();
        let deck = |first: &str| -> Vec<String> {
            std::iter::once(first.to_string())
                .chain(units_except(first))
                .collect()
        };
        let decks = (deck(SHEEPISH), deck(VANILLA));
        let found = (0..300).find_map(|n| {
            let seed = format!("summary-trap-{n}");
            trap_game(&seed, &decks).map(|log| (seed, log))
        });
        let Some((seed, log)) = found else {
            panic!("a seed below 300 deals the Sheepish and the Vanilla in time");
        };

        let summary = summarize(seed, decks, log);
        assert_eq!(summary.seats.p1.fatigue_draws, Some(0));
        assert_eq!(summary.seats.p1.fatigue_damage, Some(0));
        assert_eq!(summary.seats.p1.traps_set, Some(1));
        assert_eq!(summary.seats.p1.traps_fired, Some(1));
        assert_eq!(summary.seats.p1.position_switches, Some(0));
        assert_eq!(summary.seats.p1.radiant_upgrades, Some(0));
        zeros(&summary.seats.p2);
    }

    fn switch_game(seed: &str, decks: &(Vec<String>, Vec<String>)) -> Option<Vec<Action>> {
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        hand_id(&state, PlayerId::P1, VANILLA)?;
        let mut log: Vec<Action> = Vec::new();
        state = mulligans(state, &mut log);
        let vanilla = hand_id(&state, PlayerId::P1, VANILLA)?;
        state = act_body(
            &state,
            &mut log,
            play_offer(&state, PlayerId::P1, &vanilla)?,
            PlayerId::P1,
        );
        state = end_turn(state, &mut log, PlayerId::P1);
        state = end_turn(state, &mut log, PlayerId::P2);
        state = act(
            &state,
            &mut log,
            json!({ "type": "switchPosition", "instanceId": vanilla, "playerId": "p1" }),
        );
        state = act(&state, &mut log, json!({ "type": "concede", "playerId": "p1" }));
        assert!(state.result.is_some());
        Some(log)
    }

    #[test]
    fn r1256000_a_position_switch_counts_for_the_unit_s_controller() {
        register_all();
        let deck = |first: &str| -> Vec<String> {
            std::iter::once(first.to_string())
                .chain(units_except(first))
                .collect()
        };
        let decks = (deck(VANILLA), deck(VANILLA));
        let found = (0..300).find_map(|n| {
            let seed = format!("summary-switch-{n}");
            switch_game(&seed, &decks).map(|log| (seed, log))
        });
        let Some((seed, log)) = found else {
            panic!("a seed below 300 deals the Vanilla in time");
        };

        let summary = summarize(seed, decks, log);
        assert_eq!(summary.seats.p1.position_switches, Some(1));
        assert_eq!(summary.seats.p1.fatigue_draws, Some(0));
        assert_eq!(summary.seats.p1.fatigue_damage, Some(0));
        assert_eq!(summary.seats.p1.traps_set, Some(0));
        assert_eq!(summary.seats.p1.traps_fired, Some(0));
        assert_eq!(summary.seats.p1.radiant_upgrades, Some(0));
        zeros(&summary.seats.p2);
    }

    fn glowy_game(seed: &str, decks: &(Vec<String>, Vec<String>)) -> Option<Vec<Action>> {
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        hand_id(&state, PlayerId::P1, GLOWY)?;
        let mut log: Vec<Action> = Vec::new();
        state = mulligans(state, &mut log);
        state = end_turn(state, &mut log, PlayerId::P1);
        state = end_turn(state, &mut log, PlayerId::P2);
        state = end_turn(state, &mut log, PlayerId::P1);
        state = end_turn(state, &mut log, PlayerId::P2);
        let glowy = hand_id(&state, PlayerId::P1, GLOWY)?;
        let target = state.players[PlayerId::P1]
            .hand
            .iter()
            .find(|card| card.id != glowy)
            .map(|card| card.id.clone())?;
        state = act(
            &state,
            &mut log,
            json!({ "type": "play", "instanceId": glowy, "playerId": "p1",
                "targets": [{ "pick": "instance", "instanceId": target }] }),
        );
        state = act(&state, &mut log, json!({ "type": "concede", "playerId": "p1" }));
        assert!(state.result.is_some());
        Some(log)
    }

    #[test]
    fn r1256000_a_card_made_radiant_in_hand_counts_for_its_owner() {
        register_all();
        let deck = |first: &str| -> Vec<String> {
            std::iter::once(first.to_string())
                .chain(units_except(first))
                .collect()
        };
        let decks = (deck(GLOWY), deck(VANILLA));
        let found = (0..300).find_map(|n| {
            let seed = format!("summary-glowy-{n}");
            glowy_game(&seed, &decks).map(|log| (seed, log))
        });
        let Some((seed, log)) = found else {
            panic!("a seed below 300 deals the Bean in time");
        };

        let summary = summarize(seed, decks, log);
        assert_eq!(summary.seats.p1.radiant_upgrades, Some(1));
        assert_eq!(summary.seats.p1.fatigue_draws, Some(0));
        assert_eq!(summary.seats.p1.fatigue_damage, Some(0));
        assert_eq!(summary.seats.p1.traps_set, Some(0));
        assert_eq!(summary.seats.p1.traps_fired, Some(0));
        assert_eq!(summary.seats.p1.position_switches, Some(0));
        zeros(&summary.seats.p2);
    }
}

/// R2089000: the Created cards (R943) a game's record credits to the seat that made them, read off
/// `scenario()` steps through `SummaryReader`, the reader `summarize_game` folds a log with, and off
/// recorded games by `summarize_game` itself.
mod r2089000_created_cards {
    use super::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit with no text.
    const DUPLICATING: &str = "core-012"; // (2) Unit, Cry: Summon a copy of this.
    const TOKEN_MAKER: &str = "core-015"; // (1) Unit, Cry: Summon a Rush Token.
    const RUSH_TOKEN: &str = "core-t-rush";
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const RECYCLING: &str = "core-039"; // (0) Spell, end of turn: copies of the cards played go to the hand.
    const SHADE_IRIS: &str = "meditative-028"; // (2) Unit, Cry: Ancient Curses into the opponent's deck.
    const CURSE: &str = "meditative-028-1"; // Spell token, Cast on draw: take damage.
    const FATE: &str = "meditative-026"; // (4) Field Spell, end of turn: a Mana Well into every empty backrow.
    const WELL: &str = "core-006";

    fn step(reader: &mut SummaryReader, s: &mut Scenario, verb: impl FnOnce(&mut Scenario)) {
        let before = s.state().clone();
        let from = s.events().len();
        verb(s);
        reader.step(&before, &s.events()[from..], s.state());
    }

    fn used(card: &str, turn: i32, radiant: bool) -> CreatedUse {
        CreatedUse {
            card: card.to_string(),
            turn,
            radiant,
        }
    }

    fn cards(uses: &Option<Vec<CreatedUse>>) -> Vec<String> {
        uses.iter().flatten().map(|used| used.card.clone()).collect()
    }

    fn created_in_hand(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
        s.hand(player)
            .into_iter()
            .find(|card| card.def_id == def_id && card.created == Some(true))
            .unwrap_or_else(|| panic!("a Created {def_id} in {}'s hand", player.as_str()))
    }

    #[test]
    fn r2089000_a_token_an_effect_summons_is_its_creator_s_used_card() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [TOKEN_MAKER] } }));
        let mut reader = SummaryReader::new(s.state());
        let turn = s.state().turn;

        step(&mut reader, &mut s, |s| {
            s.play(TOKEN_MAKER, json!({}));
        });

        let p1 = reader.seat(P1, vec![TOKEN_MAKER.to_string()]);
        assert_eq!(p1.played, [TOKEN_MAKER]);
        assert_eq!(p1.created_used, Some(vec![used(RUSH_TOKEN, turn, false)]));
        assert_eq!(p1.created_played, None);
        assert_eq!(p1.created_unused, None);
        assert_eq!(reader.seat(P2, Vec::new()).created_used, None);
    }

    #[test]
    fn r2089000_a_created_card_added_to_its_creator_s_hand_and_played_is_no_deck_play() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RECYCLING, VANILLA] } }));
        let mut reader = SummaryReader::new(s.state());

        step(&mut reader, &mut s, |s| {
            s.play(RECYCLING, json!({}));
        });
        step(&mut reader, &mut s, |s| {
            s.play(VANILLA, json!({}));
        });
        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
        assert_eq!(
            reader.seat(P1, Vec::new()).created_unused,
            Some(vec![VANILLA.to_string()])
        );
        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
        let copy = created_in_hand(&s, P1, VANILLA);
        let turn = s.state().turn;
        step(&mut reader, &mut s, |s| {
            s.play(copy, json!({}));
        });

        let p1 = reader.seat(P1, vec![RECYCLING.to_string(), VANILLA.to_string()]);
        assert_eq!(p1.played, [RECYCLING, VANILLA]);
        assert_eq!(p1.played_radiant, Some(vec![false, false]));
        assert_eq!(p1.created_used, Some(vec![used(VANILLA, turn, false)]));
        assert_eq!(p1.created_played, Some(vec![used(VANILLA, turn, false)]));
        assert_eq!(p1.created_unused, None);
    }

    #[test]
    fn r2089000_a_created_copy_of_a_deck_card_leaves_the_deck_card_s_figures_unchanged() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [DUPLICATING, { "def": DUPLICATING, "created": true, "radiant": true }] },
        }));
        let mut reader = SummaryReader::new(s.state());
        let turn = s.state().turn;
        let made = created_in_hand(&s, P1, DUPLICATING);
        step(&mut reader, &mut s, |s| {
            s.play(made, json!({}));
        });

        let mut p1 = reader.seat(P1, vec![DUPLICATING.to_string()]);
        assert!(p1.played.is_empty());
        assert_eq!(p1.created_played, Some(vec![used(DUPLICATING, turn, true)]));
        assert_eq!(cards(&p1.created_used), [DUPLICATING, DUPLICATING]);
        assert_eq!(
            p1.created_used.iter().flatten().next(),
            Some(&used(DUPLICATING, turn, true))
        );

        p1.opening = vec![DUPLICATING.to_string()];
        let record = |p1: SeatSummary| GameRecord {
            id: "r2089000-copy".to_string(),
            patch: "v0.0.1".to_string(),
            source: GameSource::Live,
            mode: GameMode::Bo1,
            pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
            game: GameSummary {
                first: P1,
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath,
                turns: turn,
                seats: PerPlayer::new(p1, SeatSummary::default()),
            },
            build: None,
        };
        let dealt_only = SeatSummary {
            created_used: None,
            created_unused: None,
            created_played: None,
            ..p1.clone()
        };
        let report = card_stats(&[record(p1)], &DEFAULT_CARD_STATS_FILTER);
        assert_eq!(
            report.cards,
            card_stats(&[record(dealt_only)], &DEFAULT_CARD_STATS_FILTER).cards
        );
        let [deck] = report.cards.as_slice() else {
            panic!("one deck card: {:?}", report.cards);
        };
        assert_eq!(deck.card, DUPLICATING);
        assert_eq!(deck.played.games, 0);
        assert_eq!(deck.drawn_not_played.games, 1);
        assert_eq!(deck.played_radiant.games, 0);
        let [made] = report.created.as_slice() else {
            panic!("one Created card: {:?}", report.created);
        };
        assert_eq!(made.card, DUPLICATING);
        assert_eq!((made.created_played.games, made.created_played.wins), (1, 1));
        assert_eq!(made.created_played_radiant.games, 1);
        assert_eq!(made.created_not_played.games, 0);
    }

    #[test]
    fn r2089000_an_ancient_curse_cast_on_its_draw_is_the_creating_seat_s() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SHADE_IRIS] },
            "p2": { "hand": [VANILLA] },
        }));
        let mut reader = SummaryReader::new(s.state());
        step(&mut reader, &mut s, |s| {
            s.play(SHADE_IRIS, json!({}));
        });
        assert_eq!(
            reader.seat(P1, Vec::new()).created_unused,
            Some(vec![CURSE.to_string()])
        );
        assert_eq!(reader.seat(P2, Vec::new()).created_unused, None);
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ShuffledIn { player: P2, def_id, creator: Some(P1), .. } if def_id == CURSE
        )));
        for seat in [P1, P2] {
            let events = serde_json::to_string(&s.view(seat).events).expect("a view serializes");
            assert!(!events.contains("creator"), "{}'s view", seat.as_str());
        }

        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
        assert!(
            s.events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { player: P2, def_id, .. } if def_id == CURSE)
            ),
            "p2 cast the Curse on its draw"
        );
        let turn = s.state().turn;
        let p1 = reader.seat(P1, Vec::new());
        assert_eq!(p1.created_used, Some(vec![used(CURSE, turn, false)]));
        assert_eq!(p1.created_unused, None);
        // A cast is no play from the hand (§2.4, R70), and the Curse is neither a draw nor p2's.
        let p2 = reader.seat(P2, Vec::new());
        assert!(p2.played.is_empty());
        assert!(p2.drawn.is_empty());
        assert_eq!(p2.created_used, None);
        assert_eq!(p2.created_played, None);
        assert_eq!(p2.created_unused, None);
    }

    #[test]
    fn r2089000_a_token_summoned_into_the_other_seat_s_backrow_is_its_summoner_s() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [FATE, VANILLA], "library": [VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        let mut reader = SummaryReader::new(s.state());
        let turn = s.state().turn;
        step(&mut reader, &mut s, |s| {
            s.play(FATE, json!({}));
        });
        // p1's end of turn summons a Mana Well into every empty backrow of both sides, each its own
        // side's (R942), and each one p1's Created card, used as it lands.
        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
        let wells = |player: PlayerId| {
            (1..=5)
                .filter_map(|lane| s.backrow(player, lane))
                .filter(|card| card.def_id == WELL)
                .count()
        };
        assert!(wells(P2) > 0, "a Well in p2's backrow");
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::Summoned { player: P2, def_id, creator: Some(P1), .. } if def_id == WELL
        )));
        for seat in [P1, P2] {
            let events = serde_json::to_string(&s.view(seat).events).expect("a view serializes");
            assert!(!events.contains("creator"), "{}'s view", seat.as_str());
        }
        let p1 = reader.seat(P1, Vec::new());
        assert_eq!(
            p1.created_used,
            Some(vec![used(WELL, turn, false); wells(P1) + wells(P2)])
        );
        assert_eq!(p1.created_unused, None);
        let p2 = reader.seat(P2, Vec::new());
        assert_eq!(p2.created_used, None);
        assert_eq!(p2.created_unused, None);
    }

    #[test]
    fn r2089000_a_created_card_never_used_is_its_creator_s_not_used_card() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SHADE_IRIS] },
            "p2": { "library": [VANILLA, VANILLA] },
        }));
        let mut reader = SummaryReader::new(s.state());
        step(&mut reader, &mut s, |s| {
            s.play(SHADE_IRIS, json!({}));
        });
        step(&mut reader, &mut s, |s| {
            s.act(ActionBody::Concede, P1, "p1 concedes");
        });
        assert!(s.state().result.is_some());

        let p1 = reader.seat(P1, Vec::new());
        assert_eq!(p1.created_unused, Some(vec![CURSE.to_string()]));
        assert_eq!(p1.created_used, None);
        let p2 = reader.seat(P2, Vec::new());
        assert_eq!(p2.created_unused, None);
        assert_eq!(p2.created_used, None);
    }

    #[test]
    fn r2089000_an_instance_used_bounced_and_used_again_counts_once() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [{ "def": VANILLA, "created": true }, FLOOD] },
        }));
        let mut reader = SummaryReader::new(s.state());
        let turn = s.state().turn;
        let vanilla = created_in_hand(&s, P1, VANILLA);
        step(&mut reader, &mut s, |s| {
            s.play(vanilla.clone(), json!({}));
        });
        step(&mut reader, &mut s, |s| {
            s.play(FLOOD, json!({}));
        });
        assert_eq!(s.card(vanilla.clone()).zone, Zone::Hand { player: P1 });
        step(&mut reader, &mut s, |s| {
            s.play(vanilla, json!({}));
        });

        let p1 = reader.seat(P1, vec![FLOOD.to_string()]);
        assert_eq!(p1.played, [FLOOD]);
        assert_eq!(p1.created_used, Some(vec![used(VANILLA, turn, false)]));
        assert_eq!(cards(&p1.created_played), [VANILLA, VANILLA]);
        assert_eq!(p1.created_unused, None);
    }

    fn act(state: &GameState, log: &mut Vec<Action>, body: ActionBody, player: PlayerId) -> GameState {
        let action = Action::new(body, player, format!("created-coin-{}", log.len()));
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        result.state
    }

    #[test]
    fn r2089000_the_coin_never_played_is_the_second_seat_s_created_card_not_used() {
        register_all();
        let units: Vec<String> = pool()
            .into_iter()
            .filter(|id| CATALOG.get(id).is_some_and(|def| def.type_ == CardType::Unit))
            .take(DECK_SIZE as usize)
            .collect();
        let decks = (units.clone(), units);
        let seed = "created-coin".to_string();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.clone(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act(&state, &mut log, ActionBody::Mulligan { keep }, player);
        }
        assert!(
            state
                .players
                .p2
                .hand
                .iter()
                .any(|card| card.def_id == COIN_DEF_ID)
        );
        state = act(&state, &mut log, ActionBody::Concede, P1);
        assert!(state.result.is_some());

        let Some(summary) = summarize_game(&FoldArgs {
            seed,
            decks,
            log,
            ..FoldArgs::default()
        }) else {
            panic!("a finished game has a summary");
        };
        let p2 = &summary.seats.p2;
        assert_eq!(p2.created_unused, Some(vec![COIN_DEF_ID.to_string()]));
        assert_eq!(p2.created_used, None);
        assert!(!p2.opening.iter().any(|id| id == COIN_DEF_ID));
        assert_eq!(summary.seats.p1.created_unused, None);
    }
}
