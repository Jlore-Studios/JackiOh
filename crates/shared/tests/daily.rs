//! The daily game's clock and its notification decision (SPEC §9.13, #3727), pure: who holds the move
//! after each kind of action, the deadline, an expiry, the reminder and the gap between emails. The
//! games are real ones under the shipped catalog; time is a number the tests move themselves.

use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, GameState, Rng, SandboxOp, SpawnZone, legal_actions,
};
use jackioh_shared::config::{
    DAILY_EMAIL_MIN_GAP_HOURS, DAILY_REMINDER_HOURS, DAILY_TIME_PER_MOVE_DAYS, MS_PER_DAY, MS_PER_HOUR,
};
use jackioh_shared::host::daily::{
    DailyClock, daily_allows, daily_board, daily_expired, daily_holders, daily_legal, daily_sync,
    is_daily_time,
};
use jackioh_shared::notify::{NotifyInput, WaitingGame, decide_notify};

/// The epoch millisecond the games start at.
const NOW: i64 = 1_700_000_000_000;

/// Meditative+ #20 KY's Competition, whose question sequence R1563006 times in a live game.
const COMPETITION: &str = "meditativeplus-020";

fn deck(salt: &str) -> Vec<String> {
    jackioh_ai::build_ai_deck(
        &mut Rng::new(&format!("daily:{salt}"), 0),
        DECK_SIZE,
        &jackioh_ai::AiDeckOptions {
            banned: Some(vec![]),
            gated_sets: Some(vec![]),
            ..jackioh_ai::AiDeckOptions::default()
        },
    )
}

/// A begun game, both mulligans open (R265). `sandbox` lets a test hand a player the cards and the
/// mana a rule needs (R923000).
fn begun(seed: &str, sandbox: bool) -> GameState {
    jackioh_cards::register_all();
    let options: CreateGameOptions = serde_json::from_value(serde_json::json!({
        "seed": seed,
        "decks": [deck(&format!("{seed}-a")), deck(&format!("{seed}-b"))],
        "sandbox": sandbox,
    }))
    .expect("create options");
    jackioh_engine::begin_game(&jackioh_engine::create_game(&options)).state
}

/// Applies `body` for `player` under a nonce no earlier action of the game used (a replayed nonce is
/// the first answer again, §9.3): one past the last applied.
fn act(state: &GameState, player: PlayerId, body: ActionBody) -> GameState {
    let last = state
        .applied
        .last()
        .and_then(|entry| entry.nonce.strip_prefix("test-"))
        .and_then(|count| count.parse::<u32>().ok())
        .unwrap_or(0);
    let nonce = format!("test-{}", last + 1);
    let result = jackioh_engine::reduce(state, &Action::new(body.clone(), player, nonce));
    assert!(result.error.is_none(), "{body:?}: {:?}", result.error);
    result.state
}

/// `player` keeps their opening hand.
fn mulligan(state: &GameState, player: PlayerId) -> GameState {
    let keep = legal_actions(state, player)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Mulligan { .. }))
        .expect("a mulligan is owed");
    act(state, player, keep)
}

/// A clock at the shortest time per move, synced over `state` at `now`.
fn clock_over(state: &GameState, now: i64) -> DailyClock {
    daily_sync(&DailyClock::new(DAILY_TIME_PER_MOVE_DAYS[0]), state, now)
}

fn window() -> i64 {
    i64::from(DAILY_TIME_PER_MOVE_DAYS[0]) * MS_PER_DAY
}

/// The game after both mulligans, on the first turn.
fn first_turn(seed: &str, sandbox: bool) -> GameState {
    let state = begun(seed, sandbox);
    let state = mulligan(&state, P1);
    mulligan(&state, P2)
}

#[test]
fn r3727002_both_seats_hold_the_mulligan_each_with_its_own_window() {
    let state = begun("daily-r3727002", false);
    assert_eq!(daily_holders(&state), vec![P1, P2]);
    let clock = clock_over(&state, NOW);
    assert_eq!(clock.holds.len(), 2);
    for hold in &clock.holds {
        assert_eq!((hold.since, hold.deadline), (NOW, NOW + window()));
    }

    // One answers: the other keeps its own window, deadline and all.
    let later = NOW + window() / 2;
    let answered = mulligan(&state, P2);
    let clock = daily_sync(&clock, &answered, later);
    assert_eq!(clock.holds.len(), 1);
    assert_eq!(clock.holds[0].player, P1);
    assert_eq!(clock.holds[0].deadline, NOW + window());

    // Both answered: the first turn starts with a whole window of its own, not the mulligan's.
    let first = mulligan(&answered, P1);
    let clock = daily_sync(&clock, &first, later + 1);
    assert_eq!(daily_holders(&first), vec![first.active]);
    assert_eq!(clock.holds[0].since, later + 1);
    assert_eq!(clock.holds[0].deadline, later + 1 + window());

    // Two windows that run out at the same moment expire in seat order.
    let both = clock_over(&state, NOW);
    assert_eq!(daily_expired(&both, NOW + window()), Some(P1));
}

#[test]
fn end_turn_gives_the_next_player_a_full_window() {
    let state = first_turn("daily-end-turn", false);
    let active = state.active;
    let started = NOW + MS_PER_HOUR;
    let clock = clock_over(&state, started);
    assert_eq!(
        clock.hold_of(active).map(|hold| hold.deadline),
        Some(started + window())
    );

    // Actions inside the turn keep its window.
    let emote_free = daily_sync(&clock, &state, started + 5 * MS_PER_HOUR);
    assert_eq!(emote_free, clock);

    // End turn passes the move: the other player gets the whole time again, unused time not kept.
    let ended = act(&state, active, ActionBody::EndTurn);
    let sent = started + 2 * MS_PER_HOUR;
    let clock = daily_sync(&clock, &ended, sent);
    let next = jackioh_engine::opponent_of(active);
    assert_eq!(daily_holders(&ended), vec![next]);
    assert_eq!(clock.holds.len(), 1);
    assert_eq!(
        (clock.holds[0].since, clock.holds[0].deadline),
        (sent, sent + window())
    );

    // And back: a fresh window again, not what was left of the first.
    let back = act(&ended, next, ActionBody::EndTurn);
    let returned = sent + MS_PER_HOUR;
    let clock = daily_sync(&clock, &back, returned);
    assert_eq!(
        clock.hold_of(active).map(|hold| hold.deadline),
        Some(returned + window())
    );
    assert_eq!(clock.next_deadline(), Some(returned + window()));
}

/// R677: a Glitch's swap hands the seat holding the move to the other account mid-turn. That account
/// gets a whole window from the swap, not what was left of the one who swapped.
#[test]
fn r3727001_a_seat_swap_hands_the_move_to_the_other_account_with_a_fresh_window() {
    let state = first_turn("daily-swap", false);
    let active = state.active;
    let clock = clock_over(&state, NOW);
    let late = NOW + window() - MS_PER_HOUR;
    assert_eq!(daily_sync(&clock, &state, late), clock, "no swap, the same hold");

    let mut swapped = state.clone();
    swapped.seat_swaps = Some(1);
    let clock = daily_sync(&clock, &swapped, late);
    assert_eq!(daily_holders(&swapped), vec![active]);
    assert_eq!(
        clock.hold_of(active).map(|hold| (hold.since, hold.deadline)),
        Some((late, late + window()))
    );
    assert_eq!(daily_expired(&clock, NOW + window()), None);

    // And a swap back hands it on again, fresh.
    let mut back = swapped.clone();
    back.seat_swaps = Some(2);
    let again = late + MS_PER_HOUR;
    let clock = daily_sync(&clock, &back, again);
    assert_eq!(
        clock.hold_of(active).map(|hold| hold.deadline),
        Some(again + window())
    );
}

#[test]
fn r3727000_an_expired_move_loses_for_its_holder() {
    let state = first_turn("daily-r3727000", false);
    let clock = clock_over(&state, NOW);
    let deadline = NOW + window();
    assert_eq!(daily_expired(&clock, deadline - 1), None, "a move with time left");
    assert_eq!(daily_expired(&clock, deadline), Some(state.active));
    assert_eq!(daily_expired(&clock, deadline + MS_PER_DAY), Some(state.active));
    // The player waiting holds nothing, so nothing of theirs runs out.
    assert!(!clock.has_move(jackioh_engine::opponent_of(state.active)));

    // A game over holds no move, so nothing expires.
    let over = act(&state, state.active, ActionBody::Concede);
    let clock = daily_sync(&clock, &over, NOW + 1);
    assert!(clock.holds.is_empty());
    assert_eq!(daily_expired(&clock, deadline + MS_PER_DAY), None);
    assert_eq!(daily_board(&over).result, over.result);
}

#[test]
fn r3727001_a_prompt_for_the_other_player_hands_over_the_move_with_a_fresh_window() {
    let state = first_turn("daily-r3727001", true);
    let active = state.active;
    let other = jackioh_engine::opponent_of(active);
    let clock = clock_over(&state, NOW);

    // A prompt the active player's own action opens for the other player (a trap that fires on its
    // owner's opponent's turn, say). The rule reads only who holds the prompt, so the test hands the
    // other player a prompt that the sequence below opens for the active one.
    let mut asked = competition_cast(&state);
    asked.pending.as_mut().expect("a question is open").player_id = other;
    let handed = NOW + 3 * MS_PER_HOUR;
    assert_eq!(daily_holders(&asked), vec![other]);
    let clock = daily_sync(&clock, &asked, handed);
    assert_eq!(clock.holds.len(), 1);
    assert_eq!(clock.holds[0].player, other);
    assert_eq!(clock.holds[0].deadline, handed + window(), "a fresh window");

    // Its answer hands the move back, fresh again.
    let answered = NOW + 4 * MS_PER_HOUR;
    let clock = daily_sync(&clock, &state, answered);
    assert_eq!(clock.holds[0].player, active);
    assert_eq!(clock.holds[0].deadline, answered + window());
}

/// The active player (in a sandbox game) casts KY's Competition, which opens its question sequence's
/// first question for them.
fn competition_cast(state: &GameState) -> GameState {
    let player = state.active;
    let tools = [
        SandboxOp::SetMana {
            player,
            current: Some(10),
            max: Some(10),
        },
        SandboxOp::Spawn {
            player,
            def_id: COMPETITION.to_string(),
            radiant: None,
            to: SpawnZone::Hand,
        },
    ];
    let mut state = state.clone();
    for op in tools {
        state = act(&state, player, ActionBody::Sandbox { op });
    }
    let card = state.players[player]
        .hand
        .iter()
        .find(|card| card.def_id == COMPETITION)
        .map(|card| card.id.clone())
        .expect("the spawned card is in hand");
    let play = legal_actions(&state, player)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Play { instance_id, .. } if *instance_id == card))
        .expect("the card can be cast");
    act(&state, player, play)
}

#[test]
fn r3727003_a_question_sequence_arms_no_window_of_its_own() {
    let state = first_turn("daily-r3727003", true);
    let active = state.active;
    let clock = clock_over(&state, NOW);
    let deadline = NOW + window();

    let asked = competition_cast(&state);
    let question = asked.pending.clone().expect("the first question is open");
    assert_eq!(question.player_id, active);
    assert_eq!(question.streak, Some(0));
    // The question is the active player's, inside their move: the move's deadline is its window.
    let clock = daily_sync(&clock, &asked, NOW + MS_PER_HOUR);
    assert_eq!(clock.holds.len(), 1);
    assert_eq!(clock.holds[0].deadline, deadline);

    // A right answer asks the next question, and the deadline still does not move.
    let correct = jackioh_engine::answer_key_of(&question.resume.data).expect("the question's key");
    let right = question
        .options
        .iter()
        .find(|option| option.key == format!("mode:{correct}"))
        .expect("the right option")
        .selection
        .clone();
    let next = act(
        &asked,
        active,
        ActionBody::Answer {
            choice_id: question.id.clone(),
            selection: vec![right],
        },
    );
    assert_eq!(next.pending.as_ref().and_then(|prompt| prompt.streak), Some(1));
    let clock = daily_sync(&clock, &next, NOW + 2 * MS_PER_HOUR);
    assert_eq!(clock.holds[0].deadline, deadline);
    assert_eq!(daily_expired(&clock, deadline), Some(active));
}

#[test]
fn r3727006_a_standing_draw_offer_hands_the_move_to_the_other_player() {
    let state = first_turn("daily-r3727006", false);
    let active = state.active;
    let other = jackioh_engine::opponent_of(active);
    let clock = clock_over(&state, NOW);

    let offered = act(&state, active, ActionBody::OfferDraw);
    let handed = NOW + MS_PER_HOUR;
    assert_eq!(daily_holders(&offered), vec![other]);
    let clock = daily_sync(&clock, &offered, handed);
    assert_eq!(clock.holds[0].player, other);
    assert_eq!(clock.holds[0].deadline, handed + window());

    // The offerer may only concede until it is answered: ending the turn would let the offer lapse
    // (R269) before an absent opponent ever saw it.
    assert!(!daily_allows(&offered, active, &ActionBody::EndTurn));
    assert!(daily_allows(&offered, active, &ActionBody::Concede));
    let legal = daily_legal(&offered, active, legal_actions(&offered, active));
    assert_eq!(legal, vec![ActionBody::Concede]);
    // The other player answers it.
    assert!(daily_allows(
        &offered,
        other,
        &ActionBody::AnswerDraw { accept: false }
    ));

    // A decline hands the move back to the offerer, whose turn goes on with a fresh window.
    let declined = act(&offered, other, ActionBody::AnswerDraw { accept: false });
    let answered = NOW + 2 * MS_PER_HOUR;
    let clock = daily_sync(&clock, &declined, answered);
    assert_eq!(clock.holds[0].player, active);
    assert_eq!(clock.holds[0].deadline, answered + window());
    assert!(daily_allows(&declined, active, &ActionBody::EndTurn));
}

#[test]
fn a_seat_holding_no_move_may_only_concede() {
    let state = first_turn("daily-no-move", false);
    let active = state.active;
    let other = jackioh_engine::opponent_of(active);
    assert_eq!(daily_holders(&state), vec![active]);
    // The engine takes `setAutoEndTurn` from either seat at any time; a daily game takes it only from
    // the holder, so the other player never appends a row that makes the holder's move stale.
    let auto = ActionBody::SetAutoEndTurn { enabled: true };
    assert!(daily_allows(&state, active, &auto));
    assert!(!daily_allows(&state, other, &auto));
    assert!(!daily_allows(&state, other, &ActionBody::EndTurn));
    assert!(daily_allows(&state, other, &ActionBody::Concede));
    assert_eq!(
        daily_legal(&state, other, legal_actions(&state, other)),
        vec![ActionBody::Concede]
    );
}

#[test]
fn only_the_four_times_per_move_are_daily_times() {
    for days in DAILY_TIME_PER_MOVE_DAYS {
        assert!(is_daily_time(days));
    }
    for days in [0, 2, 5, 30, -1] {
        assert!(!is_daily_time(days), "{days}");
    }
}

#[test]
fn the_board_a_list_shows_is_the_turn_and_both_lives() {
    let state = first_turn("daily-board", false);
    let board = daily_board(&state);
    assert_eq!(board.turn, state.turn);
    assert_eq!(board.life.p1, state.players.p1.hero.health);
    assert_eq!(board.life.p2, state.players.p2.hero.health);
    assert!(!board.seats_swapped);
    assert_eq!(board.result, None);
}

// The notification decision.

const HOUR: i64 = MS_PER_HOUR;

fn waiting(id: &str, since: i64) -> WaitingGame {
    WaitingGame {
        match_id: id.to_string(),
        since,
        deadline: since + window(),
    }
}

fn account(waiting: Vec<WaitingGame>) -> NotifyInput {
    NotifyInput {
        email_on: true,
        has_email: true,
        push_devices: 0,
        last_email_at: None,
        last_push_at: None,
        waiting,
    }
}

#[test]
fn notify_emails_a_new_waiting_game() {
    let plan = decide_notify(
        &account(vec![waiting("b", NOW + HOUR), waiting("a", NOW)]),
        NOW + HOUR,
    );
    assert!(plan.email);
    assert!(!plan.push, "no device");
    // Every game waiting, the nearest deadline first.
    let ids: Vec<&str> = plan.games.iter().map(|game| game.match_id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);

    // Sent: nothing new until another game starts waiting.
    let mut input = account(vec![waiting("a", NOW)]);
    input.last_email_at = Some(NOW + HOUR);
    let later = NOW + (DAILY_EMAIL_MIN_GAP_HOURS + 1) * HOUR;
    assert!(!decide_notify(&input, later).email);
    input.waiting.push(waiting("c", later - HOUR));
    assert!(decide_notify(&input, later).email);

    // Nothing waiting, nothing sent.
    assert_eq!(decide_notify(&account(Vec::new()), NOW).games, Vec::new());
    assert!(!decide_notify(&account(Vec::new()), NOW).email);
}

#[test]
fn notify_reminds_once_before_the_deadline() {
    let game = waiting("a", NOW);
    let reminder = game.deadline - DAILY_REMINDER_HOURS * HOUR;
    let mut input = account(vec![game]);
    input.last_email_at = Some(NOW);
    assert!(!decide_notify(&input, reminder - 1).email, "not yet");
    assert!(decide_notify(&input, reminder).email, "the reminder");
    input.last_email_at = Some(reminder);
    assert!(!decide_notify(&input, reminder + 3 * HOUR).email, "once only");
    assert!(
        !decide_notify(&input, reminder + (DAILY_EMAIL_MIN_GAP_HOURS + 1) * HOUR).email,
        "once only, gap or no gap"
    );
}

#[test]
fn notify_holds_the_email_gap() {
    let mut input = account(vec![waiting("a", NOW)]);
    input.last_email_at = Some(NOW - HOUR);
    input.waiting.push(waiting("b", NOW + HOUR));
    let gap = DAILY_EMAIL_MIN_GAP_HOURS * HOUR;
    assert!(!decide_notify(&input, NOW + HOUR).email, "inside the gap");
    assert!(!decide_notify(&input, NOW - HOUR + gap - 1).email);
    let plan = decide_notify(&input, NOW - HOUR + gap);
    assert!(plan.email, "the gap passed: the held game goes out");
    assert_eq!(plan.games.len(), 2);
}

#[test]
fn notify_sends_no_email_when_off_or_without_an_address() {
    let mut off = account(vec![waiting("a", NOW)]);
    off.email_on = false;
    assert!(!decide_notify(&off, NOW).email);
    let mut none = account(vec![waiting("a", NOW)]);
    none.has_email = false;
    assert!(!decide_notify(&none, NOW).email);
}

#[test]
fn notify_pushes_without_a_gap() {
    let mut input = account(vec![waiting("a", NOW)]);
    input.email_on = false;
    input.push_devices = 2;
    assert!(decide_notify(&input, NOW).push);
    input.last_push_at = Some(NOW);
    assert!(!decide_notify(&input, NOW + HOUR).push, "nothing new");
    // A second game an hour later pushes at once, with no gap held.
    input.waiting.push(waiting("b", NOW + HOUR));
    assert!(decide_notify(&input, NOW + HOUR).push);
    // And the reminder pushes too.
    let mut reminded = account(vec![waiting("a", NOW)]);
    reminded.push_devices = 1;
    reminded.last_push_at = Some(NOW);
    let reminder = NOW + window() - DAILY_REMINDER_HOURS * HOUR;
    assert!(decide_notify(&reminded, reminder).push);
}
