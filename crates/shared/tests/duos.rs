//! The Duos co-op match host's machine (#3101's second part; fullsend SPEC S4 to S6, B35 to B40, R268,
//! R744, R776000, R1047000, R1253000): played on the real catalog, with a `Table` standing in for a host
//! — it keeps the timers the machine asks for, appends the rows it asks for to a log, collects what it
//! sends each person, answers the AI's searches with nothing (so the fallback plays) — and nothing else
//! decides anything.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::duos::{DuosState, duos_begin, duos_create};
use jackioh_engine::wire::{ActionBody, DuosAction, DuosActionBody, DuosCreateOptions, DuosResult, DuosSeat};
use jackioh_engine::{DECK_SIZE, Rng};
use jackioh_shared::config::{
    BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_SECONDS, DUOS_AI_STEP_DELAY_MS, DUOS_MULLIGAN_CLOCK_MS,
    DUOS_ROUND_CLOCK_MS, MATCH_CEILING_MINUTES, MATCH_SERVER_ACTION_RETRY_MS,
};
use jackioh_shared::host::duos::{DuosAiMode, DuosDecide, DuosLetGo, DuosNote, DuosSeatMessage};
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionAnswer, DuosHost, DuosHostEvent, DuosHostOutput, DuosSetup, GreetingPart, LogOutcome,
};

const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const EPOCH: i64 = 1_700_000_000_000;
const A1: DuosSeat = DuosSeat::A1;
const A2: DuosSeat = DuosSeat::A2;

fn grace_ms() -> i64 {
    DISCONNECT_GRACE_SECONDS * SECOND
}

/// A co-op game dealt from the real catalog and begun: all four mulligans open.
fn opening(seed: &str) -> DuosState {
    jackioh_cards::register_all();
    let deck = |seat: &str| {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:{seat}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
                gated_sets: Some(vec![]),
                ..AiDeckOptions::default()
            },
        )
    };
    let options: DuosCreateOptions = serde_json::from_value(serde_json::json!({
        "seed": seed,
        "decks": { "a1": deck("a1"), "a2": deck("a2"), "b1": deck("b1"), "b2": deck("b2") },
    }))
    .expect("create options");
    let begun = duos_begin(&duos_create(&options));
    assert!(begun.error.is_none(), "{:?}", begun.error);
    begun.state
}

fn setup(game: DuosState) -> DuosSetup {
    DuosSetup {
        match_id: "duos-1".to_string(),
        created_at: EPOCH,
        game,
        finished: false,
        next_seq: 1,
        acked: Vec::new(),
    }
}

struct Table {
    host: DuosHost,
    now: i64,
    timers: Vec<(TimerKey, u32, i64)>,
    log: Vec<(i64, DuosAction)>,
    sent: [Vec<DuosSeatMessage>; 2],
    answers: Vec<(String, ActionAnswer)>,
    records: Vec<DuosResult>,
    decides: Vec<DuosDecide>,
    notes: Vec<DuosNote>,
    refused: Vec<DuosSeat>,
    greetings: Vec<(DuosSeat, GreetingPart)>,
    let_go: Vec<DuosLetGo>,
    ceiling_draws: usize,
    row_checks: usize,
    pushed: usize,
    failing_appends: usize,
}

impl Table {
    fn new(setup: DuosSetup) -> Table {
        let (host, outputs) = DuosHost::new(setup, EPOCH);
        let mut table = Table {
            host,
            now: EPOCH,
            timers: Vec::new(),
            log: Vec::new(),
            sent: [Vec::new(), Vec::new()],
            answers: Vec::new(),
            records: Vec::new(),
            decides: Vec::new(),
            notes: Vec::new(),
            refused: Vec::new(),
            greetings: Vec::new(),
            let_go: Vec::new(),
            ceiling_draws: 0,
            row_checks: 0,
            pushed: 0,
            failing_appends: 0,
        };
        table.run(outputs);
        table
    }

    fn feed(&mut self, event: DuosHostEvent) {
        let outputs = self.host.step(event, self.now);
        self.run(outputs);
    }

    fn run(&mut self, outputs: Vec<DuosHostOutput>) {
        for output in outputs {
            match output {
                DuosHostOutput::Timer { command } => match command {
                    TimerCommand::Set { key, id, at } => {
                        self.timers.retain(|(held, _, _)| *held != key);
                        self.timers.push((key, id, at));
                    }
                    TimerCommand::Cancel { key, id } => {
                        self.timers
                            .retain(|(held, held_id, _)| !(*held == key && *held_id == id));
                    }
                },
                DuosHostOutput::Send { seat, message } => match seat {
                    DuosSeat::A1 => self.sent[0].push(message),
                    DuosSeat::A2 => self.sent[1].push(message),
                    _ => panic!("the AI's seat is sent nothing: {seat}"),
                },
                DuosHostOutput::Append { seq, action, .. } => {
                    let outcome = if self.failing_appends > 0 {
                        self.failing_appends -= 1;
                        LogOutcome::Failed
                    } else {
                        self.log.push((seq, action));
                        LogOutcome::Written
                    };
                    self.feed(DuosHostEvent::Logged { outcome });
                }
                DuosHostOutput::Answer { nonce, answer } => self.answers.push((nonce, answer)),
                DuosHostOutput::Greeting { seat, part } => self.greetings.push((seat, part)),
                DuosHostOutput::Refused { seat } => self.refused.push(seat),
                DuosHostOutput::Pushed => self.pushed += 1,
                DuosHostOutput::Decide { request } => self.decides.push(request),
                DuosHostOutput::Record { result, .. } => self.records.push(result),
                DuosHostOutput::CeilingDraw { .. } => self.ceiling_draws += 1,
                DuosHostOutput::CheckRow => self.row_checks += 1,
                DuosHostOutput::LetGo { reason } => self.let_go.push(reason),
                DuosHostOutput::Note { note } => self.notes.push(note),
                DuosHostOutput::Panicked { .. } => {}
            }
        }
    }

    /// Lets `ms` pass: every timer due by then comes due, in deadline order.
    fn sleep(&mut self, ms: i64) {
        let until = self.now + ms;
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
            self.feed(DuosHostEvent::TimerFired { key, id });
        }
        self.now = until;
    }

    fn connect(&mut self, seat: DuosSeat) {
        self.feed(DuosHostEvent::Connected { seat });
        self.feed(DuosHostEvent::Attached { seat });
    }

    fn act(&mut self, seat: DuosSeat, nonce: &str, body: DuosActionBody) -> ActionAnswer {
        self.feed(DuosHostEvent::Action {
            seat,
            nonce: nonce.to_string(),
            body,
        });
        self.answer(nonce)
    }

    fn answer(&self, nonce: &str) -> ActionAnswer {
        self.answers
            .iter()
            .rev()
            .find(|(answered, _)| answered == nonce)
            .map(|(_, answer)| answer.clone())
            .unwrap_or_else(|| panic!("no answer for {nonce}: {:?}", self.answers))
    }

    fn timer(&self, key: TimerKey) -> Option<i64> {
        self.timers
            .iter()
            .find(|(held, _, _)| *held == key)
            .map(|(_, _, at)| *at)
    }

    fn nonces(&self) -> Vec<String> {
        self.log.iter().map(|(_, action)| action.nonce.clone()).collect()
    }

    /// Answers every search the machine asked for with nothing, so the fallback plays.
    fn answer_searches(&mut self) {
        while let Some(request) = self.decides.pop() {
            self.feed(DuosHostEvent::Decided {
                seat: request.seat,
                body: None,
                cursor: request.cursor + 1,
                failed: false,
            });
        }
    }

    /// Plays on, the AI's pauses and searches included, until `done` or `limit` milliseconds have passed.
    fn play_on(&mut self, limit: i64, done: impl Fn(&Table) -> bool) {
        let until = self.now + limit;
        while self.now < until && !done(self) {
            self.sleep(DUOS_AI_STEP_DELAY_MS);
            self.answer_searches();
        }
    }

    fn round(&self) -> i32 {
        self.host.game().round
    }

    /// Both people answer their mulligans and the AI plays its own: round 1 begins.
    fn through_the_mulligans(&mut self) {
        for (n, seat) in [A1, A2].into_iter().enumerate() {
            let keep = jackioh_shared::host::duos::keep_whole_hand(&self.host.legal(seat))
                .expect("a mulligan is legal");
            let answer = self.act(seat, &format!("m-{n}"), keep);
            assert!(matches!(answer, ActionAnswer::Ack { .. }), "{answer:?}");
        }
        self.play_on(60 * SECOND, |table| table.round() >= 1);
        assert!(
            self.round() >= 1,
            "round {} nonces {:?}",
            self.round(),
            self.nonces()
        );
    }
}

mod the_clocks {
    use super::*;

    #[test]
    fn b2_r744_arming_starts_the_one_mulligan_clock_and_the_ceiling_and_the_seats_that_never_came_start_their_grace()
     {
        let mut table = Table::new(setup(opening("duos-open-1")));
        table.feed(DuosHostEvent::Arm);
        assert_eq!(
            table.timer(TimerKey::DuosMulligan),
            Some(EPOCH + DUOS_MULLIGAN_CLOCK_MS)
        );
        assert_eq!(
            table.timer(TimerKey::DuosCeiling),
            Some(EPOCH + MATCH_CEILING_MINUTES * MINUTE)
        );
        // R744: a seat with no socket starts its grace, so one nobody opens goes to the AI.
        assert_eq!(
            table.timer(TimerKey::DuosGrace { seat: A1 }),
            Some(EPOCH + grace_ms())
        );
        assert_eq!(
            table.timer(TimerKey::DuosGrace { seat: A2 }),
            Some(EPOCH + grace_ms())
        );
        // The watchers' record starts with the state the host opens on.
        assert_eq!(table.pushed, 1);
        // The AI team's seats owe their mulligans: one pause each, then a search each.
        assert!(table.timer(TimerKey::DuosAi { seat: DuosSeat::B1 }).is_some());
        assert!(table.timer(TimerKey::DuosAi { seat: DuosSeat::B2 }).is_some());
        assert!(table.timer(TimerKey::DuosAi { seat: A1 }).is_none());
    }

    #[test]
    fn s13_2_r268_a_lapsed_mulligan_clock_keeps_the_hand_of_each_person_still_owing() {
        let mut table = Table::new(setup(opening("duos-open-2")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_MULLIGAN_CLOCK_MS - 1);
        assert!(table.nonces().iter().all(|nonce| !nonce.contains("-ai-")) || table.round() >= 1);
        let before = table.log.len();
        table.sleep(1);
        assert!(table.notes.iter().any(|note| matches!(
            note,
            DuosNote::ClockExpired { kind } if kind == "mulligan"
        )));
        // The expiry hands the seats to the AI, which acts after its pause: each person keeps the hand.
        table.play_on(30 * SECOND, |table| table.round() >= 1);
        assert!(table.round() >= 1);
        let kept: Vec<&DuosAction> = table.log[before..]
            .iter()
            .map(|(_, action)| action)
            .filter(|action| {
                matches!(
                    action.body,
                    DuosActionBody::Duel {
                        body: ActionBody::Mulligan { .. }
                    }
                )
            })
            .collect();
        assert!(kept.iter().any(|action| action.seat == A1) && kept.iter().any(|action| action.seat == A2));
        assert!(
            kept.iter()
                .filter(|action| action.seat == A1 || action.seat == A2)
                .all(|action| action.nonce.starts_with("srv-ai-")),
            "{:?}",
            table.nonces()
        );
        // The mulligan clock is gone with the window.
        assert!(table.timer(TimerKey::DuosMulligan).is_none());
    }

    #[test]
    fn b38_a_round_with_a_person_runs_the_round_clock_and_a_lapsed_one_ends_the_turn_of_each_round_seat() {
        let mut table = Table::new(setup(opening("duos-round-1")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.through_the_mulligans();
        let round = table.round();
        let human_round_seats = table
            .host
            .game()
            .round_seats
            .iter()
            .any(|seat| *seat == A1 || *seat == A2);
        if human_round_seats {
            assert!(table.timer(TimerKey::DuosRound).is_some(), "round {round}");
            let rows = table.log.len();
            table.sleep(DUOS_ROUND_CLOCK_MS);
            assert!(table.notes.iter().any(|note| matches!(
                note,
                DuosNote::ClockExpired { kind } if kind == "round"
            )));
            table.play_on(60 * SECOND, |table| table.round() != round);
            assert_ne!(table.round(), round, "{:?}", &table.nonces()[rows..]);
            assert!(
                table.nonces()[rows..]
                    .iter()
                    .any(|nonce| nonce.starts_with("srv-ai-"))
            );
        } else {
            // Team B's round: no person to wait for, no round clock.
            assert!(table.timer(TimerKey::DuosRound).is_none(), "round {round}");
        }
    }

    #[test]
    fn b37_a_grace_running_out_hands_the_seat_to_the_ai_and_the_person_coming_back_takes_it_back() {
        let mut table = Table::new(setup(opening("duos-grace-1")));
        table.connect(A1);
        table.feed(DuosHostEvent::Arm);
        // a2 never came: its grace runs from the arm.
        assert_eq!(
            table.timer(TimerKey::DuosGrace { seat: A2 }),
            Some(EPOCH + grace_ms())
        );
        assert!(table.timer(TimerKey::DuosGrace { seat: A1 }).is_none());
        let told = table.sent[0].len();
        table.sleep(grace_ms());
        assert!(table.host.ai_playing(A2));
        assert!(
            table
                .notes
                .iter()
                .any(|note| matches!(note, DuosNote::SeatAiPlaying { seat } if *seat == A2))
        );
        // The teammate is told the AI plays a2's seat.
        assert!(table.sent[0][told..].iter().any(|message| matches!(
            message,
            DuosSeatMessage::Teammate {
                connected: false,
                ai_playing: true
            }
        )));
        // No loss by disconnect: the AI owes a2's mulligan and plays it.
        table.play_on(10 * SECOND, |table| {
            table.log.iter().any(|(_, action)| action.seat == A2)
        });
        assert!(
            table.log.iter().any(|(_, action)| action.seat == A2),
            "{:?}",
            table.nonces()
        );
        // a2 comes back: the AI lets go, and a1 is told.
        let told = table.sent[0].len();
        table.connect(A2);
        assert!(!table.host.ai_playing(A2));
        assert!(table.sent[0][told..].iter().any(|message| matches!(
            message,
            DuosSeatMessage::Teammate {
                connected: true,
                ai_playing: false
            }
        )));
        assert!(
            table
                .notes
                .iter()
                .any(|note| matches!(note, DuosNote::SeatBack { seat } if *seat == A2))
        );
    }

    #[test]
    fn b37_a_drop_starts_a_grace_once_and_the_teammate_is_told() {
        let mut table = Table::new(setup(opening("duos-grace-2")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        assert!(table.timer(TimerKey::DuosGrace { seat: A1 }).is_none());
        let told = table.sent[1].len();
        table.feed(DuosHostEvent::Left { seat: A1 });
        table.feed(DuosHostEvent::Disconnected { seat: A1 });
        let started = table.timer(TimerKey::DuosGrace { seat: A1 });
        assert_eq!(started, Some(table.now + grace_ms()));
        // The teammate's frames: who is here, and the clock with the grace.
        assert!(table.sent[1][told..].iter().any(|message| matches!(
            message,
            DuosSeatMessage::Teammate {
                connected: false,
                ai_playing: false
            }
        )));
        assert!(table.sent[1][told..].iter().any(|message| matches!(
            message,
            DuosSeatMessage::Clock {
                grace_a1: Some(_),
                ..
            }
        )));
        // A second drop, a flapping socket, does not move the deadline.
        table.sleep(10 * SECOND);
        table.feed(DuosHostEvent::Disconnected { seat: A1 });
        assert_eq!(table.timer(TimerKey::DuosGrace { seat: A1 }), started);
        // A socket that is back by the time the queue runs the disconnect starts nothing.
        let mut back = Table::new(setup(opening("duos-grace-2")));
        back.connect(A1);
        back.connect(A2);
        back.feed(DuosHostEvent::Arm);
        back.feed(DuosHostEvent::Left { seat: A2 });
        back.feed(DuosHostEvent::Connected { seat: A2 });
        back.feed(DuosHostEvent::Disconnected { seat: A2 });
        assert!(back.timer(TimerKey::DuosGrace { seat: A2 }).is_none());
    }

    #[test]
    fn r1047000_a_game_folded_back_past_its_ceiling_ends_at_once_in_a_draw_and_one_inside_arms_it() {
        let mut late = setup(opening("duos-ceiling-1"));
        late.created_at = EPOCH - MATCH_CEILING_MINUTES * MINUTE - 1;
        let mut table = Table::new(late);
        table.feed(DuosHostEvent::Arm);
        assert_eq!(table.nonces(), vec!["srv-ceiling-1"]);
        assert_eq!(table.log[0].1.seat, A1);
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].reason, "ceiling");
        assert!(table.host.is_finished());
        // Every clock went with the game.
        assert!(
            table.timer(TimerKey::DuosCeiling).is_none() && table.timer(TimerKey::DuosMulligan).is_none()
        );

        let mut table = Table::new(setup(opening("duos-ceiling-2")));
        table.feed(DuosHostEvent::Arm);
        table.sleep(MATCH_CEILING_MINUTES * MINUTE);
        assert!(
            table
                .nonces()
                .iter()
                .any(|nonce| nonce.starts_with("srv-ceiling-"))
        );
        assert_eq!(table.records.len(), 1);
    }

    #[test]
    fn r776000_a_ceiling_row_the_log_would_not_take_runs_the_ceiling_out_again_with_the_same_nonce() {
        let mut late = setup(opening("duos-ceiling-3"));
        late.created_at = EPOCH - MATCH_CEILING_MINUTES * MINUTE - 1;
        let mut table = Table::new(late);
        table.failing_appends = 1;
        table.feed(DuosHostEvent::Arm);
        assert!(table.log.is_empty());
        assert!(table.notes.contains(&DuosNote::CeilingRetrying));
        assert_eq!(
            table.timer(TimerKey::DuosCeiling),
            Some(table.now + MATCH_SERVER_ACTION_RETRY_MS)
        );
        table.sleep(MATCH_SERVER_ACTION_RETRY_MS);
        assert_eq!(table.nonces(), vec!["srv-ceiling-1"]);
        assert_eq!(table.records.len(), 1);
    }
}

mod actions {
    use super::*;

    fn keep(table: &Table, seat: DuosSeat) -> DuosActionBody {
        jackioh_shared::host::duos::keep_whole_hand(&table.host.legal(seat)).expect("a mulligan is legal")
    }

    #[test]
    fn s9_3_a_resent_nonce_is_acked_again_and_one_from_the_other_seat_is_taken() {
        let mut table = Table::new(setup(opening("duos-act-1")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        let body = keep(&table, A1);
        let first = table.act(A1, "mine", body.clone());
        let ActionAnswer::Ack { seq } = first else {
            panic!("{first:?}")
        };
        let rows = table.log.len();
        assert_eq!(table.act(A1, "mine", body.clone()), ActionAnswer::Ack { seq });
        assert_eq!(table.log.len(), rows);
        assert_eq!(table.act(A2, "mine", body), ActionAnswer::NonceTaken);
    }

    #[test]
    fn s9_8_a_refusal_is_answered_verbatim_and_counted_for_a_person_but_not_for_the_servers_own() {
        let mut table = Table::new(setup(opening("duos-act-2")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        // Ending a turn in the mulligan is illegal.
        let answer = table.act(
            A1,
            "early",
            DuosActionBody::Duel {
                body: ActionBody::EndTurn,
            },
        );
        assert!(matches!(answer, ActionAnswer::Illegal { .. }), "{answer:?}");
        assert_eq!(table.refused, vec![A1]);
        assert!(
            table
                .notes
                .iter()
                .any(|note| matches!(note, DuosNote::ActionRejected { seat, .. } if *seat == A1))
        );
        let answer = table.act(
            A1,
            "srv-early",
            DuosActionBody::Duel {
                body: ActionBody::EndTurn,
            },
        );
        assert!(matches!(answer, ActionAnswer::Illegal { .. }), "{answer:?}");
        assert_eq!(
            table.refused,
            vec![A1],
            "a server nonce is never a person's refusal"
        );
        assert!(table.log.is_empty());
    }

    #[test]
    fn s9_3_a_row_the_log_refused_is_not_applied_and_a_diverged_log_has_the_host_let_go() {
        let mut table = Table::new(setup(opening("duos-act-3")));
        table.connect(A1);
        table.feed(DuosHostEvent::Arm);
        let seq = table.host.next_seq();
        let hash = jackioh_engine::duos::duos_hash(table.host.game());
        let body = keep(&table, A1);
        table.failing_appends = 1;
        assert_eq!(table.act(A1, "m", body.clone()), ActionAnswer::NotRecorded);
        assert_eq!(table.host.next_seq(), seq);
        assert_eq!(jackioh_engine::duos::duos_hash(table.host.game()), hash);
        assert_eq!(table.act(A1, "m", body.clone()), ActionAnswer::Ack { seq });

        let mut diverged = Table::new(setup(opening("duos-act-3")));
        diverged.connect(A1);
        diverged.feed(DuosHostEvent::Arm);
        let body = keep(&diverged, A1);
        let _ = diverged.host.step(
            DuosHostEvent::Action {
                seat: A1,
                nonce: "m".to_string(),
                body,
            },
            diverged.now,
        );
        let outputs = diverged.host.step(
            DuosHostEvent::Logged {
                outcome: LogOutcome::Diverged,
            },
            diverged.now,
        );
        assert!(outputs.iter().any(|output| matches!(
            output,
            DuosHostOutput::LetGo {
                reason: DuosLetGo::Diverged
            }
        )));
    }

    #[test]
    fn b40_the_result_is_recorded_once_every_clock_goes_and_a_finished_match_takes_no_more() {
        let mut table = Table::new(setup(opening("duos-act-4")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.through_the_mulligans();
        // Both people concede: team A gives up (B11).
        assert!(matches!(
            table.act(A1, "gg-1", DuosActionBody::Concede),
            ActionAnswer::Ack { .. }
        ));
        assert!(matches!(
            table.act(A2, "gg-2", DuosActionBody::Concede),
            ActionAnswer::Ack { .. }
        ));
        assert_eq!(table.records.len(), 1, "{:?}", table.host.game().result);
        assert!(table.host.is_finished());
        assert_eq!(table.host.finished_at(), Some(table.now));
        assert!(
            table
                .timers
                .iter()
                .all(|(key, _, _)| matches!(key, TimerKey::DuosAi { .. } | TimerKey::DuosAiRetry)),
            "{:?}",
            table.timers
        );
        assert_eq!(
            table.act(A1, "late", DuosActionBody::Concede),
            ActionAnswer::MatchOver
        );
        // The AI's pauses that are still running find nothing to play.
        let rows = table.log.len();
        table.sleep(5 * SECOND);
        assert_eq!(table.log.len(), rows);
    }
}

mod the_ai {
    use super::*;

    #[test]
    fn b36_an_ai_seat_pauses_then_asks_for_its_search_once_and_plays_what_it_is_told_under_a_server_nonce() {
        let mut table = Table::new(setup(opening("duos-ai-1")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        assert!(table.decides.is_empty());
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        // b1 and b2 owe their mulligans in the mode that plays everything.
        assert_eq!(table.decides.len(), 2);
        let request = table.decides[0].clone();
        assert_eq!(request.mode, DuosAiMode::Full);
        assert_eq!(request.version, table.host.next_seq());
        // A second drive while the search is out asks for nothing more.
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        assert_eq!(table.decides.len(), 2);
        let body = table.host.legal(request.seat).into_iter().find(|body| {
            matches!(
                body,
                DuosActionBody::Duel {
                    body: ActionBody::Mulligan { .. }
                }
            )
        });
        table.feed(DuosHostEvent::Decided {
            seat: request.seat,
            body,
            cursor: 9,
            failed: false,
        });
        assert_eq!(table.nonces(), vec![format!("srv-ai-{}", request.version)]);
        assert_eq!(table.log[0].1.seat, request.seat);
    }

    #[test]
    fn b36_a_search_that_found_nothing_falls_back_on_a_legal_move_and_one_that_threw_plays_nothing() {
        let mut table = Table::new(setup(opening("duos-ai-2")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        let found_nothing = table.decides[0].clone();
        let threw = table.decides[1].clone();
        table.feed(DuosHostEvent::Decided {
            seat: found_nothing.seat,
            body: None,
            cursor: 4,
            failed: false,
        });
        assert_eq!(table.log.len(), 1, "the fallback played");
        assert!(matches!(
            table.log[0].1.body,
            DuosActionBody::Duel {
                body: ActionBody::Mulligan { .. }
            }
        ));
        table.feed(DuosHostEvent::Decided {
            seat: threw.seat,
            body: None,
            cursor: 0,
            failed: true,
        });
        assert_eq!(table.log.len(), 1, "a search that threw plays nothing");
    }

    #[test]
    fn r188_a_move_the_ai_may_never_play_is_dropped_for_the_fallback() {
        let mut table = Table::new(setup(opening("duos-ai-3")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        let request = table.decides[0].clone();
        table.feed(DuosHostEvent::Decided {
            seat: request.seat,
            body: Some(DuosActionBody::Concede),
            cursor: 1,
            failed: false,
        });
        assert!(
            table
                .log
                .iter()
                .all(|(_, action)| action.body != DuosActionBody::Concede)
        );
        assert_eq!(table.log.len(), 1);
    }

    #[test]
    fn r1452_a_move_the_log_would_not_take_is_decided_again_after_the_write_retry_pace_unless_the_match_is_gone()
     {
        let mut table = Table::new(setup(opening("duos-ai-4")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        let request = table.decides[0].clone();
        table.failing_appends = 1;
        table.feed(DuosHostEvent::Decided {
            seat: request.seat,
            body: None,
            cursor: 1,
            failed: false,
        });
        assert!(table.log.is_empty());
        // The store is asked whether the match row is still there.
        assert_eq!(table.row_checks, 1);
        table.feed(DuosHostEvent::RowChecked { exists: true });
        assert_eq!(
            table.timer(TimerKey::DuosAiRetry),
            Some(table.now + BOT_WRITE_RETRY_MS)
        );
        // Gone: the host lets go rather than retrying for ever.
        table.feed(DuosHostEvent::RowChecked { exists: false });
        assert_eq!(table.let_go, vec![DuosLetGo::MatchGone]);
    }

    #[test]
    fn b37_a_person_who_is_back_is_never_played_for() {
        // a2 never comes, so its grace runs out and the AI holds its seat; a2 returns before the AI acts.
        let mut table = Table::new(setup(opening("duos-ai-5")));
        table.connect(A1);
        table.feed(DuosHostEvent::Arm);
        table.through_the_mulligans();
        table.sleep(grace_ms());
        assert!(table.host.ai_playing(A2));
        table.connect(A2);
        assert!(!table.host.ai_playing(A2));
        let rows = table.log.len();
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        table.answer_searches();
        assert!(
            table.log[rows..].iter().all(|(_, action)| action.seat != A2),
            "the AI played for a person who was back: {:?}",
            table.nonces()
        );
    }
}

mod the_checkpoint {
    use super::*;

    #[test]
    fn a_host_resumes_from_its_state_as_json_and_steps_as_the_original_does() {
        let mut table = Table::new(setup(opening("duos-save-1")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        table.sleep(DUOS_AI_STEP_DELAY_MS);
        let json = serde_json::to_string(&table.host).expect("the host serializes");
        let mut resumed: DuosHost = serde_json::from_str(&json).expect("the host reads back");
        assert_eq!(resumed, table.host);
        let event = DuosHostEvent::Decided {
            seat: DuosSeat::B1,
            body: None,
            cursor: 3,
            failed: false,
        };
        let theirs = resumed.step(event.clone(), table.now);
        let ours = table.host.step(event, table.now);
        assert_eq!(
            serde_json::to_value(&theirs).unwrap(),
            serde_json::to_value(&ours).unwrap()
        );
        assert_eq!(resumed, table.host);
    }
}

mod stop {
    use super::*;

    #[test]
    fn stop_takes_every_clock_back_and_later_timers_do_nothing() {
        let mut table = Table::new(setup(opening("duos-stop-1")));
        table.connect(A1);
        table.connect(A2);
        table.feed(DuosHostEvent::Arm);
        let stale = table.timers.clone();
        table.feed(DuosHostEvent::Stop);
        assert!(
            table
                .timers
                .iter()
                .all(|(key, _, _)| matches!(key, TimerKey::DuosAi { .. } | TimerKey::DuosAiRetry)),
            "{:?}",
            table.timers
        );
        for (key, id, _) in stale {
            table.feed(DuosHostEvent::TimerFired { key, id });
        }
        assert!(table.log.is_empty());
        assert!(table.host.is_stopped());
    }
}
