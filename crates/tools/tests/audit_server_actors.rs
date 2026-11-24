//! Audit: the 1v1 match host's prompt clock (`jackioh_shared::host`; SPEC §9.5, R79), on a `Table` standing in
//! for a host as `crates/shared/tests/duel.rs`'s does: it keeps the timers the machine asks for, writes every
//! row it asks for, and decides nothing.
//!
//! Both tests fail today: a question for the seat not on turn that follows one its prompt clock timed out has
//! no clock, and the turn clock stays paused under it, so the match waits on it until the 120 minute ceiling.

use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::testkit::{cursor_where, json, scenario};
use jackioh_engine::{Action, ActionBody, GameState, PerPlayer, Rng};
use jackioh_shared::config::PROMPT_CLOCK_SECONDS;
use jackioh_shared::host::clock::initial_clocks;
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{ActionBy, DuelEvent, DuelHost, DuelOutput, DuelSetup, LogOutcome};

const EPOCH: i64 = 1_700_000_000_000;
const SECOND: i64 = 1000;

/// S #22 Gatinsir, a Trap: "30% chance to reveal when an enemy Unit attacks: Summon 2 Warp Disruptors, then 2
/// Phalanxes. The attack goes on." Each Phalanx it summons runs its Cry for Gatinsir's controller (R2176000).
const GATINSIR: &str = "stinkyscape-022";
/// S #82 Phalanx: "Cry: Discover 2 Rare Weapons and Equip them into this", two Discovers in turn.
const PHALANX: &str = "stinkyscape-082";
const MR_VANILLA: &str = "core-008";

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
}

impl Table {
    fn new(setup: DuelSetup) -> Table {
        let (host, outputs) = DuelHost::new(setup, EPOCH);
        let mut table = Table {
            host,
            now: EPOCH,
            timers: Vec::new(),
            log: Vec::new(),
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
                DuelOutput::Record { .. } => self.feed(DuelEvent::ResultRecorded),
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

    fn timer(&self, key: TimerKey) -> Option<i64> {
        self.timers
            .iter()
            .find(|(held, _, _)| *held == key)
            .map(|(_, _, at)| *at)
    }

    fn rows(&self) -> Vec<String> {
        self.log
            .iter()
            .map(|(seq, action)| format!("{seq}:{}:{}", action.player_id, action.nonce))
            .collect()
    }
}

/// p1's Mr. Vanilla has attacked into p2's face-down Gatinsir, which revealed: both seats are here, it is p1's
/// turn, and the first Phalanx's Cry asks p2 its first Discover.
fn gatinsir_asks_p2() -> Table {
    jackioh_cards::register_all();
    let filler = json!([MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA]);
    let mut built = scenario(json!({
        "seed": "audit-r79-second-prompt",
        "active": "p1",
        "p1": { "field": [{ "def": MR_VANILLA, "lane": 1 }], "library": filler },
        "p2": { "backrow": [{ "def": GATINSIR, "lane": 5 }], "library": filler },
    }));
    // Gatinsir's 30% roll is the match rng's next draw: the cursor is put where it reveals.
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
    table.feed(DuelEvent::Action {
        by: ActionBy::Home { home: P1 },
        nonce: "attack".to_string(),
        body: attack,
    });
    let phalanxes = table.host.game().players[P2]
        .units
        .iter()
        .flatten()
        .flatten()
        .filter(|card| card.def_id == PHALANX)
        .count();
    assert_eq!(phalanxes, 2, "PREMISE: Gatinsir revealed and summoned its Phalanxes");
    table
}

/// The id of the question open for p2 on p1's turn.
fn asked_of_p2(table: &Table) -> String {
    let game = table.host.game();
    let pending = game.pending.as_ref().expect("PREMISE: a question is open");
    assert_eq!(
        (game.active, pending.player_id),
        (P1, P2),
        "PREMISE: it asks p2 on p1's turn"
    );
    pending.id.clone()
}

/// SPEC §9.5: "A prompt held by the non-active player has its own 30 s clock." R79: the turn clock "pauses
/// while a prompt is open for the non-active player (a trap firing on the opponent's turn), which runs its own
/// prompt clock of 30 s", and a timeout that leaves a prompt open is carried on by "the next expiry".
///
/// p2 says nothing. The first Discover's clock runs out and its `timeout` answers it; the Cry's second
/// Discover then asks p2 again and is given no clock at all (`MatchClock::sync` arms the prompt clock only when
/// the holder changes), while p1's turn clock stays paused.
#[test]
fn r79_a_question_that_follows_a_timed_out_one_for_the_seat_not_on_turn_has_a_prompt_clock() {
    let mut table = gatinsir_asks_p2();
    let first = asked_of_p2(&table);
    assert_eq!(
        table.timer(TimerKey::Prompt),
        Some(table.now + PROMPT_CLOCK_SECONDS * SECOND),
        "the first question has its 30 s"
    );

    table.sleep(PROMPT_CLOCK_SECONDS * SECOND);
    let (_, last) = table.log.last().expect("the timeout is logged");
    assert_eq!(
        (last.player_id, &last.body),
        (P2, &ActionBody::Timeout),
        "{:?}",
        table.rows()
    );
    let second = asked_of_p2(&table);
    assert_ne!(
        first, second,
        "PREMISE: the timeout answered the first Discover and the second asks p2"
    );

    assert!(
        table.timer(TimerKey::Prompt).is_some() && table.host.clocks().prompt_deadline.is_some(),
        "§9.5, R79: no prompt clock runs for p2's second question; timers armed: {:?}, clocks: {:?}",
        table.timers,
        table.host.clocks()
    );
}

/// The same game, read by what the players live through: both seats here and silent, the four Discovers at
/// 30 s each and what is left of p1's 75 s turn clock end the turn well inside ten minutes.
#[test]
fn r79_a_silent_seat_not_on_turn_cannot_hold_the_match_past_its_prompt_clocks() {
    let mut table = gatinsir_asks_p2();
    asked_of_p2(&table);
    let turn = table.host.game().turn;

    table.sleep(10 * 60 * SECOND);
    let game = table.host.game();
    assert!(
        game.result.is_some() || game.turn != turn,
        "R79: ten minutes on, turn {} is still p1's and waits on p2's question {:?}; rows {:?}; timers {:?}",
        game.turn,
        game.pending
            .as_ref()
            .map(|pending| (pending.player_id, pending.prompt.clone())),
        table.rows(),
        table.timers
    );
}
