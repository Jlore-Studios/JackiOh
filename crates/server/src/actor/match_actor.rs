//! One actor per match (SPEC §9.2, §9.3, §9.5, BUILD M6-T4). Port of `apps/server/src/match/actor.ts`.
//!
//! "The match actor is the only stateful component: it holds the state in memory, one WebSocket per
//! player, an alarm for the turn clock, and it appends every resolved action to the log" (§9.2).
//! This file is that, and nothing else: it owns no rules, no timer and no socket library.
//!
//!  - The rules live in `jackioh_engine`, reached through `actor::engine` (SURFACE §11.3: no port),
//!    under the match's pinned catalog (R388, R767000).
//!    §9.3: "`reduce` refuses illegal actions itself and returns the reason", so a refusal is relayed
//!    verbatim and never re-derived here.
//!  - The clock lives in `actor::clock` (R79). Every expiry comes back as a `ClockExpiry` and is
//!    dispatched as a *server action* through the same path a client action takes, because
//!    "(seed, log) reconstructs any match" (§9.3) — a timeout that skipped the log would break that.
//!    The mulligan clock (R268) is the one expiry that becomes more than one action: a `timeout` for
//!    each seat still owing its mulligan, in seat order, each its own row.
//!  - The results row lives behind `crate::api::results::record_result` (§9.5, M7-T2).
//!  - The only per-player payload is `engine::view_for(state, player)` (§9.1, §10.8). No socket
//!    ever sees a state, the other hand, library order, or the other seat's view.
//!  - R1452: a ladder bot's seat has no socket. The actor plays it itself (`drive_bots`): the move is
//!    decided and paced in `bots::seat`, off the queue, and comes back as a server action through the
//!    same path a client's takes; its persona (`bots::persona`) reads the bot seat's own view and its
//!    emotes are relayed as a player's are. R639000: its aim goes through the relay a beat before a
//!    targeted move, it lets an occasional turn run out, and once a ranked match ends it stays a
//!    while and may offer or meet a rematch (`bots::signals`), as a person does.
//!
//! Every task runs on one serialized queue, so the actor really is single-threaded: a socket frame
//! and a clock alarm can never interleave two `reduce` calls or two log appends. In Rust the queue
//! is one tokio task per match draining an mpsc inbox, one task at a time (each run as its own
//! spawned task, so a task that panics is logged as `match.task.threw` and the queue goes on, as
//! TS's `fireAndForget` caught a throw). What TS ran synchronously outside the queue (taking a
//! socket, parsing a frame, relaying an emote or an aim, a socket going away) runs synchronously
//! here too, on the caller's thread, under the actor's lock — which is never held across an
//! `.await` and never while a socket is closed (a close calls back into the actor).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::{mpsc, oneshot};

use jackioh_engine::{
    Action, ActionBody, Aim, AimEnd, EmoteId, GameEvent, GameOverReason, HandView, PLAYER_IDS, PendingView,
    PerPlayer, Pilot, PlayerId, PlayerView, PortraitId, Rng, aim_key, deal_emote_hand, emote_gate,
    hand_holds, portrait_or_default,
};

use crate::actor::clock::{Timer, after, create_match_clock};
use crate::actor::contracts::{
    ActorDeps, ClockExpiry, ClockView, CreateMatchClockInput, ExpiryHandler, MatchClock, RecordResultInput,
    SocketHandlers, VoidMatchInput, one_tx,
};
use crate::actor::engine::{self, EngineState, LastBoards, MatchSnapshot, Pinned};
use crate::actor::guard::{CLOSE_POLICY_VIOLATED, SeatGuard, alert_unusual_rejections};
use crate::actor::protocol::{
    ClientMessage, MATCH_VOIDED_CLOSE_REASON, SERVER_NONCE_PREFIX, ServerMessage, SocketErrorCode,
    ack_message, aim_relay_message, clock_message, emote_relay_message, encode, error_message,
    parse_client_message, portraits_message, prompt_for_opponent, prompt_for_you, view_message,
};
use crate::actor::telemetry::{self, Recorder};
use crate::actor::ws_server::Socket;
use crate::api::rematch::{RematchStakes, STAKE_NORMAL, offer_for_bot};
use crate::app::now_ms;
use crate::bots::persona::{AiEmote, EmotePersona};
use crate::bots::seat::{self, BotJob, BotMove, fallback_action, never_played};
use crate::bots::signals::{RematchPlan, aim_lead_ms, idles_turn, rematch_plan};
use crate::config::{
    AI_EMOTE, AIM_RELAY_INTERVAL_MS, BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT, BOT_REMATCH_FIT,
    BOT_STRENGTHS, BOT_TIMEOUT_FIT, BOT_WRITE_RETRY_MS, MATCH_RECORD_RESULT_ATTEMPTS,
    MATCH_RECORD_RESULT_BACKOFF_MS, MATCH_SERVER_ACTION_RETRY_MS, MATCH_VOIDED_CLOSE_CODE,
};
use crate::db::store::{BotRow, MatchActionRow, MatchClocks, MatchRow, MatchSeat, MatchStatus, StoreError};
use crate::ranked::ladder::RankTier;

const PLAYERS: [PlayerId; 2] = PLAYER_IDS;

/// R639000: a bot home's rematch answer on its way (`Core::bot_rematch_replying`), taken off when
/// the task that sends it ends.
struct Replying {
    actor: Weak<Shared>,
    home: PlayerId,
}

impl Drop for Replying {
    fn drop(&mut self) {
        if let Some(shared) = self.actor.upgrade() {
            MatchActor { shared }.lock().bot_rematch_replying[self.home] = false;
        }
    }
}

/// TS `MatchActorInput`.
pub struct MatchActorInput {
    /// TS `match` (a Rust keyword): the row as it stands.
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
}

/// R417, R678: the boards a match was created with — its seats' last boards and the two other
/// players' boards a Glitch may put on the field — as `create_game` and `fold` take them:
/// `(last_boards, glitch_boards)`.
pub fn last_boards_of(match_row: &MatchRow) -> (Option<LastBoards>, Option<LastBoards>) {
    (match_row.last_boards.clone(), match_row.glitch_boards.clone())
}

/// R1452, R639000: by home seat, the turn of each bot's last move in `log` (its nonce `srv-bot-…`),
/// as `on_bot_move` keeps it in memory, so an actor rebuilt in the middle of a bot's turn knows that
/// turn had already begun: its first move's think time, its concession and its idle roll come only
/// at a turn's start, not again after a restart. Folded step by step as `telemetry::replay` folds,
/// under the match's pinned catalog (R388, R767000).
pub fn bot_moved_turns(
    pinned: Pinned,
    match_row: &MatchRow,
    log: &[MatchActionRow],
    dealt: Option<Vec<PlayerId>>,
) -> PerPlayer<Option<i32>> {
    let (last_boards, glitch_boards) = last_boards_of(match_row);
    let mut state = engine::fold(
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
    let bot_nonce = format!("{SERVER_NONCE_PREFIX}bot-");
    let mut moved = PerPlayer::new(None, None);
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

/// One task on the actor's serialized queue (TS's `enqueue`d closures, named).
enum Task {
    /// The clock has to be armed before the first action (R79).
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
    Expire(ClockExpiry),
    Disconnect {
        home: PlayerId,
    },
    Attach {
        home: PlayerId,
    },
    /// TS `idle()`: answered once every task queued before it has run.
    Idle(oneshot::Sender<()>),
    /// R1452: the move a bot home's `seat::play` came back with, decided on the state as it stood
    /// when the log's next seq was `version`, in turn `turn`.
    BotMove {
        home: PlayerId,
        version: i64,
        turn: i32,
        played: BotMove,
    },
}

impl Task {
    /// TS's `what` label for `match.task.threw`.
    fn what(&self) -> String {
        match self {
            Task::Arm => "arm".to_string(),
            Task::Hello { .. } => "hello".to_string(),
            Task::Action { body, .. } | Task::Submit { body, .. } => format!("action:{}", body.action_type()),
            Task::Emote { .. } => "emote".to_string(),
            Task::Expire(expiry) => format!("expiry:{}", expiry.kind()),
            Task::Disconnect { .. } => "disconnect".to_string(),
            Task::Attach { .. } => "attach".to_string(),
            Task::Idle(_) => "idle".to_string(),
            Task::BotMove { .. } => "bot-move".to_string(),
        }
    }
}

/// R738: one seat's aim relay (see `Core::aims`).
struct AimSlot {
    /// The newest aim received and not yet relayed: `None` when there is none (TS `undefined`),
    /// `Some(None)` an aim that has ended (TS `null`).
    pending: Option<Option<Aim>>,
    /// The one wait armed to relay it.
    timer: Option<Timer>,
    /// When the seat's last relay went; `None` is never (TS `-Infinity`).
    last_at: Option<i64>,
    /// What the opponent was last told, so a repeat is never sent and a seat that leaves mid-aim has
    /// its arrow cleared.
    relayed: Option<Aim>,
}

fn empty_aim_slot() -> AimSlot {
    AimSlot {
        pending: None,
        timer: None,
        last_at: None,
        relayed: None,
    }
}

/// The actor's in-memory state (§9.2) and the append-only log's bookkeeping (§9.3).
struct Core {
    state: EngineState,
    next_seq: i64,
    /// §9.3: "every action carries a client nonce, deduped server-side". `reduce` dedupes internally
    /// too, but the actor must not write a second log row or push a second view, so it keeps the ack
    /// it already answered with. Rebuilt from the log, so a client that reconnects and retries an
    /// action from before the crash still gets its original ack.
    acks: IndexMap<String, ServerMessage>,
    /// Keyed by the seat each account BEGAN in (`home`).
    sockets: PerPlayer<Option<Socket>>,
    /// R744: which accounts (keyed by home, like `sockets`) have had a socket on this actor.
    attached_once: PerPlayer<bool>,
    /// R137, R835000, R835002: each seat's abuse guard (`actor::guard`), by home seat: its action
    /// flood window, its refusals toward the unusual-rejection alert and its malformed frames.
    guards: PerPlayer<SeatGuard>,
    /// R835001: the socket generation each seat's queued-but-unrun Hello belongs to (`None` when
    /// none is queued). A repeat for the same home and generation is absorbed; a Hello from a
    /// newer generation always enqueues.
    hello_pending: PerPlayer<Option<u64>>,
    /// R835001: bumped on every `attach` (the first included, so generations start at 1), keying
    /// `hello_pending` so a reattach's Hello is never swallowed by the older socket's queued one.
    socket_generation: PerPlayer<u64>,
    /// R643: each seat's recent emote sends, the one input the shared `emote_gate` decides on. The
    /// client runs the same gate against its own copy, so the two can never disagree about the
    /// limit, and a dropped emote is simply never relayed — not an error, not a rejected action.
    emote_history: PerPlayer<Vec<i64>>,
    /// R738: each seat's aim relay.
    aims: PerPlayer<AimSlot>,
    /// R679: set once a Glitch voided the match; nothing more is sent, written or attached.
    voided: bool,
    /// A rebuilt actor whose log already ends in a result and whose row is already `finished` has
    /// nothing left to record. One that is still `live` crashed between the terminal action and the
    /// results write, so it heals itself on arming (§9.5: every ending records a result).
    finished: bool,
    /// When this actor's rematch window opened (R672): the game's end, or, for an actor rebuilt on a
    /// match already `finished`, the rebuild. `None` while the game is on. `Registry::reap_finished`
    /// counts `REMATCH_OFFER_TTL_MS` from here.
    finished_at: Option<i64>,
    stopped: bool,
    persisted_clocks: MatchClocks,
    /// R744: the grace deadlines the match row held when this actor was built, keyed by engine seat
    /// as stored, until the first attach hands them to the clock (`start_absent_grace`). Until then a
    /// write of the clocks keeps them, so the arm task's first `persist_clocks` cannot erase them.
    stored_grace: Option<PerPlayer<Option<i64>>>,
    /// R677: whether the accounts hold each other's seat, as of the last state change. Every socket
    /// is keyed by the seat its account began in (`home`), and every engine read by the seat played
    /// now, so this one flag is the whole mapping. Only the engine's state sets it — a Glitch's
    /// swap, which a client reaches only by legally playing Glitch — never a frame.
    swapped: bool,
    last_pending_for: Option<PlayerId>,
    /// R265: which seats owed a mulligan at the last push, so a `prompt` frame goes out on a change.
    last_mulligan_owed: String,
    /// R1442: the match's play telemetry, written once its result has landed.
    telemetry: Recorder,
    /// R1442: each player's ladder tier as the actor armed (seat order, `MatchRow.players`), the
    /// telemetry's rank bucket; `None` until then, or when it could not be read.
    tiers: Option<PerPlayer<RankTier>>,
    /// R1452: whether each bot home's move is being decided now (`seat::play` in flight), by home.
    bot_thinking: PerPlayer<bool>,
    /// R1452: where each bot home's two streams stand: the search's and the think time's. A rebuilt
    /// actor starts both again, which nothing replays.
    bot_cursors: PerPlayer<(u32, u32)>,
    /// R1452: the turn each bot home last moved in, so a think time knows a turn's first move.
    bot_moved_turn: PerPlayer<Option<i32>>,
    /// R645, R1344: each bot home's persona, reading the bot seat's own view.
    personas: PerPlayer<Option<EmotePersona>>,
    /// The bot seat's view as its persona last read it (`on_events`' `prev`).
    bot_views: PerPlayer<Option<PlayerView>>,
    /// R645: the long-turn wait each bot home's persona keeps on the person's turn.
    bot_long_turn: PerPlayer<Option<Timer>>,
    /// R639000: each bot home's plan for the end of a ranked match, drawn as the game ends, and when
    /// its linger ends (epoch ms); `None` while the game is on, and for a match it offers nothing in.
    bot_rematch: PerPlayer<Option<(RematchPlan, i64)>>,
    /// R639000: whether each bot home has already answered a person's rematch offer.
    bot_rematch_answered: PerPlayer<bool>,
    /// R639000: whether each bot home's answer is on its way, which keeps it on the match until it
    /// lands, past its linger if need be.
    bot_rematch_replying: PerPlayer<bool>,
    /// Social (#939): B50's watchers and the delayed views they are sent (`Spectate`).
    spectate: Spectate,
}

struct Shared {
    deps: ActorDeps,
    match_row: MatchRow,
    /// The seats as the match began: `.0` is the account that started in p1.
    seats: (MatchSeat, MatchSeat),
    /// R642: the pair the `portraits` frame carries, `vanilla` for a match that predates them.
    portraits: (PortraitId, PortraitId),
    /// R1341, R1342: each account's emote hand, keyed by the seat it began in (`home`) like the
    /// sockets, dealt once from the match seed when the actor is built — the same eight after a
    /// restart, a reconnect or a Glitch's swap (R677), since a hand belongs to the player.
    emote_hands: PerPlayer<Vec<EmoteId>>,
    clock: MatchClock,
    inbox: mpsc::UnboundedSender<Task>,
    on_voided: Option<Box<dyn Fn() + Send + Sync>>,
    /// R1452: the ladder bot at each home seat, played by this actor; `None` for a person.
    bots: PerPlayer<Option<BotRow>>,
    /// R641000: a game of a Conquest series (`MatchActorInput::series_game`).
    series_game: bool,
    /// R388, R767000: the catalog the match is played under.
    pinned: Pinned,
    core: Mutex<Core>,
}

/// TS `MatchActor`: a cheap handle (`Clone`) on one match's actor.
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
    if core.swapped { home.opponent() } else { home }
}

fn mulligan_key(owed: &[PlayerId]) -> String {
    owed.iter()
        .map(|player| player.as_str())
        .collect::<Vec<_>>()
        .join(",")
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

fn panic_message(error: tokio::task::JoinError) -> String {
    if !error.is_panic() {
        return error.to_string();
    }
    jackioh_ai::panic_message(&*error.into_panic())
}

/// The serialized queue: one task at a time, so nothing interleaves (TS's `enqueue` chain).
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

/// TS `createMatchActor`. Must be called inside a tokio runtime: it spawns the actor's queue and arms
/// the clock.
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
    } = input;
    let seats = (
        MatchSeat {
            profile_id: match_row.players.0.clone(),
            player: PlayerId::P1,
            deck: match_row.decks.0.clone(),
            portrait: None,
        },
        MatchSeat {
            profile_id: match_row.players.1.clone(),
            player: PlayerId::P2,
            deck: match_row.decks.1.clone(),
            portrait: None,
        },
    );

    let next_seq = log.iter().fold(0, |highest, row| highest.max(row.seq)) + 1;
    let acks: IndexMap<String, ServerMessage> = log
        .iter()
        .map(|row| (row.action.nonce.clone(), ack_message(&row.action.nonce, row.seq)))
        .collect();

    let portraits = match_row
        .portraits
        .unwrap_or((portrait_or_default(None), portrait_or_default(None)));
    let emote_hands = PerPlayer::new(
        deal_emote_hand(&match_row.seed, PlayerId::P1),
        deal_emote_hand(&match_row.seed, PlayerId::P2),
    );
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

    let core = Core {
        state,
        next_seq,
        acks,
        sockets: PerPlayer::new(None, None),
        // R1452: a bot's seat is never away, so R744's grace never starts for it.
        attached_once: PerPlayer::new(bots.p1.is_some(), bots.p2.is_some()),
        guards: PerPlayer::new(SeatGuard::default(), SeatGuard::default()),
        hello_pending: PerPlayer::new(None, None),
        socket_generation: PerPlayer::new(0, 0),
        emote_history: PerPlayer::new(Vec::new(), Vec::new()),
        aims: PerPlayer::new(empty_aim_slot(), empty_aim_slot()),
        voided: false,
        finished: matches!(match_row.status, MatchStatus::Finished),
        finished_at: matches!(match_row.status, MatchStatus::Finished).then(now_ms),
        stopped: false,
        persisted_clocks: match_row.clocks.clone(),
        stored_grace: Some(match_row.clocks.grace_deadline.clone()),
        swapped: opening.seats_swapped,
        last_pending_for: opening.pending_for,
        last_mulligan_owed: mulligan_key(&mulligan_window(&opening)),
        telemetry,
        tiers: None,
        bot_thinking: PerPlayer::new(false, false),
        bot_cursors: PerPlayer::new((0, 0), (0, 0)),
        bot_moved_turn,
        personas,
        bot_views,
        bot_long_turn: PerPlayer::new(None, None),
        bot_rematch: PerPlayer::new(None, None),
        bot_rematch_answered: PerPlayer::new(false, false),
        bot_rematch_replying: PerPlayer::new(false, false),
        spectate: Spectate::new(),
    };

    let (inbox, inbox_rx) = mpsc::unbounded_channel();
    let expiries = inbox.downgrade();
    let on_expire: ExpiryHandler = Arc::new(move |expiry: ClockExpiry| {
        if let Some(inbox) = expiries.upgrade() {
            let _ = inbox.send(Task::Expire(expiry));
        }
    });
    let clock = create_match_clock(CreateMatchClockInput {
        started_at: match_row.created_at,
        on_expire,
    });

    let shared = Arc::new(Shared {
        deps,
        match_row,
        seats,
        portraits,
        emote_hands,
        clock,
        inbox,
        on_voided,
        bots,
        series_game,
        pinned,
        core: Mutex::new(core),
    });
    let actor = MatchActor { shared };

    // The clock has to be armed before the first action, not on the first one (R79: the turn clock
    // is already running when the match opens). TS's arm task ran its synchronous half (the sync)
    // in the microtask after `createMatchActor` returned, so the actor a caller got back was already
    // armed: the same here, before anything can read it; the queued half persists the clocks and
    // resolves a match that was already over.
    actor.shared.clock.sync(&clock_view_for(&opening));
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

    /// TS `fireAndForget(task)`: onto the queue; a task that panics is logged by `run_queue`.
    fn enqueue(&self, task: Task) {
        if self.shared.inbox.send(task).is_err() {
            tracing::warn!(event = "match.queue.closed", matchId = %self.match_id_str());
        }
    }

    async fn run(&self, task: Task) {
        match task {
            Task::Arm => {
                let snapshot = engine::snapshot(&self.lock().state);
                self.shared.clock.sync(&clock_view_for(&snapshot));
                // R744, R1452: a bot's seat is here from the start, as an opponent whose socket has
                // attached, so a person who has not opened the match yet is away from now.
                if snapshot.result.is_none() && PLAYERS.iter().any(|home| self.shared.bots[*home].is_some()) {
                    let mut core = self.lock();
                    self.start_absent_grace(&mut core);
                }
                self.persist_clocks().await;
                // R1442: the rank bucket is the tier each player holds as the match arms.
                if !self.lock().finished {
                    let tiers =
                        telemetry::ladder_tiers_or_none(&self.shared.deps, &self.shared.match_row.players)
                            .await;
                    self.lock().tiers = tiers;
                }
                if snapshot.result.is_some() {
                    self.on_terminal(&snapshot).await;
                } else {
                    // R1452: a bot that owes the match a move, as it opens or after a restart, makes it.
                    {
                        let mut core = self.lock();
                        for home in PLAYERS {
                            if let Some(view) = core.bot_views[home].clone() {
                                self.rearm_long_turn(&mut core, home, &view);
                            }
                        }
                    }
                    self.drive_bots();
                }
            }
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
                let core = self.lock();
                let player = playing(&core, home);
                self.push_view(&core, player);
                self.push_clock(&core, player);
                // R642: portraits ride again on a reconnect, as on join, with the account's hand (R1342).
                self.send_to_home(&core, home, &self.portraits_for(home));
            }
            Task::Action { home, nonce, body } => {
                // R677: the seat is the one this account plays when the action runs — a swap queued
                // ahead of it has already moved the account — and the reply goes to the account that
                // sent it.
                let player = playing(&self.lock(), home);
                let reply = self.apply_action(player, nonce, body).await;
                let core = self.lock();
                self.send_to_home(&core, home, &reply);
            }
            Task::Submit {
                player,
                nonce,
                body,
                reply,
            } => {
                let answer = self.apply_action(player, nonce, body).await;
                let _ = reply.send(answer);
            }
            Task::Emote { home, emote } => {
                // MD-D29, R1127: the seat is the one this account plays when the emote runs, and the
                // `Emote` action is minted only while a card hears it — otherwise the relay above is
                // all the emote ever was (R643). The reply is discarded: the sender already showed
                // the emote locally, and a rate-limited emote is silently dropped.
                let player = playing(&self.lock(), home);
                let wanted = ActionBody::Emote { emote };
                let nonce = {
                    let core = self.lock();
                    if engine::legal_actions(self.shared.pinned, &core.state, player).contains(&wanted) {
                        Some(format!("{SERVER_NONCE_PREFIX}emote-{}", core.next_seq))
                    } else {
                        None
                    }
                };
                if let Some(nonce) = nonce {
                    let _ = self.apply_action(player, nonce, wanted).await;
                }
            }
            Task::Expire(expiry) => self.on_expire(expiry).await,
            Task::Disconnect { home } => {
                // §9.5: grace starts, and "the clock keeps running while a player is disconnected" — on
                // the seat this account plays (R677).
                let player = playing(&self.lock(), home);
                self.shared.clock.start_grace(player, None);
                self.persist_clocks().await;
                let core = self.lock();
                self.push_clock(&core, player.opponent());
            }
            Task::Attach { home } => {
                let player = {
                    let mut core = self.lock();
                    let player = playing(&core, home);
                    self.shared.clock.clear_grace(player);
                    // R744: an account that has not been here yet is away from now.
                    self.start_absent_grace(&mut core);
                    player
                };
                self.persist_clocks().await;
                // §9.5: a fresh full view, never a log replay. R642: the portraits ride with it, and
                // R1342 the account's own emote hand with them.
                let core = self.lock();
                self.push_view(&core, player);
                self.push_clock(&core, player);
                self.send_to_home(&core, home, &self.portraits_for(home));
                self.push_clock(&core, player.opponent());
                // Social (#939): B51's count, which a socket that was away missed.
                self.push_watchers_to(&core, home);
            }
            Task::Idle(done) => {
                let _ = done.send(());
            }
            Task::BotMove {
                home,
                version,
                turn,
                played,
            } => self.on_bot_move(home, version, turn, played).await,
        }
    }

    // ---------------------------------------------------------------------
    // Sending. §10.8: a socket only ever carries this player's own view.
    // ---------------------------------------------------------------------

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

    fn view_of(&self, core: &Core, player: PlayerId) -> PlayerView {
        // R79: the engine never reads a clock, so the view's `clockMs` is filled in here from the one
        // component that knows. Nothing else about the view is touched.
        let view = engine::view_for(self.shared.pinned, &core.state, player);
        PlayerView {
            clock_ms: self
                .shared
                .clock
                .remaining_for(player)
                .map(|ms| i32::try_from(ms).unwrap_or(i32::MAX)),
            ..view
        }
    }

    /// §10.8's view and, beside it, BUILD M5-T2's array ("the client never computes legality itself;
    /// it asks `legalActions` and greys out the rest").
    ///
    /// `player` is passed to BOTH calls, which is the whole security property: a socket is only ever
    /// handed the actions its own seat may take. `legal_actions(state, player.opponent())` would name
    /// every `play` in the opponent's hand and so hand over the hidden information §9.1 lists first.
    ///
    /// Both are read off the same `state` under the same lock, so the array is always true of the
    /// view it travels with — the actor's queue means no `reduce` can land between them.
    fn push_view(&self, core: &Core, player: PlayerId) {
        let message = view_message(
            self.view_of(core, player),
            &engine::legal_actions(self.shared.pinned, &core.state, player),
        );
        self.send(core, player, &message);
    }

    fn push_clock(&self, core: &Core, player: PlayerId) {
        self.send(
            core,
            player,
            &clock_message(now_ms(), self.shared.clock.snapshot()),
        );
    }

    // ---------------------------------------------------------------------
    // The clock (R79, §9.5)
    // ---------------------------------------------------------------------

    /// R79, exactly: the turn clock ends the active player's turn; a prompt clock answers only that
    /// prompt; grace becomes a loss; the ceiling becomes a draw. R268 adds the mulligan clock, which
    /// times out every seat still owing its mulligan. Each one is a real action, appended to the log
    /// like any other, because `(seed, log)` must reconstruct the match (§9.3).
    fn server_actions_for(&self, core: &Core, expiry: ClockExpiry) -> Vec<(PlayerId, ActionBody)> {
        let snapshot = engine::snapshot(&core.state);
        match expiry {
            ClockExpiry::Turn { player } | ClockExpiry::Prompt { player } => {
                vec![(player, ActionBody::Timeout)]
            }
            // R268: one clock for both seats, so on expiry every seat still owing is timed out — read
            // when the expiry runs, since a seat may have answered between the alarm and this task — in
            // seat order, each stamped with its own seat (R146: a timeout belongs to the player whose
            // clock ran out). What a timed-out mulligan keeps is the engine's business (R268: the whole
            // hand); a seat that has already answered is owed nothing and gets no row.
            ClockExpiry::Mulligan => mulligan_window(&snapshot)
                .into_iter()
                .map(|player| (player, ActionBody::Timeout))
                .collect(),
            // SPEC §11 R146: "a disconnect timeout belongs to the player who disconnected, because the
            // loss is theirs". `disconnectExpired` names the player in its body (§10.2), so the seat
            // the action is *stamped* with would otherwise be free; R146 fixes it so folding the log
            // reads one seat rather than "whoever happened to be active".
            ClockExpiry::Grace { player } => vec![(player, ActionBody::DisconnectExpired { player })],
            // SPEC §11 R146: reaching the turn ceiling "belongs to neither and is stamped with the
            // active seat as a convention", so a fold never has to guess. R79 makes it a draw and R112
            // covers the reaper's version of the same ending.
            ClockExpiry::Ceiling => vec![(snapshot.active, ActionBody::CeilingReached)],
        }
    }

    async fn on_expire(&self, expiry: ClockExpiry) {
        let actions = {
            let core = self.lock();
            if core.stopped || core.finished {
                return;
            }
            self.server_actions_for(&core, expiry)
        };
        for (player, body) in actions {
            // `stop()` sets `stopped` from outside the queue, so it can land between two of R268's
            // timeouts; nothing is dispatched into an actor that is going away or a match that is over.
            let next_seq = {
                let core = self.lock();
                if core.stopped || core.finished {
                    return;
                }
                core.next_seq
            };
            tracing::info!(event = "match.clock.expired", matchId = %self.match_id_str(), kind = expiry.kind(), player = %player);
            // The nonce is derived from the seq this action will occupy: unique, and stable across a
            // rebuild, so a rebuilt actor cannot collide with a nonce already in the log. Its prefix is
            // one no client may send (R270), so no client can pre-empt it.
            let nonce = format!("{SERVER_NONCE_PREFIX}{}-{next_seq}", expiry.kind());
            let reply = self.apply_action(player, nonce, body).await;
            // R776000: a row the store did not take. The clock that ran out runs out again, and the
            // action is derived again then, under the same nonce while no other row has taken its seq.
            // The `return` ends R268's loop: the retried mulligan expiry derives the seats still owing.
            if matches!(
                reply,
                ServerMessage::Error {
                    code: SocketErrorCode::Internal,
                    ..
                }
            ) {
                if !self.lock().stopped {
                    tracing::warn!(event = "match.clock.retrying", matchId = %self.match_id_str(), kind = expiry.kind(), player = %player, inMs = MATCH_SERVER_ACTION_RETRY_MS);
                    self.shared.clock.retry(expiry, MATCH_SERVER_ACTION_RETRY_MS);
                }
                return;
            }
        }
    }

    /// The clocks as stored: the clock's own, plus any stored grace not yet handed to it (R744).
    fn clocks_to_store(&self, core: &Core) -> MatchClocks {
        let clocks = self.shared.clock.snapshot();
        let Some(stored) = &core.stored_grace else {
            return clocks;
        };
        let grace = PerPlayer::new(
            clocks.grace_deadline.p1.or(stored.p1),
            clocks.grace_deadline.p2.or(stored.p2),
        );
        MatchClocks {
            grace_deadline: grace,
            ..clocks
        }
    }

    async fn persist_clocks(&self) {
        let clocks = {
            let mut core = self.lock();
            let clocks = self.clocks_to_store(&core);
            if clocks == core.persisted_clocks {
                return;
            }
            core.persisted_clocks = clocks.clone();
            clocks
        };
        // §9.5: "the grace countdown is stored on the match so both clients show it".
        let written = one_tx!(self.shared.deps.db, |t| t
            .matches_set_clocks(self.match_id_str(), &clocks)
            .await?);
        if let Err(error) = written {
            tracing::warn!(event = "match.clocks.persistFailed", matchId = %self.match_id_str(), message = %error);
        }
    }

    // ---------------------------------------------------------------------
    // Applying an action
    // ---------------------------------------------------------------------

    /// Pushes everything the clients need after a state change, in one order for both seats. `fresh`
    /// is how many events the change produced, which the bots' personas read (R645).
    async fn after_change(&self, fresh: usize) {
        let snapshot = {
            let mut core = self.lock();
            let snapshot = engine::snapshot(&core.state);
            if snapshot.seats_swapped != core.swapped {
                self.on_seats_swapped(&mut core, snapshot.seats_swapped);
            }
            self.shared.clock.sync(&clock_view_for(&snapshot));
            self.feed_personas(&mut core, fresh);
            snapshot
        };
        self.persist_clocks().await;

        {
            let mut core = self.lock();
            for player in PLAYERS {
                self.push_view(&core, player);
            }
            self.push_prompts(&mut core, &snapshot);
            for player in PLAYERS {
                self.push_clock(&core, player);
            }
            // R1442: a seat's think time runs from this push.
            core.telemetry.on_pushed(&snapshot, now_ms());
            // Social (#939): B50, the watchers' record of the change, sent SPECTATE_DELAY_MS on.
            self.spectate_record_in(&mut core);
        }

        if snapshot.result.is_some() {
            self.on_terminal(&snapshot).await;
        } else {
            self.drive_bots();
        }
    }

    /// The `prompt` frames for a change in who owes an answer. Each is built off that seat's own
    /// `view_for`, so a frame never carries more than the view it travels with (§10.8).
    fn push_prompts(&self, core: &mut Core, snapshot: &MatchSnapshot) {
        let deadline = self.shared.clock.snapshot().prompt_deadline;

        // R265, R266: while both mulligans are open neither seat holds `pending`, and each is sent the
        // frame that fits it — a seat that owes its mulligan its own prompt, a seat that has answered
        // only that the other seat still owes one. The sealed answer is in neither: R266 makes that a
        // seat is ready public and what it kept private, and the opponent's frame names no choice.
        let owed = mulligan_window(snapshot);
        let owed_key = mulligan_key(&owed);
        if !owed.is_empty() && owed_key != core.last_mulligan_owed {
            for player in PLAYERS {
                if !owed.contains(&player) {
                    self.send(core, player, &prompt_for_opponent(player.opponent(), deadline));
                    continue;
                }
                let view = engine::view_for(self.shared.pinned, &core.state, player);
                if let Some(PendingView::ForYou(pending)) = &view.pending {
                    self.send(
                        core,
                        player,
                        &prompt_for_you(player, &pending.choice_id, pending.kind, deadline),
                    );
                }
            }
        }
        core.last_mulligan_owed = owed_key;

        // §10.6: one prompt at a time; the player who does not hold it learns only that it is open.
        let pending_for = snapshot.pending_for;
        if let Some(holder) = pending_for
            && Some(holder) != core.last_pending_for
        {
            let view = engine::view_for(self.shared.pinned, &core.state, holder);
            if let Some(PendingView::ForYou(pending)) = &view.pending {
                self.send(
                    core,
                    holder,
                    &prompt_for_you(holder, &pending.choice_id, pending.kind, deadline),
                );
            }
            self.send(core, holder.opponent(), &prompt_for_opponent(holder, deadline));
        }
        core.last_pending_for = pending_for;
    }

    /// R677: a Glitch swapped the seats. From here every socket reads the view of, and acts for, the
    /// other seat. A disconnect grace belongs to the account, so one running moves with it to the seat
    /// it plays now (restarted there: the clock keeps one deadline per seat, not per account).
    fn on_seats_swapped(&self, core: &mut Core, now: bool) {
        // R1452: a bot has no socket and is never away.
        let away: Vec<PlayerId> = PLAYERS
            .into_iter()
            .filter(|home| core.sockets[*home].is_none() && self.shared.bots[*home].is_none())
            .collect();
        for home in &away {
            self.shared.clock.clear_grace(playing(core, *home));
        }
        core.swapped = now;
        for home in &away {
            self.shared.clock.start_grace(playing(core, *home), None);
        }
        // R645: a bot's persona reads the seat its bot plays now, from a fresh start: the view it read
        // last is the other seat's, and no difference between the two is anything that happened.
        for home in PLAYERS {
            let seat = playing(core, home);
            if let Some(persona) = core.personas[home].as_mut() {
                persona.set_seat(seat);
                core.bot_views[home] = None;
            }
        }
        tracing::info!(event = "match.seats.swapped", matchId = %self.match_id_str(), swapped = core.swapped);
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
            },
            MatchSeat {
                profile_id: if swapped { first } else { second }.clone(),
                player: PlayerId::P2,
                deck: second_deck.clone(),
                portrait: None,
            },
        )
    }

    /// R679: a Glitch voided the match. It never happened: no result, no rating move, no last board
    /// and no game record. Both sockets are closed with `MATCH_VOIDED_CLOSE_CODE` (the last view they
    /// got already shows the game over with reason `voided`), the registry drops the actor, and
    /// `void_match` removes the match from the store and logs the one line abuse checks read.
    async fn on_void(&self) {
        let sockets = {
            let mut core = self.lock();
            core.voided = true;
            core.stopped = true;
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
            at: now_ms(),
        };
        if let Err(error) = crate::api::results::void_match(&self.shared.deps, input).await {
            // The row stays live, so the next socket for it rebuilds this actor, folds to the same void
            // and tries again.
            tracing::error!(event = "match.void.failed", matchId = %self.match_id_str(), message = %error);
        }
    }

    /// §9.5: "Every ending records a result and clears both players' in-match state." Once.
    async fn on_terminal(&self, snapshot: &MatchSnapshot) {
        let Some(result) = snapshot.result else { return };
        let at = now_ms();
        let (swapped, last_boards, final_hash) = {
            let mut core = self.lock();
            if core.finished {
                return;
            }
            core.finished = true;
            core.finished_at = Some(at);
            self.shared.clock.stop();
            // R738: no aim outlives the game.
            for player in PLAYERS {
                self.end_aim(&mut core, player);
            }
            // R639000: under the lock that ends the game, so a bot never reads as gone while the
            // result is written and then as present again.
            if result.reason != GameOverReason::Voided {
                self.plan_bot_rematches(&mut core, at);
            }
            (
                core.swapped,
                engine::last_boards(&core.state),
                engine::hash_state(&core.state),
            )
        };
        // R679: the one ending that records nothing. Only the engine's state reaches it.
        if result.reason == GameOverReason::Voided {
            self.on_void().await;
            return;
        }

        // The results row, the rating move and clearing `inMatchId` are `api/results.rs` (M7-T2); the
        // actor never writes them itself.
        let input = RecordResultInput {
            match_id: self.shared.match_row.id.clone(),
            // R677: credited by who plays each seat now, not by who began in it.
            seats: self.credited_seats(swapped),
            outcome: result,
            turns: snapshot.turn,
            at,
            // R417, R565: each seat's board as the game ended, from its own side.
            last_boards: Some(last_boards),
            // R768: the final state's hash, which the match's replay is held to.
            final_hash: Some(final_hash),
        };
        // The write finishes the match row in the same transaction, so nothing else here does: a row
        // finished apart from it would outlive a failed write, with no result and both players stuck
        // in the match. R1437: a write that never lands leaves the match `live` for the next socket.
        if !self.record_result_with_retries(input).await {
            self.let_go_unrecorded();
            return;
        }
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
        self.send_bot_rematch_offers(at);

        tracing::info!(
            event = "match.over",
            matchId = %self.match_id_str(),
            winner = %result.winner,
            reason = %result.reason,
            turns = snapshot.turn,
        );
    }

    /// R639000, R672: each bot home of a ranked match draws its plan for the game's end as the game
    /// ends (`at`), not voided: it stays present from then until its linger ends, as a person's open
    /// socket does, and meets a person's offer as `on_rematch_offer` says. An end whose result never
    /// lands lets the actor go (`let_go_unrecorded`), and its plan with it.
    fn plan_bot_rematches(&self, core: &mut Core, at: i64) {
        // R641000: a series game offers no rematch, so its bot stays for none.
        if self.shared.match_row.ranked != Some(true) || self.shared.series_game {
            return;
        }
        for home in PLAYERS {
            if self.shared.bots[home].is_some() {
                let plan = rematch_plan(&self.shared.match_row.seed, home, &BOT_REMATCH_FIT);
                core.bot_rematch[home] = Some((plan, at + plan.linger_ms));
            }
        }
    }

    /// R639000, R672: once the result landed, each bot home whose plan offers makes a normal rematch
    /// offer `offer_after_ms` after the game's end (`at`).
    fn send_bot_rematch_offers(&self, at: i64) {
        for home in PLAYERS {
            let planned = self.lock().bot_rematch[home];
            if let Some((plan, _)) = planned
                && plan.offers
            {
                self.bot_offers_rematch_after(home, at + plan.offer_after_ms - now_ms(), STAKE_NORMAL);
            }
        }
    }

    /// R639000: the bot home offers a rematch at `stakes` once `ms` have passed, through the route a
    /// person's offer takes (`api::rematch::offer_for_bot`), off the queue.
    fn bot_offers_rematch_after(&self, home: PlayerId, ms: i64, stakes: RematchStakes) {
        let deps = Arc::clone(&self.shared.deps);
        let match_id = self.shared.match_row.id.clone();
        let wait = Duration::from_millis(u64::try_from(ms).unwrap_or(0));
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if let Err(error) = offer_for_bot(&deps, &match_id, home, stakes).await {
                tracing::warn!(event = "bot.rematch.failed", matchId = %match_id, home = %home, message = %error.message);
            }
        });
    }

    /// R639000, R672: a person in seat `from` offered a rematch that made none yet. The bot home
    /// opposite meets it, at the person's stakes and `reply_ms` later, when its plan accepts, it has
    /// not answered already and it is still on the match (its linger has not ended). Once it means to
    /// meet it, it stays on the match until it has, as a person reaching for the button does.
    pub fn on_rematch_offer(&self, from: PlayerId, stakes: RematchStakes) {
        let bot = from.opponent();
        if self.shared.bots[bot].is_none() {
            return;
        }
        let reply_ms = {
            let mut core = self.lock();
            let Some((plan, until)) = core.bot_rematch[bot] else {
                return;
            };
            if !plan.accepts || core.bot_rematch_answered[bot] || now_ms() >= until {
                return;
            }
            core.bot_rematch_answered[bot] = true;
            core.bot_rematch_replying[bot] = true;
            plan.reply_ms
        };
        let deps = Arc::clone(&self.shared.deps);
        let match_id = self.shared.match_row.id.clone();
        let replying = Replying {
            actor: Arc::downgrade(&self.shared),
            home: bot,
        };
        let wait = Duration::from_millis(u64::try_from(reply_ms).unwrap_or(0));
        tokio::spawn(async move {
            // Dropped however the task ends, a panic included, so the bot never stays for good.
            let _replying = replying;
            tokio::time::sleep(wait).await;
            if let Err(error) = offer_for_bot(&deps, &match_id, bot, stakes).await {
                tracing::warn!(event = "bot.rematch.failed", matchId = %match_id, home = %bot, message = %error.message);
            }
        });
    }

    /// R1437: `record_result`, tried up to `MATCH_RECORD_RESULT_ATTEMPTS` times, the first wait
    /// `MATCH_RECORD_RESULT_BACKOFF_MS` and each next one twice the last. A repeat is safe: a write
    /// that finds the match's row already written answers that row (`api/results.rs`). It runs on the
    /// actor's queue, which the game's end has left nothing to do but refuse actions and push views.
    /// Answers whether the result landed.
    async fn record_result_with_retries(&self, input: RecordResultInput) -> bool {
        let mut wait_ms = MATCH_RECORD_RESULT_BACKOFF_MS;
        for attempt in 1..=MATCH_RECORD_RESULT_ATTEMPTS {
            match crate::api::results::record_result(&self.shared.deps, input.clone()).await {
                Ok(_) => return true,
                Err(error) => {
                    tracing::error!(event = "match.recordResult.failed", matchId = %self.match_id_str(), attempt = attempt, message = %error);
                }
            }
            if attempt < MATCH_RECORD_RESULT_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(wait_ms as u64)).await;
                wait_ms *= 2;
            }
        }
        false
    }

    /// R1437: the result never landed, so the match is still `live` and both players are still in it.
    /// The registry forgets this actor (as R679's void does, since `stop` would wait on the queue this
    /// runs in) and both sockets close as `stop` closes them, which the client reconnects on: the next
    /// socket rebuilds the actor from the log, and the rebuilt actor writes the result as it arms.
    fn let_go_unrecorded(&self) {
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = "match.result.unrecorded", matchId = %self.match_id_str());
    }

    /// R776000: the log holds a row at the seq this actor was about to write (a write that committed and
    /// whose answer was lost, or a second writer), so the actor is behind its log. It lets go as
    /// `let_go_unrecorded` does, and the next socket folds `(seed, log)`, that row included. An actor
    /// already stopped lets nothing go: the registry may hold its rebuilt successor by now.
    fn let_go_diverged(&self) {
        if self.lock().stopped {
            return;
        }
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = "match.log.diverged", matchId = %self.match_id_str());
    }

    /// The one path every action takes — a client's and the clock's alike. Reduce, append exactly one
    /// log row at the next gapless seq, then push.
    async fn apply_action(&self, player: PlayerId, nonce: String, body: ActionBody) -> ServerMessage {
        let (action, result, seq) = {
            let core = self.lock();
            if let Some(existing) = core.acks.get(&nonce) {
                return existing.clone();
            }
            if core.finished {
                return error_message(
                    SocketErrorCode::MatchOver,
                    "this match already has a result",
                    Some(&nonce),
                );
            }
            // §9.1: the seat is the server's, never the client's.
            let action = Action::new(body, player, nonce.clone());
            let result = engine::reduce(self.shared.pinned, &core.state, &action);
            (action, result, core.next_seq)
        };

        if let Some(reason) = &result.error {
            // §9.8: "Every rejected action is logged with its reason."
            tracing::warn!(
                event = "match.action.rejected",
                matchId = %self.match_id_str(),
                player = %player,
                "type" = %action.action_type(),
                reason = %reason,
            );
            // R835000: a reducer refusal counts toward the seat's unusual-rejection alert.
            // `player` is an engine seat; `playing` maps it back to the account's home (its own
            // inverse, R677). The dedupe-ack and `MatchOver` paths above return before this, and
            // store failures below never reach `reduce`'s verdict, so none of them counts.
            let alert = {
                let mut core = self.lock();
                let home = playing(&core, player);
                self.note_rejection(&mut core, home, now_ms())
            };
            if let Some((profile_id, rejected)) = alert {
                alert_unusual_rejections(self.match_id_str(), &profile_id, rejected);
            }
            return error_message(SocketErrorCode::IllegalAction, reason, Some(&nonce));
        }

        // §9.3: append-only. The row goes in *before* the new state is committed, so a failed write
        // can never leave the in-memory state ahead of the log that has to reconstruct it.
        let row = MatchActionRow {
            match_id: self.shared.match_row.id.clone(),
            seq,
            action,
            at: now_ms(),
        };
        let appended = one_tx!(self.shared.deps.db, |t| t
            .matches_append_actions(std::slice::from_ref(&row))
            .await?);
        if let Err(error) = appended {
            tracing::error!(event = "match.append.failed", matchId = %self.match_id_str(), seq = seq, message = %error);
            if matches!(error, StoreError::SeqClash(_)) {
                self.let_go_diverged();
            }
            return error_message(
                SocketErrorCode::Internal,
                "the action could not be recorded; try again",
                Some(&nonce),
            );
        }

        let ack = ack_message(&nonce, seq);
        {
            let mut guard = self.lock();
            let core = &mut *guard;
            let before = std::mem::replace(&mut core.state, result.state);
            core.next_seq = seq + 1;
            core.acks.insert(nonce, ack.clone());
            // R1442: the move as it arrived, on the state it was made in, once the state has moved on
            // with the log, so nothing the telemetry does can leave the two apart.
            core.telemetry.on_action(
                &before,
                &row.action,
                seq,
                row.at,
                self.shared.clock.remaining_for(player),
                result.events.last().map(GameEvent::event_type),
            );
        }

        self.after_change(result.events.len()).await;
        ack
    }

    // ---------------------------------------------------------------------
    // The bot seat (R1452)
    // ---------------------------------------------------------------------

    /// R1452: starts the move of every bot home whose seat owes the match one (R677: the engine seat
    /// it plays now) and is not already deciding. `seat::play` decides and waits out the think time
    /// off the queue, then queues the move as `Task::BotMove`; the queue goes on meanwhile, so a
    /// clock that runs out first still times the bot out as it would a person.
    ///
    /// R639000: a bot whose own turn is just starting first asks `signals::idles_turn` whether it
    /// lets this one run out; when it does, no move is started, so its turn clock times it out
    /// (R79) as it does a person who is away. The answer is drawn on the turn's own stream, so every
    /// later call this turn answers the same. A targeted move's aim goes through the relay
    /// (`receive_aim`) once `play` comes back, and the move `aim_lead_ms` after it.
    fn drive_bots(&self) {
        let mut core = self.lock();
        if core.finished || core.stopped {
            return;
        }
        for home in PLAYERS {
            let Some(bot) = &self.shared.bots[home] else {
                continue;
            };
            if core.bot_thinking[home] {
                continue;
            }
            let player = playing(&core, home);
            if !self
                .shared
                .pinned
                .run(|| jackioh_ai::ai_to_act(&core.state, player))
            {
                continue;
            }
            let Some(strength) = BOT_STRENGTHS.get(bot.strength).or(BOT_STRENGTHS.last()).copied() else {
                continue;
            };
            let seed = &self.shared.match_row.seed;
            let turn = core.state.turn;
            let first_in_turn = core.bot_moved_turn[home] != Some(turn);
            let own_turn_start = first_in_turn
                && core.state.active == player
                && core.state.mulligan.is_none()
                && core.state.pending.is_none();
            let clock_left_ms = self.shared.clock.remaining_for(player);
            // Only a running clock can time the bot out; with none, an idle turn would never end.
            if own_turn_start && clock_left_ms.is_some() && idles_turn(seed, home, turn, &BOT_TIMEOUT_FIT) {
                continue;
            }
            let (ai_cursor, timing_cursor) = core.bot_cursors[home];
            let version = core.next_seq;
            let job = BotJob {
                state: core.state.clone(),
                seat: player,
                home,
                seed: seed.clone(),
                strength,
                speed: bot.speed,
                ai_rng: Rng::new(&format!("{seed}:bot-ai:{home}"), ai_cursor),
                timing_rng: Rng::new(&format!("{seed}:bot-timing:{home}"), timing_cursor),
                first_in_turn,
                own_turn_start,
                aim_lead_ms: aim_lead_ms(seed, home, version),
                clock_left_ms,
                pinned: self.shared.pinned,
            };
            core.bot_thinking[home] = true;
            let actor = Arc::downgrade(&self.shared);
            tokio::spawn(async move {
                let played = seat::play(job).await;
                if let Some(aim) = played.aim {
                    // R738: the aim shows a beat before the move, through the relay a person's goes by.
                    let Some(shared) = actor.upgrade() else { return };
                    let strong = MatchActor { shared };
                    {
                        let mut core = strong.lock();
                        let player = playing(&core, home);
                        strong.receive_aim(&mut core, player, Some(aim));
                    }
                    // Not held across the wait, so the actor can still go meanwhile.
                    drop(strong);
                    tokio::time::sleep(Duration::from_millis(
                        u64::try_from(played.aim_lead_ms).unwrap_or(0),
                    ))
                    .await;
                }
                if let Some(shared) = actor.upgrade() {
                    MatchActor { shared }.enqueue(Task::BotMove {
                        home,
                        version,
                        turn,
                        played,
                    });
                }
            });
        }
    }

    /// R1452: plays the move a bot home decided, as a server action (R270's prefix, so no client can
    /// pre-empt it and R1442 records no think time for it, numbered by the seq it takes) stamped with
    /// the seat it plays now. R639000: a bot concedes and answers a draw offer as `seat::play` drew
    /// it, never offers a draw (`never_played`), and its aim, once relayed, is taken down after the
    /// move as a person's client takes its own down.
    ///
    /// A move decided on a state the log has since moved past (the other seat acted meanwhile: a
    /// person's auto-end-turn switch, their side of the mulligan, an answer to a prompt) still goes,
    /// its think time waited out, while it is the same turn and the engine takes it; else the bot
    /// decides again. So no stream of the person's own actions can hold the bot's move back. A fresh
    /// move the AI could not give, or the engine refused, becomes `fallback_action`'s, tried once;
    /// one whose write failed is decided again `BOT_WRITE_RETRY_MS` later, never traded for the
    /// fallback.
    async fn on_bot_move(&self, home: PlayerId, version: i64, turn: i32, played: BotMove) {
        let aimed = played.aim.is_some();
        self.on_bot_move_inner(home, version, turn, played).await;
        if aimed {
            let mut core = self.lock();
            let player = playing(&core, home);
            self.receive_aim(&mut core, player, None);
        }
    }

    async fn on_bot_move_inner(&self, home: PlayerId, version: i64, turn: i32, played: BotMove) {
        let (player, body, stale, previous_turn, nonce) = {
            let mut core = self.lock();
            core.bot_thinking[home] = false;
            core.bot_cursors[home] = (played.ai_cursor, played.timing_cursor);
            if core.finished || core.stopped {
                return;
            }
            let player = playing(&core, home);
            let stale = core.next_seq != version;
            let body = played.body.filter(|body| !never_played(body));
            let body = if !stale {
                body.or_else(|| fallback_action(self.shared.pinned, &core.state, player))
            } else if core.state.turn == turn {
                body
            } else {
                None
            };
            let Some(body) = body else {
                drop(core);
                if stale {
                    self.drive_bots();
                }
                return;
            };
            // Set before the move goes, since the move's own push already starts the next one.
            let now_turn = core.state.turn;
            let nonce = format!("{SERVER_NONCE_PREFIX}bot-{}", core.next_seq);
            (
                player,
                body,
                stale,
                core.bot_moved_turn[home].replace(now_turn),
                nonce,
            )
        };
        let mut reply = self.apply_action(player, nonce.clone(), body).await;
        let refused = matches!(
            &reply,
            ServerMessage::Error {
                code: SocketErrorCode::IllegalAction,
                ..
            }
        );
        let retry = if refused && !stale {
            fallback_action(self.shared.pinned, &self.lock().state, player)
        } else {
            None
        };
        if let Some(body) = retry {
            reply = self.apply_action(player, format!("{nonce}-r"), body).await;
        }
        if let ServerMessage::Error { code, message, .. } = &reply {
            self.lock().bot_moved_turn[home] = previous_turn;
            tracing::warn!(event = "bot.move.refused", matchId = %self.match_id_str(), player = %player, code = ?code, message = %message);
            if *code != SocketErrorCode::IllegalAction {
                self.drive_bots_after(BOT_WRITE_RETRY_MS);
            } else if stale {
                self.drive_bots();
            }
        }
    }

    /// `drive_bots` once `ms` have passed: a bot whose move could not be written decides again at
    /// that pace (`BOT_WRITE_RETRY_MS`).
    fn drive_bots_after(&self, ms: i64) {
        let actor = Arc::downgrade(&self.shared);
        let wait = Duration::from_millis(u64::try_from(ms).unwrap_or(0));
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if let Some(shared) = actor.upgrade() {
                MatchActor { shared }.drive_bots();
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
            let view = engine::view_for(self.shared.pinned, &core.state, playing(core, home));
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
        if core.finished || core.stopped || view.result.is_some() || view.active == view.viewer {
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
                if core.finished || core.stopped {
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
                if !core.stopped {
                    actor.relay_emote(&mut core, home, intent.emote);
                }
            });
        }
    }

    /// R643, R1342: relays the emote of the account that began in `home`, a person's or a bot's, to
    /// its opponent. One outside the sender's dealt hand is refused first, as silently as a
    /// rate-limited one and without spending the limit: the menu never offers it, so only a stale or
    /// a hand-made client sends one. Then the same gate the client ran: a fail is a silent drop (no
    /// error, no rejected-action log); a pass relays to the opponent only (the sender already showed
    /// it locally), then runs as an `Emote` action while a card hears it (R1127). R645: a person's
    /// emote that goes out is offered to the opposing bot's persona, which may answer it; a bot never
    /// answers a bot.
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
        self.send(core, player.opponent(), &emote_relay_message(player, emote));
        // R1442: recorded until the result, which the telemetry is written with.
        if !core.finished {
            let turn = core.state.turn;
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

    // ---------------------------------------------------------------------
    // Sockets
    // ---------------------------------------------------------------------

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
                // while one for the same home and socket generation is still queued is absorbed —
                // reconnect is idempotent — so a hello flood costs one view push. A newer
                // generation (a reattach) always enqueues.
                let generation = core.socket_generation[home];
                if core.hello_pending[home] != Some(generation) {
                    core.hello_pending[home] = Some(generation);
                    self.enqueue(Task::Hello { home, generation });
                }
            }
            Ok(ClientMessage::Emote(emote)) => self.relay_emote(&mut core, home, emote.emote),
            Ok(ClientMessage::Aim(aim)) => self.receive_aim(&mut core, player, aim.aim),
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

    // ---------------------------------------------------------------------
    // The opponent's aim (R738)
    // ---------------------------------------------------------------------

    /// R738: an aim is relayed to the opponent at most once per `AIM_RELAY_INTERVAL_MS` per seat.
    /// Faster ones are coalesced, not dropped outright: the newest waits out the interval and goes
    /// alone, so the opponent's arrow always ends where the sender's did. Nothing is answered.
    fn receive_aim(&self, core: &mut Core, player: PlayerId, aim: Option<Aim>) {
        if core.finished || core.stopped {
            return;
        }
        let slot = &mut core.aims[player];
        slot.pending = Some(aim);
        if slot.timer.is_some() {
            return;
        }
        let wait = match slot.last_at {
            None => -1,
            Some(last_at) => last_at + AIM_RELAY_INTERVAL_MS - now_ms(),
        };
        if wait <= 0 {
            self.flush_aim(core, player);
            return;
        }
        let actor = Arc::downgrade(&self.shared);
        slot.timer = Some(after(wait, move |id| {
            let Some(shared) = actor.upgrade() else { return };
            let actor = MatchActor { shared };
            let mut core = actor.lock();
            if core.aims[player].timer.as_ref().map(Timer::id) != Some(id) {
                return;
            }
            if let Some(timer) = core.aims[player].timer.take() {
                timer.fired();
            }
            actor.flush_aim(&mut core, player);
        }));
    }

    /// Relays the seat's pending aim, checked against the state as it stands NOW rather than when the
    /// frame arrived: an aim whose ends name anything the opponent may not see is dropped silently,
    /// and the opponent's arrow is cleared instead if it showed one (R738, R97). A repeat of what
    /// the opponent was last told is not sent again.
    fn flush_aim(&self, core: &mut Core, player: PlayerId) {
        let Some(aim) = core.aims[player].pending.take() else {
            return;
        };
        if core.finished || core.stopped {
            return;
        }
        let receiver = player.opponent();
        let shown = match aim {
            None => None,
            Some(aim) => {
                if aim_is_public(
                    &aim,
                    player,
                    &engine::view_for(self.shared.pinned, &core.state, receiver),
                ) {
                    Some(aim)
                } else {
                    None
                }
            }
        };
        if aim_key(shown.as_ref()) == aim_key(core.aims[player].relayed.as_ref()) {
            return;
        }
        core.aims[player].last_at = Some(now_ms());
        core.aims[player].relayed = shown;
        self.send(core, receiver, &aim_relay_message(player, shown));
    }

    /// A seat that leaves, or a match that ends, takes its arrow with it.
    fn end_aim(&self, core: &mut Core, player: PlayerId) {
        let slot = &mut core.aims[player];
        if let Some(timer) = slot.timer.take() {
            timer.cancel();
        }
        slot.pending = None;
        if slot.relayed.is_none() {
            return;
        }
        slot.relayed = None;
        self.send(core, player.opponent(), &aim_relay_message(player, None));
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
            if core.stopped || core.finished {
                return;
            }
            // R738: the aim of the seat this account plays (R677) ends with its socket.
            let player = playing(&core, home);
            self.end_aim(&mut core, player);
            tracing::info!(event = "match.socket.closed", matchId = %self.match_id_str(), player = %player, home = %home);
        }
        self.enqueue(Task::Disconnect { home });
    }

    /// R744: an account that has had no socket on this actor (the actor was rebuilt after a restart,
    /// or the player never opened the match) is disconnected, so its grace starts with the first
    /// socket that does attach, on the seat it plays (R677): at the deadline stored on the match when
    /// there is one, else a fresh window. A bot's seat is attached from the start, so in a match with
    /// a bot it starts as the actor arms (R1452). Nothing starts at boot, so a match no socket returns
    /// to stays the reaper's draw (R112) and nobody loses by seat order. Safe to repeat: a running
    /// grace keeps its deadline (R147).
    fn start_absent_grace(&self, core: &mut Core) {
        for home in PLAYERS {
            if core.attached_once[home] {
                continue;
            }
            let player = playing(core, home);
            let stored = core.stored_grace.as_ref().and_then(|grace| grace[player]);
            self.shared.clock.start_grace(player, stored);
        }
        core.stored_grace = None;
    }

    // ---------------------------------------------------------------------
    // The actor's surface (TS `MatchActor`)
    // ---------------------------------------------------------------------

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

    /// Hands a connection to the account that began in `home` (`seat_of`). Replaces (and closes) a
    /// socket that account already held.
    pub fn attach(&self, home: PlayerId, socket: Socket) {
        let previous = {
            let mut core = self.lock();
            if core.voided {
                None
            } else {
                let previous = core.sockets[home].replace(socket.clone());
                core.attached_once[home] = true;
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

    pub fn view_for(&self, player: PlayerId) -> PlayerView {
        let core = self.lock();
        self.view_of(&core, player)
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        engine::snapshot(&self.lock().state)
    }

    /// For the registry's hash checks and the reaper; never sent to a socket.
    pub fn engine_state(&self) -> EngineState {
        self.lock().state.clone()
    }

    pub fn clocks(&self) -> MatchClocks {
        self.shared.clock.snapshot()
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
            core.stopped = true;
            self.shared.clock.stop();
            for player in PLAYERS {
                if let Some(timer) = core.aims[player].timer.take() {
                    timer.cancel();
                }
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

    /// R672: which seats hold an open socket right now. The registry's `presence_of` reads this, so a
    /// rematch offer learns whether the opponent is still on the match. A seat is present while it
    /// holds a socket that is still open. R1452: a bot's seat holds none, and is present until the
    /// game ends, as a person playing it would be; R639000: after that it stays present until its
    /// rematch plan's linger ends, or past it until an answer it is giving lands, as a person on the
    /// death screen is, and then reads as gone.
    pub fn presence(&self) -> PerPlayer<bool> {
        let core = self.lock();
        let playing_on = !core.finished && !core.voided;
        let now = now_ms();
        let lingering = |home: PlayerId| {
            core.finished
                && (core.bot_rematch_replying[home]
                    || core.bot_rematch[home].is_some_and(|(_, until)| now < until))
        };
        let open = |home: PlayerId| {
            core.sockets[home].as_ref().is_some_and(Socket::is_open)
                || (self.shared.bots[home].is_some() && (playing_on || lingering(home)))
        };
        PerPlayer::new(open(PlayerId::P1), open(PlayerId::P2))
    }

    /// When this actor's rematch window opened (`Core::finished_at`), or `None` while the game is on.
    pub fn finished_at(&self) -> Option<i64> {
        self.lock().finished_at
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
    /// R835000: counts one refused action for `home` (a reducer refusal or a flood rejection) and
    /// answers the alert's payload the first time the seat's window crosses
    /// `REJECTION_ALERT_THRESHOLD`: the seat's profile id and the count. `None` before that and
    /// after the alert has gone — exactly one alert per match per seat. The window is pruned to
    /// `REJECTION_ALERT_WINDOW_MS` first, so old refusals fall out and the crossing is strictly
    /// over the threshold.
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

fn clock_view_for(snapshot: &MatchSnapshot) -> ClockView {
    ClockView {
        turn: snapshot.turn,
        active: snapshot.active,
        pending_for: snapshot.pending_for,
        mulligan_owed: snapshot.mulligan_owed.clone(),
        over: snapshot.result.is_some() || snapshot.phase == jackioh_engine::Phase::Over,
    }
}

/// R738, R97, R177: whether every end of `sender`'s aim names something `view` (the receiver's own)
/// shows. A hero and a zone of the field are public wherever they are — a face-down card is named
/// by its zone, never itself — so a zone need only exist. A hand card is named by its position, and
/// only in the sender's own hand, which the receiver sees as that many backs.
pub fn aim_is_public(aim: &Aim, sender: PlayerId, view: &PlayerView) -> bool {
    match &aim.source {
        AimEnd::Hand { player, index } => {
            if *player != sender {
                return false;
            }
            let hand = if sender == view.viewer {
                &view.you.hand
            } else {
                &view.opponent.hand
            };
            let count = match hand {
                HandView::Cards(cards) => cards.len(),
                HandView::Count { count } => usize::try_from(*count).unwrap_or(0),
            };
            if *index >= count {
                return false;
            }
        }
        source => {
            if !end_is_public(source, view) {
                return false;
            }
        }
    }
    match &aim.target {
        None => true,
        Some(AimEnd::Hand { .. }) => false,
        Some(target) => end_is_public(target, view),
    }
}

fn end_is_public(end: &AimEnd, view: &PlayerView) -> bool {
    match end {
        AimEnd::Hand { .. } => false,
        AimEnd::Hero { .. } => true,
        AimEnd::Zone { player, row, lane } => {
            let side = if *player == view.viewer {
                &view.you
            } else {
                &view.opponent
            };
            i64::from(*lane) <= side.locks.row(*row).len() as i64
        }
    }
}

/// R265: the seats that owe a mulligan while the window is open — both mulligans open and no other
/// prompt in front of them — and nothing otherwise. The engine never reports both at once; the
/// clock (`clock.rs`) reads the window the same way.
fn mulligan_window(snapshot: &MatchSnapshot) -> Vec<PlayerId> {
    if snapshot.pending_for.is_none() && snapshot.result.is_none() {
        snapshot.mulligan_owed.clone()
    } else {
        Vec::new()
    }
}

// ---- Social: watching and shared replays (#939, #940) ----
//
// B50, B51: a friend's live, unranked game, watched from their seat. Each change records the view
// of each seat as the change left it, keyed by the account that began in it (`home`; R677: a
// Glitch's swap moves the record with the account), and each watcher is sent each recorded view of
// its friend's seat SPECTATE_DELAY_MS after the state it shows, by the one wait the actor arms for
// the next view due. A watcher who joins starts from the newest view already that old. Nothing a
// watcher's socket sends is read: it reaches no reducer, log, relay or clock, and a watcher's
// leaving changes nothing in the match but the count the players are told (B51). Once the match is
// over, a watcher sent its seat's last view, the one with the result, is let go (R1107000).

use crate::actor::protocol::watchers_frame;
use crate::actor::ws_server::WS_CLOSE;
use crate::config::{SPECTATE_DELAY_MS, SPECTATORS_PER_MATCH};

/// What a revoked watcher's `forbidden` frame and close say: the words every refused hello carries
/// (`social::watch`), so a socket learns that it may no longer watch, never which change said so.
pub const WATCH_NOT_ALLOWED: &str = "you may not watch this match";

/// RFC 6455's normal closure and "going away": a watcher's socket is closed with the first when a
/// newer socket of the same account replaces it or once it has had a finished match's last view
/// (R1107000), and with the second when the actor goes (wire vocabulary, like
/// `CLOSE_POLICY_VIOLATED`).
const CLOSE_NORMAL: u16 = 1000;
const CLOSE_GOING_AWAY: u16 = 1001;
/// R1107000: what a finished match's watcher's close says, once it has been sent its seat's last view.
const MATCH_OVER_CLOSE_REASON: &str = "the match is over";

/// One recorded view of a seat: its encoded `view` frame (`legal: []`, B50), when the state it shows
/// was reached, and its place in the record.
struct SpectateFrame {
    seq: u64,
    shown_at: i64,
    frame: String,
}

/// One watching socket: the account behind it, the seat its friend began the match in, and the
/// last recorded view it was sent.
struct Watcher {
    id: u64,
    profile_id: String,
    home: PlayerId,
    socket: Socket,
    sent: Option<u64>,
}

/// B50, B51: what the actor keeps for its watchers, and only for an unranked match, the one kind S7
/// lets anyone watch. Each seat's record holds the newest view at least SPECTATE_DELAY_MS old and
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
    /// The match is ranked, voided, or its actor is going away.
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
    /// B50: a watcher's socket, for `profile_id`'s account, on the seat the account that began in
    /// `home` plays. A newer socket of the same account replaces its older one, so an account is one
    /// watcher; otherwise a match already watched by SPECTATORS_PER_MATCH sockets is `Full`. The
    /// socket's frames go unread from here on, and the players are told the new count (B51).
    pub fn attach_watcher(
        &self,
        home: PlayerId,
        profile_id: &str,
        socket: Socket,
    ) -> Result<(), WatchRefused> {
        let (id, replaced, done) = {
            let mut core = self.lock();
            if core.stopped
                || core.voided
                || core.spectate.closed
                || self.shared.match_row.ranked == Some(true)
            {
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
                home,
                socket: socket.clone(),
                sent: None,
            });
            (id, replaced, done)
        };

        let actor = Arc::downgrade(&self.shared);
        socket.attach(SocketHandlers {
            // B50: whatever a watcher sends, nothing reads it.
            message: Box::new(|_text: String| {}),
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

    /// B50, B46: the watchers `revoked(watcher, friend)` names (the watcher's account and the
    /// account whose seat it watches) are let go at once: each socket is told `forbidden`, as a
    /// refused hello is, and closed with R148's 4403, and the players hear the new count (B51). The
    /// friend turning watching off, or the two unfriending or blocking, calls this: being let in once
    /// is no standing leave. Answers how many went.
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
            if !gone.is_empty() && !core.stopped {
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

    /// A watcher's socket went: it leaves, and the players hear the new count (B51).
    fn detach_watcher(&self, id: u64) {
        let mut core = self.lock();
        let before = core.spectate.watchers.len();
        core.spectate.watchers.retain(|watcher| watcher.id != id);
        if core.spectate.watchers.len() == before || core.stopped {
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
    /// its player was shown, and sends whatever has come due. Nothing is kept for a ranked match.
    fn spectate_record_in(&self, core: &mut Core) {
        if core.spectate.closed || self.shared.match_row.ranked == Some(true) {
            return;
        }
        let now = now_ms();
        for home in PLAYERS {
            let frame = encode(&view_message(self.view_of(core, playing(core, home)), &[]));
            let seq = core.spectate.next_seq;
            core.spectate.next_seq += 1;
            core.spectate.records[home].push_back(SpectateFrame {
                seq,
                shown_at: now,
                frame,
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

    /// R1107000: once the match is over, each watcher that has been sent its seat's last view (the
    /// newest in its record: nothing is recorded after the game's end, so it is the one with the
    /// result) is let go: out of the list, and the players told the new count (B51). Answers those
    /// watchers, for the caller to close with `close_finished_watchers` once it has let go of the
    /// lock. Called after each `spectate_flush` that can send that view: the wait's and an attach's.
    /// `spectate_record_in`'s cannot, since the view it records is SPECTATE_DELAY_MS from due.
    #[must_use]
    fn spectate_let_go_finished(&self, core: &mut Core) -> Vec<Watcher> {
        if !core.finished || core.spectate.closed {
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
        if !done.is_empty() && !core.stopped {
            tracing::info!(event = "match.watcher.finished", matchId = %self.match_id_str(), finished = done.len(), watchers = core.spectate.watchers.len());
            self.push_watchers(core);
        }
        done
    }

    /// The actor stops or the match is voided: every watcher's socket closes with `code` and
    /// `reason`, the wait is cancelled, and nothing more is kept or taken.
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
// ---- end Social: watching and shared replays ----
