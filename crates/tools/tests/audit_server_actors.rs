//! Audit: the 1v1 match host's liveness (SPEC §9.5, R79). Temporary search harness.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{Action, ActionBody, ActionType, DECK_SIZE, GameState, PLAYER_IDS, PerPlayer, Rng};
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
                    self.log.push((seq, action));
                    self.feed(DuelEvent::Logged {
                        outcome: LogOutcome::Written,
                    });
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
            .map(|pending| (pending.player_id, format!("{:?}", pending.kind), pending.prompt.clone())),
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

/// With every client silent, does the match move on within `within_ms`? The turn key changing, the game
/// ending, or the prompt's holder changing all count.
fn probe(table: &Table, within_ms: i64) -> Result<(), String> {
    let mut silent = table.clone();
    let game = silent.host.game();
    if game.result.is_some() {
        return Ok(());
    }
    let key = |game: &GameState| (game.turn, game.active);
    let before = key(game);
    let rows = silent.log.len();
    let step = 5 * SECOND;
    let mut waited = 0;
    while waited < within_ms {
        silent.sleep(step);
        waited += step;
        let game = silent.host.game();
        if game.result.is_some() || key(game) != before {
            // A clock action the engine refused is worth a look too.
            if let Some((nonce, answer)) = silent
                .answers
                .iter()
                .skip(table.answers.len())
                .find(|(nonce, answer)| nonce.starts_with("srv-") && answer.is_illegal())
            {
                return Err(format!("REFUSED {nonce} {answer:?}; before: {}", describe(table)));
            }
            return Ok(());
        }
    }
    Err(format!(
        "STUCK for {} s with {} rows written meanwhile; before: {} || after: {}",
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
    for seed in from..from + seeds {
        let name = format!("audit-live-{seed}");
        let mut rng = Rng::new(&format!("{name}:policy"), 0);
        let mut table = Table::new(setup(opening(&name)));
        table.feed(DuelEvent::Arm);
        table.connect(P1);
        table.connect(P2);
        for step in 0..400 {
            if table.host.game().result.is_some() {
                break;
            }
            if let Err(why) = probe(&table, 600 * SECOND) {
                found.push(format!("seed {seed} step {step}: {why}"));
                break;
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
    }
    for line in &found {
        println!("{line}\n");
    }
    println!("{} findings over {seeds} seeds", found.len());
    assert!(found.is_empty());
}
