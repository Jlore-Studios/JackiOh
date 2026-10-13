//! BUILD M7-T1's fake-timer tests for the match clock (SPEC §9.5, R79).
//!
//! One test per clock path, on tokio's paused clock (`start_paused = true`): `advance(ms)` fires the
//! due timers in deadline order, so the assertions are on the millisecond rather than a tolerance.
//! Every duration is derived from the server's config (`crate::config`), so these tests state R79's
//! numbers nowhere.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_server::actor::clock::{create_match_clock, initial_clocks, match_ceiling_at};
use jackioh_server::actor::contracts::{ClockExpiry, ClockView, CreateMatchClockInput, MatchClock};
use jackioh_server::app::now_ms;
use jackioh_server::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MULLIGAN_CLOCK_SECONDS, PROMPT_CLOCK_SECONDS,
    TURN_CLOCK_SECONDS,
};
use serde_json::{Value, json};

/// Unit conversion only: R79's durations come from the config below.
const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;

/// How many times `settle` yields: enough for every task woken at this instant to run.
const SETTLE_YIELDS: usize = 8;

/// A config number as milliseconds' arithmetic wants it, whatever integer type `config.rs` gives it.
fn int(value: impl Into<i64>) -> i64 {
    value.into()
}

/// The server's own epoch-millisecond clock, the one the match clock reads.
fn now() -> i64 {
    now_ms()
}

/// Lets every task woken at this instant run: the clock's timers report from their own tasks.
async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// Runs every callback due within `ms`, in deadline order, advancing `now` as it goes.
async fn advance(ms: i64) {
    tokio::time::sleep(Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0))).await;
    settle().await;
}

struct Harness {
    expiries: Arc<Mutex<Vec<ClockExpiry>>>,
    started_at: i64,
    clock: MatchClock,
    turn_ms: i64,
    prompt_ms: i64,
    mulligan_ms: i64,
    grace_ms: i64,
    ceiling_ms: i64,
}

impl Harness {
    /// Every expiry the clock has reported, in order.
    fn expiries(&self) -> Vec<ClockExpiry> {
        self.expiries.lock().expect("the expiry list").clone()
    }

    /// `clock.snapshot()` as its JSON (`MatchClocks`' shape, the one `matches.clocks` stores).
    fn snapshot(&self) -> Value {
        serde_json::to_value(self.clock.snapshot()).expect("MatchClocks serialises")
    }

    /// One deadline of the snapshot (`turnDeadline`, `promptDeadline`), null as `None`.
    fn deadline(&self, key: &str) -> Option<i64> {
        self.snapshot()[key].as_i64()
    }

    /// One seat's grace deadline, null as `None`.
    fn grace(&self, player: PlayerId) -> Option<i64> {
        self.snapshot()["graceDeadline"][player.as_str()].as_i64()
    }
}

/// `started_offset_ms`: how long before now the match started.
fn harness(started_offset_ms: i64) -> Harness {
    let expiries: Arc<Mutex<Vec<ClockExpiry>>> = Arc::new(Mutex::new(Vec::new()));
    let started_at = now() - started_offset_ms;
    let sink = Arc::clone(&expiries);
    let clock = create_match_clock(CreateMatchClockInput {
        started_at,
        on_expire: Arc::new(move |expiry: ClockExpiry| {
            sink.lock().expect("the expiry list").push(expiry);
        }),
    });
    Harness {
        expiries,
        started_at,
        clock,
        turn_ms: int(TURN_CLOCK_SECONDS) * SECOND,
        prompt_ms: int(PROMPT_CLOCK_SECONDS) * SECOND,
        mulligan_ms: int(MULLIGAN_CLOCK_SECONDS) * SECOND,
        grace_ms: int(DISCONNECT_GRACE_SECONDS) * SECOND,
        ceiling_ms: int(MATCH_CEILING_MINUTES) * MINUTE,
    }
}

fn view() -> ClockView {
    ClockView {
        turn: 1,
        active: P1,
        pending_for: None,
        mulligan_owed: vec![],
        over: false,
    }
}

/// R265: setup, turn 0 with p1 as its placeholder active seat, and these seats still owing.
fn mulligan_window(owed: &[PlayerId]) -> ClockView {
    ClockView {
        turn: 0,
        active: P1,
        mulligan_owed: owed.to_vec(),
        ..view()
    }
}

mod match_clock {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r79_expires_the_turn_clock_after_turn_clock_seconds_and_names_the_active_player() {
        let h = harness(0);
        h.clock.sync(&view());

        advance(h.turn_ms - 1).await;
        assert_eq!(h.expiries(), vec![]);

        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[tokio::test(start_paused = true)]
    async fn resets_the_turn_clock_when_the_turn_changes() {
        let h = harness(0);
        h.clock.sync(&view());
        advance(h.turn_ms - SECOND).await;
        assert_eq!(h.expiries(), vec![]);

        // R79: "The turn clock belongs to the active player" — the new turn starts from full.
        h.clock.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        assert_eq!(h.clock.remaining_for(P2), Some(h.turn_ms));
        assert_eq!(h.clock.remaining_for(P1), None);

        advance(h.turn_ms - 1).await;
        assert_eq!(h.expiries(), vec![]);
        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Turn { player: P2 }]);
    }

    #[tokio::test(start_paused = true)]
    async fn r79_a_prompt_held_by_the_non_active_player_pauses_the_turn_clock_and_only_its_own_clock_expires()
    {
        let h = harness(0);
        h.clock.sync(&view());

        // Spend most of p1's turn, then a trap fires and asks p2 a question.
        let spent = h.turn_ms - 15 * SECOND;
        advance(spent).await;
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms - spent));

        h.clock.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), Some(now() + h.prompt_ms));

        // The prompt outlives what was left of p1's turn: 60 s spent + a 30 s prompt is past the 75 s
        // turn clock, so an unpaused turn clock would have fired in here.
        advance(h.prompt_ms).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Prompt { player: P2 }]);
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms - spent));

        // R79: when the pause ends the turn clock resumes with the time it had left, not from full.
        h.clock.sync(&view());
        assert_eq!(h.deadline("turnDeadline"), Some(now() + (h.turn_ms - spent)));
        advance(h.turn_ms - spent - 1).await;
        assert_eq!(h.expiries().len(), 1);
        advance(1).await;
        assert_eq!(
            h.expiries(),
            vec![
                ClockExpiry::Prompt { player: P2 },
                ClockExpiry::Turn { player: P1 }
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_pause_the_turn_clock_for_a_prompt_held_by_the_active_player() {
        let h = harness(0);
        // §2.5: "When the active player's turn clock runs out, their open prompts are answered by the
        // AI policy and the turn ends" — their own prompt gets no second, shorter deadline.
        h.clock.sync(&ClockView {
            pending_for: Some(P1),
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), Some(now() + h.turn_ms));
        assert_eq!(h.deadline("promptDeadline"), None);

        advance(h.prompt_ms).await;
        assert_eq!(h.expiries(), vec![]);

        advance(h.turn_ms - h.prompt_ms).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[tokio::test(start_paused = true)]
    async fn counts_disconnect_grace_per_player_and_reports_whose_it_was() {
        let h = harness(0);
        h.clock.start_grace(P2, None);
        assert_eq!(
            h.snapshot()["graceDeadline"],
            json!({ "p1": null, "p2": now() + h.grace_ms })
        );

        advance(h.grace_ms - 1).await;
        assert_eq!(h.expiries(), vec![]);
        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Grace { player: P2 }]);
        assert_eq!(h.grace(P2), None);
    }

    #[tokio::test(start_paused = true)]
    async fn r147_keeps_the_first_deadline_when_a_second_grace_starts_so_a_flapping_socket_cannot_extend_it()
    {
        let h = harness(0);
        h.clock.start_grace(P1, None);
        let deadline = h.grace(P1);
        assert_eq!(deadline, Some(now() + h.grace_ms));

        // The socket flaps: another drop arrives while the first countdown is still running. Re-arming
        // here would hand the player a fresh window every time they bounced, and the match would stall
        // for as long as they kept it up.
        advance(h.grace_ms - SECOND).await;
        h.clock.start_grace(P1, None);
        assert_eq!(h.grace(P1), deadline);
        assert_eq!(h.expiries(), vec![]);

        // It expires at the deadline the *first* drop set, not a second later.
        advance(SECOND).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Grace { player: P1 }]);

        // And a grace that has run out can be started again: the guard is about extending a live
        // countdown, not about refusing the next one.
        h.clock.start_grace(P1, None);
        assert_eq!(h.grace(P1), Some(now() + h.grace_ms));
    }

    #[tokio::test(start_paused = true)]
    async fn r744_starts_a_grace_at_a_stored_deadline_holds_it_to_a_fresh_window_and_fires_one_already_past_at_once()
     {
        let h = harness(0);

        // A deadline the restart left stored is kept, not replaced by a fresh window.
        let stored = now() + h.grace_ms / 2;
        h.clock.start_grace(P1, Some(stored));
        assert_eq!(h.grace(P1), Some(stored));
        advance(h.grace_ms / 2 - 1).await;
        assert_eq!(h.expiries(), vec![]);
        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Grace { player: P1 }]);

        // A stored deadline later than a fresh window would end is held to it (R147): a rebuild can
        // shorten a grace, never extend one.
        h.clock.start_grace(P2, Some(now() + h.grace_ms * 2));
        assert_eq!(h.grace(P2), Some(now() + h.grace_ms));

        // One that already ran out while the server was down fires at once.
        h.clock.start_grace(P1, Some(now() - 1));
        advance(0).await;
        assert_eq!(h.expiries().last(), Some(&ClockExpiry::Grace { player: P1 }));
        assert_eq!(
            h.expiries()
                .iter()
                .filter(|expiry| matches!(expiry, ClockExpiry::Grace { .. }))
                .count(),
            2
        );
    }

    #[tokio::test(start_paused = true)]
    async fn cancels_grace_when_the_player_returns_inside_the_window() {
        let h = harness(0);
        h.clock.start_grace(P1, None);
        advance(h.grace_ms - SECOND).await;
        h.clock.clear_grace(P1);
        assert_eq!(h.grace(P1), None);

        advance(h.grace_ms * 2).await;
        assert_eq!(
            h.expiries()
                .iter()
                .filter(|expiry| matches!(expiry, ClockExpiry::Grace { .. }))
                .count(),
            0
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_turn_clock_running_while_a_player_is_disconnected_9_5() {
        let h = harness(0);
        h.clock.sync(&view());
        let deadline = h.deadline("turnDeadline");

        h.clock.start_grace(P2, None);
        // §9.5: "The clock keeps running while a player is disconnected."
        assert_eq!(h.deadline("turnDeadline"), deadline);

        advance(h.turn_ms).await;
        assert_eq!(
            h.expiries(),
            vec![
                ClockExpiry::Grace { player: P2 },
                ClockExpiry::Turn { player: P1 }
            ]
        );
        assert!(h.grace_ms < h.turn_ms);
    }

    #[tokio::test(start_paused = true)]
    async fn r79_fires_the_ceiling_once_measured_from_started_at() {
        let elapsed = 10 * MINUTE;
        let h = harness(elapsed);
        assert_eq!(
            h.snapshot()["ceilingAt"].as_i64(),
            Some(match_ceiling_at(h.started_at))
        );
        assert_eq!(
            h.snapshot()["ceilingAt"].as_i64(),
            Some(h.started_at + h.ceiling_ms)
        );

        advance(h.ceiling_ms - elapsed - 1).await;
        assert_eq!(h.expiries(), vec![]);
        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Ceiling]);

        // Once: a second ceiling's worth fires nothing.
        advance(h.ceiling_ms).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Ceiling]);
    }

    #[tokio::test(start_paused = true)]
    async fn exposes_every_deadline_through_snapshot_and_counts_down_through_remaining_for() {
        let h = harness(0);
        assert_eq!(
            h.snapshot(),
            json!({
                "turnDeadline": null,
                "promptDeadline": null,
                "graceDeadline": { "p1": null, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );

        h.clock.sync(&view());
        assert_eq!(h.deadline("turnDeadline"), Some(now() + h.turn_ms));

        advance(10 * SECOND).await;
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
        assert_eq!(h.clock.remaining_for(P2), None);

        h.clock.start_grace(P1, None);
        h.clock.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(
            h.snapshot(),
            json!({
                "turnDeadline": null,
                "promptDeadline": now() + h.prompt_ms,
                "graceDeadline": { "p1": now() + h.grace_ms, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );
        // The paused turn clock still reports its banked remainder; the prompt holder reports theirs.
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
        assert_eq!(h.clock.remaining_for(P2), Some(h.prompt_ms));

        advance(SECOND).await;
        assert_eq!(h.clock.remaining_for(P2), Some(h.prompt_ms - SECOND));
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_everything_once_the_match_is_over() {
        let h = harness(0);
        h.clock.sync(&view());
        h.clock.start_grace(P1, None);
        h.clock.start_grace(P2, None);
        assert!(h.deadline("turnDeadline").is_some());
        assert!(h.grace(P1).is_some() && h.grace(P2).is_some());

        h.clock.sync(&ClockView { over: true, ..view() });
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.snapshot()["graceDeadline"], json!({ "p1": null, "p2": null }));
        assert_eq!(h.clock.remaining_for(P1), None);

        // A late sync must never re-arm a deadline on a finished match.
        h.clock.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), None);
        advance(60 * MINUTE).await;
        assert_eq!(h.expiries(), vec![]);
    }

    #[tokio::test(start_paused = true)]
    async fn stop_cancels_every_timer_and_is_idempotent() {
        let h = harness(0);
        h.clock.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        h.clock.start_grace(P1, None);
        assert!(h.deadline("promptDeadline").is_some());
        assert!(h.grace(P1).is_some());

        h.clock.stop();
        h.clock.stop();
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.grace(P1), None);
        advance(60 * MINUTE).await;
        assert_eq!(h.expiries(), vec![]);
    }

    #[tokio::test(start_paused = true)]
    async fn r268_the_mulligan_clock_runs_one_deadline_for_both_seats_at_once_and_no_turn_clock_runs_under_it()
     {
        let h = harness(0);
        h.clock.sync(&mulligan_window(&[P1, P2]));

        let deadline = now() + h.mulligan_ms;
        // One deadline, reported as the prompt deadline both clients render; setup is nobody's turn.
        assert_eq!(h.deadline("promptDeadline"), Some(deadline));
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.clock.remaining_for(P1), Some(h.mulligan_ms));
        assert_eq!(h.clock.remaining_for(P2), Some(h.mulligan_ms));

        // A turn clock's worth of waiting is not what ends it: only the mulligan deadline is armed.
        advance(h.mulligan_ms - 1).await;
        assert_eq!(h.expiries(), vec![]);
        assert_eq!(h.clock.remaining_for(P1), Some(1));
        assert_eq!(h.clock.remaining_for(P2), Some(1));
        advance(1).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Mulligan]);
        assert!(h.mulligan_ms < h.turn_ms);

        // It fires once: the window is still open until the actor's timeouts land, and a `sync` that
        // still reports it arms nothing new.
        h.clock.sync(&mulligan_window(&[P2]));
        advance(h.mulligan_ms * 2).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Mulligan]);
    }

    #[tokio::test(start_paused = true)]
    async fn r268_one_seat_answering_neither_re_arms_nor_extends_the_mulligan_clock() {
        let h = harness(0);
        h.clock.sync(&mulligan_window(&[P1, P2]));
        let deadline = h.deadline("promptDeadline");

        // p2 answers first, well into the window. The seat still owing gets what was left, not a fresh
        // window, and the seat that answered still reads the same countdown: it is waiting on it.
        advance(20 * SECOND).await;
        h.clock.sync(&mulligan_window(&[P1]));
        assert_eq!(h.deadline("promptDeadline"), deadline);
        assert_eq!(h.clock.remaining_for(P1), Some(h.mulligan_ms - 20 * SECOND));
        assert_eq!(h.clock.remaining_for(P2), Some(h.mulligan_ms - 20 * SECOND));

        advance(h.mulligan_ms - 20 * SECOND).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Mulligan]);
    }

    #[tokio::test(start_paused = true)]
    async fn r268_the_window_closing_cancels_the_mulligan_clock_and_turn_1_starts_p1s_turn_clock_from_full() {
        let h = harness(0);
        h.clock.sync(&mulligan_window(&[P1, P2]));
        advance(10 * SECOND).await;
        h.clock.sync(&mulligan_window(&[P2]));

        // The second answer resolves both and starts turn 1 (R265, §2.1).
        advance(5 * SECOND).await;
        h.clock.sync(&ClockView {
            turn: 1,
            active: P1,
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.deadline("turnDeadline"), Some(now() + h.turn_ms));
        assert_eq!(h.clock.remaining_for(P1), Some(h.turn_ms));
        assert_eq!(h.clock.remaining_for(P2), None);

        // The mulligan deadline passes with nothing to report, and the turn clock is p1's.
        advance(h.mulligan_ms).await;
        assert_eq!(h.expiries(), vec![]);
        advance(h.turn_ms - h.mulligan_ms).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[tokio::test(start_paused = true)]
    async fn r268_a_cards_question_during_setup_is_still_timed_by_r79_on_either_side_of_the_window() {
        let h = harness(0);

        // A cast-on-draw card in p2's opening deal asks p2 something before the mulligans open (§2.4):
        // the non-active holder's prompt clock, as R79 has it.
        h.clock.sync(&ClockView {
            turn: 0,
            active: P1,
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), Some(now() + h.prompt_ms));

        // Answered, and the window opens: the prompt clock goes and the one mulligan deadline takes over.
        advance(5 * SECOND).await;
        h.clock.sync(&mulligan_window(&[P1, P2]));
        assert_eq!(h.deadline("promptDeadline"), Some(now() + h.mulligan_ms));
        advance(h.prompt_ms).await;
        assert_eq!(h.expiries(), vec![]);

        // Both answered, and a replacement draw's cast asks p2 during the resolution (phase still
        // mulligan, turn 0, nobody owing): R79's prompt clock again, and the mulligan clock is gone.
        h.clock.sync(&ClockView {
            turn: 0,
            active: P1,
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), Some(now() + h.prompt_ms));
        advance(h.prompt_ms).await;
        assert_eq!(h.expiries(), vec![ClockExpiry::Prompt { player: P2 }]);
    }

    #[tokio::test(start_paused = true)]
    async fn r268_stop_cancels_a_running_mulligan_clock() {
        let h = harness(0);
        h.clock.sync(&mulligan_window(&[P1, P2]));
        h.clock.sync(&ClockView {
            over: true,
            ..mulligan_window(&[P1])
        });
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.clock.remaining_for(P1), None);
        advance(h.mulligan_ms).await;
        assert_eq!(h.expiries(), vec![]);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_a_fresh_match_row_only_the_ceiling_9_5() {
        let h = harness(0);
        assert_eq!(
            serde_json::to_value(initial_clocks(h.started_at)).expect("MatchClocks serialises"),
            json!({
                "turnDeadline": null,
                "promptDeadline": null,
                "graceDeadline": { "p1": null, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );
    }
}

mod r389_the_match_ceiling_doubled_with_the_turn_cap_patch_v0_2_0_b4_3 {
    use super::*;

    /// B4.3, R389: the engine's turn cap, 30 player-turns each.
    const PLAYER_TURNS_AT_CAP: i64 = 60;

    #[test]
    fn r389_the_hard_ceiling_is_120_minutes_and_the_server_runs_with_it() {
        assert_eq!(int(MATCH_CEILING_MINUTES), 120);
        // The Rust clock reads `config::MATCH_CEILING_MINUTES` itself (no `ServerConfig`).
        assert_eq!(match_ceiling_at(0), int(MATCH_CEILING_MINUTES) * MINUTE);
    }

    #[test]
    fn r389_a_game_that_plays_every_turn_to_the_cap_at_a_full_turn_clock_ends_on_the_cap_not_the_ceiling() {
        // 60 player-turns × 75 s is 75 minutes: under 120.
        let longest_game_ms = PLAYER_TURNS_AT_CAP * int(TURN_CLOCK_SECONDS) * SECOND;
        assert!(longest_game_ms < int(MATCH_CEILING_MINUTES) * MINUTE);
        assert!(longest_game_ms > 60 * MINUTE);
    }

    #[test]
    fn r389_the_clock_arms_the_ceiling_at_the_configured_minutes_from_the_matchs_start() {
        let started_at = 5 * SECOND;
        assert_eq!(
            match_ceiling_at(started_at),
            started_at + int(MATCH_CEILING_MINUTES) * MINUTE
        );
    }
}
