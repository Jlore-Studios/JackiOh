//! The match clock (SPEC §9.5, R79, R268, R744, R776000, R1563006), pure: these tests moved from
//! `crates/server`'s actor tests, where they ran on tokio's paused clock, unchanged but for the time
//! they keep themselves. `Sim` plays the part of a host: it keeps the timers the clock asks for, advances
//! its own time and tells the clock which came due, in deadline order, as tokio does.

use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::config::KY_COMPETITION_SECONDS;
use jackioh_shared::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MATCH_SERVER_ACTION_RETRY_MS, MULLIGAN_CLOCK_SECONDS,
    PROMPT_CLOCK_SECONDS, TURN_CLOCK_SECONDS,
};
use jackioh_shared::host::clock::{
    ClockExpiry, ClockView, MatchClock, SequenceClockView, initial_clocks, match_ceiling_at,
};
use jackioh_shared::host::timer::{Fx, TimerCommand, TimerKey};
use serde_json::{Value, json};

/// Unit conversion only: R79's durations come from the config below.
const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;

/// The epoch millisecond a simulated host starts at.
const EPOCH: i64 = 1_700_000_000_000;

/// A config number as milliseconds' arithmetic wants it, whatever integer type `config.rs` gives it.
fn int(value: impl Into<i64>) -> i64 {
    value.into()
}

struct Sim {
    now: i64,
    started_at: i64,
    clock: MatchClock,
    fx: Fx,
    /// The timers the clock has asked for and not taken back, in the order they were set.
    timers: Vec<(TimerKey, u32, i64)>,
    /// Every expiry the clock has reported, in order.
    expiries: Vec<ClockExpiry>,
    turn_ms: i64,
    prompt_ms: i64,
    mulligan_ms: i64,
    grace_ms: i64,
    ceiling_ms: i64,
}

impl Sim {
    /// `started_offset_ms`: how long before now the match started.
    fn new(started_offset_ms: i64) -> Sim {
        let started_at = EPOCH - started_offset_ms;
        let mut sim = Sim {
            now: EPOCH,
            started_at,
            clock: MatchClock::new(started_at),
            fx: Fx::default(),
            timers: Vec::new(),
            expiries: Vec::new(),
            turn_ms: int(TURN_CLOCK_SECONDS) * SECOND,
            prompt_ms: int(PROMPT_CLOCK_SECONDS) * SECOND,
            mulligan_ms: int(MULLIGAN_CLOCK_SECONDS) * SECOND,
            grace_ms: int(DISCONNECT_GRACE_SECONDS) * SECOND,
            ceiling_ms: int(MATCH_CEILING_MINUTES) * MINUTE,
        };
        sim.clock.arm_ceiling(sim.now, &mut sim.fx);
        sim.take_commands();
        sim
    }

    fn take_commands(&mut self) {
        for command in self.fx.take() {
            match command {
                TimerCommand::Set { key, id, at } => {
                    self.timers.retain(|(held, _, _)| *held != key);
                    self.timers.push((key, id, at));
                }
                TimerCommand::Cancel { key, id } => {
                    self.timers
                        .retain(|(held, held_id, _)| !(*held == key && *held_id == id));
                }
            }
        }
    }

    fn sync(&mut self, view: &ClockView) {
        self.clock.sync(view, self.now, &mut self.fx);
        self.take_commands();
    }

    fn start_grace(&mut self, player: PlayerId, deadline: Option<i64>) {
        self.clock.start_grace(player, deadline, self.now, &mut self.fx);
        self.take_commands();
    }

    fn clear_grace(&mut self, player: PlayerId) {
        self.clock.clear_grace(player, &mut self.fx);
        self.take_commands();
    }

    fn retry(&mut self, expiry: ClockExpiry, ms: i64) {
        self.clock.retry(expiry, ms, self.now, &mut self.fx);
        self.take_commands();
    }

    fn stop(&mut self) {
        self.clock.stop(&mut self.fx);
        self.take_commands();
    }

    /// Lets `ms` pass: every timer due by then fires, in deadline order.
    fn sleep(&mut self, ms: i64) {
        let until = self.now + ms.max(0);
        loop {
            let next = self
                .timers
                .iter()
                .enumerate()
                .filter(|(_, (_, _, at))| *at <= until)
                .min_by_key(|(order, (_, _, at))| (*at, *order))
                .map(|(order, _)| order);
            let Some(order) = next else { break };
            let (key, id, at) = self.timers.remove(order);
            self.now = self.now.max(at);
            if let Some(expiry) = self.clock.fire(key, id) {
                self.expiries.push(expiry);
            }
            self.take_commands();
        }
        self.now = until;
    }

    fn snapshot(&self) -> Value {
        serde_json::to_value(self.clock.snapshot()).expect("MatchClocks serializes")
    }

    /// One deadline of the snapshot (`turnDeadline`, `promptDeadline`), null as `None`.
    fn deadline(&self, key: &str) -> Option<i64> {
        self.snapshot()[key].as_i64()
    }

    /// One seat's grace deadline, null as `None`.
    fn grace(&self, player: PlayerId) -> Option<i64> {
        self.snapshot()["graceDeadline"][player.as_str()].as_i64()
    }

    fn remaining_for(&self, player: PlayerId) -> Option<i64> {
        self.clock.remaining_for(player, self.now)
    }

    fn graces_reported(&self) -> usize {
        self.expiries
            .iter()
            .filter(|expiry| matches!(expiry, ClockExpiry::Grace { .. }))
            .count()
    }
}

fn view() -> ClockView {
    ClockView {
        turn: 1,
        active: P1,
        pending_for: None,
        mulligan_owed: vec![],
        over: false,
        sequence: None,
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

    #[test]
    fn r79_expires_the_turn_clock_after_turn_clock_seconds_and_names_the_active_player() {
        let mut h = Sim::new(0);
        h.sync(&view());

        h.sleep(h.turn_ms - 1);
        assert_eq!(h.expiries, vec![]);

        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[test]
    fn resets_the_turn_clock_when_the_turn_changes() {
        let mut h = Sim::new(0);
        h.sync(&view());
        h.sleep(h.turn_ms - SECOND);
        assert_eq!(h.expiries, vec![]);

        // R79: "The turn clock belongs to the active player" — the new turn starts from full.
        h.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        assert_eq!(h.remaining_for(P2), Some(h.turn_ms));
        assert_eq!(h.remaining_for(P1), None);

        h.sleep(h.turn_ms - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P2 }]);
    }

    #[test]
    fn r79_a_prompt_held_by_the_non_active_player_pauses_the_turn_clock_and_only_its_own_clock_expires() {
        let mut h = Sim::new(0);
        h.sync(&view());

        // Spend most of p1's turn, then a trap fires and asks p2 a question.
        let spent = h.turn_ms - 15 * SECOND;
        h.sleep(spent);
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms - spent));

        h.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), Some(h.now + h.prompt_ms));

        // The prompt outlives what was left of p1's turn: 60 s spent + a 30 s prompt is past the 75 s
        // turn clock, so an unpaused turn clock would have fired in here.
        h.sleep(h.prompt_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Prompt { player: P2 }]);
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms - spent));

        // R79: when the pause ends the turn clock resumes with the time it had left, not from full.
        h.sync(&view());
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + (h.turn_ms - spent)));
        h.sleep(h.turn_ms - spent - 1);
        assert_eq!(h.expiries.len(), 1);
        h.sleep(1);
        assert_eq!(
            h.expiries,
            vec![
                ClockExpiry::Prompt { player: P2 },
                ClockExpiry::Turn { player: P1 }
            ]
        );
    }

    #[test]
    fn does_not_pause_the_turn_clock_for_a_prompt_held_by_the_active_player() {
        let mut h = Sim::new(0);
        // §2.5: "When the active player's turn clock runs out, their open prompts are answered by the
        // AI policy and the turn ends" — their own prompt gets no second, shorter deadline.
        h.sync(&ClockView {
            pending_for: Some(P1),
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + h.turn_ms));
        assert_eq!(h.deadline("promptDeadline"), None);

        h.sleep(h.prompt_ms);
        assert_eq!(h.expiries, vec![]);

        h.sleep(h.turn_ms - h.prompt_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[test]
    fn counts_disconnect_grace_per_player_and_reports_whose_it_was() {
        let mut h = Sim::new(0);
        h.start_grace(P2, None);
        assert_eq!(
            h.snapshot()["graceDeadline"],
            json!({ "p1": null, "p2": h.now + h.grace_ms })
        );

        h.sleep(h.grace_ms - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Grace { player: P2 }]);
        assert_eq!(h.grace(P2), None);
    }

    #[test]
    fn r147_keeps_the_first_deadline_when_a_second_grace_starts_so_a_flapping_socket_cannot_extend_it() {
        let mut h = Sim::new(0);
        h.start_grace(P1, None);
        let deadline = h.grace(P1);
        assert_eq!(deadline, Some(h.now + h.grace_ms));

        // Re-arming on flaps would allow stalling; keep original deadline.
        h.sleep(h.grace_ms - SECOND);
        h.start_grace(P1, None);
        assert_eq!(h.grace(P1), deadline);
        assert_eq!(h.expiries, vec![]);

        // It expires at the deadline the *first* drop set, not a second later.
        h.sleep(SECOND);
        assert_eq!(h.expiries, vec![ClockExpiry::Grace { player: P1 }]);

        // An expired grace can be started again.
        h.start_grace(P1, None);
        assert_eq!(h.grace(P1), Some(h.now + h.grace_ms));
    }

    #[test]
    fn r744_starts_a_grace_at_a_stored_deadline_holds_it_to_a_fresh_window_and_fires_one_already_past_at_once()
     {
        let mut h = Sim::new(0);

        // A deadline the restart left stored is kept, not replaced by a fresh window.
        let stored = h.now + h.grace_ms / 2;
        h.start_grace(P1, Some(stored));
        assert_eq!(h.grace(P1), Some(stored));
        h.sleep(h.grace_ms / 2 - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Grace { player: P1 }]);

        // Stored deadline later than fresh window is held to it (R147); rebuilds never extend grace.
        h.start_grace(P2, Some(h.now + h.grace_ms * 2));
        assert_eq!(h.grace(P2), Some(h.now + h.grace_ms));

        // One that already ran out while the server was down fires at once.
        h.start_grace(P1, Some(h.now - 1));
        h.sleep(0);
        assert_eq!(h.expiries.last(), Some(&ClockExpiry::Grace { player: P1 }));
        assert_eq!(h.graces_reported(), 2);
    }

    #[test]
    fn cancels_grace_when_the_player_returns_inside_the_window() {
        let mut h = Sim::new(0);
        h.start_grace(P1, None);
        h.sleep(h.grace_ms - SECOND);
        h.clear_grace(P1);
        assert_eq!(h.grace(P1), None);

        h.sleep(h.grace_ms * 2);
        assert_eq!(h.graces_reported(), 0);
    }

    #[test]
    fn keeps_the_turn_clock_running_while_a_player_is_disconnected_9_5() {
        let mut h = Sim::new(0);
        h.sync(&view());
        let deadline = h.deadline("turnDeadline");

        h.start_grace(P2, None);
        // §9.5: "The clock keeps running while a player is disconnected."
        assert_eq!(h.deadline("turnDeadline"), deadline);

        h.sleep(h.turn_ms);
        assert_eq!(
            h.expiries,
            vec![
                ClockExpiry::Grace { player: P2 },
                ClockExpiry::Turn { player: P1 }
            ]
        );
        assert!(h.grace_ms < h.turn_ms);
    }

    #[test]
    fn r79_fires_the_ceiling_once_measured_from_started_at() {
        let elapsed = 10 * MINUTE;
        let mut h = Sim::new(elapsed);
        assert_eq!(
            h.snapshot()["ceilingAt"].as_i64(),
            Some(match_ceiling_at(h.started_at))
        );
        assert_eq!(
            h.snapshot()["ceilingAt"].as_i64(),
            Some(h.started_at + h.ceiling_ms)
        );

        h.sleep(h.ceiling_ms - elapsed - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Ceiling]);

        // Once: a second ceiling's worth fires nothing.
        h.sleep(h.ceiling_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Ceiling]);
    }

    #[test]
    fn exposes_every_deadline_through_snapshot_and_counts_down_through_remaining_for() {
        let mut h = Sim::new(0);
        assert_eq!(
            h.snapshot(),
            json!({
                "turnDeadline": null,
                "promptDeadline": null,
                "graceDeadline": { "p1": null, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );

        h.sync(&view());
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + h.turn_ms));

        h.sleep(10 * SECOND);
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
        assert_eq!(h.remaining_for(P2), None);

        h.start_grace(P1, None);
        h.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(
            h.snapshot(),
            json!({
                "turnDeadline": null,
                "promptDeadline": h.now + h.prompt_ms,
                "graceDeadline": { "p1": h.now + h.grace_ms, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );
        // The paused turn clock still reports its banked remainder; the prompt holder reports theirs.
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
        assert_eq!(h.remaining_for(P2), Some(h.prompt_ms));

        h.sleep(SECOND);
        assert_eq!(h.remaining_for(P2), Some(h.prompt_ms - SECOND));
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms - 10 * SECOND));
    }

    #[test]
    fn stops_everything_once_the_match_is_over() {
        let mut h = Sim::new(0);
        h.sync(&view());
        h.start_grace(P1, None);
        h.start_grace(P2, None);
        assert!(h.deadline("turnDeadline").is_some());
        assert!(h.grace(P1).is_some() && h.grace(P2).is_some());

        h.sync(&ClockView { over: true, ..view() });
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.snapshot()["graceDeadline"], json!({ "p1": null, "p2": null }));
        assert_eq!(h.remaining_for(P1), None);

        // A late sync must never re-arm a deadline on a finished match.
        h.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        assert_eq!(h.deadline("turnDeadline"), None);
        h.sleep(60 * MINUTE);
        assert_eq!(h.expiries, vec![]);
    }

    #[test]
    fn stop_cancels_every_timer_and_is_idempotent() {
        let mut h = Sim::new(0);
        h.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        h.start_grace(P1, None);
        assert!(h.deadline("promptDeadline").is_some());
        assert!(h.grace(P1).is_some());

        h.stop();
        h.stop();
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.grace(P1), None);
        assert!(h.timers.is_empty(), "every timer was taken back: {:?}", h.timers);
        h.sleep(60 * MINUTE);
        assert_eq!(h.expiries, vec![]);
    }

    #[test]
    fn r268_the_mulligan_clock_runs_one_deadline_for_both_seats_at_once_and_no_turn_clock_runs_under_it() {
        let mut h = Sim::new(0);
        h.sync(&mulligan_window(&[P1, P2]));

        let deadline = h.now + h.mulligan_ms;
        // One deadline, reported as the prompt deadline both clients render; setup is nobody's turn.
        assert_eq!(h.deadline("promptDeadline"), Some(deadline));
        assert_eq!(h.deadline("turnDeadline"), None);
        assert_eq!(h.remaining_for(P1), Some(h.mulligan_ms));
        assert_eq!(h.remaining_for(P2), Some(h.mulligan_ms));

        // A turn clock's worth of waiting is not what ends it: only the mulligan deadline is armed.
        h.sleep(h.mulligan_ms - 1);
        assert_eq!(h.expiries, vec![]);
        assert_eq!(h.remaining_for(P1), Some(1));
        assert_eq!(h.remaining_for(P2), Some(1));
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Mulligan]);
        assert!(h.mulligan_ms < h.turn_ms);

        // Fires once; sync while still open arms nothing new.
        h.sync(&mulligan_window(&[P2]));
        h.sleep(h.mulligan_ms * 2);
        assert_eq!(h.expiries, vec![ClockExpiry::Mulligan]);
    }

    #[test]
    fn r268_one_seat_answering_neither_re_arms_nor_extends_the_mulligan_clock() {
        let mut h = Sim::new(0);
        h.sync(&mulligan_window(&[P1, P2]));
        let deadline = h.deadline("promptDeadline");

        // Answering does not give the other seat a fresh window.
        h.sleep(20 * SECOND);
        h.sync(&mulligan_window(&[P1]));
        assert_eq!(h.deadline("promptDeadline"), deadline);
        assert_eq!(h.remaining_for(P1), Some(h.mulligan_ms - 20 * SECOND));
        assert_eq!(h.remaining_for(P2), Some(h.mulligan_ms - 20 * SECOND));

        h.sleep(h.mulligan_ms - 20 * SECOND);
        assert_eq!(h.expiries, vec![ClockExpiry::Mulligan]);
    }

    #[test]
    fn r268_the_window_closing_cancels_the_mulligan_clock_and_turn_1_starts_p1s_turn_clock_from_full() {
        let mut h = Sim::new(0);
        h.sync(&mulligan_window(&[P1, P2]));
        h.sleep(10 * SECOND);
        h.sync(&mulligan_window(&[P2]));

        // The second answer resolves both and starts turn 1 (R265, §2.1).
        h.sleep(5 * SECOND);
        h.sync(&ClockView {
            turn: 1,
            active: P1,
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + h.turn_ms));
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms));
        assert_eq!(h.remaining_for(P2), None);

        // The mulligan deadline passes with nothing to report, and the turn clock is p1's.
        h.sleep(h.mulligan_ms);
        assert_eq!(h.expiries, vec![]);
        h.sleep(h.turn_ms - h.mulligan_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P1 }]);
    }

    #[test]
    fn r268_a_cards_question_during_setup_is_still_timed_by_r79_on_either_side_of_the_window() {
        let mut h = Sim::new(0);

        // A cast-on-draw card in p2's opening deal asks p2 something before the mulligans open (§2.4):
        // the non-active holder's prompt clock, as R79 has it.
        h.sync(&ClockView {
            turn: 0,
            active: P1,
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), Some(h.now + h.prompt_ms));

        // Answered, and the window opens: the prompt clock goes and the one mulligan deadline takes over.
        h.sleep(5 * SECOND);
        h.sync(&mulligan_window(&[P1, P2]));
        assert_eq!(h.deadline("promptDeadline"), Some(h.now + h.mulligan_ms));
        h.sleep(h.prompt_ms);
        assert_eq!(h.expiries, vec![]);

        // Both answered, and a replacement draw's cast asks p2 during the resolution (phase still
        // mulligan, turn 0, nobody owing): R79's prompt clock again, and the mulligan clock is gone.
        h.sync(&ClockView {
            turn: 0,
            active: P1,
            pending_for: Some(P2),
            ..view()
        });
        assert_eq!(h.deadline("promptDeadline"), Some(h.now + h.prompt_ms));
        h.sleep(h.prompt_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Prompt { player: P2 }]);
    }

    #[test]
    fn r268_stop_cancels_a_running_mulligan_clock() {
        let mut h = Sim::new(0);
        h.sync(&mulligan_window(&[P1, P2]));
        h.sync(&ClockView {
            over: true,
            ..mulligan_window(&[P1])
        });
        assert_eq!(h.deadline("promptDeadline"), None);
        assert_eq!(h.remaining_for(P1), None);
        h.sleep(h.mulligan_ms);
        assert_eq!(h.expiries, vec![]);
    }

    #[test]
    fn gives_a_fresh_match_row_only_the_ceiling_9_5() {
        let h = Sim::new(0);
        assert_eq!(
            serde_json::to_value(initial_clocks(h.started_at)).expect("MatchClocks serializes"),
            json!({
                "turnDeadline": null,
                "promptDeadline": null,
                "graceDeadline": { "p1": null, "p2": null },
                "ceilingAt": h.started_at + h.ceiling_ms,
            })
        );
    }

    #[test]
    fn r776000_a_retry_runs_an_expired_turn_clock_out_once_more_and_a_new_turn_cancels_it() {
        let retry_ms = int(MATCH_SERVER_ACTION_RETRY_MS);
        let mut h = Sim::new(0);
        h.sync(&view());
        h.sleep(h.turn_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P1 }]);

        // One countdown, one timer: a second retry while the first runs changes nothing.
        h.retry(ClockExpiry::Turn { player: P1 }, retry_ms);
        h.retry(ClockExpiry::Turn { player: P1 }, retry_ms);
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + retry_ms));
        h.sleep(retry_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Turn { player: P1 }; 2]);

        // Nothing re-arms a clock that ran out and was not retried.
        h.sleep(h.turn_ms);
        assert_eq!(h.expiries.len(), 2);

        // The turn passing to p2 cancels a retry that was waiting.
        h.retry(ClockExpiry::Turn { player: P1 }, retry_ms);
        h.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        h.sleep(retry_ms);
        assert_eq!(h.expiries.len(), 2);
        assert_eq!(h.deadline("turnDeadline"), Some(h.now + h.turn_ms - retry_ms));
    }

    #[test]
    fn r776000_a_retry_runs_an_expired_grace_out_once_more_and_the_player_returning_cancels_it() {
        let retry_ms = int(MATCH_SERVER_ACTION_RETRY_MS);
        let mut h = Sim::new(0);
        h.start_grace(P1, None);
        h.sleep(h.grace_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Grace { player: P1 }]);

        h.retry(ClockExpiry::Grace { player: P1 }, retry_ms);
        assert_eq!(h.grace(P1), Some(h.now + retry_ms));
        // R147: a countdown already running is kept, not moved.
        h.retry(ClockExpiry::Grace { player: P1 }, retry_ms * 2);
        assert_eq!(h.grace(P1), Some(h.now + retry_ms));
        h.sleep(retry_ms);
        assert_eq!(h.expiries, vec![ClockExpiry::Grace { player: P1 }; 2]);

        h.retry(ClockExpiry::Grace { player: P1 }, retry_ms);
        h.clear_grace(P1);
        assert_eq!(h.grace(P1), None);
        h.sleep(retry_ms);
        assert_eq!(h.expiries.len(), 2);
    }

    #[test]
    fn r776000_a_retried_prompt_mulligan_or_ceiling_clock_runs_out_once_more() {
        let retry_ms = int(MATCH_SERVER_ACTION_RETRY_MS);

        let mut prompt = Sim::new(0);
        prompt.sync(&ClockView {
            pending_for: Some(P2),
            ..view()
        });
        prompt.sleep(prompt.prompt_ms);
        assert_eq!(prompt.expiries, vec![ClockExpiry::Prompt { player: P2 }]);
        prompt.retry(ClockExpiry::Prompt { player: P2 }, retry_ms);
        prompt.sleep(retry_ms);
        assert_eq!(prompt.expiries, vec![ClockExpiry::Prompt { player: P2 }; 2]);
        prompt.stop();

        let mut mulligan = Sim::new(0);
        mulligan.sync(&mulligan_window(&[P1, P2]));
        mulligan.sleep(mulligan.mulligan_ms);
        assert_eq!(mulligan.expiries, vec![ClockExpiry::Mulligan]);
        mulligan.retry(ClockExpiry::Mulligan, retry_ms);
        mulligan.sleep(retry_ms);
        assert_eq!(mulligan.expiries, vec![ClockExpiry::Mulligan; 2]);
        // The window closing cancels a retry that was waiting.
        mulligan.retry(ClockExpiry::Mulligan, retry_ms);
        mulligan.sync(&view());
        mulligan.sleep(retry_ms);
        assert_eq!(mulligan.expiries.len(), 2);
        mulligan.stop();

        let mut ceiling = Sim::new(int(MATCH_CEILING_MINUTES) * MINUTE - SECOND);
        ceiling.sleep(SECOND);
        assert_eq!(ceiling.expiries, vec![ClockExpiry::Ceiling]);
        ceiling.retry(ClockExpiry::Ceiling, retry_ms);
        ceiling.sleep(retry_ms);
        assert_eq!(ceiling.expiries, vec![ClockExpiry::Ceiling; 2]);
        ceiling.stop();
    }

    /// A timer the clock gave up (replaced or taken back) reports nothing if the host cannot take it back
    /// in time.
    #[test]
    fn a_stale_timer_reports_nothing() {
        let mut h = Sim::new(0);
        h.sync(&view());
        let (key, id, _) = *h
            .timers
            .iter()
            .find(|(key, _, _)| *key == TimerKey::Turn)
            .expect("armed");
        h.sync(&ClockView {
            turn: 2,
            active: P2,
            ..view()
        });
        assert_eq!(h.clock.fire(key, id), None);
        assert_eq!(h.clock.fire(TimerKey::Turn, id + 1000), None);
    }
}

mod r389_the_match_ceiling_doubled_with_the_turn_cap_patch_v0_2_0_b4_3 {
    use super::*;

    /// B4.3, R389: the engine's turn cap, 30 player-turns each.
    const PLAYER_TURNS_AT_CAP: i64 = 60;

    #[test]
    fn r389_the_hard_ceiling_is_120_minutes_and_the_server_runs_with_it() {
        assert_eq!(int(MATCH_CEILING_MINUTES), 120);
        assert_eq!(match_ceiling_at(0), int(MATCH_CEILING_MINUTES) * MINUTE);
    }

    #[test]
    fn r389_a_game_that_plays_every_turn_to_the_cap_at_a_full_turn_clock_ends_on_the_cap_not_the_ceiling() {
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

/// R1563006: Meditative+ #20 KY's Competition's sequence clock.
mod r1563006_the_question_sequence_clock {
    use super::*;

    /// p1's turn with a question of a sequence open for p1.
    fn asking(streak: i32, prompt_id: &str) -> ClockView {
        ClockView {
            pending_for: Some(P1),
            sequence: Some(SequenceClockView {
                player: P1,
                streak,
                prompt_id: prompt_id.to_string(),
            }),
            ..view()
        }
    }

    fn sequence_ms() -> i64 {
        int(KY_COMPETITION_SECONDS) * SECOND
    }

    #[test]
    fn r1563006_a_sequence_runs_out_60_seconds_after_its_first_question_whatever_it_answered_since() {
        let mut h = Sim::new(0);
        h.sync(&asking(0, "q1"));
        h.sleep(20 * SECOND);
        // Two right answers later the window is the same one.
        h.sync(&asking(1, "q2"));
        h.sleep(20 * SECOND);
        h.sync(&asking(2, "q3"));
        h.sleep(sequence_ms() - 40 * SECOND - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Sequence { player: P1 }]);
    }

    #[test]
    fn r1563006_a_sequence_that_ends_clears_its_clock_and_a_new_one_starts_from_full() {
        let mut h = Sim::new(0);
        h.sync(&asking(0, "q1"));
        h.sleep(50 * SECOND);
        // A miss or Stop ends it: no question is open.
        h.sync(&view());
        h.sleep(20 * SECOND);
        assert_eq!(h.expiries, vec![]);
        // A second Competition, on p1's next turn (inside the turn clock): a fresh window.
        h.sync(&ClockView {
            turn: 3,
            ..asking(0, "q9")
        });
        h.sleep(sequence_ms() - 1);
        assert_eq!(h.expiries, vec![]);
        h.sleep(1);
        assert_eq!(h.expiries, vec![ClockExpiry::Sequence { player: P1 }]);
    }

    #[test]
    fn r1563006_it_runs_beside_the_turn_clock_never_instead_of_it() {
        let mut h = Sim::new(0);
        h.sync(&asking(0, "q1"));
        assert_eq!(h.remaining_for(P1), Some(h.turn_ms));
        h.sleep(sequence_ms());
        assert_eq!(h.expiries, vec![ClockExpiry::Sequence { player: P1 }]);
        h.sleep(h.turn_ms - sequence_ms());
        assert_eq!(
            h.expiries,
            vec![
                ClockExpiry::Sequence { player: P1 },
                ClockExpiry::Turn { player: P1 }
            ]
        );
    }

    #[test]
    fn r1563006_the_expiry_is_named_sequence() {
        assert_eq!(ClockExpiry::Sequence { player: P2 }.kind(), "sequence");
        assert_eq!(KY_COMPETITION_SECONDS, 60);
    }
}
