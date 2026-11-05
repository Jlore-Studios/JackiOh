//! One actor per match (SPEC §9.2, §9.3, §9.5): it holds a socket per player and carries out what the
//! match host's machine asks of it, and appends every resolved action to the log. It decides no rule and
//! owns no clock: the decisions are `jackioh_shared::host`'s (#3101) — which clock runs and what an
//! expiry becomes (R79; R268's mulligan clock times out each seat still owing), what a disconnect and a
//! reconnect do (§9.5, R744), what a rematch offer meets (R672), what an aim may show (R738), the order
//! every view is sent in — and the actor feeds the machine its events (a frame, a socket gone, a timer
//! that fired, a row written) and does what it answers with: a tokio timer, a socket send, a store write.
//! The rules are `actor::engine`'s (SURFACE §11.3) under the match's pinned catalog (R388, R767000), and
//! a refusal is relayed verbatim, never re-derived. Every clock expiry is a server action on a client's
//! path, since "(seed, log) reconstructs any match" (§9.3). A socket is only sent `view_for` its player
//! (§9.1, §10.8). R1452, R639000: a ladder bot's seat has no socket; the machine asks for its moves and
//! the actor plays it. Every task runs on one serialized queue, so two `reduce` calls or log appends never
//! interleave; a panicking `reduce` is caught (R1253000). The lock is never held across an `.await` nor
//! while a socket is closed (a close calls back into the actor).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::{mpsc, oneshot};

use jackioh_engine::{
    ActionBody, EmoteId, PLAYER_IDS, PerPlayer, Pilot, PlayerId, PlayerView, PortraitId, Rng,
    deal_emote_hand, emote_gate, emote_gate_within, hand_holds, portrait_or_default,
};
use jackioh_shared::host::bot::BotSeat;
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionAnswer, ActionBy, DecideRequest, DuelEvent, DuelHost, DuelOutput, DuelSetup, GreetingPart,
    HostNote, LetGo, Played, RecordRequest, SeatMessage,
};

use crate::actor::checkpoint::{CheckpointBody, checkpoint_row};
use crate::actor::clock::{Timer, after};
use crate::actor::contracts::{ActorDeps, RecordResultInput, SocketHandlers, VoidMatchInput, one_tx};
use crate::actor::engine::{self, EngineState, LastBoards, MatchSnapshot, Pinned};
use crate::actor::guard::{CLOSE_POLICY_VIOLATED, SeatGuard, alert_unusual_rejections};
use crate::actor::protocol::{
    ACTION_PANICKED_MESSAGE, ClientMessage, MATCH_VOIDED_CLOSE_REASON, SERVER_NONCE_PREFIX, ServerMessage,
    SocketErrorCode, ack_message, aim_relay_message, card_loadouts_message, choice_hover_relay_message,
    clock_message, emote_relay_message, encode, error_message, loadouts_message, parse_client_message,
    portraits_message, prompt_for_opponent, prompt_for_you, view_message,
};
use crate::actor::telemetry::{self, Recorder};
use crate::actor::ws_server::Socket;
use crate::api::rematch::{RematchStakes, offer_for_bot};
use crate::app::now_ms;
use crate::bots::persona::{AiEmote, EmotePersona};
use crate::bots::seat::{self, BotJob, BotMove};
use crate::bots::signals::rematch_plan;
use crate::bug_reports::{Finding, FoundIn, RefusedRow};
use crate::config::{
    AI_EMOTE, BOT_AIM_LEAD_MAX_MS, BOT_AIM_LEAD_MIN_MS, BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT,
    BOT_REMATCH_FIT, BOT_STRENGTHS, BOT_TIMEOUT_FIT, MATCH_CHECKPOINT_MIN_ACTIONS,
    MATCH_RECORD_RESULT_ATTEMPTS, MATCH_RECORD_RESULT_BACKOFF_MS, MATCH_VOIDED_CLOSE_CODE,
};
use crate::db::store::{
    BotRow, MatchActionRow, MatchClocks, MatchRow, MatchSeat, MatchStatus, PlayTelemetry, StoreError,
};
use crate::economy::rewards::GameEconomy;
use crate::loadout::{Loadout, cards_due, shown_defs};
use crate::ranked::ladder::RankTier;

const PLAYERS: [PlayerId; 2] = PLAYER_IDS;

/// R639000: a bot home's rematch answer on its way (the machine's `bot_rematch_replying`), taken off when
/// the task that sends it ends, however it ends.
struct Replying {
    actor: Weak<Shared>,
    home: PlayerId,
}

impl Drop for Replying {
    fn drop(&mut self) {
        if let Some(shared) = self.actor.upgrade() {
            let actor = MatchActor { shared };
            let mut core = actor.lock();
            let outputs = actor.step_in(&mut core, DuelEvent::RematchSent { home: self.home });
            actor.exec_now(&mut core, outputs);
        }
    }
}

/// Input parameters for creating a match actor.
pub struct MatchActorInput {
    pub match_row: MatchRow,
    /// The live state: the opening `Registry::start` began, or the fold `Registry::rebuild` ran off the
    /// runtime (#1035).
    pub state: EngineState,
    /// The rows already in `match_actions`: the seq counter and the nonce map are rebuilt from them.
    pub log: Vec<MatchActionRow>,
    /// R679: called once, synchronously, when a Glitch voids the match, before the store forgets it —
    /// the registry drops the actor here, so nothing can reach a match that no longer exists.
    pub on_voided: Option<Box<dyn Fn() + Send + Sync>>,
    /// R1442: the play telemetry a rebuild replayed from `log` (`telemetry::replay`). `None`, the
    /// actor starts recording from the match's creation.
    pub telemetry: Option<Recorder>,
    /// R1452: the ladder bot at each home seat, which the actor plays itself; `None` for a person.
    pub bots: PerPlayer<Option<BotRow>>,
    /// R639000: by home seat, the turn each bot last moved in, which a rebuild reads off `log`
    /// (`bot_moved_turns`); a new match has none.
    pub bot_moved_turn: PerPlayer<Option<i32>>,
    /// R641000: the match is a game of a Conquest series, which offers no rematch (R672), so a bot
    /// draws no rematch plan when it ends: the series' next pick phase is what comes next.
    pub series_game: bool,
    /// R388, R767000: the catalog the match is played under (`registry`'s `pinned_for`).
    pub pinned: Pinned,
    /// R1834003: every card the decks dealt, by instance id (`engine::dealt_cards` of the opening),
    /// which says whose deck a card a view shows came from; empty when no loadout names a card.
    pub dealt_cards: BTreeMap<String, (PlayerId, String)>,
}

/// R417, R678: the boards a match was created with: `(last_boards, glitch_boards)`.
pub fn last_boards_of(match_row: &MatchRow) -> (Option<LastBoards>, Option<LastBoards>) {
    (match_row.last_boards.clone(), match_row.glitch_boards.clone())
}

/// R1452, R639000: each bot's last move's turn, so an actor rebuilt mid-turn draws no second think
/// time, concession or idle roll for it. Folded under the pinned catalog (R388, R767000).
pub fn bot_moved_turns(
    pinned: Pinned,
    match_row: &MatchRow,
    log: &[MatchActionRow],
    dealt: Option<Vec<PlayerId>>,
) -> PerPlayer<Option<i32>> {
    let (last_boards, glitch_boards) = last_boards_of(match_row);
    let state = engine::fold(
        pinned,
        &engine::fold_args(
            &match_row.seed,
            &match_row.decks,
            Vec::new(),
            last_boards,
            glitch_boards,
            dealt,
        ),
    )
    .state;
    bot_moved_turns_from(pinned, state, log, PerPlayer::new(None, None))
}

/// Resumes bot turns from checkpoint state (#1169).
pub fn bot_moved_turns_from(
    pinned: Pinned,
    mut state: EngineState,
    log: &[MatchActionRow],
    mut moved: PerPlayer<Option<i32>>,
) -> PerPlayer<Option<i32>> {
    let bot_nonce = format!("{SERVER_NONCE_PREFIX}bot-");
    for row in log {
        let result = engine::reduce(pinned, &state, &row.action);
        if result.error.is_some() {
            continue;
        }
        if row.action.nonce.starts_with(&bot_nonce) {
            // R677: the account that began in the other seat plays this one after a swap.
            let player = row.action.player_id;
            let home = if engine::snapshot(&state).seats_swapped {
                player.opponent()
            } else {
                player
            };
            moved[home] = Some(state.turn);
        }
        state = result.state;
    }
    moved
}

/// One task on the actor's serialized queue.
enum Task {
    /// The machine is armed: the clocks are brought up to date and the opening is driven (R79).
    Arm,
    Hello {
        home: PlayerId,
        /// R835001: the socket generation that sent it (`Core::socket_generation`), so a Hello
        /// queued by an older socket is never mistaken for the newer socket's own.
        generation: u64,
    },
    Action {
        home: PlayerId,
        nonce: String,
        body: ActionBody,
    },
    /// MD-D29, R1127: an emote that passed the rate gate, to mint as an `Emote` action once it runs
    /// — the seat resolved then, the legality read off the live state.
    Emote {
        home: PlayerId,
        emote: EmoteId,
    },
    Submit {
        player: PlayerId,
        nonce: String,
        body: ActionBody,
        reply: oneshot::Sender<ServerMessage>,
    },
    /// A clock's timer came due (R79): the machine says which clock ran out.
    Timer {
        key: TimerKey,
        id: u32,
    },
    Disconnect {
        home: PlayerId,
    },
    Attach {
        home: PlayerId,
    },
    /// Resolves once every task queued before it has run.
    Idle(oneshot::Sender<()>),
    /// #1169: the restart drain's checkpoint, answered once it is written or passed over.
    Checkpoint(oneshot::Sender<()>),
    /// R1452: the move a bot home's `seat::play` came back with, decided on the state as it stood
    /// when the log's next seq was `version`, in turn `turn`.
    Decided {
        home: PlayerId,
        version: i64,
        turn: i32,
        played: BotMove,
    },
}

impl Task {
    /// Label for `match.task.threw` telemetry.
    fn what(&self) -> String {
        match self {
            Task::Arm => "arm".to_string(),
            Task::Hello { .. } => "hello".to_string(),
            Task::Action { body, .. } | Task::Submit { body, .. } => format!("action:{}", body.action_type()),
            Task::Emote { .. } => "emote".to_string(),
            Task::Timer { key, .. } => format!("timer:{}", timer_name(*key)),
            Task::Disconnect { .. } => "disconnect".to_string(),
            Task::Attach { .. } => "attach".to_string(),
            Task::Idle(_) => "idle".to_string(),
            Task::Checkpoint(_) => "checkpoint".to_string(),
            Task::Decided { .. } => "bot-move".to_string(),
        }
    }
}

/// The word a timer is named by in a log label.
fn timer_name(key: TimerKey) -> &'static str {
    match key {
        TimerKey::Turn => "turn",
        TimerKey::Prompt => "prompt",
        TimerKey::Mulligan => "mulligan",
        TimerKey::Sequence => "sequence",
        TimerKey::Ceiling => "ceiling",
        TimerKey::Grace { .. } => "grace",
        TimerKey::Aim { .. } => "aim",
        TimerKey::Hover { .. } => "hover",
        TimerKey::BotOffer { .. } => "bot-offer",
        TimerKey::BotReply { .. } => "bot-reply",
        TimerKey::BotRetry => "bot-retry",
        _ => "duos",
    }
}

/// Whether a timer's firing is a task on the queue (the clocks', which act on the match) or answered at
/// once (the relays' and the bots', which never touch the log).
fn queued_timer(key: TimerKey) -> bool {
    matches!(
        key,
        TimerKey::Turn
            | TimerKey::Prompt
            | TimerKey::Mulligan
            | TimerKey::Sequence
            | TimerKey::Ceiling
            | TimerKey::Grace { .. }
    )
}

/// The actor's in-memory state (§9.2): the machine's, and what is the actor's own — its sockets and the
/// abuse guards on them, the emote relay, the bots' personas, the telemetry and the watchers.
struct Core {
    /// The match host's machine (#3101): the game, the clocks, the log's bookkeeping, who is here.
    host: DuelHost,
    /// The timers the machine asked for, by key, each with the id it was asked under.
    timers: IndexMap<TimerKey, (u32, Timer)>,
    /// Keyed by the seat each account BEGAN in (`home`).
    sockets: PerPlayer<Option<Socket>>,
    /// R137, R835000, R835002, R2686000: abuse guard per home seat (flood window, refusals, malformed
    /// frames, last answered hello).
    guards: PerPlayer<SeatGuard>,
    /// R835001: socket generation for queued unrun Hello per seat.
    hello_pending: PerPlayer<Option<u64>>,
    /// R835001: bumped on every `attach` (the first included, so generations start at 1), keying
    /// `hello_pending` so a reattach's Hello is never swallowed by the older socket's queued one.
    socket_generation: PerPlayer<u64>,
    /// R643: recent emote sends for the shared `emote_gate`, which the client runs too.
    emote_history: PerPlayer<Vec<i64>>,
    /// R1351000: mutes emotes if either blocks the other; never cleared until the next match.
    emotes_blocked: bool,
    /// R1483000: daily Jlore cap status per seat.
    jlore_capped: PerPlayer<bool>,
    /// #1657: what the result's write paid each seat; `None` when nothing was, so nothing is told.
    economy: PerPlayer<Option<GameEconomy>>,
    /// R1442: the match's play telemetry, written once its result has landed.
    telemetry: Recorder,
    /// R1442: each player's ladder tier as the actor armed (seat order, `MatchRow.players`), the
    /// telemetry's rank bucket; `None` until then, or when it could not be read.
    tiers: Option<PerPlayer<RankTier>>,
    /// R645, R1344: each bot home's persona, reading the bot seat's own view.
    personas: PerPlayer<Option<EmotePersona>>,
    /// The bot seat's view as its persona last read it (`on_events`' `prev`).
    bot_views: PerPlayer<Option<PlayerView>>,
    /// R645: the long-turn wait each bot home's persona keeps on the person's turn.
    bot_long_turn: PerPlayer<Option<Timer>>,
    /// Social (#939): B50's watchers and the delayed views they are sent (`Spectate`).
    spectate: Spectate,
    /// R1834003: card picks sent to each socket; every hello and attach empties it.
    card_picks_sent: PerPlayer<BTreeSet<(PlayerId, String)>>,
}

struct Shared {
    deps: ActorDeps,
    match_row: MatchRow,
    /// The seats as the match began: `.0` is the account that started in p1.
    seats: (MatchSeat, MatchSeat),
    /// R642: the pair the `portraits` frame carries, `vanilla` for a match that predates them.
    portraits: (PortraitId, PortraitId),
    /// R1834002: both seats' loadouts, by engine seat as the row froze them, every default for a
    /// match that predates them.
    loadouts: (Loadout, Loadout),
    /// R1834003: `MatchActorInput::dealt_cards`.
    dealt_cards: BTreeMap<String, (PlayerId, String)>,
    /// R1341, R1342: each account's emote hand, keyed by the seat it began in (`home`) like the
    /// sockets, dealt once from the match seed when the actor is built — the same eight after a
    /// restart, a reconnect or a Glitch's swap (R677), since a hand belongs to the player.
    emote_hands: PerPlayer<Vec<EmoteId>>,
    inbox: mpsc::UnboundedSender<Task>,
    on_voided: Option<Box<dyn Fn() + Send + Sync>>,
    /// R1452: the ladder bot at each home seat, played by this actor; `None` for a person.
    bots: PerPlayer<Option<BotRow>>,
    /// R388, R767000: the catalog the match is played under.
    pinned: Pinned,
    core: Mutex<Core>,
}

/// Cheap handle (`Clone`) on one match's actor.
#[derive(Clone)]
pub struct MatchActor {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for MatchActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchActor")
            .field("match_id", &self.shared.match_row.id)
            .finish()
    }
}

/// R677: the seat the account that began in `home` plays now — and, read the other way, the account
/// (by its home) that plays engine seat `seat`; a swap is its own inverse.
fn playing(core: &Core, home: PlayerId) -> PlayerId {
    core.host.playing(home)
}

/// R645: the persona of the bot that began in `home`, playing engine seat `seat`: its own rng stream
/// (`{seed}:bot-persona:{home}`, apart from the match rng and the search's) and its dealt hand
/// (R1344), waiting out its emotes inside the ladder bots' refit windows (R639000). `None` for a
/// person's home.
fn persona_for(
    seed: &str,
    home: PlayerId,
    seat: PlayerId,
    bot: Option<&BotRow>,
    hand: &[EmoteId],
) -> Option<EmotePersona> {
    let bot = bot?;
    let mut rng = Rng::new(&format!("{seed}:bot-persona:{home}"), 0);
    Some(
        EmotePersona::new(
            bot.persona,
            seat,
            Box::new(move || rng.next()),
            Some(hand.to_vec()),
        )
        .with_delays(BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT),
    )
}

/// What the machine is told of the ladder bot at `home` (R639000): its fits, and its plan for the end of
/// a ranked match, drawn on the match's seed so every call agrees.
fn bot_seat_for(seed: &str, home: PlayerId, bot: Option<&BotRow>) -> Option<BotSeat> {
    bot.map(|_| BotSeat {
        idle_chance_per_turn: BOT_TIMEOUT_FIT.chance_per_turn,
        aim_lead_ms: (BOT_AIM_LEAD_MIN_MS, BOT_AIM_LEAD_MAX_MS),
        rematch: rematch_plan(seed, home, &BOT_REMATCH_FIT),
    })
}

/// R1253000: the answer a panicked action gets, first and on every resend of its nonce.
fn panicked_answer(nonce: &str) -> ServerMessage {
    error_message(SocketErrorCode::Internal, ACTION_PANICKED_MESSAGE, Some(nonce))
}

/// The frame the machine's answer to an action is sent as (§9.3).
fn answer_message(nonce: &str, answer: ActionAnswer) -> ServerMessage {
    match answer {
        ActionAnswer::Ack { seq } => ack_message(nonce, seq),
        ActionAnswer::Illegal { reason } => {
            error_message(SocketErrorCode::IllegalAction, &reason, Some(nonce))
        }
        ActionAnswer::MatchOver => error_message(
            SocketErrorCode::MatchOver,
            "this match already has a result",
            Some(nonce),
        ),
        ActionAnswer::Panicked => panicked_answer(nonce),
        ActionAnswer::NotRecorded => error_message(
            SocketErrorCode::Internal,
            "the action could not be recorded; try again",
            Some(nonce),
        ),
        // Only a co-op host answers so; a 1v1 nonce is the seat's, whoever sends it.
        ActionAnswer::NonceTaken => error_message(
            SocketErrorCode::Malformed,
            "that nonce is already taken",
            Some(nonce),
        ),
    }
}

fn panic_message(error: tokio::task::JoinError) -> String {
    if !error.is_panic() {
        return error.to_string();
    }
    jackioh_ai::panic_message(&*error.into_panic())
}

/// The serialized queue: one task at a time, so nothing interleaves.
async fn run_queue(shared: Weak<Shared>, mut inbox: mpsc::UnboundedReceiver<Task>) {
    while let Some(task) = inbox.recv().await {
        let Some(strong) = shared.upgrade() else { break };
        let actor = MatchActor { shared: strong };
        let match_id = actor.shared.match_row.id.clone();
        let what = task.what();
        if let Err(error) = tokio::spawn(async move { actor.run(task).await }).await {
            tracing::error!(event = "match.task.threw", matchId = %match_id, what = %what, message = %panic_message(error));
        }
    }
}

/// Spawns the actor's queue and arms the clock; must be called inside a tokio runtime.
pub fn create_match_actor(deps: ActorDeps, input: MatchActorInput) -> MatchActor {
    let MatchActorInput {
        match_row,
        state,
        log,
        on_voided,
        telemetry,
        bots,
        bot_moved_turn,
        series_game,
        pinned,
        dealt_cards,
    } = input;
    let seats = (
        MatchSeat {
            profile_id: match_row.players.0.clone(),
            player: PlayerId::P1,
            deck: match_row.decks.0.clone(),
            portrait: None,
            deck_slots: None,
        },
        MatchSeat {
            profile_id: match_row.players.1.clone(),
            player: PlayerId::P2,
            deck: match_row.decks.1.clone(),
            portrait: None,
            deck_slots: None,
        },
    );

    let next_seq = log.iter().fold(0, |highest, row| highest.max(row.seq)) + 1;
    let acked: Vec<(String, i64)> = log
        .iter()
        .map(|row| (row.action.nonce.clone(), row.seq))
        .collect();

    let portraits = match_row
        .portraits
        .unwrap_or((portrait_or_default(None), portrait_or_default(None)));
    let emote_hands = PerPlayer::new(
        deal_emote_hand(&match_row.seed, PlayerId::P1),
        deal_emote_hand(&match_row.seed, PlayerId::P2),
    );
    let loadouts = match_row
        .loadouts
        .clone()
        .unwrap_or_else(|| (Loadout::defaults(), Loadout::defaults()));
    let opening = engine::snapshot(&state);
    let telemetry =
        telemetry.unwrap_or_else(|| Recorder::new(pinned, &match_row.id, match_row.created_at, &opening));
    // R677: a rebuilt actor's bot may already play the other seat.
    let seat_of_home = |home: PlayerId| {
        if opening.seats_swapped {
            home.opponent()
        } else {
            home
        }
    };
    let personas = PerPlayer::new(
        persona_for(
            &match_row.seed,
            PlayerId::P1,
            seat_of_home(PlayerId::P1),
            bots.p1.as_ref(),
            &emote_hands.p1,
        ),
        persona_for(
            &match_row.seed,
            PlayerId::P2,
            seat_of_home(PlayerId::P2),
            bots.p2.as_ref(),
            &emote_hands.p2,
        ),
    );
    let bot_view = |home: PlayerId, bot: &Option<BotRow>| {
        bot.as_ref()
            .map(|_| engine::view_for(pinned, &state, seat_of_home(home)))
    };
    let bot_views = PerPlayer::new(bot_view(PlayerId::P1, &bots.p1), bot_view(PlayerId::P2, &bots.p2));

    let setup = DuelSetup {
        match_id: match_row.id.clone(),
        seed: match_row.seed.clone(),
        created_at: match_row.created_at,
        game: state,
        ranked: match_row.ranked == Some(true),
        series_game,
        finished: matches!(match_row.status, MatchStatus::Finished),
        next_seq,
        acked,
        clocks: match_row.clocks.clone(),
        bots: PerPlayer::new(
            bot_seat_for(&match_row.seed, PlayerId::P1, bots.p1.as_ref()),
            bot_seat_for(&match_row.seed, PlayerId::P2, bots.p2.as_ref()),
        ),
        bot_moved_turn,
    };
    // R79: the turn clock runs when the match opens, and the ceiling from its start; both armed
    // synchronously so callers see them armed, while the queued task persists the clocks and resolves a
    // finished match.
    let (host, opening_outputs) = engine::run_pinned(pinned, || DuelHost::new(setup, now_ms()));

    let core = Core {
        host,
        timers: IndexMap::new(),
        sockets: PerPlayer::new(None, None),
        guards: PerPlayer::new(SeatGuard::default(), SeatGuard::default()),
        hello_pending: PerPlayer::new(None, None),
        socket_generation: PerPlayer::new(0, 0),
        emote_history: PerPlayer::new(Vec::new(), Vec::new()),
        emotes_blocked: false,
        jlore_capped: PerPlayer::new(false, false),
        economy: PerPlayer::new(None, None),
        telemetry,
        tiers: None,
        personas,
        bot_views,
        bot_long_turn: PerPlayer::new(None, None),
        spectate: Spectate::new(),
        card_picks_sent: PerPlayer::new(BTreeSet::new(), BTreeSet::new()),
    };

    let (inbox, inbox_rx) = mpsc::unbounded_channel();

    let shared = Arc::new(Shared {
        deps,
        match_row,
        seats,
        portraits,
        loadouts,
        dealt_cards,
        emote_hands,
        inbox,
        on_voided,
        bots,
        pinned,
        core: Mutex::new(core),
    });
    let actor = MatchActor { shared };
    {
        let mut core = actor.lock();
        actor.exec_now(&mut core, opening_outputs);
    }
    // Social (#939): B50's record starts with the state the actor opens on.
    actor.spectate_record();
    actor.enqueue(Task::Arm);
    tokio::spawn(run_queue(Arc::downgrade(&actor.shared), inbox_rx));
    actor
}

impl MatchActor {
    fn lock(&self) -> MutexGuard<'_, Core> {
        self.shared.core.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn match_id_str(&self) -> &str {
        &self.shared.match_row.id
    }

    /// Enqueues task; panicking tasks are logged by `run_queue`.
    fn enqueue(&self, task: Task) {
        if self.shared.inbox.send(task).is_err() {
            tracing::warn!(event = "match.queue.closed", matchId = %self.match_id_str());
        }
    }

    async fn run(&self, task: Task) {
        match task {
            Task::Arm => self.on_arm().await,
            Task::Hello { home, generation } => {
                // §9.5: "Reconnect gets a fresh full view, never a log replay." R835001: the flag is
                // cleared only for the generation that set it, so a newer socket's queued Hello is
                // never unmarked by the older one's run.
                {
                    let mut core = self.lock();
                    if core.hello_pending[home] == Some(generation) {
                        core.hello_pending[home] = None;
                    }
                }
                self.drive(DuelEvent::Hello { home }).await;
            }
            Task::Action { home, nonce, body } => {
                // R677: the seat is the one this account plays when the action runs — a swap queued
                // ahead of it has already moved the account — and the reply goes to the account that
                // sent it.
                let reply = self.answer_action(ActionBy::Home { home }, nonce, body).await;
                let core = self.lock();
                self.send_to_home(&core, home, &reply);
            }
            Task::Submit {
                player,
                nonce,
                body,
                reply,
            } => {
                let answer = self
                    .answer_action(ActionBy::Seat { seat: player }, nonce, body)
                    .await;
                let _ = reply.send(answer);
            }
            Task::Emote { home, emote } => {
                // MD-D29, R1127: the machine mints an `Emote` action only if a card hears it; otherwise
                // the relay suffices (R643). The sender showed it already, so the reply is dropped.
                self.drive(DuelEvent::Emote { home, emote }).await;
            }
            Task::Timer { key, id } => {
                {
                    // The timer that reported is spent; a newer one under the same key is not.
                    let mut core = self.lock();
                    if core.timers.get(&key).is_some_and(|(held, _)| *held == id) {
                        core.timers.shift_remove(&key);
                    }
                }
                self.drive(DuelEvent::TimerFired { key, id }).await;
            }
            Task::Disconnect { home } => {
                self.drive(DuelEvent::Disconnected { home }).await;
            }
            Task::Attach { home } => {
                self.drive(DuelEvent::Attached { home }).await;
            }
            Task::Idle(done) => {
                let _ = done.send(());
            }
            Task::Checkpoint(done) => {
                self.write_checkpoint().await;
                let _ = done.send(());
            }
            Task::Decided {
                home,
                version,
                turn,
                played,
            } => {
                let played = Played {
                    aimed: played.aim.is_some(),
                    body: played.body,
                    ai_cursor: played.ai_cursor,
                    timing_cursor: played.timing_cursor,
                };
                self.drive(DuelEvent::Decided {
                    home,
                    version,
                    turn,
                    played,
                })
                .await;
            }
        }
    }

    /// Arms the machine (R79): the opening's reads the actor makes first, then the machine brings the
    /// clocks up to date and drives the opening or ends a game that is over.
    async fn on_arm(&self) {
        let (snapshot, was_finished) = {
            let core = self.lock();
            (engine::snapshot(core.host.game()), core.host.is_finished())
        };
        // R1351000: a block either way between the two accounts mutes their emotes; one made after
        // this read reaches the actor through `mute_emotes_between`, and `|=` never undoes it.
        let (first, second) = &self.shared.match_row.players;
        match one_tx!(self.shared.deps.db, |t| t.blocks_between(first, second).await?) {
            Ok(blocked) => self.lock().emotes_blocked |= blocked,
            Err(error) => {
                tracing::warn!(event = "match.emote.blocks_failed", matchId = %self.match_id_str(), message = %error);
            }
        }
        // R1442: the rank bucket is the tier each player holds as the match arms.
        if !was_finished {
            let tiers = telemetry::ladder_tiers_or_none(
                &self.shared.deps,
                &self.shared.match_row.id,
                &self.shared.match_row.players,
            )
            .await;
            self.lock().tiers = tiers;
        }
        if snapshot.result.is_none() {
            // R1452: a bot that owes the match a move, as it opens or after a restart, makes it.
            let mut core = self.lock();
            for home in PLAYERS {
                if let Some(view) = core.bot_views[home].clone() {
                    self.rearm_long_turn(&mut core, home, &view);
                }
            }
        }
        self.drive(DuelEvent::Arm).await;
        // R1483000: an actor rebuilt on a match whose result is already in learns what the cap held back
        // from the ledger; one that is about to write it does so as the result lands.
        if snapshot.result.is_some() && was_finished {
            self.load_jlore_caps(false).await;
        }
    }

    // The machine: events in, outputs carried out

    /// Steps the machine under the match's pinned catalog (R388, R767000).
    fn step_in(&self, core: &mut Core, event: DuelEvent) -> Vec<DuelOutput> {
        let now = now_ms();
        engine::run_pinned(self.shared.pinned, || core.host.step(event, now))
    }

    /// Feeds the machine one event and carries out what it answers with, the awaits included, in order.
    /// The answers it gave to actions, by nonce.
    async fn drive(&self, event: DuelEvent) -> Vec<(String, ActionAnswer)> {
        let outputs = {
            let mut core = self.lock();
            self.step_in(&mut core, event)
        };
        let mut answers = Vec::new();
        self.exec(outputs, &mut answers).await;
        answers
    }

    /// One action for a seat, through the machine (§9.3): the frame its sender is answered with.
    async fn answer_action(&self, by: ActionBy, nonce: String, body: ActionBody) -> ServerMessage {
        let answers = self
            .drive(DuelEvent::Action {
                by,
                nonce: nonce.clone(),
                body,
            })
            .await;
        match answers.into_iter().find(|(answered, _)| *answered == nonce) {
            Some((nonce, answer)) => answer_message(&nonce, answer),
            None => answer_message(&nonce, ActionAnswer::NotRecorded),
        }
    }

    /// Carries out `outputs` in order: what needs no await under the lock as it comes, the rest apart.
    async fn exec(&self, outputs: Vec<DuelOutput>, answers: &mut Vec<(String, ActionAnswer)>) {
        for output in outputs {
            let deferred = {
                let mut core = self.lock();
                self.run_sync(&mut core, output, answers)
            };
            if let Some(output) = deferred {
                Box::pin(self.run_async(output, answers)).await;
            }
        }
    }

    /// Carries out `outputs` that need no await (those of a frame, a relay's timer, a socket's arrival),
    /// under the lock the caller holds.
    fn exec_now(&self, core: &mut Core, outputs: Vec<DuelOutput>) {
        let mut ignored = Vec::new();
        for output in outputs {
            if let Some(output) = self.run_sync(core, output, &mut ignored) {
                tracing::error!(event = "match.output.deferred", matchId = %self.match_id_str(), output = ?output);
            }
        }
    }

    /// One output that needs no await; `Some(output)` back when it does.
    fn run_sync(
        &self,
        core: &mut Core,
        output: DuelOutput,
        answers: &mut Vec<(String, ActionAnswer)>,
    ) -> Option<DuelOutput> {
        match output {
            DuelOutput::Send { home, message } => self.send_seat(core, home, message),
            DuelOutput::Timer { command } => self.apply_timer(core, command),
            DuelOutput::Answer { nonce, answer } => answers.push((nonce, answer)),
            DuelOutput::Greeting { home, part } => match part {
                // R1834003: the seats' loadouts ride before the view, and the cards' picks after it
                // anew: the page may have lost what it was sent.
                GreetingPart::Before => {
                    core.card_picks_sent[home].clear();
                    self.send_to_home(core, home, &loadouts_message(&self.shared.loadouts));
                }
                // R642: portraits ride again on a reconnect, as on join, with the account's hand (R1342).
                GreetingPart::After => self.send_to_home(core, home, &self.portraits_for(home)),
                // Social (#939): B51's count, which a socket that was away missed.
                GreetingPart::Watchers => self.push_watchers_to(core, home),
            },
            DuelOutput::Refused { home } => {
                // R835000: reducer refusals count toward the unusual-rejection alert; dedupes and store
                // failures never count.
                if let Some((profile_id, rejected)) = self.note_rejection(core, home, now_ms()) {
                    alert_unusual_rejections(self.match_id_str(), &profile_id, rejected);
                }
            }
            DuelOutput::Panicked { action, seq, message } => {
                self.on_reduce_panicked(&action, seq, &message);
            }
            DuelOutput::Applied {
                before,
                action,
                seq,
                at,
                clock_left_ms,
                last_event,
            } => {
                // R1442: the move as it arrived, on the state it was made in.
                if let Some(before) = before {
                    core.telemetry
                        .on_action(&before, &action, seq, at, clock_left_ms, last_event);
                }
            }
            DuelOutput::SeatsSwapped { swapped } => {
                // R645: a bot's persona reads the seat its bot plays now, from a fresh start: the view it
                // read last is the other seat's, and no difference between the two is anything that
                // happened.
                for home in PLAYERS {
                    let seat = playing(core, home);
                    if let Some(persona) = core.personas[home].as_mut() {
                        persona.set_seat(seat);
                        core.bot_views[home] = None;
                    }
                }
                tracing::info!(event = "match.seats.swapped", matchId = %self.match_id_str(), swapped = swapped);
            }
            DuelOutput::Changed { fresh } => self.feed_personas(core, fresh),
            DuelOutput::Pushed { snapshot } => {
                // R1442: a seat's think time runs from this push.
                core.telemetry.on_pushed(&snapshot, now_ms());
                // Social (#939): B50, the watchers' record of the change, sent SPECTATE_DELAY_MS on.
                self.spectate_record_in(core);
            }
            DuelOutput::Decide { request } => self.spawn_bot(core, request),
            DuelOutput::OfferRematch { home, stakes, reply } => {
                self.spawn_rematch_offer(home, stakes, reply);
            }
            DuelOutput::Note { note } => self.log_note(note),
            other => return Some(other),
        }
        None
    }

    /// One output that awaits: a write to the store, or the end of the match.
    async fn run_async(&self, output: DuelOutput, answers: &mut Vec<(String, ActionAnswer)>) {
        match output {
            DuelOutput::PersistClocks { clocks } => self.persist_clocks(clocks).await,
            DuelOutput::Append { seq, action, at } => {
                // §9.3: row before state, so a failed write never leaves the state ahead of the log.
                let row = MatchActionRow {
                    match_id: self.shared.match_row.id.clone(),
                    seq,
                    action,
                    at,
                };
                let appended = one_tx!(self.shared.deps.db, |t| t
                    .matches_append_actions(std::slice::from_ref(&row))
                    .await?);
                let outcome = match appended {
                    Ok(()) => jackioh_shared::host::LogOutcome::Written,
                    Err(error) => {
                        tracing::error!(event = "match.append.failed", matchId = %self.match_id_str(), seq = seq, message = %error);
                        if matches!(error, StoreError::SeqClash(_)) {
                            jackioh_shared::host::LogOutcome::Diverged
                        } else {
                            jackioh_shared::host::LogOutcome::Failed
                        }
                    }
                };
                let outputs = {
                    let mut core = self.lock();
                    self.step_in(&mut core, DuelEvent::Logged { outcome })
                };
                self.exec(outputs, answers).await;
            }
            DuelOutput::Record { request } => self.on_record(request, answers).await,
            DuelOutput::Voided { at } => self.on_void(at).await,
            DuelOutput::CeilingDraw { at } => self.end_as_ceiling_draw(at).await,
            DuelOutput::LetGo { reason } => match reason {
                LetGo::Diverged => self.let_go_diverged(),
            },
            other => {
                tracing::error!(event = "match.output.unhandled", matchId = %self.match_id_str(), output = ?other);
            }
        }
    }

    fn log_note(&self, note: HostNote) {
        match note {
            HostNote::ClockExpired { kind, player } => {
                tracing::info!(event = "match.clock.expired", matchId = %self.match_id_str(), kind = %kind, player = %player);
            }
            HostNote::ClockRetrying { kind, player, in_ms } => {
                tracing::warn!(event = "match.clock.retrying", matchId = %self.match_id_str(), kind = %kind, player = %player, inMs = in_ms);
            }
            HostNote::ActionRejected {
                player,
                action_type,
                reason,
            } => {
                // §9.8: "Every rejected action is logged with its reason."
                tracing::warn!(
                    event = "match.action.rejected",
                    matchId = %self.match_id_str(),
                    player = %player,
                    "type" = %action_type,
                    reason = %reason,
                );
            }
            HostNote::BotMoveRefused { player, answer } => {
                if let ServerMessage::Error { code, message, .. } = answer_message("", answer) {
                    tracing::warn!(event = "bot.move.refused", matchId = %self.match_id_str(), player = %player, code = ?code, message = %message);
                }
            }
        }
    }

    /// Carries out a timer the machine asked for: a tokio wait that comes back as the machine's own
    /// `TimerFired` (the clocks' on the queue, the relays' at once).
    fn apply_timer(&self, core: &mut Core, command: TimerCommand) {
        match command {
            TimerCommand::Set { key, id, at } => {
                let actor = Arc::downgrade(&self.shared);
                let timer = after(at - now_ms(), move |_| {
                    let Some(shared) = actor.upgrade() else { return };
                    let actor = MatchActor { shared };
                    if queued_timer(key) {
                        actor.enqueue(Task::Timer { key, id });
                    } else {
                        actor.on_relay_timer(key, id);
                    }
                });
                core.timers.insert(key, (id, timer));
            }
            TimerCommand::Cancel { key, id } => {
                if core.timers.get(&key).is_some_and(|(held, _)| *held == id) {
                    core.timers.shift_remove(&key);
                }
            }
        }
    }

    /// A relay's or a bot's timer came due: answered at once, off the queue, as the relays always were.
    fn on_relay_timer(&self, key: TimerKey, id: u32) {
        let mut core = self.lock();
        if core.timers.get(&key).is_some_and(|(held, _)| *held == id)
            && let Some((_, timer)) = core.timers.shift_remove(&key)
        {
            timer.fired();
        }
        let outputs = self.step_in(&mut core, DuelEvent::TimerFired { key, id });
        self.exec_now(&mut core, outputs);
    }

    // Sending. §10.8: a socket only ever carries this player's own view.

    /// R642, R1342: the portraits frame for the account that began in `home`, with its own hand.
    fn portraits_for(&self, home: PlayerId) -> ServerMessage {
        portraits_message(
            self.shared.portraits.0,
            self.shared.portraits.1,
            &self.shared.emote_hands[home],
        )
    }

    /// Sends to the account playing engine seat `player` now.
    fn send(&self, core: &Core, player: PlayerId, message: &ServerMessage) {
        self.send_to_home(core, playing(core, player), message);
    }

    /// Sends to the account that began the match in `home`, whatever seat it plays.
    fn send_to_home(&self, core: &Core, home: PlayerId, message: &ServerMessage) {
        let Some(socket) = &core.sockets[home] else { return };
        if !socket.is_open() {
            return;
        }
        socket.send(encode(message));
    }

    /// §10.8: the seat's view, its clock filled in (R79), read under the match's catalog (R388, R767000).
    fn view_of(&self, core: &Core, player: PlayerId) -> PlayerView {
        engine::run_pinned(self.shared.pinned, || core.host.view_for(player, now_ms()))
    }

    /// A message of the machine's, framed for the account's socket. §10.8: the view and its legal
    /// actions are both for the seat — the opponent's would name its hand (§9.1). R1834003: then new card
    /// picks (`cards_due`).
    fn send_seat(&self, core: &mut Core, home: PlayerId, message: SeatMessage) {
        match message {
            SeatMessage::View { view, legal } => {
                let view = *view;
                let shown = shown_defs(&view, &self.shared.dealt_cards);
                let frame = view_message(view, &legal, core.jlore_capped[home], core.economy[home].clone());
                self.send_to_home(core, home, &frame);
                if !core.sockets[home].as_ref().is_some_and(Socket::is_open) {
                    return;
                }
                for cards in cards_due(&shown, &self.shared.loadouts, &mut core.card_picks_sent[home]) {
                    self.send_to_home(core, home, &card_loadouts_message(cards));
                }
            }
            SeatMessage::Clock { now, clocks } => {
                self.send_to_home(core, home, &clock_message(now, clocks));
            }
            SeatMessage::PromptForYou {
                player,
                choice_id,
                kind,
                deadline,
            } => {
                self.send_to_home(core, home, &prompt_for_you(player, &choice_id, kind, deadline));
            }
            SeatMessage::PromptForOpponent {
                pending_for,
                deadline,
            } => {
                self.send_to_home(core, home, &prompt_for_opponent(pending_for, deadline));
            }
            SeatMessage::Aim { from, aim } => {
                self.send_to_home(core, home, &aim_relay_message(from, aim));
            }
            SeatMessage::ChoiceHover { from, index } => {
                self.send_to_home(core, home, &choice_hover_relay_message(from, index));
            }
        }
    }

    /// The seat's view and legal actions, sent anew to the account that plays it.
    fn push_view(&self, core: &mut Core, player: PlayerId) {
        let home = playing(core, player);
        let view = Box::new(self.view_of(core, player));
        let legal = engine::run_pinned(self.shared.pinned, || core.host.legal(player));
        self.send_seat(core, home, SeatMessage::View { view, legal });
    }

    // The clock's stored copy (§9.5)

    async fn persist_clocks(&self, clocks: MatchClocks) {
        // §9.5: "the grace countdown is stored on the match so both clients show it".
        let written = one_tx!(self.shared.deps.db, |t| t
            .matches_set_clocks(self.match_id_str(), &clocks)
            .await?);
        if let Err(error) = written {
            tracing::warn!(event = "match.clocks.persistFailed", matchId = %self.match_id_str(), message = %error);
        }
    }

    // The end of the match

    /// R679: match voided with no result, ratings, or records. Closes sockets with
    /// `MATCH_VOIDED_CLOSE_CODE`, drops the actor, and `void_match` deletes it from the store.
    async fn on_void(&self, at: i64) {
        let sockets = {
            let mut core = self.lock();
            PLAYERS.map(|home| core.sockets[home].take())
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(MATCH_VOIDED_CLOSE_CODE), Some(MATCH_VOIDED_CLOSE_REASON));
            }
        }
        // Social (#939): the watchers go with the match.
        self.spectate_close(MATCH_VOIDED_CLOSE_CODE, MATCH_VOIDED_CLOSE_REASON);
        if let Some(on_voided) = &self.shared.on_voided {
            on_voided();
        }
        let input = VoidMatchInput {
            match_id: self.shared.match_row.id.clone(),
            players: self.shared.match_row.players.clone(),
            at,
        };
        if let Err(error) = crate::api::results::void_match(&self.shared.deps, input).await {
            // The row stays live, so the next socket for it rebuilds this actor, folds to the same void
            // and tries again.
            tracing::error!(event = "match.void.failed", matchId = %self.match_id_str(), message = %error);
        }
    }

    /// R677: the seats as the results writer credits them — engine seat p1 with the account that plays
    /// it NOW, and its deck — so the winning seat's current account gets the win (Elo included), and
    /// the last board each seat ended with goes to the account that ended it.
    fn credited_seats(&self, swapped: bool) -> (MatchSeat, MatchSeat) {
        let (first, second) = &self.shared.match_row.players;
        let (first_deck, second_deck) = &self.shared.match_row.decks;
        (
            MatchSeat {
                profile_id: if swapped { second } else { first }.clone(),
                player: PlayerId::P1,
                deck: first_deck.clone(),
                portrait: None,
                deck_slots: None,
            },
            MatchSeat {
                profile_id: if swapped { first } else { second }.clone(),
                player: PlayerId::P2,
                deck: second_deck.clone(),
                portrait: None,
                deck_slots: None,
            },
        )
    }

    /// §9.5: "Every ending records a result and clears both players' in-match state." Once.
    async fn on_record(&self, request: RecordRequest, answers: &mut Vec<(String, ActionAnswer)>) {
        let RecordRequest {
            outcome,
            turns,
            at,
            seats_swapped,
            last_boards,
            final_hash,
        } = request;
        // The results row, the rating move and clearing `inMatchId` are `api/results.rs` (M7-T2); the
        // actor never writes them itself.
        let input = RecordResultInput {
            match_id: self.shared.match_row.id.clone(),
            // R677: credited by who plays each seat now, not by who began in it.
            seats: self.credited_seats(seats_swapped),
            outcome,
            turns,
            at,
            // R417, R565: each seat's board as the game ended, from its own side.
            last_boards: Some(last_boards),
            // R768: the final state's hash, which the match's replay is held to.
            final_hash: Some(final_hash),
        };
        // The write finishes the match row in the same transaction, so nothing else here does: a row
        // finished apart from it would outlive a failed write, with no result and both players stuck
        // in the match. R1437: a write that never lands leaves the match `live` for the next socket.
        let Some(mut economy) = self.record_result_with_retries(input, true).await else {
            self.let_go_unrecorded();
            return;
        };
        // #1657: the write answers each seat by the engine seat it credited, the core keeps each
        // account's by home; `load_jlore_caps` sends it.
        {
            let mut core = self.lock();
            for seat in PLAYERS {
                let home = playing(&core, seat);
                core.economy[home] = economy[seat].take();
            }
        }
        self.load_jlore_caps(true).await;
        // R1442: the play telemetry goes after the result, so its write can never cost it.
        // R1452: a bot home's rows are the AI's, so no human figure reads them unasked.
        let pilot = |home: PlayerId| {
            if self.shared.bots[home].is_some() {
                Pilot::Ai
            } else {
                Pilot::Human
            }
        };
        let pilots = PerPlayer::new(pilot(PlayerId::P1), pilot(PlayerId::P2));
        let played = {
            let core = self.lock();
            core.telemetry.finish(&pilots, core.tiers.as_ref())
        };
        telemetry::record(&self.shared.deps, &played).await;
        // R639000: the bots' rematch offers, once the result has landed.
        let outputs = {
            let mut core = self.lock();
            self.step_in(&mut core, DuelEvent::ResultRecorded)
        };
        self.exec(outputs, answers).await;

        tracing::info!(
            event = "match.over",
            matchId = %self.match_id_str(),
            winner = %outcome.winner,
            reason = %outcome.reason,
            turns = turns,
        );
    }

    /// R1483000: whether the daily Jlore cap held back pay; with `announce`, capped or paid
    /// (`Core::economy`, #1657) seats get fresh views. A failed read is only logged.
    async fn load_jlore_caps(&self, announce: bool) {
        let caps = match crate::api::results::game_jlore_caps(&self.shared.deps, self.match_id_str()).await {
            Ok(caps) => caps,
            Err(error) => {
                tracing::warn!(event = "match.jloreCaps.failed", matchId = %self.match_id_str(), message = %error);
                PerPlayer::new(false, false)
            }
        };
        let mut core = self.lock();
        if caps.p1 || caps.p2 {
            core.jlore_capped = caps;
        }
        for home in PLAYERS {
            if announce && (core.jlore_capped[home] || core.economy[home].is_some()) {
                let player = playing(&core, home);
                self.push_view(&mut core, player);
            }
        }
    }

    /// R639000: the bot home offers a rematch at `stakes` through the route a person's offer takes
    /// (`api::rematch::offer_for_bot`), off the queue. A reply keeps the bot on the match until it lands.
    fn spawn_rematch_offer(&self, home: PlayerId, stakes: RematchStakes, reply: bool) {
        let deps = Arc::clone(&self.shared.deps);
        let match_id = self.shared.match_row.id.clone();
        let replying = reply.then(|| Replying {
            actor: Arc::downgrade(&self.shared),
            home,
        });
        tokio::spawn(async move {
            // Dropped however the task ends, a panic included, so the bot never stays for good.
            let _replying = replying;
            if let Err(error) = offer_for_bot(&deps, &match_id, home, stakes).await {
                tracing::warn!(event = "bot.rematch.failed", matchId = %match_id, home = %home, message = %error.message);
            }
        });
    }

    /// R639000, R672: the person at `from` offered a rematch at `stakes`: the bot opposite may meet it.
    pub fn on_rematch_offer(&self, from: PlayerId, stakes: RematchStakes) {
        let mut core = self.lock();
        let outputs = self.step_in(&mut core, DuelEvent::RematchOffered { from, stakes });
        self.exec_now(&mut core, outputs);
    }

    /// R1437: retries `record_result` with exponential backoff; idempotent, as a write that finds the
    /// row answers it (`api/results.rs`). Returns payouts (`results::Recorded`, #1657) once landed, or
    /// `None` if exhausted. Ceiling draws pay seats but return no payouts as the actor halts at once.
    async fn record_result_with_retries(
        &self,
        input: RecordResultInput,
        rated: bool,
    ) -> Option<PerPlayer<Option<GameEconomy>>> {
        let mut wait_ms = MATCH_RECORD_RESULT_BACKOFF_MS;
        for attempt in 1..=MATCH_RECORD_RESULT_ATTEMPTS {
            let written = if rated {
                crate::api::results::record_result_with_economy(&self.shared.deps, input.clone())
                    .await
                    .map(|recorded| recorded.economy)
            } else {
                crate::api::results::record_ceiling_draw(&self.shared.deps, input.clone())
                    .await
                    .map(|_| PerPlayer::new(None, None))
            };
            match written {
                Ok(economy) => return Some(economy),
                Err(error) => {
                    tracing::error!(event = "match.recordResult.failed", matchId = %self.match_id_str(), attempt = attempt, message = %error);
                }
            }
            if attempt < MATCH_RECORD_RESULT_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(wait_ms as u64)).await;
                wait_ms *= 2;
            }
        }
        None
    }

    /// R1253000, R112: a clock action whose `reduce` panicked ends the match as a ceiling draw with
    /// turns=0 and ratings unchanged, written as R1437 writes any ending. Halts the actor without
    /// waiting on the queue; unrecorded writes invoke `let_go_unrecorded`.
    async fn end_as_ceiling_draw(&self, at: i64) {
        let input = crate::api::results::ceiling_draw_input(&self.shared.match_row, at);
        if self.record_result_with_retries(input, false).await.is_none() {
            self.let_go_unrecorded();
            return;
        }
        tracing::warn!(event = "match.panicked.drawn", matchId = %self.match_id_str());
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
    }

    /// R1437, R679: unrecorded result leaves match live; registry forgets actor (`stop` would wait on
    /// this queue) and closes sockets, so the reconnect's rebuilt actor writes the result.
    fn let_go_unrecorded(&self) {
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = "match.result.unrecorded", matchId = %self.match_id_str());
    }

    /// R776000: the log holds this actor's next seq already, so it lets go as `let_go_unrecorded`
    /// does (the machine says so unless it is stopped: the registry may hold its rebuilt successor).
    fn let_go_diverged(&self) {
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = "match.log.diverged", matchId = %self.match_id_str());
    }

    /// R1253000: pure `reduce` panicked; state and log stand untouched. Files a bug report with seq and
    /// action type (omitting private panic text); the machine remembers the nonce.
    fn on_reduce_panicked(&self, action: &jackioh_engine::Action, seq: i64, message: &str) {
        tracing::error!(
            event = "match.action.panicked",
            matchId = %self.match_id_str(),
            seq = seq,
            player = %action.player_id,
            "type" = %action.action_type(),
            message = %message,
        );
        self.shared.deps.bug_reports.file(
            self.shared.deps.env.deployed_commit.as_deref(),
            Finding {
                match_id: self.match_id_str().to_string(),
                found_in: FoundIn::Live,
                catalog_version: self.shared.match_row.catalog_version.clone(),
                actions: usize::try_from(seq - 1).unwrap_or(0),
                refused: vec![RefusedRow::of(
                    seq,
                    &serde_json::to_value(action).unwrap_or(serde_json::Value::Null),
                )],
            },
        );
    }

    // The bot seat (R1452)

    /// R1452: launches the asynchronous move the machine asked for. The think time is waited off the
    /// queue; clocks can still expire. The move comes back as a `Decided` task.
    fn spawn_bot(&self, core: &mut Core, request: DecideRequest) {
        let DecideRequest {
            home,
            seat,
            version,
            turn,
            first_in_turn,
            own_turn_start,
            aim_lead_ms,
            clock_left_ms,
            ai_cursor,
            timing_cursor,
        } = request;
        let actor = Arc::downgrade(&self.shared);
        let strength = self.shared.bots[home].as_ref().and_then(|bot| {
            BOT_STRENGTHS
                .get(bot.strength)
                .or(BOT_STRENGTHS.last())
                .copied()
                .map(|s| (s, bot.speed))
        });
        let Some((strength, speed)) = strength else {
            // No bot or no strength to play it with: nothing to decide, and the machine is told so.
            self.enqueue(Task::Decided {
                home,
                version,
                turn,
                played: BotMove {
                    body: None,
                    ai_cursor,
                    timing_cursor,
                    aim: None,
                    aim_lead_ms,
                },
            });
            return;
        };
        let seed = &self.shared.match_row.seed;
        let job = BotJob {
            state: core.host.game().clone(),
            seat,
            home,
            seed: seed.clone(),
            strength,
            speed,
            ai_rng: Rng::new(&format!("{seed}:bot-ai:{home}"), ai_cursor),
            timing_rng: Rng::new(&format!("{seed}:bot-timing:{home}"), timing_cursor),
            first_in_turn,
            own_turn_start,
            aim_lead_ms,
            clock_left_ms,
            pinned: self.shared.pinned,
        };
        tokio::spawn(async move {
            let played = seat::play(job).await;
            if let Some(aim) = played.aim {
                // R738: the aim shows a beat before the move, through the relay a person's goes by.
                let Some(shared) = actor.upgrade() else { return };
                let strong = MatchActor { shared };
                {
                    let mut core = strong.lock();
                    let outputs = strong.step_in(&mut core, DuelEvent::Aim { home, aim: Some(aim) });
                    strong.exec_now(&mut core, outputs);
                }
                // Not held across the wait, so the actor can still go meanwhile.
                drop(strong);
                tokio::time::sleep(Duration::from_millis(
                    u64::try_from(played.aim_lead_ms).unwrap_or(0),
                ))
                .await;
            }
            if let Some(shared) = actor.upgrade() {
                MatchActor { shared }.enqueue(Task::Decided {
                    home,
                    version,
                    turn,
                    played,
                });
            }
        });
    }

    /// R645: each bot's persona reads the change off its own seat's view: the newest `fresh` events
    /// that view carries (the change's own, redacted for that seat) against the view it read last.
    /// What it rolls waits out its delay and goes out as the bot's emote (`schedule_emotes`).
    fn feed_personas(&self, core: &mut Core, fresh: usize) {
        let now = now_ms();
        for home in PLAYERS {
            if core.personas[home].is_none() {
                continue;
            }
            let seat = playing(core, home);
            let view = engine::view_for(self.shared.pinned, core.host.game(), seat);
            // R2652000: less the face-down rolls this seat's view leaves out of the change.
            let fresh = engine::viewed_of_newest(core.host.game(), seat, fresh);
            let from = view.events.len().saturating_sub(fresh);
            let prev = core.bot_views[home].take();
            let intents = match core.personas[home].as_mut() {
                Some(persona) => persona.on_events(&view.events[from..], prev.as_ref(), &view, now),
                None => Vec::new(),
            };
            self.rearm_long_turn(core, home, &view);
            core.bot_views[home] = Some(view);
            self.schedule_emotes(home, intents);
        }
    }

    /// R645's long-turn trigger, armed as practice arms it: on every change that leaves the person the
    /// active seat, a fresh `AI_EMOTE.long_turn_ms` wait (the persona's once-a-turn check makes a
    /// second arm harmless); none on the bot's own turn or once the game is over.
    fn rearm_long_turn(&self, core: &mut Core, home: PlayerId, view: &PlayerView) {
        if let Some(timer) = core.bot_long_turn[home].take() {
            timer.cancel();
        }
        if core.host.is_finished()
            || core.host.is_stopped()
            || view.result.is_some()
            || view.active == view.viewer
        {
            return;
        }
        let actor = Arc::downgrade(&self.shared);
        core.bot_long_turn[home] = Some(after(AI_EMOTE.long_turn_ms, move |id| {
            let Some(shared) = actor.upgrade() else { return };
            let actor = MatchActor { shared };
            let intents = {
                let mut core = actor.lock();
                if core.bot_long_turn[home].as_ref().map(Timer::id) != Some(id) {
                    return;
                }
                if let Some(timer) = core.bot_long_turn[home].take() {
                    timer.fired();
                }
                if core.host.is_finished() || core.host.is_stopped() {
                    return;
                }
                let Some(view) = core.bot_views[home].clone() else {
                    return;
                };
                match core.personas[home].as_mut() {
                    Some(persona) => persona.on_player_turn_long(&view, now_ms()),
                    None => Vec::new(),
                }
            };
            actor.schedule_emotes(home, intents);
        }));
    }

    /// R645: each of a bot's intents waits out its own delay, then goes out as the bot's emote.
    fn schedule_emotes(&self, home: PlayerId, intents: Vec<AiEmote>) {
        for intent in intents {
            let actor = Arc::downgrade(&self.shared);
            let wait = Duration::from_millis(intent.delay_ms.max(0.0).round() as u64);
            tokio::spawn(async move {
                tokio::time::sleep(wait).await;
                let Some(shared) = actor.upgrade() else { return };
                let actor = MatchActor { shared };
                let mut core = actor.lock();
                if !core.host.is_stopped() {
                    actor.relay_emote(&mut core, home, intent.emote);
                }
            });
        }
    }

    /// R643, R1342: drops undealt or rate-limited emotes silently. Passes relay to opponent and
    /// enqueue `Emote` if a card listens (R1127). R645: offers person emote to opposing bot persona
    /// (bots never answer bots). R1351000: blocked accounts relay to no one.
    fn relay_emote(&self, core: &mut Core, home: PlayerId, emote: EmoteId) {
        let player = playing(core, home);
        if !hand_holds(&self.shared.emote_hands[home], emote) {
            tracing::warn!(event = "match.emote.outside_hand", matchId = %self.match_id_str(), player = %player, emote = %emote);
            return;
        }
        let now = now_ms();
        let gate = emote_gate(&core.emote_history[home], now);
        if !gate.ok {
            return;
        }
        let mut sent_at = gate.sent_at;
        sent_at.push(now);
        core.emote_history[home] = sent_at;
        // R1351000: between two accounts either of which blocks the other, the emote is relayed to no
        // one, as silently as the gate's drop, recorded nowhere and answered by no persona; a card that
        // hears emotes still hears it.
        if core.emotes_blocked {
            self.enqueue(Task::Emote { home, emote });
            return;
        }
        self.send(core, player.opponent(), &emote_relay_message(player, emote));
        // R1442: recorded until the result, which the telemetry is written with.
        if !core.host.is_finished() {
            let turn = core.host.game().turn;
            core.telemetry.on_emote(home, emote, turn, now);
        }
        // MD-D29, R1127: after the relay, the emote also runs as a task, so a card that hears it
        // answers it in turn order behind every action queued ahead of it. A bot's emote is minted
        // like a person's (R1452): a hearing card answers either.
        self.enqueue(Task::Emote { home, emote });
        if self.shared.bots[home].is_some() {
            return;
        }
        let foe = home.opponent();
        let intents = match core.personas[foe].as_mut() {
            Some(persona) => persona.on_player_emote(emote, now),
            None => Vec::new(),
        };
        self.schedule_emotes(foe, intents);
    }

    // Sockets

    /// A frame from the account that began in `home`; the seat it acts for is read as it runs.
    fn on_frame(&self, home: PlayerId, text: &str) {
        let message = parse_client_message(text);
        let mut core = self.lock();
        let player = playing(&core, home);

        match message {
            Err(malformed) => {
                // A bad frame is answered, not fatal: the actor stays up and the other seat is untouched.
                tracing::warn!(event = "match.frame.malformed", matchId = %self.match_id_str(), player = %player, reason = %malformed.reason);
                // R835002: past `MALFORMED_FRAMES_PER_WINDOW` in the window the seat is a flooder,
                // not a stale client, so its socket is closed after the usual answer. The guard is
                // dropped first: the socket's close handler re-locks the actor (`attach` closes a
                // replaced socket the same way).
                let over = core.guards[home].note_malformed(now_ms());
                self.send_to_home(
                    &core,
                    home,
                    &error_message(SocketErrorCode::Malformed, &malformed.reason, None),
                );
                if over {
                    let socket = core.sockets[home].clone();
                    drop(core);
                    if let Some(socket) = socket {
                        socket.close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
                    }
                }
            }
            Ok(ClientMessage::Hello(_)) => {
                // §9.5: "Reconnect gets a fresh full view, never a log replay." R835001: a Hello
                // while one for the same home and socket generation is still queued is absorbed, so
                // a flood costs one view push. R2686000: one within WATCH_HELLO_INTERVAL_MS of the
                // last answered is dropped, the socket left open; a reattach always enqueues.
                let generation = core.socket_generation[home];
                if core.hello_pending[home] != Some(generation)
                    && core.guards[home].hello_answer_allowed(generation, now_ms())
                {
                    core.hello_pending[home] = Some(generation);
                    self.enqueue(Task::Hello { home, generation });
                }
            }
            Ok(ClientMessage::Emote(emote)) => self.relay_emote(&mut core, home, emote.emote),
            Ok(ClientMessage::Aim(aim)) => {
                let outputs = self.step_in(&mut core, DuelEvent::Aim { home, aim: aim.aim });
                self.exec_now(&mut core, outputs);
            }
            Ok(ClientMessage::ChoiceHover(hover)) => {
                let outputs = self.step_in(
                    &mut core,
                    DuelEvent::Hover {
                        home,
                        index: hover.index,
                    },
                );
                self.exec_now(&mut core, outputs);
            }
            Ok(ClientMessage::Action(action)) => {
                let now = now_ms();
                if !core.guards[home].action_allowed(now) {
                    // §9.8: reject the overflow; the socket stays open.
                    tracing::warn!(event = "match.action.flooded", matchId = %self.match_id_str(), player = %player);
                    // R835000: a flood rejection counts toward the seat's unusual-rejection alert.
                    let alert = self.note_rejection(&mut core, home, now);
                    self.send_to_home(
                        &core,
                        home,
                        &error_message(
                            SocketErrorCode::RateLimited,
                            "too many actions; slow down",
                            Some(&action.nonce),
                        ),
                    );
                    if let Some((profile_id, rejected)) = alert {
                        alert_unusual_rejections(self.match_id_str(), &profile_id, rejected);
                    }
                    return;
                }
                self.enqueue(Task::Action {
                    home,
                    nonce: action.nonce,
                    body: action.body,
                });
            }
        }
    }

    fn on_socket_gone(&self, home: PlayerId) {
        {
            let mut core = self.lock();
            // A socket that has already been replaced (the seat holds its open successor) or dropped
            // by `stop()` is not a disconnect: only the seat's own socket, now closed, is.
            if core.sockets[home].as_ref().is_none_or(Socket::is_open) {
                return;
            }
            core.sockets[home] = None;
            // R738, R2739001: the aim and the choice hover of the seat this account plays (R677) end
            // with its socket.
            let outputs = self.step_in(&mut core, DuelEvent::Left { home });
            self.exec_now(&mut core, outputs);
            if core.host.is_stopped() || core.host.is_finished() {
                return;
            }
            let player = playing(&core, home);
            tracing::info!(event = "match.socket.closed", matchId = %self.match_id_str(), player = %player, home = %home);
        }
        self.enqueue(Task::Disconnect { home });
    }

    // The actor's surface

    pub fn match_id(&self) -> &str {
        &self.shared.match_row.id
    }

    /// The seats as the match began: `.0` is the account that started in p1.
    pub fn seats(&self) -> &(MatchSeat, MatchSeat) {
        &self.shared.seats
    }

    /// The seat a profile BEGAN the match in, or `None` when it is not in this match. It names the
    /// account's connection, not the seat it plays: after a Glitch's swap (R677) the account that
    /// began in p1 plays p2, and the actor routes its socket there (`playing`).
    pub fn seat_of(&self, profile_id: &str) -> Option<PlayerId> {
        let (first, second) = &self.shared.seats;
        [first, second]
            .into_iter()
            .find(|seat| seat.profile_id == profile_id)
            .map(|seat| seat.player)
    }

    /// R1351000: `x` and `y` were just blocked one way or the other. When they are this match's two
    /// accounts, their emotes are muted toward each other from now until the actor goes. Answers
    /// whether they were.
    pub fn mute_emotes_between(&self, x: &str, y: &str) -> bool {
        let (first, second) = &self.shared.match_row.players;
        let seated = (first == x && second == y) || (first == y && second == x);
        if seated {
            self.lock().emotes_blocked = true;
        }
        seated
    }

    /// Hands a connection to the account that began in `home` (`seat_of`). Replaces (and closes) a
    /// socket that account already held.
    pub fn attach(&self, home: PlayerId, socket: Socket) {
        let previous = {
            let mut core = self.lock();
            if core.host.is_voided() {
                None
            } else {
                let previous = core.sockets[home].replace(socket.clone());
                // R744: the account has now been here, so no grace starts for it as another attaches.
                let outputs = self.step_in(&mut core, DuelEvent::Connected { home });
                self.exec_now(&mut core, outputs);
                // R835001: every attach — the first included — moves the seat to a new socket
                // generation, so a Hello from this socket is never absorbed by an older one's
                // queued Hello.
                core.socket_generation[home] += 1;
                Some(previous)
            }
        };
        let Some(previous) = previous else {
            // R679: the match no longer exists; a socket that arrives late hears only that.
            socket.close(Some(MATCH_VOIDED_CLOSE_CODE), Some(MATCH_VOIDED_CLOSE_REASON));
            return;
        };

        let on_message = Arc::downgrade(&self.shared);
        let on_close = on_message.clone();
        socket.attach(SocketHandlers {
            message: Box::new(move |text: String| {
                if let Some(shared) = on_message.upgrade() {
                    MatchActor { shared }.on_frame(home, &text);
                }
            }),
            close: Box::new(move || {
                if let Some(shared) = on_close.upgrade() {
                    MatchActor { shared }.on_socket_gone(home);
                }
            }),
        });
        if let Some(previous) = previous
            && previous != socket
        {
            previous.close(Some(1000), Some("replaced by a new socket"));
        }

        let player = playing(&self.lock(), home);
        tracing::info!(event = "match.socket.attached", matchId = %self.match_id_str(), player = %player, home = %home);
        self.enqueue(Task::Attach { home });
    }

    /// §9.5: lets go of that account's socket and starts its disconnect grace.
    pub fn detach(&self, home: PlayerId) {
        let socket = self.lock().sockets[home].clone();
        let Some(socket) = socket else { return };
        if socket.is_open() {
            socket.close(Some(1000), Some("detached"));
        }
        // A transport that does not call back synchronously still leaves the seat empty.
        self.on_socket_gone(home);
    }

    /// Applies one action as engine seat `player` — the seat is stamped here, never taken from the
    /// client (§9.1). Answers the `ack` or the `error` the client is sent.
    pub async fn submit(&self, player: PlayerId, nonce: String, body: ActionBody) -> ServerMessage {
        let (reply, answer) = oneshot::channel();
        let failed = error_message(
            SocketErrorCode::Internal,
            "the match is not running",
            Some(&nonce),
        );
        self.enqueue(Task::Submit {
            player,
            nonce,
            body,
            reply,
        });
        answer.await.unwrap_or(failed)
    }

    /// Resolves when the actor's queue has drained (every task queued before this call has run).
    pub async fn idle(&self) {
        let (done, drained) = oneshot::channel();
        if self.shared.inbox.send(Task::Idle(done)).is_err() {
            return;
        }
        let _ = drained.await;
    }

    /// #1169: resolves once the checkpoint of the state after the log's newest row is written (or
    /// passed over, `write_checkpoint`), after every task queued before it. The restart drain calls it
    /// before `stop`; a crash or the reaper's `stop` writes none, and the next rebuild folds from the
    /// last checkpoint there is or from the start.
    pub async fn checkpoint(&self) {
        let (done, written) = oneshot::channel();
        if self.shared.inbox.send(Task::Checkpoint(done)).is_err() {
            return;
        }
        let _ = written.await;
    }

    /// #1169: writes state checkpoint, telemetry (R1442), and bot turns (R639000) for rebuilds.
    /// Skipped if ended, voided, stopped, or below `MATCH_CHECKPOINT_MIN_ACTIONS`.
    /// Failed writes are logged (no state in the line) and fall back to full log folds on rebuild.
    async fn write_checkpoint(&self) {
        let (seq, nonce, body) = {
            let core = self.lock();
            let seq = core.host.next_seq() - 1;
            if core.host.is_finished()
                || core.host.is_voided()
                || core.host.is_stopped()
                || seq < MATCH_CHECKPOINT_MIN_ACTIONS
            {
                return;
            }
            let Some(nonce) = core.host.last_nonce() else {
                return;
            };
            let body = CheckpointBody {
                state: core.host.game().clone(),
                telemetry: core.telemetry.clone(),
                bot_moved_turn: core.host.bot_moved_turn().clone(),
            };
            (seq, nonce.to_string(), body)
        };
        let match_id = self.match_id_str();
        let row = match checkpoint_row(&self.shared.match_row, seq, &nonce, &body, now_ms()) {
            Ok(row) => row,
            Err(error) => {
                tracing::warn!(event = "match.checkpoint.failed", matchId = %match_id, seq = seq, message = %error);
                return;
            }
        };
        match one_tx!(self.shared.deps.db, |t| t.matches_put_checkpoint(&row).await?) {
            Ok(true) => {
                tracing::info!(event = "match.checkpoint.written", matchId = %match_id, seq = seq, bytes = row.body.len());
            }
            Ok(false) => tracing::info!(event = "match.checkpoint.kept", matchId = %match_id, seq = seq),
            Err(error) => {
                tracing::warn!(event = "match.checkpoint.failed", matchId = %match_id, seq = seq, message = %error);
            }
        }
    }

    /// R1442: the play telemetry the actor holds now, as both seats' people would write it. For tests.
    pub fn telemetry(&self) -> PlayTelemetry {
        self.lock().telemetry.finish(&telemetry::LIVE_PILOTS, None)
    }

    pub fn view_for(&self, player: PlayerId) -> PlayerView {
        let core = self.lock();
        self.view_of(&core, player)
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        engine::snapshot(self.lock().host.game())
    }

    /// For the registry's hash checks and the reaper; never sent to a socket.
    pub fn engine_state(&self) -> EngineState {
        self.lock().host.game().clone()
    }

    pub fn clocks(&self) -> MatchClocks {
        self.lock().host.clocks()
    }

    /// Drops the actor: stops the clock and lets both sockets go, leaving the log alone.
    pub async fn stop(&self) {
        self.halt();
        self.idle().await;
    }

    /// `stop` without waiting for the queue to drain, so a task on the queue can call it.
    fn halt(&self) {
        let sockets = {
            let mut core = self.lock();
            // The machine stops its clocks and relays, and takes their timers back.
            let outputs = self.step_in(&mut core, DuelEvent::Stop);
            self.exec_now(&mut core, outputs);
            for player in PLAYERS {
                if let Some(timer) = core.bot_long_turn[player].take() {
                    timer.cancel();
                }
            }
            PLAYERS.map(|home| core.sockets[home].take())
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(1001), Some("the actor is going away"));
            }
        }
        // Social (#939): and so do its watchers.
        self.spectate_close(CLOSE_GOING_AWAY, "the actor is going away");
    }

    /// R672: seats with an open socket, read by the registry's `presence_of`. R1452: bot seats are
    /// present during play; R639000: post-game they linger until rematch plans or pending replies
    /// finish, after which they report as absent.
    pub fn presence(&self) -> PerPlayer<bool> {
        let core = self.lock();
        // A socket the machine has not heard of closing is closed all the same: the actor's is the
        // truth about its socket, the machine's the truth about a bot.
        let machine = core.host.presence(now_ms());
        PerPlayer::new(
            machine.p1
                && (self.shared.bots.p1.is_some() || core.sockets.p1.as_ref().is_some_and(Socket::is_open)),
            machine.p2
                && (self.shared.bots.p2.is_some() || core.sockets.p2.as_ref().is_some_and(Socket::is_open)),
        )
    }

    /// When this actor's rematch window opened, or `None` while the game is on.
    pub fn finished_at(&self) -> Option<i64> {
        self.lock().host.finished_at()
    }

    /// A handle that does not keep the actor alive, for a test to watch it go.
    pub fn downgrade(&self) -> WeakMatchActor {
        WeakMatchActor {
            shared: Arc::downgrade(&self.shared),
        }
    }
}

/// `MatchActor::downgrade`. Everything else that outlives a call holds the actor this way too (its
/// queue task, its sockets' handlers, its timers), and `Shared` owns the queue's only sender, so the
/// queue task ends once no `MatchActor` is left.
#[derive(Clone)]
pub struct WeakMatchActor {
    shared: Weak<Shared>,
}

impl WeakMatchActor {
    /// Whether any `MatchActor` still holds the actor: its state, its log's acks and its queue task.
    pub fn is_alive(&self) -> bool {
        self.shared.strong_count() > 0
    }
}

impl MatchActor {
    /// R835000: tracks refused actions per seat, triggering at most one alert per match when
    /// crossing `REJECTION_ALERT_THRESHOLD` within `REJECTION_ALERT_WINDOW_MS`.
    fn note_rejection(&self, core: &mut Core, home: PlayerId, now: i64) -> Option<(String, usize)> {
        let rejected = core.guards[home].note_rejection(now)?;
        let (first, second) = self.seats();
        let profile_id = [first, second]
            .into_iter()
            .find(|seat| seat.player == home)
            .map(|seat| seat.profile_id.clone())
            .unwrap_or_default();
        Some((profile_id, rejected))
    }
}

pub use jackioh_shared::host::relay::{aim_is_public, hover_is_shown};

// Social: watching and shared replays (#939, #940)
//
// B50, B51: a friend's live 1v1 game watched from their seat. Each change records each seat's view,
// keyed by home (R677: a Glitch's swap moves the record with the account), and a watcher is sent each
// view of its friend's seat SPECTATE_DELAY_MS after the state it shows, by one armed wait. Nothing a
// watcher's socket sends reaches a reducer, log or clock: its emotes alone are relayed, live, to the
// players and the other watchers, from its own hand under its own limit (R2990000-R2990003), and never
// to a persona, a card that hears emotes or the telemetry. Past its malformed budget it is closed with
// 1008. Its leaving changes only the count the players are told.

use crate::actor::protocol::{spectator_emote_message, spectator_hand_message, watchers_frame};
// A watcher's socket closes with `CLOSE_NORMAL` when a newer socket of its account replaces it or
// after a finished match's last view (R1107000), and with `CLOSE_GOING_AWAY` when the actor goes.
use crate::actor::ws_server::{CLOSE_GOING_AWAY, CLOSE_NORMAL, WS_CLOSE};
use crate::config::{SPECTATE_DELAY_MS, SPECTATORS_PER_MATCH};
use crate::social::watch::{
    SPECTATOR_EMOTE_LIMITS, WatchFrame, deal_spectator_emote_hand, parse_watch_frame, send_spectator_emote,
};

/// What a revoked watcher's `forbidden` frame and close say: the words every refused hello carries
/// (`social::watch`), so a socket learns that it may no longer watch, never which change said so.
pub const WATCH_NOT_ALLOWED: &str = "you may not watch this match";

/// R1107000: what a finished match's watcher's close says, once it has been sent its seat's last view.
pub(crate) const MATCH_OVER_CLOSE_REASON: &str = "the match is over";

/// One recorded view of a seat: its encoded `view` frame (`legal: []`, B50), when the state it shows
/// was reached, and its place in the record. R1834003: and the `(seat, card)` pairs it shows
/// (`shown_defs`), whose card picks follow it to a watcher that has not had them.
struct SpectateFrame {
    seq: u64,
    shown_at: i64,
    frame: String,
    shown: BTreeSet<(PlayerId, String)>,
}

/// One watching socket: the account behind it, the seat its friend began the match in, the last
/// recorded view it was sent, and (R1834003) the pairs whose card picks it has been sent.
/// R2609000: and its own `SeatGuard`, holding a seat's malformed budget and its answered hellos.
/// R2990000, R2990001: and the name its emotes are shown under, and its own emote hand.
struct Watcher {
    id: u64,
    profile_id: String,
    name: String,
    home: PlayerId,
    socket: Socket,
    sent: Option<u64>,
    card_picks_sent: BTreeSet<(PlayerId, String)>,
    guard: SeatGuard,
    emote_hand: Vec<EmoteId>,
}

/// R1834003: sends `socket` the card picks `shown` makes due, marking them in `sent`.
fn send_cards_due(
    socket: &Socket,
    shown: &BTreeSet<(PlayerId, String)>,
    loadouts: &(Loadout, Loadout),
    sent: &mut BTreeSet<(PlayerId, String)>,
) {
    for cards in cards_due(shown, loadouts, sent) {
        socket.send(encode(&card_loadouts_message(cards)));
    }
}

/// B50, B51: what the actor keeps for its watchers, for any live 1v1 match, ranked or unranked.
/// Each seat's record holds the newest view at least SPECTATE_DELAY_MS old and
/// every view since: all a watcher can still be owed, and nothing older.
struct Spectate {
    records: PerPlayer<VecDeque<SpectateFrame>>,
    watchers: Vec<Watcher>,
    next_seq: u64,
    next_id: u64,
    /// The one wait armed for the next view due to a watcher, and when it is due.
    timer: Option<(Timer, i64)>,
    /// Set once the actor stops or the match is voided: no watcher is taken or sent anything more.
    closed: bool,
    /// R2990002: each watching account's recent emote sends for `emote_gate_within`, kept by account
    /// for the actor's life, so a new socket of the account starts from where its last one stopped.
    emote_sent_at: BTreeMap<String, Vec<i64>>,
}

impl Spectate {
    fn new() -> Spectate {
        Spectate {
            records: PerPlayer::new(VecDeque::new(), VecDeque::new()),
            watchers: Vec::new(),
            next_seq: 0,
            next_id: 0,
            timer: None,
            closed: false,
            emote_sent_at: BTreeMap::new(),
        }
    }
}

/// R1107000: closes the sockets of the watchers `MatchActor::spectate_let_go_finished` let go, each
/// with CLOSE_NORMAL. Called without the actor's lock, which each socket's close handler takes; the
/// handler finds its watcher already out of the list.
fn close_finished_watchers(watchers: Vec<Watcher>) {
    for watcher in watchers {
        if watcher.socket.is_open() {
            watcher
                .socket
                .close(Some(CLOSE_NORMAL), Some(MATCH_OVER_CLOSE_REASON));
        }
    }
}

/// Why `MatchActor::attach_watcher` turned a watcher away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchRefused {
    /// SPECTATORS_PER_MATCH sockets already watch the match.
    Full,
    /// The match is voided, or its actor is going away.
    Closed,
}

impl WatchRefused {
    pub fn message(self) -> &'static str {
        match self {
            WatchRefused::Full => "this match already has as many watchers as it takes",
            WatchRefused::Closed => "this match cannot be watched",
        }
    }
}

impl MatchActor {
    /// B50: `profile_id`'s watcher socket on the seat `home`'s account plays. A newer socket of the
    /// account replaces its older one; otherwise a match with SPECTATORS_PER_MATCH watchers is `Full`.
    /// Its hellos are answered (R1103000), its emotes relayed under `name` (R2990000), its other
    /// frames count against its malformed budget (R2609000).
    pub fn attach_watcher(
        &self,
        home: PlayerId,
        profile_id: &str,
        name: &str,
        socket: Socket,
    ) -> Result<(), WatchRefused> {
        // R2990001: the watcher's own hand, the same on every socket of the account.
        let hand = deal_spectator_emote_hand(&self.shared.match_row.seed, profile_id);
        let (id, replaced, done) = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.host.is_voided() || core.spectate.closed {
                return Err(WatchRefused::Closed);
            }
            let at = core
                .spectate
                .watchers
                .iter()
                .position(|watcher| watcher.profile_id == profile_id);
            let replaced = at.map(|at| core.spectate.watchers.remove(at));
            if replaced.is_none() && core.spectate.watchers.len() >= SPECTATORS_PER_MATCH {
                return Err(WatchRefused::Full);
            }
            // Those already watching are sent what has come due first, so each record is down to
            // the view the newcomer starts from.
            self.spectate_flush(&mut core, now_ms());
            let done = self.spectate_let_go_finished(&mut core);
            let id = core.spectate.next_id;
            core.spectate.next_id += 1;
            core.spectate.watchers.push(Watcher {
                id,
                profile_id: profile_id.to_string(),
                name: name.to_string(),
                home,
                socket: socket.clone(),
                sent: None,
                card_picks_sent: BTreeSet::new(),
                guard: SeatGuard::default(),
                emote_hand: hand.clone(),
            });
            (id, replaced, done)
        };

        let actor = Arc::downgrade(&self.shared);
        let heard = actor.clone();
        socket.attach(SocketHandlers {
            // B50: nothing a watcher sends reaches the reducer. R1103000: a hello is answered;
            // R2990000: an emote is relayed, and one naming no pool id answered `malformed`;
            // R2609000: the latter and anything else count against the watcher's malformed budget.
            message: Box::new(move |text: String| {
                let Some(shared) = heard.upgrade() else { return };
                let actor = MatchActor { shared };
                match parse_watch_frame(&text) {
                    WatchFrame::Hello => actor.answer_watcher_hello(id),
                    WatchFrame::Emote(emote) => actor.relay_spectator_emote(id, emote),
                    WatchFrame::Malformed(reason) => actor.note_watcher_junk(id, Some(&reason)),
                    WatchFrame::Junk => actor.note_watcher_junk(id, None),
                }
            }),
            close: Box::new(move || {
                if let Some(shared) = actor.upgrade() {
                    MatchActor { shared }.detach_watcher(id);
                }
            }),
        });
        let replacing = replaced.is_some();
        if let Some(previous) = replaced {
            // Already out of the list, so its close handler finds nothing to take off.
            previous
                .socket
                .close(Some(CLOSE_NORMAL), Some("replaced by a new socket"));
        }
        // R1107000: a watcher already in may just have had the finished match's last view.
        close_finished_watchers(done);
        // A socket that went before its handler was in has no close left to run.
        if !socket.is_open() {
            self.detach_watcher(id);
            return Ok(());
        }

        let mut core = self.lock();
        // R1834002: the seats' loadouts before any view, as a seat's socket has them.
        socket.send(encode(&loadouts_message(&self.shared.loadouts)));
        // R2990001: the watcher's own emote hand, to it alone.
        socket.send(encode(&spectator_hand_message(&hand)));
        // The newcomer's first view, when one is already SPECTATE_DELAY_MS old.
        self.spectate_flush(&mut core, now_ms());
        if !replacing {
            self.push_watchers(&core);
        }
        tracing::info!(event = "match.watcher.attached", matchId = %self.match_id_str(), home = %home, watchers = core.spectate.watchers.len());
        // R1107000: the newcomer's first view may be the finished match's last, and then it goes at once.
        let done = self.spectate_let_go_finished(&mut core);
        drop(core);
        close_finished_watchers(done);
        Ok(())
    }

    /// B51: how many sockets watch this match now.
    pub fn watcher_count(&self) -> usize {
        self.lock().spectate.watchers.len()
    }

    /// B50, B46: revokes matching watchers (watching turned off, unfriended, blocked), sending
    /// forbidden and closing with R148's 4403; updates players' count (B51). Answers how many.
    pub fn revoke_watchers(&self, revoked: &dyn Fn(&str, &str) -> bool) -> usize {
        let gone: Vec<Watcher> = {
            let mut core = self.lock();
            let (first, second) = &self.shared.seats;
            let friend_of = |home: PlayerId| {
                [first, second]
                    .into_iter()
                    .find(|seat| seat.player == home)
                    .map(|seat| seat.profile_id.as_str())
            };
            let (gone, kept): (Vec<Watcher>, Vec<Watcher>) = std::mem::take(&mut core.spectate.watchers)
                .into_iter()
                .partition(|watcher| {
                    friend_of(watcher.home).is_some_and(|friend| revoked(&watcher.profile_id, friend))
                });
            core.spectate.watchers = kept;
            if !gone.is_empty() && !core.host.is_stopped() {
                tracing::info!(event = "match.watcher.revoked", matchId = %self.match_id_str(), revoked = gone.len(), watchers = core.spectate.watchers.len());
                self.push_watchers(&core);
            }
            gone
        };
        // Already out of the list, so their close handlers find nothing to take off.
        let refusal = encode(&error_message(
            SocketErrorCode::Forbidden,
            WATCH_NOT_ALLOWED,
            None,
        ));
        for watcher in &gone {
            if watcher.socket.is_open() {
                watcher.socket.send(&refusal);
                watcher
                    .socket
                    .close(Some(WS_CLOSE.forbidden), Some(WATCH_NOT_ALLOWED));
            }
        }
        gone.len()
    }

    /// R1103000: a watcher's later hello is answered with its last view again, so its page hears the
    /// socket is alive (R1441's probe); one sent no view yet is sent nothing. R1834003: no loadouts
    /// ride with it (a page that lost them comes back on a new socket). R2609000: at most once per
    /// `WATCH_HELLO_INTERVAL_MS`; a hello that finds no view to send starts no interval.
    fn answer_watcher_hello(&self, id: u64) {
        let mut core = self.lock();
        if core.host.is_stopped() || core.spectate.closed {
            return;
        }
        let Spectate {
            records, watchers, ..
        } = &mut core.spectate;
        let Some(watcher) = watchers.iter_mut().find(|watcher| watcher.id == id) else {
            return;
        };
        let Some(record) = records[watcher.home]
            .iter()
            .find(|record| watcher.sent == Some(record.seq))
        else {
            return;
        };
        // R2609000: one answer per WATCH_HELLO_INTERVAL_MS; a hello sooner is dropped unanswered. The
        // watcher's id is its one generation (R2686000 gives a seat's socket the same gate).
        if !watcher.guard.hello_answer_allowed(id, now_ms()) {
            return;
        }
        watcher.socket.send(record.frame.as_str());
    }

    /// R2609000: a frame of a watcher's that is neither a hello nor an emote is still unanswered
    /// (R1103000) but counts against the watcher's own `MALFORMED_FRAMES_PER_WINDOW`, as a seat's does
    /// (R835002); R2990000: an emote frame naming no pool id counts too, and is answered `malformed`
    /// with `answer` first, as a seat's is. Past the budget the socket is closed with 1008, and its
    /// close handler takes the watcher off and tells the players the new count (B51).
    fn note_watcher_junk(&self, id: u64, answer: Option<&str>) {
        let socket = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.spectate.closed {
                return;
            }
            let Some(watcher) = core.spectate.watchers.iter_mut().find(|watcher| watcher.id == id) else {
                return;
            };
            let over = watcher.guard.note_malformed(now_ms());
            if let Some(reason) = answer {
                watcher
                    .socket
                    .send(encode(&error_message(SocketErrorCode::Malformed, reason, None)));
            }
            if !over {
                return;
            }
            watcher.socket.clone()
        };
        tracing::warn!(event = "match.watcher.flooded", matchId = %self.match_id_str());
        // The close calls `detach_watcher` back, which takes the actor's lock.
        socket.close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
    }

    /// R2990000: watcher `id`'s emote. R2990001: one its hand does not hold is dropped silently, and
    /// R2990002: so is one past its limit (`SPECTATOR_EMOTE_LIMITS`), neither spending any of it nor
    /// counting as a refusal. One let through goes out live, as `spectatorEmote` under the watcher's
    /// name, to each player's own socket and every other watcher, never back to its sender, and
    /// (R2990003) to none that blocks it or is blocked by it (`send_spectator_emote`, off the lock). It
    /// is never queued, so no reducer, log, hash, record, telemetry, persona (R645) or card that hears
    /// emotes (R1127) ever sees it.
    fn relay_spectator_emote(&self, id: u64, emote: EmoteId) {
        let (sender_id, frame, recipients) = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.spectate.closed {
                return;
            }
            let Some(watcher) = core.spectate.watchers.iter().find(|watcher| watcher.id == id) else {
                return;
            };
            if !hand_holds(&watcher.emote_hand, emote) {
                tracing::warn!(event = "match.spectator_emote.outside_hand", matchId = %self.match_id_str(), emote = %emote);
                return;
            }
            let sender_id = watcher.profile_id.clone();
            let frame = encode(&spectator_emote_message(&watcher.name, emote));
            let now = now_ms();
            let history = core
                .spectate
                .emote_sent_at
                .get(&sender_id)
                .map_or(&[][..], Vec::as_slice);
            let gate = emote_gate_within(history, now, SPECTATOR_EMOTE_LIMITS);
            if !gate.ok {
                return;
            }
            let mut sent_at = gate.sent_at;
            sent_at.push(now);
            core.spectate.emote_sent_at.insert(sender_id.clone(), sent_at);
            let (first, second) = &self.shared.seats;
            let players = [first, second].into_iter().filter_map(|seat| {
                core.sockets[seat.player]
                    .as_ref()
                    .map(|socket| (seat.profile_id.clone(), socket.clone()))
            });
            let watchers = core
                .spectate
                .watchers
                .iter()
                .filter(|other| other.id != id)
                .map(|other| (other.profile_id.clone(), other.socket.clone()));
            let recipients: Vec<(String, Socket)> = players.chain(watchers).collect();
            (sender_id, frame, recipients)
        };
        tokio::spawn(send_spectator_emote(
            self.shared.deps.clone(),
            sender_id,
            recipients,
            frame,
        ));
    }

    /// A watcher's socket went: it leaves, and the players hear the new count (B51).
    fn detach_watcher(&self, id: u64) {
        let mut core = self.lock();
        let before = core.spectate.watchers.len();
        core.spectate.watchers.retain(|watcher| watcher.id != id);
        if core.spectate.watchers.len() == before || core.host.is_stopped() {
            return;
        }
        tracing::info!(event = "match.watcher.detached", matchId = %self.match_id_str(), watchers = core.spectate.watchers.len());
        self.push_watchers(&core);
    }

    /// B51: the count, to both players' own sockets.
    fn push_watchers(&self, core: &Core) {
        let frame = watchers_frame(core.spectate.watchers.len());
        for home in PLAYERS {
            self.send_text_to_home(core, home, &frame);
        }
    }

    /// B51: the count, to the account that began in `home`, when anyone watches: what its socket
    /// missed while it was away.
    fn push_watchers_to(&self, core: &Core, home: PlayerId) {
        let count = core.spectate.watchers.len();
        if count > 0 {
            self.send_text_to_home(core, home, &watchers_frame(count));
        }
    }

    fn send_text_to_home(&self, core: &Core, home: PlayerId, text: &str) {
        if let Some(socket) = &core.sockets[home]
            && socket.is_open()
        {
            socket.send(text);
        }
    }

    /// B50: the record's first views, of the state the actor opens on.
    fn spectate_record(&self) {
        let mut core = self.lock();
        self.spectate_record_in(&mut core);
    }

    /// B50: records each seat's view of the state as it stands now, the clock's time left in it as
    /// its player was shown, and sends whatever has come due.
    fn spectate_record_in(&self, core: &mut Core) {
        if core.spectate.closed {
            return;
        }
        let now = now_ms();
        for home in PLAYERS {
            let view = self.view_of(core, playing(core, home));
            let shown = shown_defs(&view, &self.shared.dealt_cards);
            let frame = encode(&view_message(view, &[], false, None));
            let seq = core.spectate.next_seq;
            core.spectate.next_seq += 1;
            core.spectate.records[home].push_back(SpectateFrame {
                seq,
                shown_at: now,
                frame,
                shown,
            });
        }
        self.spectate_flush(core, now);
    }

    /// Sends each watcher, oldest first, every view of its friend's seat that has come due
    /// (SPECTATE_DELAY_MS after the state it shows) and that it has not had; then drops each
    /// record's views older than its newest due one, and arms the wait for the next view due.
    fn spectate_flush(&self, core: &mut Core, now: i64) {
        let Spectate {
            records,
            watchers,
            timer,
            closed,
            ..
        } = &mut core.spectate;
        if *closed {
            return;
        }
        for watcher in watchers.iter_mut() {
            for record in &records[watcher.home] {
                if record.shown_at + SPECTATE_DELAY_MS > now {
                    break;
                }
                if watcher.sent.is_some_and(|sent| record.seq <= sent) {
                    continue;
                }
                watcher.socket.send(record.frame.as_str());
                watcher.sent = Some(record.seq);
                // R1834003: the card picks of what this view newly shows, after it.
                send_cards_due(
                    &watcher.socket,
                    &record.shown,
                    &self.shared.loadouts,
                    &mut watcher.card_picks_sent,
                );
            }
        }
        for home in PLAYERS {
            let record = &mut records[home];
            while record
                .get(1)
                .is_some_and(|next| next.shown_at + SPECTATE_DELAY_MS <= now)
            {
                record.pop_front();
            }
        }

        let next_due = watchers
            .iter()
            .filter_map(|watcher| {
                records[watcher.home]
                    .iter()
                    .map(|record| record.shown_at + SPECTATE_DELAY_MS)
                    .find(|due| *due > now)
            })
            .min();
        if timer.as_ref().map(|(_, due)| *due) == next_due {
            return;
        }
        if let Some((armed, _)) = timer.take() {
            armed.cancel();
        }
        let Some(due) = next_due else { return };
        let actor = Arc::downgrade(&self.shared);
        let armed = after(due - now, move |id| {
            let Some(shared) = actor.upgrade() else { return };
            let actor = MatchActor { shared };
            let mut core = actor.lock();
            if core.spectate.timer.as_ref().map(|(armed, _)| armed.id()) != Some(id) {
                return;
            }
            if let Some((armed, _)) = core.spectate.timer.take() {
                armed.fired();
            }
            actor.spectate_flush(&mut core, now_ms());
            let done = actor.spectate_let_go_finished(&mut core);
            drop(core);
            close_finished_watchers(done);
        });
        *timer = Some((armed, due));
    }

    /// R1107000, B51: removes watchers who received the final view of a finished match and
    /// updates player counts. Returns watchers to be closed via `close_finished_watchers`; called
    /// after each `spectate_flush` that can send that view.
    #[must_use]
    fn spectate_let_go_finished(&self, core: &mut Core) -> Vec<Watcher> {
        if !core.host.is_finished() || core.spectate.closed {
            return Vec::new();
        }
        let records = &core.spectate.records;
        let (done, kept): (Vec<Watcher>, Vec<Watcher>) = std::mem::take(&mut core.spectate.watchers)
            .into_iter()
            .partition(|watcher| {
                records[watcher.home]
                    .back()
                    .is_some_and(|last| watcher.sent == Some(last.seq))
            });
        core.spectate.watchers = kept;
        if !done.is_empty() && !core.host.is_stopped() {
            tracing::info!(event = "match.watcher.finished", matchId = %self.match_id_str(), finished = done.len(), watchers = core.spectate.watchers.len());
            self.push_watchers(core);
        }
        done
    }

    /// The actor stops or the match is voided: every watcher's socket closes with `code` and
    /// `reason`, the wait is canceled, and nothing more is kept or taken.
    fn spectate_close(&self, code: u16, reason: &str) {
        let watchers = {
            let mut core = self.lock();
            let spectate = &mut core.spectate;
            spectate.closed = true;
            if let Some((armed, _)) = spectate.timer.take() {
                armed.cancel();
            }
            spectate.records = PerPlayer::new(VecDeque::new(), VecDeque::new());
            std::mem::take(&mut spectate.watchers)
        };
        for watcher in watchers {
            if watcher.socket.is_open() {
                watcher.socket.close(Some(code), Some(reason));
            }
        }
    }
}
