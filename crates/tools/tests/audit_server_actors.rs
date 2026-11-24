//! Audit: the 1v1 match host's liveness (SPEC §9.5, R79). Temporary search harness.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{Action, ActionBody, ActionType, DECK_SIZE, GameState, PLAYER_IDS, PerPlayer, Rng};
use jackioh_shared::config::PROMPT_CLOCK_SECONDS;
use jackioh_shared::host::clock::initial_clocks;
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionAnswer, ActionBy, DuelEvent, DuelHost, DuelOutput, DuelSetup, LogOutcome,
};

const EPOCH: i64 = 1_700_000_000_000;
const SECOND: i64 = 1000;

fn opening(seed: &str) -> GameState {
    jackioh_cards::register_all();
    let deck = |salt: &str| {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:{salt}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
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

/// A host standing in for the real one: it keeps the timers, writes every row, and nothing else.
#[derive(Clone)]
struct Table {
    host: DuelHost,
    now: i64,
    timers: Vec<(TimerKey, u32, i64)>,
    log: Vec<(i64, Action)>,
    answers: Vec<(String, ActionAnswer)>,
    records: usize,
    /// How many of the next appends the log refuses.
    failing_appends: usize,
}

impl Table {
    fn new(setup: DuelSetup) -> Table {
        let (host, outputs) = DuelHost::new(setup, EPOCH);
        let mut table = Table {
            host,
            now: EPOCH,
            timers: Vec::new(),
            log: Vec::new(),
            answers: Vec::new(),
            records: 0,
            failing_appends: 0,
        };
        table.run(outputs);
        table
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
                DuelOutput::Record { .. } => {
                    self.records += 1;
                    self.feed(DuelEvent::ResultRecorded);
                }
                _ => {}
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

    fn act(&mut self, seat: PlayerId, nonce: &str, body: ActionBody) {
        self.feed(DuelEvent::Action {
            by: ActionBy::Seat { seat },
            nonce: nonce.to_string(),
            body,
        });
    }

    fn timer(&self, key: TimerKey) -> Option<i64> {
        self.timers
            .iter()
            .find(|(held, _, _)| *held == key)
            .map(|(_, _, at)| *at)
    }
}

fn describe(table: &Table) -> String {
    let game = table.host.game();
    format!(
        "turn {} active {:?} pending {:?} mulligan {} timers {:?} last rows {:?}",
        game.turn,
        game.active,
        game.pending
            .as_ref()
            .map(|pending| (
                pending.player_id,
                format!("{:?}", pending.kind),
                pending.prompt.clone(),
                pending.id.clone(),
                pending.options.len(),
                serde_json::to_string(&pending.resume).unwrap_or_default().chars().take(400).collect::<String>()
            )),
        game.mulligan.is_some(),
        table
            .timers
            .iter()
            .map(|(key, _, at)| (format!("{key:?}"), at - table.now))
            .collect::<Vec<_>>(),
        table
            .log
            .iter()
            .rev()
            .take(4)
            .map(|(seq, action)| format!("{seq}:{}:{:?}:{}", action.player_id, action.action_type(), action.nonce))
            .collect::<Vec<_>>(),
    )
}

/// Whether the match waits on a prompt of the seat not on turn with no clock of its own armed (finding A).
fn unclocked_holder(table: &Table) -> Option<PlayerId> {
    let game = table.host.game();
    let holder = game.pending.as_ref().map(|pending| pending.player_id)?;
    (holder != game.active
        && game.result.is_none()
        && table.timer(TimerKey::Prompt).is_none()
        && table.timer(TimerKey::Sequence).is_none())
    .then_some(holder)
}

/// With every client silent, does the match move on within `within_ms`? The turn key changing or the game
/// ending counts. Finding A (a prompt with no clock) is counted in `known` and stepped over with the
/// timeout its clock would have sent, so the search goes on past it.
fn probe(table: &Table, within_ms: i64, failing: usize, known: &mut usize) -> Result<(), String> {
    let mut silent = table.clone();
    silent.failing_appends = failing;
    let game = silent.host.game();
    if game.result.is_some() {
        return Ok(());
    }
    let key = |game: &GameState| (game.turn, game.active);
    let before = key(game);
    let rows = silent.log.len();
    let step = 5 * SECOND;
    let mut waited = 0;
    let mut stepped = 0;
    while waited < within_ms {
        silent.sleep(step);
        waited += step;
        if let Some((nonce, answer)) = silent
            .answers
            .iter()
            .skip(table.answers.len())
            .find(|(nonce, answer)| nonce.starts_with("srv-") && answer.is_illegal())
        {
            return Err(format!("REFUSED {nonce} {answer:?}; before: {}", describe(table)));
        }
        let game = silent.host.game();
        if game.result.is_some() || key(game) != before {
            return Ok(());
        }
        if let Some(holder) = unclocked_holder(&silent) {
            if stepped == 0 {
                *known += 1;
            }
            stepped += 1;
            silent.act(holder, &format!("fix-{stepped}-{waited}"), ActionBody::Timeout);
        }
    }
    Err(format!(
        "STUCK for {} s with {} rows written meanwhile (failing {failing}); before: {} || after: {}",
        within_ms / SECOND,
        silent.log.len() - rows,
        describe(table),
        describe(&silent)
    ))
}

#[test]
fn search_liveness() {
    let seeds: u64 = std::env::var("AUDIT_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let from: u64 = std::env::var("AUDIT_FROM").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut found = Vec::new();
    let mut known = 0;
    let mut ends: Vec<String> = Vec::new();
    for seed in from..from + seeds {
        let name = format!("audit-live-{seed}");
        let mut rng = Rng::new(&format!("{name}:policy"), 0);
        let mut table = Table::new(setup(opening(&name)));
        table.feed(DuelEvent::Arm);
        table.connect(P1);
        table.connect(P2);
        let mut here = PerPlayer::new(true, true);
        for step in 0..400 {
            if table.host.game().result.is_some() {
                break;
            }
            if let Some((nonce, answer)) = table
                .answers
                .iter()
                .find(|(nonce, answer)| nonce.starts_with("srv-") && answer.is_illegal())
            {
                found.push(format!("seed {seed} step {step}: REFUSED {nonce} {answer:?} {}", describe(&table)));
                break;
            }
            let failing = if rng.int(4) == 0 { rng.int(4).max(0) as usize } else { 0 };
            if let Err(why) = probe(&table, 600 * SECOND, failing, &mut known) {
                found.push(format!("seed {seed} step {step}: {why}"));
                break;
            }
            // A step over finding A in the played game too, or the game never leaves it.
            if let Some(holder) = unclocked_holder(&table) {
                if rng.int(3) == 0 {
                    table.act(holder, &format!("fix-{step}"), ActionBody::Timeout);
                    continue;
                }
            }
            // A socket drops now and then, and comes back soon.
            let away: Vec<PlayerId> = PLAYER_IDS.into_iter().filter(|home| !here[*home]).collect();
            if let Some(home) = away.first().copied() {
                if rng.int(3) == 0 {
                    table.connect(home);
                    here[home] = true;
                    continue;
                }
            } else if rng.int(80) == 0 {
                let home = if rng.int(2) == 0 { P1 } else { P2 };
                table.feed(DuelEvent::Left { home });
                table.feed(DuelEvent::Disconnected { home });
                here[home] = false;
                continue;
            }
            let mut choices: Vec<(PlayerId, ActionBody)> = Vec::new();
            for player in PLAYER_IDS {
                for body in table.host.legal(player) {
                    if matches!(
                        body.action_type(),
                        ActionType::Concede
                            | ActionType::OfferDraw
                            | ActionType::AnswerDraw
                            | ActionType::Emote
                            | ActionType::SetAutoEndTurn
                    ) {
                        continue;
                    }
                    choices.push((player, body));
                }
            }
            if choices.is_empty() {
                table.sleep(80 * SECOND);
                continue;
            }
            // Sometimes nobody acts for a while, so clocks run out mid-game too.
            if rng.int(10) == 0 {
                table.sleep(i64::from(rng.int(40)) * SECOND);
                continue;
            }
            let index = rng.int(choices.len() as i32).max(0) as usize;
            let (player, body) = choices.swap_remove(index);
            table.act(player, &format!("c-{step}"), body);
            table.sleep(i64::from(rng.int(3)) * SECOND);
        }
        let game = table.host.game();
        ends.push(format!("{}:{}:{}", game.turn, table.log.len(), game.result.map_or("-".to_string(), |r| format!("{:?}", r.reason))));
    }
    for line in &found {
        println!("{line}\n");
    }
    println!("ends: {ends:?}");
    println!("{} findings over {seeds} seeds; finding A met {known} times", found.len());
    assert!(found.is_empty());
}

const GATINSIR: &str = "stinkyscape-022";
const PHALANX: &str = "stinkyscape-082";
const MR_VANILLA: &str = "core-008";

/// SPEC §9.5: "A prompt held by the non-active player has its own 30 s clock." R79: the turn clock "pauses
/// while a prompt is open for the non-active player (a trap firing on the opponent's turn), which runs its own
/// prompt clock of 30 s", and a timeout that leaves a prompt open is carried on by "the next expiry".
///
/// p2's Gatinsir (S #22) reveals as p1's Unit attacks and summons two Phalanxes (S #82), whose Cries ask p2,
/// who is not on turn, to Discover a Weapon four times in a row (R2176000). p2 says nothing. The first
/// question's clock runs out and its `timeout` answers it; the second question then has no clock at all, and
/// p1's turn clock stays paused, so nothing but the 120 minute ceiling (or a concession) moves the match on.
#[test]
fn r79_a_question_that_follows_a_timed_out_one_for_the_seat_not_on_turn_has_a_prompt_clock() {
    use jackioh_engine::testkit::{cursor_where, json, scenario};
    jackioh_cards::register_all();
    let filler = json!([MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA]);
    let mut built = scenario(json!({
        "seed": "audit-r79-second-prompt",
        "active": "p1",
        "p1": { "field": [{ "def": MR_VANILLA, "lane": 1 }], "library": filler },
        "p2": { "backrow": [{ "def": GATINSIR, "lane": 5 }], "library": filler },
    }));
    // Gatinsir's 30% roll is the match rng's next draw: put the cursor where it reveals.
    let seed = built.state().seed.clone();
    built.state_mut().rng_cursor = cursor_where(&seed, |cursor| Rng::new(&seed, cursor).next() < 0.30);
    let attacker = built.unit(P1, 1).expect("p1's Mr. Vanilla").id;

    let mut table = Table::new(setup(built.state().clone()));
    table.feed(DuelEvent::Arm);
    table.connect(P1);
    table.connect(P2);
    let attack = table
        .host
        .legal(P1)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Attack { attacker_id, .. } if *attacker_id == attacker))
        .expect("Mr. Vanilla may attack");
    table.act(P1, "attack", attack);

    let asked = |table: &Table| {
        let game = table.host.game();
        let pending = game.pending.as_ref().expect("a question is open");
        assert_eq!((game.active, pending.player_id), (P1, P2), "PREMISE: it asks p2 on p1's turn");
        pending.id.clone()
    };
    let phalanxes = table.host.game().players[P2]
        .units
        .iter()
        .flatten()
        .flatten()
        .filter(|card| card.def_id == PHALANX)
        .count();
    assert!(phalanxes > 0, "PREMISE: Gatinsir revealed and summoned its Phalanxes");
    let first = asked(&table);
    assert_eq!(
        table.timer(TimerKey::Prompt),
        Some(table.now + PROMPT_CLOCK_SECONDS * SECOND),
        "the first question has its 30 s"
    );

    // p2 says nothing: the question's clock runs out, and its timeout answers it (R79).
    table.sleep(PROMPT_CLOCK_SECONDS * SECOND);
    let (_, last) = table.log.last().expect("the timeout is logged");
    assert_eq!((last.player_id, &last.body), (P2, &ActionBody::Timeout));
    let second = asked(&table);
    assert_ne!(first, second, "PREMISE: the Cry's next Discover asks p2 again");

    // §9.5, R79: this question is held by the non-active player too, so it has a 30 s clock of its own.
    assert!(
        table.timer(TimerKey::Prompt).is_some(),
        "no prompt clock runs for the second question; timers armed: {:?}",
        table.timers
    );
    assert!(table.host.clocks().prompt_deadline.is_some());
}

/// The same state, read by what the players see: ten minutes on, with both seats here and silent, the match
/// still waits on that question.
#[test]
fn r79_a_silent_seat_not_on_turn_cannot_hold_the_match_past_its_prompt_clocks() {
    use jackioh_engine::testkit::{cursor_where, json, scenario};
    jackioh_cards::register_all();
    let filler = json!([MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA]);
    let mut built = scenario(json!({
        "seed": "audit-r79-second-prompt",
        "active": "p1",
        "p1": { "field": [{ "def": MR_VANILLA, "lane": 1 }], "library": filler },
        "p2": { "backrow": [{ "def": GATINSIR, "lane": 5 }], "library": filler },
    }));
    let seed = built.state().seed.clone();
    built.state_mut().rng_cursor = cursor_where(&seed, |cursor| Rng::new(&seed, cursor).next() < 0.30);
    let attacker = built.unit(P1, 1).expect("p1's Mr. Vanilla").id;
    let mut table = Table::new(setup(built.state().clone()));
    table.feed(DuelEvent::Arm);
    table.connect(P1);
    table.connect(P2);
    let attack = table
        .host
        .legal(P1)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Attack { attacker_id, .. } if *attacker_id == attacker))
        .expect("Mr. Vanilla may attack");
    table.act(P1, "attack", attack);
    assert!(
        table.host.game().pending.as_ref().is_some_and(|pending| pending.player_id == P2),
        "PREMISE: Gatinsir's Phalanx asks p2 on p1's turn"
    );
    let turn = table.host.game().turn;

    // Four questions at 30 s each and what is left of p1's 75 s turn fit in ten minutes many times over.
    table.sleep(10 * 60 * SECOND);
    let game = table.host.game();
    assert!(
        game.result.is_some() || game.turn != turn,
        "R79: ten minutes on, turn {} is still p1's and waits on p2's question {:?}; rows {:?}; timers {:?}",
        game.turn,
        game.pending.as_ref().map(|pending| (pending.player_id, pending.prompt.clone())),
        table.log.iter().map(|(seq, action)| format!("{seq}:{}", action.nonce)).collect::<Vec<_>>(),
        table.timers
    );
}
