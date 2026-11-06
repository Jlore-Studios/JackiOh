//! The 1v1 match host's machine (#3101; SPEC §9.2, §9.3, §9.5, R79, R265, R268, R638, R672, R738, R744,
//! R776000, R1253000, R1452, R639000): played on the real catalog, with a `Table` standing in for a host
//! — it keeps the timers the machine asks for, appends the rows it asks for to a log, collects what it
//! sends each seat — and nothing else decides anything.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{
    Action, ActionBody, Aim, AimEnd, DECK_SIZE, EmoteId, GameOverReason, GameState, PerPlayer, Rng, Winner,
};
use jackioh_shared::bots::decide::BotChoice;
use jackioh_shared::bots::signals::{idles_turn, rematch_plan};
use jackioh_shared::config::{
    AIM_RELAY_INTERVAL_MS, BOT_REMATCH_FIT, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS, BOT_TIMEOUT_FIT,
    BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MATCH_SERVER_ACTION_RETRY_MS,
    MULLIGAN_CLOCK_SECONDS, PersonaName, STAKE_NORMAL, TURN_CLOCK_SECONDS,
};
use jackioh_shared::host::bot::BotSeat;
use jackioh_shared::host::clock::{ClockExpiry, MatchClocks, initial_clocks};
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionAnswer, ActionBy, DecideRequest, DuelEvent, DuelHost, DuelOutput, DuelSetup, GreetingPart,
    HostNote, LetGo, LogOutcome, RecordRequest, SeatMessage,
};

const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const EPOCH: i64 = 1_700_000_000_000;

fn turn_ms() -> i64 {
    TURN_CLOCK_SECONDS * SECOND
}

fn mulligan_ms() -> i64 {
    MULLIGAN_CLOCK_SECONDS * SECOND
}

fn grace_ms() -> i64 {
    DISCONNECT_GRACE_SECONDS * SECOND
}

/// A game dealt from the real catalog and begun: both mulligans open.
fn opening(seed: &str) -> GameState {
    jackioh_cards::register_all();
    let deck = |salt: &str| {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:{salt}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
                gated_sets: Some(vec![]),
                ..AiDeckOptions::default()
            },
        )
    };
    let options = serde_json::from_value(serde_json::json!({
        "seed": seed,
        "decks": [deck("a"), deck("b")],
    }))
    .expect("create options");
    let created = jackioh_engine::create_game(&options);
    let begun = jackioh_engine::begin_game(&created);
    assert!(begun.error.is_none(), "{:?}", begun.error);
    begun.state
}

fn setup(game: GameState) -> DuelSetup {
    DuelSetup {
        match_id: "match-1".to_string(),
        seed: "seed-host".to_string(),
        created_at: EPOCH,
        game,
        ranked: false,
        series_game: false,
        finished: false,
        next_seq: 1,
        acked: Vec::new(),
        clocks: initial_clocks(EPOCH),
        bots: PerPlayer::new(None, None),
        bot_moved_turn: PerPlayer::new(None, None),
    }
}

/// A host standing in for the real one.
struct Table {
    host: DuelHost,
    now: i64,
    timers: Vec<(TimerKey, u32, i64)>,
    log: Vec<(i64, Action)>,
    sent: PerPlayer<Vec<SeatMessage>>,
    persisted: Vec<MatchClocks>,
    answers: Vec<(String, ActionAnswer)>,
    records: Vec<RecordRequest>,
    decides: Vec<DecideRequest>,
    emotes: Vec<(PlayerId, EmoteId)>,
    offers: Vec<(PlayerId, i32, bool)>,
    notes: Vec<HostNote>,
    refused: Vec<PlayerId>,
    greetings: Vec<(PlayerId, GreetingPart)>,
    let_go: Vec<LetGo>,
    ceiling_draws: usize,
    voids: usize,
    /// R3213000: how many times who is here changed with no socket arriving or leaving.
    presence_changes: usize,
    /// How many of the next appends the log refuses.
    failing_appends: usize,
}

impl Table {
    fn new(setup: DuelSetup) -> Table {
        let (host, outputs) = DuelHost::new(setup, EPOCH);
        let mut table = Table::of(host);
        table.run(outputs);
        table
    }

    fn of(host: DuelHost) -> Table {
        Table {
            host,
            now: EPOCH,
            timers: Vec::new(),
            log: Vec::new(),
            sent: PerPlayer::new(Vec::new(), Vec::new()),
            persisted: Vec::new(),
            answers: Vec::new(),
            records: Vec::new(),
            decides: Vec::new(),
            emotes: Vec::new(),
            offers: Vec::new(),
            notes: Vec::new(),
            refused: Vec::new(),
            greetings: Vec::new(),
            let_go: Vec::new(),
            ceiling_draws: 0,
            voids: 0,
            presence_changes: 0,
            failing_appends: 0,
        }
    }

    fn feed(&mut self, event: DuelEvent) {
        let outputs = self.host.step(event, self.now);
        self.run(outputs);
    }

    fn run(&mut self, outputs: Vec<DuelOutput>) {
        for output in outputs {
            match output {
                DuelOutput::Timer { command } => match command {
                    TimerCommand::Set { key, id, at } => {
                        self.timers.retain(|(held, _, _)| *held != key);
                        self.timers.push((key, id, at));
                    }
                    TimerCommand::Cancel { key, id } => {
                        self.timers
                            .retain(|(held, held_id, _)| !(*held == key && *held_id == id));
                    }
                },
                DuelOutput::Send { home, message } => match home {
                    P1 => self.sent.p1.push(message),
                    P2 => self.sent.p2.push(message),
                },
                DuelOutput::PersistClocks { clocks } => self.persisted.push(clocks),
                DuelOutput::Append { seq, action, .. } => {
                    let outcome = if self.failing_appends > 0 {
                        self.failing_appends -= 1;
                        LogOutcome::Failed
                    } else {
                        self.log.push((seq, action));
                        LogOutcome::Written
                    };
                    self.feed(DuelEvent::Logged { outcome });
                }
                DuelOutput::Answer { nonce, answer } => self.answers.push((nonce, answer)),
                DuelOutput::Greeting { home, part } => self.greetings.push((home, part)),
                DuelOutput::Refused { home } => self.refused.push(home),
                DuelOutput::Decide { request } => self.decides.push(request),
                DuelOutput::BotEmote { home, emote } => self.emotes.push((home, emote)),
                DuelOutput::Record { request } => {
                    self.records.push(request);
                    self.feed(DuelEvent::ResultRecorded);
                }
                DuelOutput::Voided { .. } => self.voids += 1,
                DuelOutput::CeilingDraw { .. } => self.ceiling_draws += 1,
                DuelOutput::OfferRematch { home, stakes, reply } => self.offers.push((home, stakes, reply)),
                DuelOutput::PresenceChanged => self.presence_changes += 1,
                DuelOutput::LetGo { reason } => self.let_go.push(reason),
                DuelOutput::Note { note } => self.notes.push(note),
                DuelOutput::Panicked { .. }
                | DuelOutput::Applied { .. }
                | DuelOutput::SeatsSwapped { .. }
                | DuelOutput::Changed { .. }
                | DuelOutput::Pushed { .. } => {}
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
            self.feed(DuelEvent::TimerFired { key, id });
        }
        self.now = until;
    }

    fn connect(&mut self, home: PlayerId) {
        self.feed(DuelEvent::Connected { home });
        self.feed(DuelEvent::Attached { home });
    }

    fn act(&mut self, home: PlayerId, nonce: &str, body: ActionBody) -> ActionAnswer {
        self.feed(DuelEvent::Action {
            by: ActionBy::Home { home },
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

    /// A mulligan that keeps what `player` was dealt.
    fn keep_hand(&self, player: PlayerId) -> ActionBody {
        self.host
            .legal(player)
            .into_iter()
            .filter_map(|body| match &body {
                ActionBody::Mulligan { keep } => Some((keep.len(), body)),
                _ => None,
            })
            .max_by_key(|(kept, _)| *kept)
            .map(|(_, body)| body)
            .expect("a mulligan is legal")
    }

    /// Both seats keep their hands, which opens turn 1.
    fn through_the_mulligans(&mut self) {
        for (n, player) in [P1, P2].into_iter().enumerate() {
            let body = self.keep_hand(player);
            let answer = self.act(player, &format!("m-{n}"), body);
            assert!(matches!(answer, ActionAnswer::Ack { .. }), "{answer:?}");
        }
        let game = self.host.game();
        assert!(
            game.turn >= 1 && game.pending.is_none(),
            "pending {:?} nonces {:?}",
            game.pending.as_ref().map(|pending| pending.player_id),
            self.nonces()
        );
    }

    /// The seat whose turn it is (who goes first is the engine's to say).
    fn active(&self) -> PlayerId {
        self.host.game().active
    }

    fn clocks_p2_grace(&self) -> Option<i64> {
        self.host.clocks().grace_deadline.p2
    }
}

mod the_opening {
    use super::*;

    #[test]
    fn r79_a_new_host_arms_the_ceiling_and_the_mulligan_clock_at_once() {
        let (host, outputs) = DuelHost::new(setup(opening("open-1")), EPOCH);
        let timers: Vec<TimerCommand> = outputs.iter().filter_map(DuelOutput::timer).collect();
        assert!(
            timers.iter().any(|command| matches!(
                command,
                TimerCommand::Set { key: TimerKey::Ceiling, at, .. } if *at == EPOCH + MATCH_CEILING_MINUTES * MINUTE
            )),
            "{timers:?}"
        );
        // R268: setup is nobody's turn, so one mulligan deadline runs for both seats.
        assert!(
            timers.iter().any(|command| matches!(
                command,
                TimerCommand::Set { key: TimerKey::Mulligan, at, .. } if *at == EPOCH + mulligan_ms()
            )),
            "{timers:?}"
        );
        assert!(!timers.iter().any(|command| matches!(
            command,
            TimerCommand::Set {
                key: TimerKey::Turn,
                ..
            }
        )));
        assert_eq!(host.clocks().prompt_deadline, Some(EPOCH + mulligan_ms()));
        assert_eq!(host.remaining_for(P1, EPOCH), Some(mulligan_ms()));
        assert_eq!(host.remaining_for(P2, EPOCH), Some(mulligan_ms()));
    }

    #[test]
    fn r268_the_clock_running_out_times_out_every_seat_still_owing_in_seat_order_and_turn_1_starts() {
        let mut table = Table::new(setup(opening("open-2")));
        table.feed(DuelEvent::Arm);
        table.sleep(mulligan_ms() - 1);
        assert_eq!(table.log.len(), 0);
        table.sleep(1);
        // One `timeout` per seat, each stamped with its own seat, under the server's nonces (R270).
        assert_eq!(table.nonces(), vec!["srv-mulligan-1", "srv-mulligan-2"]);
        assert_eq!(table.log[0].1.player_id, P1);
        assert_eq!(table.log[1].1.player_id, P2);
        assert!(
            table
                .log
                .iter()
                .all(|(_, action)| action.body == ActionBody::Timeout)
        );
        assert!(table.host.game().turn >= 1);
        // The turn clock is the first seat's, from full; the mulligan clock is gone.
        let first = table.active();
        assert_eq!(table.host.remaining_for(first, table.now), Some(turn_ms()));
        assert_eq!(table.host.remaining_for(first.opponent(), table.now), None);
        assert!(table.timer(TimerKey::Mulligan).is_none());
    }

    #[test]
    fn r268_a_seat_that_answered_is_not_timed_out_and_its_answer_does_not_move_the_deadline() {
        let mut table = Table::new(setup(opening("open-3")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        let deadline = table.host.clocks().prompt_deadline;
        table.sleep(20 * SECOND);
        let keep = table.keep_hand(P1);
        assert!(matches!(
            table.act(P1, "mine", keep),
            ActionAnswer::Ack { seq: 1 }
        ));
        assert_eq!(table.host.clocks().prompt_deadline, deadline);
        table.sleep(mulligan_ms() - 20 * SECOND);
        // Seq 2 is the only seat still owing: its timeout is `srv-mulligan-2`.
        assert_eq!(table.nonces(), vec!["mine", "srv-mulligan-2"]);
        assert_eq!(table.log[1].1.player_id, P2);
    }

    #[test]
    fn r265_r266_each_seat_is_told_which_mulligan_it_owes_and_neither_names_the_sealed_answer() {
        let mut table = Table::new(setup(opening("open-4")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        let keep = table.keep_hand(P1);
        table.act(P1, "p1-keeps", keep);
        // The seat that answered learns the other still owes; the owing seat's own prompt is not resent
        // (it has not changed), and no frame carries the sealed answer.
        let prompts: Vec<&SeatMessage> = table
            .sent
            .p1
            .iter()
            .filter(|message| {
                matches!(
                    message,
                    SeatMessage::PromptForYou { .. } | SeatMessage::PromptForOpponent { .. }
                )
            })
            .collect();
        assert!(
            prompts
                .iter()
                .any(|message| matches!(message, SeatMessage::PromptForOpponent { pending_for: P2, .. })),
            "{prompts:?}"
        );
        for message in table.sent.p1.iter().chain(table.sent.p2.iter()) {
            if let SeatMessage::View { view, .. } = message {
                let json = serde_json::to_string(view).expect("view");
                assert!(!json.contains("p1-keeps"));
            }
        }
    }
}

mod the_turn_clock {
    use super::*;

    #[test]
    fn r79_the_active_players_clock_running_out_logs_a_timeout_under_a_server_nonce() {
        let mut table = Table::new(setup(opening("turn-1")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let before = table.log.len();
        let next_seq = table.host.next_seq();
        let first = table.active();
        table.sleep(turn_ms());
        assert_eq!(table.log.len(), before + 1, "{:?}", table.nonces());
        let (seq, action) = table.log.last().expect("a row");
        assert_eq!(action.body, ActionBody::Timeout);
        assert_eq!(action.player_id, first);
        assert_eq!(action.nonce, format!("srv-turn-{next_seq}"));
        assert_eq!(*seq, next_seq);
        // The turn passed to the other seat, whose clock is full.
        assert_eq!(table.active(), first.opponent());
        assert_eq!(
            table.host.remaining_for(first.opponent(), table.now),
            Some(turn_ms())
        );
        assert!(table.notes.iter().any(|note| matches!(
            note,
            HostNote::ClockExpired { kind, player } if kind == "turn" && *player == first
        )));
    }

    #[test]
    fn r776000_a_row_the_log_would_not_take_runs_the_clock_out_again_and_the_same_nonce_is_retried() {
        let mut table = Table::new(setup(opening("turn-2")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let before = table.log.len();
        let next_seq = table.host.next_seq();
        let first = table.active();
        table.failing_appends = 1;
        table.sleep(turn_ms());
        assert_eq!(table.log.len(), before);
        assert_eq!(
            table.host.next_seq(),
            next_seq,
            "the state did not move over a row the log refused"
        );
        assert!(table.notes.iter().any(|note| matches!(
            note,
            HostNote::ClockRetrying { kind, in_ms, .. } if kind == "turn" && *in_ms == MATCH_SERVER_ACTION_RETRY_MS
        )));
        assert_eq!(
            table.timer(TimerKey::Turn),
            Some(table.now + MATCH_SERVER_ACTION_RETRY_MS)
        );
        table.sleep(MATCH_SERVER_ACTION_RETRY_MS);
        let (_, action) = table.log.last().expect("the retry landed");
        assert_eq!(action.nonce, format!("srv-turn-{next_seq}"));
        assert_eq!(table.active(), first.opponent());
    }
}

mod actions {
    use super::*;

    #[test]
    fn s9_3_a_resent_nonce_is_acked_again_and_writes_no_second_row() {
        let mut table = Table::new(setup(opening("act-1")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let rows = table.log.len();
        let seat = table.active();
        let first = table.act(seat, "end-1", ActionBody::EndTurn);
        let ActionAnswer::Ack { seq } = first else {
            panic!("{first:?}")
        };
        assert_eq!(table.log.len(), rows + 1);
        let again = table.act(seat, "end-1", ActionBody::EndTurn);
        assert_eq!(again, ActionAnswer::Ack { seq });
        assert_eq!(table.log.len(), rows + 1);
    }

    #[test]
    fn s9_8_a_refused_action_is_answered_verbatim_counted_for_the_account_and_leaves_the_log_alone() {
        let mut table = Table::new(setup(opening("act-2")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let rows = table.log.len();
        // The seat whose turn it is not acts.
        let waiting = table.active().opponent();
        let answer = table.act(waiting, "early", ActionBody::EndTurn);
        assert!(matches!(answer, ActionAnswer::Illegal { .. }), "{answer:?}");
        assert_eq!(table.refused, vec![waiting]);
        assert_eq!(table.log.len(), rows);
        assert!(table.notes.iter().any(|note| matches!(
            note,
            HostNote::ActionRejected { player, .. } if *player == waiting
        )));
    }

    #[test]
    fn s9_3_a_row_the_log_refused_is_not_applied_and_the_same_nonce_may_be_sent_again() {
        let mut table = Table::new(setup(opening("act-3")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let seq = table.host.next_seq();
        let hash = jackioh_engine::hash_state(table.host.game());
        let seat = table.active();
        table.failing_appends = 1;
        assert_eq!(
            table.act(seat, "end-1", ActionBody::EndTurn),
            ActionAnswer::NotRecorded
        );
        assert_eq!(table.host.next_seq(), seq);
        assert_eq!(jackioh_engine::hash_state(table.host.game()), hash);
        assert_eq!(
            table.act(seat, "end-1", ActionBody::EndTurn),
            ActionAnswer::Ack { seq }
        );
    }

    #[test]
    fn r776000_a_diverged_log_has_the_host_let_go_unless_it_is_stopped() {
        let mut table = Table::new(setup(opening("act-4")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let seat = table.active();
        let _ = table.host.step(
            DuelEvent::Action {
                by: ActionBy::Home { home: seat },
                nonce: "end-1".to_string(),
                body: ActionBody::EndTurn,
            },
            table.now,
        );
        let outputs = table.host.step(
            DuelEvent::Logged {
                outcome: LogOutcome::Diverged,
            },
            table.now,
        );
        assert!(outputs.iter().any(|output| matches!(
            output,
            DuelOutput::LetGo {
                reason: LetGo::Diverged
            }
        )));

        let mut stopped = Table::new(setup(opening("act-4")));
        stopped.feed(DuelEvent::Arm);
        stopped.through_the_mulligans();
        stopped.feed(DuelEvent::Stop);
        let seat = stopped.active();
        let _ = stopped.host.step(
            DuelEvent::Action {
                by: ActionBy::Home { home: seat },
                nonce: "end-1".to_string(),
                body: ActionBody::EndTurn,
            },
            stopped.now,
        );
        let outputs = stopped.host.step(
            DuelEvent::Logged {
                outcome: LogOutcome::Diverged,
            },
            stopped.now,
        );
        assert!(
            !outputs
                .iter()
                .any(|output| matches!(output, DuelOutput::LetGo { .. }))
        );
    }

    #[test]
    fn s9_3_a_finished_match_takes_no_more_actions() {
        let mut table = Table::new(setup(opening("act-5")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        assert!(matches!(
            table.act(P1, "gg", ActionBody::Concede),
            ActionAnswer::Ack { .. }
        ));
        assert_eq!(table.records.len(), 1);
        assert_eq!(
            table.act(P2, "late", ActionBody::EndTurn),
            ActionAnswer::MatchOver
        );
    }

    #[test]
    fn r677_the_result_is_recorded_once_with_the_seats_credited_the_boards_and_the_hash() {
        let mut table = Table::new(setup(opening("act-6")));
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        table.act(P1, "gg", ActionBody::Concede);
        assert_eq!(table.records.len(), 1);
        let record = &table.records[0];
        assert_eq!(record.outcome.winner, Winner::P2);
        assert!(!record.seats_swapped);
        assert_eq!(record.final_hash, jackioh_engine::hash_state(table.host.game()));
        assert_eq!(record.turns, table.host.game().turn);
        assert!(table.host.is_finished());
        assert_eq!(table.host.finished_at(), Some(table.now));
        // Every clock went with the game.
        assert!(table.timers.is_empty(), "{:?}", table.timers);
    }
}

mod being_here {
    use super::*;

    #[test]
    fn s9_5_a_dropped_socket_starts_the_grace_and_the_opponent_is_told() {
        let mut table = Table::new(setup(opening("here-1")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let persisted = table.persisted.len();
        let told = table.sent.p1.len();
        table.feed(DuelEvent::Left { home: P2 });
        table.feed(DuelEvent::Disconnected { home: P2 });
        assert_eq!(table.clocks_p2_grace(), Some(table.now + grace_ms()));
        assert_eq!(
            table.timer(TimerKey::Grace { player: P2 }),
            Some(table.now + grace_ms())
        );
        // §9.5: the grace is stored on the match, and the other seat's clock frame carries it.
        assert_eq!(table.persisted.len(), persisted + 1);
        assert_eq!(
            table.persisted.last().expect("stored").grace_deadline.p2,
            Some(table.now + grace_ms())
        );
        let clock = table.sent.p1[told..]
            .iter()
            .find_map(|message| match message {
                SeatMessage::Clock { clocks, .. } => Some(clocks.clone()),
                _ => None,
            })
            .expect("p1 was sent the clock");
        assert_eq!(clock.grace_deadline.p2, Some(table.now + grace_ms()));
        // The clock keeps running meanwhile (§9.5).
        assert!(table.timer(TimerKey::Turn).is_some());
    }

    #[test]
    fn s9_5_the_grace_running_out_is_a_loss_for_the_seat_that_left() {
        let mut table = Table::new(setup(opening("here-2")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        table.feed(DuelEvent::Left { home: P2 });
        table.feed(DuelEvent::Disconnected { home: P2 });
        let seq = table.host.next_seq();
        table.sleep(grace_ms());
        let (_, action) = table.log.last().expect("the loss was logged");
        assert_eq!(action.body, ActionBody::DisconnectExpired { player: P2 });
        assert_eq!(action.player_id, P2);
        assert_eq!(action.nonce, format!("srv-grace-{seq}"));
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].outcome.winner, Winner::P1);
        assert_eq!(table.records[0].outcome.reason, GameOverReason::Disconnect);
    }

    #[test]
    fn s9_5_returning_inside_the_window_ends_the_grace_and_greets_the_seat_with_a_fresh_view() {
        let mut table = Table::new(setup(opening("here-3")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        table.feed(DuelEvent::Left { home: P2 });
        table.feed(DuelEvent::Disconnected { home: P2 });
        table.sleep(grace_ms() - SECOND);
        let rows = table.log.len();
        table.greetings.clear();
        let told = table.sent.p2.len();
        table.connect(P2);
        assert_eq!(table.clocks_p2_grace(), None);
        table.sleep(grace_ms());
        assert_eq!(
            table.log.len(),
            rows + 1,
            "only the turn clock acted: {:?}",
            table.nonces()
        );
        assert!(table.records.is_empty());
        // The greeting: the host's frames around the view, then the clock, then the watchers.
        assert_eq!(
            table.greetings,
            vec![
                (P2, GreetingPart::Before),
                (P2, GreetingPart::After),
                (P2, GreetingPart::Watchers)
            ]
        );
        let kinds: Vec<&str> = table.sent.p2[told..]
            .iter()
            .map(|message| match message {
                SeatMessage::View { .. } => "view",
                SeatMessage::Clock { .. } => "clock",
                _ => "other",
            })
            .collect();
        assert_eq!(&kinds[..2], ["view", "clock"], "{kinds:?}");
    }

    #[test]
    fn r744_an_account_that_has_not_been_here_yet_is_away_from_the_first_attach() {
        let mut table = Table::new(setup(opening("here-4")));
        table.feed(DuelEvent::Arm);
        // Nobody has attached: nothing starts at boot (R112).
        assert_eq!(table.host.clocks().grace_deadline, PerPlayer::new(None, None));
        table.connect(P1);
        assert_eq!(
            table.host.clocks().grace_deadline,
            PerPlayer::new(None, Some(table.now + grace_ms()))
        );
        // The grace is stored, and p2 coming back ends it.
        assert_eq!(
            table.persisted.last().expect("stored").grace_deadline.p2,
            Some(table.now + grace_ms())
        );
        table.connect(P2);
        assert_eq!(table.clocks_p2_grace(), None);
    }

    #[test]
    fn r744_a_grace_stored_before_a_restart_is_kept_up_to_a_fresh_window() {
        let mut stored = initial_clocks(EPOCH);
        stored.grace_deadline.p2 = Some(EPOCH + 10 * SECOND);
        let mut rebuilt = setup(opening("here-5"));
        rebuilt.clocks = stored;
        let mut table = Table::new(rebuilt);
        table.feed(DuelEvent::Arm);
        // Until the first attach, the stored deadline is what is written back.
        assert_eq!(table.host.clocks().grace_deadline.p2, None);
        table.connect(P1);
        assert_eq!(table.clocks_p2_grace(), Some(EPOCH + 10 * SECOND));
        table.sleep(10 * SECOND);
        // p2 never came back, so it forfeits (the mulligan is open, so its timeout is not what ends it).
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].outcome.winner, Winner::P1);
    }

    #[test]
    fn s9_5_a_late_dropped_socket_after_the_game_starts_no_grace() {
        let mut table = Table::new(setup(opening("here-6")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        table.act(P1, "gg", ActionBody::Concede);
        table.feed(DuelEvent::Left { home: P2 });
        table.feed(DuelEvent::Disconnected { home: P2 });
        assert_eq!(table.clocks_p2_grace(), None);
        assert!(table.timers.is_empty());
    }

    #[test]
    fn stop_takes_every_clock_back_and_later_timers_do_nothing() {
        let mut table = Table::new(setup(opening("here-7")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let stale: Vec<(TimerKey, u32, i64)> = table.timers.clone();
        table.feed(DuelEvent::Stop);
        assert!(table.timers.is_empty(), "{:?}", table.timers);
        let rows = table.log.len();
        for (key, id, _) in stale {
            table.feed(DuelEvent::TimerFired { key, id });
        }
        assert_eq!(table.log.len(), rows);
        assert!(table.host.is_stopped());
    }
}

mod relays {
    use super::*;

    fn hero_aim() -> Aim {
        Aim {
            source: AimEnd::Hero { player: P1 },
            target: Some(AimEnd::Hero { player: P2 }),
        }
    }

    fn other_aim() -> Aim {
        Aim {
            source: AimEnd::Hero { player: P1 },
            target: None,
        }
    }

    fn aims(sent: &[SeatMessage]) -> Vec<Option<Aim>> {
        sent.iter()
            .filter_map(|message| match message {
                SeatMessage::Aim { aim, .. } => Some(*aim),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r738_an_aim_reaches_the_opponent_at_once_and_faster_ones_wait_out_the_interval_coalesced() {
        let mut table = Table::new(setup(opening("aim-1")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let have = aims(&table.sent.p2).len();
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hero_aim()),
        });
        assert_eq!(aims(&table.sent.p2)[have..], [Some(hero_aim())]);
        // Two more inside the interval: neither goes yet, and only the newest will.
        table.sleep(10);
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(other_aim()),
        });
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hero_aim()),
        });
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(other_aim()),
        });
        assert_eq!(aims(&table.sent.p2)[have..], [Some(hero_aim())]);
        assert!(table.timer(TimerKey::Aim { player: P1 }).is_some());
        table.sleep(AIM_RELAY_INTERVAL_MS);
        assert_eq!(
            aims(&table.sent.p2)[have..],
            [Some(hero_aim()), Some(other_aim())]
        );
        // Nothing is relayed to the sender, and a repeat is not sent again.
        assert!(aims(&table.sent.p1).is_empty());
        table.sleep(AIM_RELAY_INTERVAL_MS);
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(other_aim()),
        });
        assert_eq!(aims(&table.sent.p2)[have..].len(), 2);
    }

    #[test]
    fn r738_r97_an_aim_naming_what_the_opponent_may_not_see_is_dropped_and_clears_a_shown_arrow() {
        let mut table = Table::new(setup(opening("aim-2")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let have = aims(&table.sent.p2).len();
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hero_aim()),
        });
        // The sender's own hand card: not public to the opponent when it is past the hand.
        table.sleep(AIM_RELAY_INTERVAL_MS);
        let hidden = Aim {
            source: AimEnd::Hand {
                player: P1,
                index: 999,
            },
            target: None,
        };
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hidden),
        });
        assert_eq!(aims(&table.sent.p2)[have..], [Some(hero_aim()), None]);
        // A hand that is the opponent's own is never the sender's to point from.
        table.sleep(AIM_RELAY_INTERVAL_MS);
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(Aim {
                source: AimEnd::Hand { player: P2, index: 0 },
                target: None,
            }),
        });
        assert_eq!(aims(&table.sent.p2)[have..].len(), 2);
    }

    #[test]
    fn r738_a_seat_that_leaves_takes_its_arrow_with_it_and_the_game_ending_clears_it_too() {
        let mut table = Table::new(setup(opening("aim-3")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let have = aims(&table.sent.p2).len();
        table.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hero_aim()),
        });
        table.feed(DuelEvent::Left { home: P1 });
        assert_eq!(aims(&table.sent.p2)[have..], [Some(hero_aim()), None]);

        let mut ended = Table::new(setup(opening("aim-3")));
        ended.connect(P1);
        ended.connect(P2);
        ended.feed(DuelEvent::Arm);
        ended.through_the_mulligans();
        let have = aims(&ended.sent.p2).len();
        ended.feed(DuelEvent::Aim {
            home: P1,
            aim: Some(hero_aim()),
        });
        ended.act(P1, "gg", ActionBody::Concede);
        assert_eq!(aims(&ended.sent.p2)[have..], [Some(hero_aim()), None]);
    }

    #[test]
    fn r2739001_a_hover_with_no_open_prompt_is_dropped_silently() {
        let mut table = Table::new(setup(opening("aim-4")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        let told = table.sent.p2.len();
        table.feed(DuelEvent::Hover {
            home: P1,
            index: Some(0),
        });
        assert!(
            !table.sent.p2[told..]
                .iter()
                .any(|message| matches!(message, SeatMessage::ChoiceHover { .. }))
        );
    }
}

mod bots {
    use super::*;

    fn bot_seat(hidden: bool) -> BotSeat {
        BotSeat {
            strength: 2,
            speed: 1.0,
            persona: PersonaName::Balanced,
            hidden,
        }
    }

    /// A match whose host seed is `seed`, with a ladder bot at p2 and a person at p1.
    fn match_on(seed: &str, hidden: bool, ranked: bool) -> Table {
        let mut setup = setup(opening("bot-1"));
        setup.seed = seed.to_string();
        setup.ranked = ranked;
        setup.bots = PerPlayer::new(None, Some(bot_seat(hidden)));
        let mut table = Table::new(setup);
        table.connect(P1);
        table.feed(DuelEvent::Arm);
        table
    }

    fn with_bot_at_p2(hidden: bool) -> Table {
        match_on("seed-host", hidden, true)
    }

    fn chose(body: Option<ActionBody>) -> BotChoice {
        BotChoice {
            body,
            ai_cursor: 7,
            legal_count: 5,
            lethal: false,
            aim: None,
        }
    }

    fn decided(table: &mut Table, request: &DecideRequest, choice: BotChoice) {
        table.feed(DuelEvent::Decided {
            home: request.home,
            version: request.version,
            turn: request.turn,
            choice,
        });
    }

    #[test]
    fn r3103000_r1452_the_machine_asks_for_a_bots_move_once_and_a_bot_that_does_not_pass_for_a_person_plays_it_at_once()
     {
        let mut table = with_bot_at_p2(false);
        // The bot owes its mulligan: one request, none again while it is thinking.
        assert_eq!(table.decides.len(), 1);
        let request = table.decides[0].clone();
        assert_eq!((request.home, request.seat), (P2, P2));
        assert_eq!(request.version, table.host.next_seq());
        assert_eq!(request.ai_cursor, 0);
        table.feed(DuelEvent::Arm);
        assert_eq!(table.decides.len(), 1);
        // The host has what it needs to run the decision: the game, the bot's strength, its identity.
        let job = table.host.job(&request).expect("a bot plays at p2");
        assert_eq!((job.home, job.seat, job.hidden), (P2, P2, false));
        assert!(
            table
                .host
                .job(&DecideRequest {
                    home: P1,
                    ..request.clone()
                })
                .is_none()
        );

        let keep = table.keep_hand(P2);
        decided(&mut table, &request, chose(Some(keep)));
        assert_eq!(table.nonces(), vec![format!("srv-bot-{}", request.version)]);
        assert_eq!(table.log[0].1.player_id, P2);
        // No think time, no timer: it moved as soon as it had decided.
        assert!(table.timer(TimerKey::BotMove { player: P2 }).is_none());
    }

    #[test]
    fn r1452_a_bot_that_passes_for_a_person_waits_a_think_time_the_decision_counts_toward() {
        // The moment the move goes out, counted from the moment the host was asked, when the decision
        // took `decision_took` ms; `None` when it went out at once.
        let moved_after = |decision_took: i64| {
            let mut table = with_bot_at_p2(true);
            let request = table.decides[0].clone();
            let asked = table.now;
            let keep = table.keep_hand(P2);
            table.sleep(decision_took);
            decided(&mut table, &request, chose(Some(keep)));
            let Some(at) = table.timer(TimerKey::BotMove { player: P2 }) else {
                assert_eq!(table.nonces(), vec![format!("srv-bot-{}", request.version)]);
                return None;
            };
            // Nothing is played until the think time is out.
            assert!(table.log.is_empty());
            table.sleep(at - table.now - 1);
            assert!(table.log.is_empty());
            table.sleep(1);
            assert_eq!(table.nonces(), vec![format!("srv-bot-{}", request.version)]);
            Some(at - asked)
        };
        let think = moved_after(0).expect("a think time");
        assert!((BOT_THINK_MIN_MS..=BOT_THINK_MAX_MS).contains(&think), "{think}");
        // The time the decision took counts toward it, so the move goes out at the same moment.
        assert_eq!(moved_after(think / 2), Some(think));
        // A decision that took longer than the think time moves at once.
        assert_eq!(moved_after(think + 1000), None);
    }

    #[test]
    fn r738_a_hidden_bot_shows_its_aim_a_beat_before_its_move_and_takes_it_down_after() {
        let aim = Aim {
            source: AimEnd::Hero { player: P2 },
            target: Some(AimEnd::Hero { player: P1 }),
        };
        let aims_p1 = |table: &Table| -> Vec<Option<Aim>> {
            table
                .sent
                .p1
                .iter()
                .filter_map(|message| match message {
                    SeatMessage::Aim { aim, .. } => Some(*aim),
                    _ => None,
                })
                .collect()
        };
        let mut table = with_bot_at_p2(true);
        let request = table.decides[0].clone();
        let keep = table.keep_hand(P2);
        decided(
            &mut table,
            &request,
            BotChoice {
                aim: Some(aim),
                ..chose(Some(keep))
            },
        );
        let move_at = table
            .timer(TimerKey::BotMove { player: P2 })
            .expect("the move waits");
        // The aim is up at least the relay's interval before the move, and not before it is due.
        let aim_at = table.timer(TimerKey::BotAim { player: P2 }).unwrap_or(table.now);
        assert!(move_at - aim_at >= AIM_RELAY_INTERVAL_MS, "{aim_at} {move_at}");
        assert!(aims_p1(&table).is_empty() || aim_at == table.now);
        table.sleep(aim_at - table.now);
        assert_eq!(aims_p1(&table), vec![Some(aim)]);
        assert!(table.log.is_empty());
        table.sleep(move_at - table.now);
        assert_eq!(table.log.len(), 1);
        assert_eq!(aims_p1(&table), vec![Some(aim), None]);

        // One that does not pass for a person shows no aim at all.
        let mut plain = with_bot_at_p2(false);
        let request = plain.decides[0].clone();
        let keep = plain.keep_hand(P2);
        decided(
            &mut plain,
            &request,
            BotChoice {
                aim: Some(Aim {
                    source: AimEnd::Hero { player: P2 },
                    target: Some(AimEnd::Hero { player: P1 }),
                }),
                ..chose(Some(keep))
            },
        );
        assert!(aims_p1(&plain).is_empty());
    }

    #[test]
    fn r1452_a_move_the_bot_could_not_make_falls_back_to_a_legal_one_and_a_stale_one_is_dropped() {
        let mut table = with_bot_at_p2(false);
        let request = table.decides[0].clone();
        // Nothing decided: the fallback plays something legal that R188 allows.
        decided(&mut table, &request, BotChoice::nothing(1));
        assert_eq!(table.log.len(), 1);
        assert!(matches!(table.log[0].1.body, ActionBody::Mulligan { .. }));

        // A move decided on a state two rows old is stale: dropped when the turn has moved on.
        let mut stale = with_bot_at_p2(false);
        let request = stale.decides[0].clone();
        let keep = stale.keep_hand(P1);
        stale.act(P1, "p1", keep);
        stale.feed(DuelEvent::Decided {
            home: P2,
            version: request.version - 5,
            turn: request.turn + 5,
            choice: chose(Some(ActionBody::EndTurn)),
        });
        assert_eq!(stale.log.len(), 1, "{:?}", stale.nonces());
    }

    #[test]
    fn r1452_a_move_the_log_would_not_take_is_decided_again_after_the_write_retry_pace() {
        let mut table = with_bot_at_p2(false);
        let request = table.decides[0].clone();
        let keep = table.keep_hand(P2);
        table.failing_appends = 1;
        decided(&mut table, &request, chose(Some(keep)));
        assert!(table.log.is_empty());
        assert_eq!(
            table.timer(TimerKey::BotRetry),
            Some(table.now + BOT_WRITE_RETRY_MS)
        );
        table.sleep(BOT_WRITE_RETRY_MS);
        assert_eq!(table.decides.len(), 2);
    }

    /// A host seed whose bot at p2 plans a rematch `wanted` says it does, by R639000's fit.
    fn seed_where(wanted: impl Fn(&jackioh_shared::bots::signals::RematchPlan) -> bool) -> String {
        (0..10_000)
            .map(|k| format!("seed-rematch-{k}"))
            .find(|seed| wanted(&rematch_plan(seed, P2, &BOT_REMATCH_FIT)))
            .expect("a seed whose bot plans so")
    }

    #[test]
    fn r639000_a_bot_lingers_after_a_ranked_game_offers_a_rematch_and_meets_a_persons_offer() {
        let seed = seed_where(|plan| plan.offers && plan.accepts);
        let plan = rematch_plan(&seed, P2, &BOT_REMATCH_FIT);
        let mut table = match_on(&seed, true, true);
        // Presence: the bot is here for the whole game.
        assert_eq!(table.host.presence(table.now), PerPlayer::new(true, true));
        let keep = table.keep_hand(P1);
        table.act(P1, "p1", keep);
        let request = table.decides[0].clone();
        let keep = table.keep_hand(P2);
        decided(&mut table, &request, chose(Some(keep)));
        let at = table
            .timer(TimerKey::BotMove { player: P2 })
            .expect("a think time");
        table.sleep(at - table.now);
        table.act(P1, "gg", ActionBody::Concede);
        assert_eq!(table.records.len(), 1);
        // The bot stays for its linger, then it is gone; the person's own socket is separate.
        assert_eq!(table.host.presence(table.now), PerPlayer::new(true, true));
        // Its own offer comes `offer_after_ms` after the game's end.
        assert_eq!(
            table.timer(TimerKey::BotOffer { player: P2 }),
            Some(table.now + plan.offer_after_ms)
        );
        table.sleep(plan.offer_after_ms);
        assert_eq!(table.offers, vec![(P2, STAKE_NORMAL, false)]);
        // A person's offer is met after the reply delay, at its stakes, and the bot stays until sent.
        table.feed(DuelEvent::RematchOffered {
            from: P1,
            stakes: STAKE_NORMAL,
        });
        assert_eq!(
            table.timer(TimerKey::BotReply { player: P2 }),
            Some(table.now + plan.reply_ms)
        );
        table.sleep(plan.reply_ms);
        assert_eq!(table.offers.last(), Some(&(P2, STAKE_NORMAL, true)));
        // Answered once only.
        table.feed(DuelEvent::RematchOffered {
            from: P1,
            stakes: STAKE_NORMAL,
        });
        assert!(table.timer(TimerKey::BotReply { player: P2 }).is_none());
        // Past its linger the bot reads as gone, unless its reply is on its way.
        table.sleep(plan.linger_ms);
        assert!(table.host.presence(table.now).p2);
        table.feed(DuelEvent::RematchSent { home: P2 });
        assert!(!table.host.presence(table.now).p2);
    }

    /// A ranked match on `seed` whose person at p1 has conceded to the bot at p2 that passes for one.
    fn conceded_to_the_bot(seed: &str) -> Table {
        let mut table = match_on(seed, true, true);
        let keep = table.keep_hand(P1);
        table.act(P1, "p1", keep);
        let request = table.decides[0].clone();
        let keep = table.keep_hand(P2);
        decided(&mut table, &request, chose(Some(keep)));
        let at = table
            .timer(TimerKey::BotMove { player: P2 })
            .expect("a think time");
        table.sleep(at - table.now);
        table.act(P1, "gg", ActionBody::Concede);
        assert_eq!(table.records.len(), 1);
        table
    }

    #[test]
    fn r3213000_a_bots_linger_ending_is_a_presence_change_unless_its_reply_is_on_its_way() {
        // A bot that meets the person's offer: its linger's end is armed with the game's end, and passes
        // with no change while its reply is on its way; the reply landing is the change.
        let seed = seed_where(|plan| plan.accepts);
        let plan = rematch_plan(&seed, P2, &BOT_REMATCH_FIT);
        let mut table = conceded_to_the_bot(&seed);
        let end = table.now;
        assert_eq!(
            table.timer(TimerKey::BotLeaves { player: P2 }),
            Some(end + plan.linger_ms)
        );
        table.feed(DuelEvent::RematchOffered {
            from: P1,
            stakes: STAKE_NORMAL,
        });
        table.sleep(plan.linger_ms.max(plan.reply_ms));
        assert_eq!(table.offers.last(), Some(&(P2, STAKE_NORMAL, true)));
        assert!(table.host.presence(table.now).p2);
        assert_eq!(table.presence_changes, 0);
        table.feed(DuelEvent::RematchSent { home: P2 });
        assert_eq!(table.presence_changes, 1);
        assert!(!table.host.presence(table.now).p2);

        // A bot that meets no offer is gone as its linger ends, and that is the change.
        let seed = seed_where(|plan| !plan.accepts);
        let plan = rematch_plan(&seed, P2, &BOT_REMATCH_FIT);
        let mut table = conceded_to_the_bot(&seed);
        table.sleep(plan.linger_ms - 1);
        assert_eq!(table.presence_changes, 0);
        assert!(table.host.presence(table.now).p2);
        table.sleep(1);
        assert_eq!(table.presence_changes, 1);
        assert!(!table.host.presence(table.now).p2);
    }

    #[test]
    fn r3103000_r639000_a_bot_that_does_not_pass_for_a_person_stays_for_no_rematch() {
        let seed = seed_where(|plan| plan.offers && plan.accepts);
        let mut table = match_on(&seed, false, true);
        table.act(P1, "gg", ActionBody::Concede);
        assert_eq!(table.records.len(), 1);
        assert!(table.timer(TimerKey::BotOffer { player: P2 }).is_none());
        assert!(!table.host.presence(table.now).p2);
        table.feed(DuelEvent::RematchOffered {
            from: P1,
            stakes: STAKE_NORMAL,
        });
        assert!(table.timer(TimerKey::BotReply { player: P2 }).is_none());
    }

    #[test]
    fn r641000_a_series_game_and_a_game_between_two_bots_offer_no_rematch() {
        let mut series = setup(opening("bot-2"));
        series.ranked = true;
        series.series_game = true;
        series.bots = PerPlayer::new(None, Some(bot_seat(true)));
        let mut table = Table::new(series);
        table.connect(P1);
        table.feed(DuelEvent::Arm);
        table.act(P1, "gg", ActionBody::Concede);
        assert_eq!(table.records.len(), 1);
        assert!(table.timer(TimerKey::BotOffer { player: P2 }).is_none());
        assert!(!table.host.presence(table.now).p2);
    }

    #[test]
    fn r3103000_r639000_a_bot_that_passes_for_a_person_may_let_its_turn_clock_run_out_and_one_that_does_not_never_does()
     {
        // The turn p2 first owes a move on its own turn, found on a scratch match.
        let mut probe = match_on("seed-probe", false, true);
        probe.through_the_mulligans();
        let first = probe.host.game().turn + i32::from(probe.active() != P2);
        let seed = (0..10_000)
            .map(|k| format!("seed-idle-{k}"))
            .find(|seed| idles_turn(seed, P2, first, &BOT_TIMEOUT_FIT))
            .expect("a seed whose bot idles on that turn");
        let owes_a_move_on_its_own_turn = |hidden: bool| {
            let mut table = match_on(&seed, hidden, true);
            // Its mulligan was asked first; the table answers it for the bot, as the opening is over.
            let mulligan = table.decides[0].clone();
            table.through_the_mulligans();
            decided(&mut table, &mulligan, BotChoice::nothing(0));
            if table.active() != P2 {
                table.act(P1, "p1-end", ActionBody::EndTurn);
            }
            assert_eq!((table.active(), table.host.game().turn), (P2, first));
            table
                .decides
                .iter()
                .any(|request| request.own_turn_start && request.turn == first)
        };
        assert!(!owes_a_move_on_its_own_turn(true));
        assert!(owes_a_move_on_its_own_turn(false));
    }

    /// The emotes a hidden bot at p2 sends in the first seconds of a match on `seed`.
    fn emotes_after_the_mulligans(seed: &str, hidden: bool) -> (Vec<(PlayerId, EmoteId)>, i64) {
        let mut table = match_on(seed, hidden, true);
        let before = table.now;
        table.through_the_mulligans();
        table.sleep(10 * SECOND);
        let at = table.now - before;
        (table.emotes, at)
    }

    #[test]
    fn r3103000_r645_a_bot_that_passes_for_a_person_emotes_by_its_persona_after_a_delay_and_one_that_does_not_never_does()
     {
        let sent = (0..60)
            .map(|k| format!("seed-chat-{k}"))
            .find(|seed| !emotes_after_the_mulligans(seed, true).0.is_empty())
            .expect("a seed whose bot greets after the mulligan");
        let (emotes, _) = emotes_after_the_mulligans(&sent, true);
        assert!(emotes.iter().all(|(home, _)| *home == P2));
        // Every emote waited out its delay before it went (the host relays it as a person's).
        let mut table = match_on(&sent, true, true);
        table.through_the_mulligans();
        assert!(table.emotes.is_empty(), "no emote before its delay");
        let due = table
            .timer(TimerKey::BotEmote { player: P2 })
            .expect("an emote is due");
        assert!(due >= table.now + 800, "{due} {}", table.now);
        // A bot that does not pass for a person has no persona.
        assert!(emotes_after_the_mulligans(&sent, false).0.is_empty());
    }

    #[test]
    fn r645_a_persons_emote_may_earn_the_bot_a_reply_unless_the_pair_is_muted() {
        let reply = |seed: &str, muted: bool| {
            let mut table = match_on(seed, true, true);
            table.through_the_mulligans();
            table.sleep(10 * SECOND);
            table.emotes.clear();
            if muted {
                table.feed(DuelEvent::EmotesMuted);
            }
            table.feed(DuelEvent::Emote {
                home: P1,
                emote: EmoteId::Greetings,
            });
            table.sleep(10 * SECOND);
            table.emotes
        };
        let seed = (0..60)
            .map(|k| format!("seed-reply-{k}"))
            .find(|seed| !reply(seed, false).is_empty())
            .expect("a seed whose bot answers a greeting");
        assert_eq!(
            reply(&seed, false)
                .iter()
                .map(|(home, _)| *home)
                .collect::<Vec<_>>(),
            vec![P2]
        );
        assert!(reply(&seed, true).is_empty());
    }
}

mod the_checkpoint {
    use super::*;

    #[test]
    fn a_host_resumes_from_its_state_as_json_and_steps_as_the_original_does() {
        let mut table = Table::new(setup(opening("save-1")));
        table.connect(P1);
        table.connect(P2);
        table.feed(DuelEvent::Arm);
        table.through_the_mulligans();
        table.feed(DuelEvent::Aim { home: P1, aim: None });
        let json = serde_json::to_string(&table.host).expect("the host serializes");
        let mut resumed: DuelHost = serde_json::from_str(&json).expect("the host reads back");
        assert_eq!(resumed, table.host);
        let event = DuelEvent::Action {
            by: ActionBy::Home { home: table.active() },
            nonce: "end".to_string(),
            body: ActionBody::EndTurn,
        };
        let theirs = resumed.step(event.clone(), table.now);
        let ours = table.host.step(event, table.now);
        assert_eq!(
            serde_json::to_value(&theirs).unwrap(),
            serde_json::to_value(&ours).unwrap()
        );
        assert_eq!(resumed, table.host);
    }

    #[test]
    fn events_and_outputs_read_back_from_json() {
        let event = DuelEvent::Decided {
            home: P2,
            version: 4,
            turn: 2,
            choice: BotChoice {
                body: Some(ActionBody::EndTurn),
                ai_cursor: 1,
                legal_count: 3,
                lethal: false,
                aim: None,
            },
        };
        let json = serde_json::to_string(&event).expect("event");
        assert_eq!(serde_json::from_str::<DuelEvent>(&json).expect("read"), event);
        let output = DuelOutput::Timer {
            command: TimerCommand::Set {
                key: TimerKey::Grace { player: P1 },
                id: 3,
                at: 99,
            },
        };
        let json = serde_json::to_string(&output).expect("output");
        assert_eq!(serde_json::from_str::<DuelOutput>(&json).expect("read"), output);
        let expiry = ClockExpiry::Grace { player: P2 };
        assert_eq!(
            serde_json::from_str::<ClockExpiry>(&serde_json::to_string(&expiry).unwrap()).unwrap(),
            expiry
        );
    }
}
