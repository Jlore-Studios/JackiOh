//! A daily game's play (SPEC §9.13, #3727): nothing of it stays in memory between moves. Every request
//! loads its `matches` row, its `daily_games` row, its log and its checkpoint, folds the game, and an
//! action is reduced, appended and the clock stored beside it in one transaction. It never has a match
//! actor (§9.2): `Registry::rebuild` refuses a daily id.
//!
//! - `GET /api/daily`: the caller's games, challenges and seeks, and how many games wait on them.
//! - `GET /api/daily/games/:matchId`: one game as the caller's seat sees it: its view, its legal actions
//!   and the forecast of its attacks (R3855000). A finished game whose log the retention purge took
//!   answers its list row with `expired: true` and no view, which is never folded (R3934000).
//! - `POST /api/daily/games/:matchId/actions` `{after, nonce, action}`: one action, sent after the log's
//!   last row `after`, so a retried request applies once (its nonce is logged) and a stale one is
//!   refused (`stale`).
//!
//! Any read past a deadline settles the game first (R3727000): the player who held the move loses,
//! `disconnectExpired` under the nonce `srv-daily-<seq>`, as the sweep (`daily::notify`) would.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::{Action, ActionBody, CombatForecast, GameOverReason, PlayerId, Winner, opponent_of};
use jackioh_shared::host::SERVER_NONCE_PREFIX;
use jackioh_shared::host::daily::{
    DailyBoard, DailyClock, daily_allows, daily_board, daily_expired, daily_legal, daily_sync,
};

use crate::actor::checkpoint::{self, CheckpointBody};
use crate::actor::contracts::{RecordResultInput, VoidMatchInput};
use crate::actor::engine::{self, EngineState, Pinned};
use crate::actor::match_actor::last_boards_of;
use crate::actor::protocol::{client_nonce, parse_action_body};
use crate::actor::registry::{Prepared, dealt_for, panic_text, pinned_for, prepare_match};
use crate::actor::telemetry::Recorder;
use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, bad_request, json as json_response, now_ms, ok, to_json};
use crate::api::results::{record_result, seats_of, void_match};
use crate::app::App;
use crate::bug_reports::{Finding, FoundIn, refused_rows};
use crate::config::{DAILY_RECENT_FINISHED, DAILY_SETTLE_ATTEMPTS, MATCH_CHECKPOINT_MIN_ACTIONS};
use crate::db::store::{
    DailyGameRow, DailyInviteKind, DailyInviteRow, MatchActionRow, MatchCheckpointRow, MatchRow, MatchStatus,
    QueueMode, StartMatchInput, StoreError, Tx,
};
use crate::notifications::Notice;
use crate::notifications::store::NotificationKind;
use crate::social::live::push_social;
use crate::social::{PlayerRef, not_found, param, profiles_by_id};

/// A daily game as one request folded it.
pub(crate) struct Loaded {
    pub match_row: MatchRow,
    pub daily: DailyGameRow,
    pub log: Vec<MatchActionRow>,
    pub state: EngineState,
    pub pinned: Pinned,
}

impl Loaded {
    /// The log's last row, the one the next action is sent `after` (0 before the first).
    pub fn last_seq(&self) -> i64 {
        self.log.last().map_or(0, |row| row.seq)
    }

    /// R677: the engine seat `profile_id` plays now, or `None` for an account that is neither.
    pub fn seat_of(&self, profile_id: &str) -> Option<PlayerId> {
        let home = home_of(&self.daily, profile_id)?;
        Some(engine::seat_played_by(&self.state, home))
    }

    /// The account playing `seat` now (R677).
    pub fn account_at(&self, seat: PlayerId) -> &str {
        account_at(&self.daily, seat, self.daily.board.seats_swapped)
    }

    fn over(&self) -> bool {
        self.state.result.is_some()
    }
}

/// The seat `profile_id` began the game in.
fn home_of(daily: &DailyGameRow, profile_id: &str) -> Option<PlayerId> {
    if daily.players.0 == profile_id {
        Some(PlayerId::P1)
    } else if daily.players.1 == profile_id {
        Some(PlayerId::P2)
    } else {
        None
    }
}

/// R677: the account at `seat` while the seats are swapped or not.
fn account_at(daily: &DailyGameRow, seat: PlayerId, swapped: bool) -> &str {
    let first = (seat == PlayerId::P1) != swapped;
    if first { &daily.players.0 } else { &daily.players.1 }
}

/// R677: the seat `profile_id` plays in a game whose list row says `board`.
fn seat_now(daily: &DailyGameRow, profile_id: &str) -> Option<PlayerId> {
    let home = home_of(daily, profile_id)?;
    Some(if daily.board.seats_swapped {
        opponent_of(home)
    } else {
        home
    })
}

/// Whether a game was lost on time: a daily game's `disconnect` is only ever its clock's (R3727000).
fn lost_on_time(board: &DailyBoard) -> bool {
    board
        .result
        .is_some_and(|result| result.reason == GameOverReason::Disconnect)
}

/// A daily game's rows as one transaction read them, before anything is folded.
struct Stored {
    match_row: MatchRow,
    daily: DailyGameRow,
    log: Vec<MatchActionRow>,
    saved: Option<MatchCheckpointRow>,
}

impl Stored {
    /// R3934000: a game over whose log the retention purge took. Only a purge empties a finished game's log.
    fn moves_gone(&self) -> bool {
        self.daily.ended_at.is_some() && self.log.is_empty()
    }
}

/// The game `match_id`, folded, or `None` when no daily game has that id.
pub(crate) async fn load(app: &Arc<App>, match_id: &str) -> Result<Option<Loaded>, ApiError> {
    match read_stored(app, match_id).await? {
        Some(stored) => Ok(Some(fold_stored(app, stored).await?)),
        None => Ok(None),
    }
}

/// The game `match_id`'s rows, or `None` when no daily game has that id.
async fn read_stored(app: &Arc<App>, match_id: &str) -> Result<Option<Stored>, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let daily = tx.daily_games_get(match_id).await?;
    let match_row = match &daily {
        Some(_) => tx.matches_get(match_id).await?,
        None => None,
    };
    let (Some(daily), Some(match_row)) = (daily, match_row) else {
        tx.commit().await?;
        return Ok(None);
    };
    let log = tx.matches_actions(match_id).await?;
    let saved = tx.matches_checkpoint(match_id).await?;
    tx.commit().await?;
    Ok(Some(Stored {
        match_row,
        daily,
        log,
        saved,
    }))
}

/// `stored`, folded from its checkpoint or its log.
async fn fold_stored(app: &Arc<App>, stored: Stored) -> Result<Loaded, ApiError> {
    let Stored {
        match_row,
        daily,
        log,
        saved,
    } = stored;
    let match_id = match_row.id.as_str();
    // R388, R767000, R3727005: the catalog the game started on, for its whole life.
    let pinned = pinned_for(&match_row);
    let (last_boards, glitch_boards) = last_boards_of(&match_row);
    let args = engine::fold_args(
        &match_row.seed,
        &match_row.decks,
        log.iter().map(|row| row.action.clone()).collect(),
        last_boards,
        glitch_boards,
        dealt_for(Some(daily.mode)),
        match_row.win_rates.clone(),
    );
    let (row, rows) = (match_row.clone(), log.clone());
    // #1035: off the runtime, as a rebuild folds. #1169: from a checkpoint that passes, else the log.
    let folded = engine::off_runtime(app.engine_thread, move || {
        if let Some(Ok(body)) = saved.as_ref().map(|saved| checkpoint::verify(saved, &row, &rows)) {
            let seq = saved.as_ref().map_or(0, |saved| saved.seq);
            let tail = &rows[usize::try_from(seq).unwrap_or(rows.len()).min(rows.len())..];
            return checkpoint::fold_tail(pinned, body.state, tail);
        }
        engine::fold(pinned, &args)
    })
    .await
    .map_err(ApiError::internal)?;
    if !folded.errors.is_empty() {
        // §9.3: the engine refused an action it once accepted. The game goes on from what folds, and
        // the night bot hears of it, as a rebuild files it.
        tracing::error!(event = "daily.fold.errors", matchId = %match_id, errors = ?folded.errors);
        app.bug_reports.file(
            app.env.deployed_commit.as_deref(),
            Finding {
                match_id: match_id.to_string(),
                found_in: FoundIn::Rebuild,
                catalog_version: match_row.catalog_version.clone(),
                actions: log.len(),
                refused: refused_rows(&log, &folded.errors),
            },
        );
    }
    Ok(Loaded {
        match_row,
        daily,
        log,
        state: folded.state,
        pinned,
    })
}

/// What an append came to.
pub(crate) enum Applied {
    Done,
    /// Another request appended first: the caller reads the game again.
    Clash,
    /// The engine refused the action, with its reason.
    Refused(String),
}

/// Reduces `action` at `now`, then appends it and stores the clock it leaves in one transaction, so a
/// game's log and its clock never disagree; records the result when the game ended.
pub(crate) async fn apply(
    app: &Arc<App>,
    loaded: &mut Loaded,
    action: Action,
    now: i64,
) -> Result<Applied, ApiError> {
    let pinned = loaded.pinned;
    let reduced = std::panic::catch_unwind(AssertUnwindSafe(|| {
        engine::reduce(pinned, &loaded.state, &action)
    }))
    .map_err(|payload| ApiError::internal(panic_text(payload)))?;
    if let Some(error) = reduced.error {
        return Ok(Applied::Refused(error));
    }
    let state = reduced.state;
    // R4039000: who sent it, as the seats stood before it (R677), whom the feed never tells of it.
    let sender = loaded.account_at(action.player_id).to_string();
    // R4233000: the server's own `disconnectExpired` is not its sender's act (R270 names its nonce),
    // so a loss on time still tells its loser; a player's own concession or winning blow is told to
    // them already read.
    let own = !action.nonce.starts_with(SERVER_NONCE_PREFIX);
    let clock = daily_sync(&loaded.daily.clock, &state, now);
    let board = daily_board(&state);
    let ended_at = state.result.is_some().then_some(now);
    let row = MatchActionRow {
        match_id: loaded.match_row.id.clone(),
        seq: loaded.last_seq() + 1,
        action,
        at: now,
    };
    // The clock's own seq, not the log's: `load` reads the two in turn, so a move committed between
    // the reads leaves a log one row ahead of its clock, and a write after the log's seq would store
    // a clock (a deadline, a holder) older than the log. After the clock's, it clashes and reads again.
    let after = loaded.daily.seq;
    let mut tx = app.db.begin(None).await?;
    match tx.matches_append_actions(std::slice::from_ref(&row)).await {
        Ok(()) => {}
        // Dropped unwritten: the other request's row stands.
        Err(StoreError::SeqClash(_)) => return Ok(Applied::Clash),
        Err(error) => return Err(error.into()),
    }
    if !tx
        .daily_games_put(
            &row.match_id,
            after,
            &clock,
            &board,
            clock.next_deadline(),
            ended_at,
        )
        .await?
    {
        // Another request wrote after the seq we read, or ended the game, between our read and this
        // write. Its row stands in the log even when it carried our nonce (the append takes a repeated
        // nonce at the same seq as a retry), so ours, never logged, writes no clock: dropped unwritten.
        return Ok(Applied::Clash);
    }
    tx.commit().await?;

    loaded.log.push(row);
    loaded.state = state;
    loaded.daily.next_deadline_at = clock.next_deadline();
    loaded.daily.seq = after + 1;
    let before = std::mem::replace(&mut loaded.daily.clock, clock);
    loaded.daily.board = board;
    loaded.daily.ended_at = ended_at;
    checkpoint_if_due(app, loaded, now).await;
    if loaded.over() {
        record(app, loaded, now).await;
    }
    crate::notifications::notify(app, feed_notices(loaded, &before, &sender, own)).await;
    Ok(Applied::Done)
}

/// R4039000: what the feed tells of an action that left `loaded` (its clock `before` it, sent by
/// `sender`), each about the game, by the other player. A game that ended tells both players its
/// result as each sees it (`dailyEnded`; a void tells nobody, R679); one that goes on tells each
/// account the move newly passed to (a hold that began with this action) unless it sent it
/// (`dailyMove`). The server's own `disconnectExpired` ends the game, so a loss on time is told too.
/// R4233000: `own` is the sender's own act, whose ended row is written already read (still listed).
fn feed_notices(loaded: &Loaded, before: &DailyClock, sender: &str, own: bool) -> Vec<Notice> {
    let match_id = loaded.match_row.id.as_str();
    let (a, b) = (loaded.daily.players.0.as_str(), loaded.daily.players.1.as_str());
    let other = |me: &str| if me == a { b } else { a };
    if let Some(result) = loaded.daily.board.result {
        if result.reason == GameOverReason::Voided {
            return Vec::new();
        }
        return [a, b]
            .into_iter()
            .filter_map(|me| {
                let seat = seat_now(&loaded.daily, me)?;
                let notice = Notice::new(NotificationKind::DailyEnded, me, other(me), match_id).with(json!({
                    "outcome": outcome_of(result.winner, seat),
                    "reason": result.reason,
                }));
                Some(if own && me == sender {
                    notice.read()
                } else {
                    notice
                })
            })
            .collect();
    }
    loaded
        .daily
        .clock
        .holds
        .iter()
        // `daily_sync` keeps a hold only while its seat holds the same move (its key), so a hold the
        // clock before did not have under that key is one this action began.
        .filter(|hold| {
            before
                .hold_of(hold.player)
                .is_none_or(|held| held.key != hold.key)
        })
        .map(|hold| loaded.account_at(hold.player))
        .filter(|holder| *holder != sender)
        .map(|holder| Notice::new(NotificationKind::DailyMove, holder, other(holder), match_id))
        .collect()
}

/// A result as `seat` sees it: `"win"`, `"loss"` or `"draw"`.
fn outcome_of(winner: Winner, seat: PlayerId) -> &'static str {
    match winner {
        Winner::Draw => "draw",
        winner if winner == Winner::from(seat) => "win",
        _ => "loss",
    }
}

/// #1169: every `MATCH_CHECKPOINT_MIN_ACTIONS` rows, the state after the last, so a long game's next
/// request folds only the rows after it. A daily game keeps no play telemetry (§9.13), so its
/// recorder is an empty one. A failed write costs only the shortcut.
async fn checkpoint_if_due(app: &Arc<App>, loaded: &Loaded, now: i64) {
    let seq = loaded.last_seq();
    if loaded.over() || seq < MATCH_CHECKPOINT_MIN_ACTIONS || seq % MATCH_CHECKPOINT_MIN_ACTIONS != 0 {
        return;
    }
    let Some(last) = loaded.log.last() else {
        return;
    };
    let body = CheckpointBody {
        state: loaded.state.clone(),
        telemetry: Recorder::new(
            loaded.pinned,
            &loaded.match_row.id,
            loaded.match_row.created_at,
            &engine::snapshot(&loaded.state),
        ),
        bot_moved_turn: jackioh_engine::PerPlayer::new(None, None),
    };
    let written = match checkpoint::checkpoint_row(&loaded.match_row, seq, &last.action.nonce, &body, now) {
        Ok(row) => async {
            let mut tx = app.db.begin(None).await?;
            tx.matches_put_checkpoint(&row).await?;
            tx.commit().await
        }
        .await
        .map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };
    if let Err(message) = written {
        tracing::warn!(event = "daily.checkpoint.failed", matchId = %loaded.match_row.id, seq = seq, message = %message);
    }
}

/// §9.5: the result, written once (a repeat changes nothing), as a friend's room game's is: unranked,
/// paying and counting as one (§9.13). A write that fails leaves the match `live`, and the game's
/// next read writes it again.
async fn record(app: &Arc<App>, loaded: &Loaded, now: i64) {
    let snapshot = engine::snapshot(&loaded.state);
    let Some(outcome) = snapshot.result else {
        return;
    };
    if outcome.reason == GameOverReason::Voided {
        // R679, as the live actor's void: the game never happened, so the store forgets its match
        // row, its log and (0089's `on delete cascade`) its daily game, and no result is written.
        // A void that fails leaves the match `live`, and the game's next read voids it again.
        let input = VoidMatchInput {
            match_id: loaded.match_row.id.clone(),
            players: loaded.match_row.players.clone(),
            at: now,
        };
        if let Err(error) = void_match(app, input).await {
            tracing::error!(event = "daily.void.failed", matchId = %loaded.match_row.id, message = %error.message);
        }
        return;
    }
    // R677: credited by who plays each seat now, as the actor's `credited_seats` does.
    let mut seats = seats_of(&loaded.match_row);
    if snapshot.seats_swapped {
        std::mem::swap(&mut seats.0.profile_id, &mut seats.1.profile_id);
    }
    let input = RecordResultInput {
        match_id: loaded.match_row.id.clone(),
        seats,
        outcome,
        turns: snapshot.turn,
        at: now,
        last_boards: Some(engine::last_boards(&loaded.state)),
        final_hash: Some(engine::hash_state(&loaded.state)),
    };
    match record_result(app, input).await {
        Ok(_) => {
            tracing::info!(event = "daily.recorded", matchId = %loaded.match_row.id, reason = ?outcome.reason)
        }
        Err(error) => {
            tracing::error!(event = "daily.record.failed", matchId = %loaded.match_row.id, message = %error.message)
        }
    }
}

/// R3727000: a game whose move ran out by `now` is lost by the player who held it, logged as the
/// server's own `disconnectExpired`. Another request moving first is read again, up to
/// `DAILY_SETTLE_ATTEMPTS` times. A game over whose result never landed is recorded again.
pub(crate) async fn settle(app: &Arc<App>, mut loaded: Loaded, now: i64) -> Result<Loaded, ApiError> {
    for _ in 0..DAILY_SETTLE_ATTEMPTS {
        if loaded.over() {
            if loaded.match_row.status == MatchStatus::Live {
                record(app, &loaded, now).await;
            }
            return Ok(loaded);
        }
        let Some(player) = daily_expired(&loaded.daily.clock, now) else {
            return Ok(loaded);
        };
        let nonce = format!("{SERVER_NONCE_PREFIX}daily-{}", loaded.last_seq() + 1);
        let action = Action::new(ActionBody::DisconnectExpired { player }, player, nonce);
        match apply(app, &mut loaded, action, now).await? {
            Applied::Done => {
                tracing::info!(event = "daily.time_up", matchId = %loaded.match_row.id, player = %player);
                let (a, b) = (loaded.daily.players.0.clone(), loaded.daily.players.1.clone());
                push_social(app, &[&a, &b]).await;
                return Ok(loaded);
            }
            Applied::Clash => {
                let id = loaded.match_row.id.clone();
                loaded = load(app, &id)
                    .await?
                    .ok_or_else(|| not_found("no such daily game"))?;
            }
            Applied::Refused(error) => {
                return Err(ApiError::internal(format!(
                    "the engine refused a daily expiry: {error}"
                )));
            }
        }
    }
    Ok(loaded)
}

/// A daily game ready to start, written by nothing yet: its `matches` row and its `daily_games` row.
/// [`NewDaily::create`] writes both in the transaction that closes the challenge or claims the seek
/// starting it, so an accepted challenge never names a game that was never written.
pub(crate) struct NewDaily {
    match_row: MatchRow,
    daily: DailyGameRow,
}

impl NewDaily {
    pub(crate) fn match_id(&self) -> &str {
        &self.match_row.id
    }

    /// Writes both rows in `tx`; the caller commits them with whatever starts the game.
    pub(crate) async fn create(&self, tx: &mut Tx<'_>) -> Result<(), ApiError> {
        tx.matches_create(&self.match_row).await?;
        tx.daily_games_create(&self.daily).await?;
        Ok(())
    }

    /// The log line, once the caller's transaction committed.
    pub(crate) fn started(&self) {
        tracing::info!(
            event = "daily.started",
            matchId = %self.match_row.id,
            mode = ?self.daily.mode,
            perMoveDays = self.daily.per_move_days
        );
    }
}

/// Prepares a daily game between `seats` (seat order) in `mode` at `per_move_days`, starting at `now`,
/// with no actor, no seat of the ledger and no in-match flag (R3727004), writing nothing.
pub(crate) async fn prepare_daily(
    app: &Arc<App>,
    match_id: String,
    seed: String,
    seats: (crate::db::store::MatchSeat, crate::db::store::MatchSeat),
    mode: QueueMode,
    per_move_days: i32,
    now: i64,
) -> Result<NewDaily, ApiError> {
    let input = StartMatchInput {
        match_id: match_id.clone(),
        seed,
        // R3727005: the catalog this build plays, which the game keeps for its whole life.
        catalog_version: app.catalog.version.clone(),
        // §9.13: daily games are unranked.
        ranked: false,
        seats,
        mode: Some(mode),
        stake: None,
        win_rates: None,
    };
    let Prepared { match_row, state, .. } = prepare_match(app, &input, now).await?;
    let clock = daily_sync(&DailyClock::new(per_move_days), &state, now);
    let daily = DailyGameRow {
        match_id,
        players: match_row.players.clone(),
        mode,
        per_move_days,
        board: daily_board(&state),
        next_deadline_at: clock.next_deadline(),
        clock,
        seq: 0,
        created_at: now,
        ended_at: None,
    };
    Ok(NewDaily { match_row, daily })
}

/// The games waiting on `profile_id` (running, the move theirs), the least time left first, as
/// `(match id, since, deadline)`.
pub(crate) fn waiting_of(rows: &[DailyGameRow], profile_id: &str) -> Vec<(String, i64, i64)> {
    let mut waiting: Vec<(String, i64, i64)> = rows
        .iter()
        .filter(|row| row.ended_at.is_none())
        .filter_map(|row| {
            let hold = row.clock.hold_of(seat_now(row, profile_id)?)?;
            Some((row.match_id.clone(), hold.since, hold.deadline))
        })
        .collect();
    waiting.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
    waiting
}

/// How many daily games wait on `profile_id` at `now`: the `social` frame's `dailyWaiting` (§9.13).
/// A move already past its deadline is lost (R3727000), so it waits on nobody, sweep or no sweep.
pub(crate) async fn waiting_count(tx: &mut Tx<'_>, profile_id: &str, now: i64) -> Result<usize, StoreError> {
    let rows = tx.daily_games_of(profile_id, 0).await?;
    Ok(waiting_of(&rows, profile_id)
        .into_iter()
        .filter(|(_, _, deadline)| *deadline > now)
        .count())
}

/// How many daily games `profile_id` plays at `now`, once every one of them past its deadline is
/// settled (R3727000): the list's read, and what holds an account from deletion (R3727008).
pub(crate) async fn running_after_settling(
    app: &Arc<App>,
    profile_id: &str,
    now: i64,
) -> Result<usize, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let rows = tx.daily_games_of(profile_id, 0).await?;
    tx.commit().await?;
    let mut running = 0;
    for row in rows {
        if row.next_deadline_at.is_some_and(|at| at <= now)
            && let Some(loaded) = load(app, &row.match_id).await?
            && settle(app, loaded, now).await?.over()
        {
            continue;
        }
        running += 1;
    }
    Ok(running)
}

/// R3855000: R3477000's forecast of `seat`'s attacks, under the game's catalog (R767000, R3727005), kept to
/// the attacks `legal` lists, in its order: none while the other seat holds the move.
fn forecast_of(pinned: Pinned, state: &EngineState, seat: PlayerId, legal: &[ActionBody]) -> CombatForecast {
    let mut forecast = engine::combat_forecast(pinned, state, seat);
    forecast.attacks.retain(|entry| {
        legal.iter().any(|action| {
            matches!(action, ActionBody::Attack { attacker_id, target_id }
                if *attacker_id == entry.attacker_id && *target_id == entry.target_id)
        })
    });
    forecast.lethal = forecast.lethal && !forecast.attacks.is_empty();
    forecast
}

/// The caller's read of one game, as `GET /api/daily/games/:matchId` and every action answer it.
async fn game_json(app: &Arc<App>, loaded: &Loaded, me: &str, seat: PlayerId) -> Result<Value, ApiError> {
    let other = opponent_of(seat);
    let mut tx = app.db.begin(Some(me)).await?;
    let opponent = tx.profiles_get_by_id(loaded.account_at(other)).await?;
    let rows = tx.daily_games_of(me, 0).await?;
    // R4233000: reading the board marks this game's move and ended rows read, inside the read's own
    // transaction and only while unread, so a slow timer's re-read (R3840000) writes nothing. A
    // failure marks nothing and never fails the read (the act's answer included): it is logged, as
    // `notify`'s is. A mark that changed any row pushes the reader's social frame, so every one of
    // their tabs drops the dot.
    let marked = match tx
        .notifications_mark_subject(
            me,
            &[NotificationKind::DailyMove, NotificationKind::DailyEnded],
            &loaded.match_row.id,
            None,
            now_ms(),
        )
        .await
    {
        Ok(marked) => marked,
        Err(error) => {
            tracing::warn!(event = "notifications.mark_failed", matchId = %loaded.match_row.id, message = %error);
            0
        }
    };
    tx.commit().await?;
    if marked > 0 {
        push_social(app, &[me]).await;
    }
    // CLAUDE.md rule 7: the caller's view, legal actions and forecast only, under the game's catalog.
    let pinned = loaded.pinned;
    let view = engine::view_for(pinned, &loaded.state, seat);
    let legal: Vec<ActionBody> = daily_legal(
        &loaded.state,
        seat,
        engine::legal_actions(pinned, &loaded.state, seat),
    )
    .into_iter()
    // R1127: an emote rides the live socket's rate gate, which a daily game has none of.
    .filter(|action| !matches!(action, ActionBody::Emote { .. }))
    .collect();
    let forecast = forecast_of(pinned, &loaded.state, seat, &legal);
    let next_game = waiting_of(&rows, me)
        .into_iter()
        .map(|(id, _, _)| id)
        .find(|id| *id != loaded.match_row.id);
    Ok(json!({
        "matchId": loaded.match_row.id,
        "seat": seat,
        "view": to_json(&view)?,
        "legal": to_json(&legal)?,
        "forecast": to_json(&forecast)?,
        "seq": loaded.last_seq(),
        "holds": to_json(&loaded.daily.clock.holds)?,
        "hasMove": loaded.daily.clock.has_move(seat),
        "perMoveDays": loaded.daily.per_move_days,
        "mode": loaded.daily.mode,
        "catalogVersion": loaded.match_row.catalog_version,
        // R3727005: played on an older card version than the server's.
        "olderCatalog": loaded.match_row.catalog_version != app.catalog.version,
        "opponent": opponent.as_ref().map(PlayerRef::of),
        "over": loaded.over(),
        "lostOnTime": lost_on_time(&loaded.daily.board),
        "nextGame": next_game,
    }))
}

/// A game route's refusal: the error, and the game as it stands, so the client renders the truth.
fn refused_with(status: u16, code: &str, message: &str, game: Value) -> ApiResult {
    Ok(json_response(
        status,
        json!({ "error": { "code": code, "message": message }, "game": game }),
    ))
}

/// What a game route found for its caller.
enum Found {
    /// The game folded and settled at now, and the seat the caller plays.
    Played(Box<Loaded>, PlayerId),
    /// R3934000: a finished game whose log the purge took, never folded.
    MovesGone(Box<Stored>),
}

impl Found {
    /// The caller's read of what was found.
    async fn json(&self, app: &Arc<App>, me: &str) -> Result<Value, ApiError> {
        match self {
            Found::Played(loaded, seat) => game_json(app, loaded, me, *seat).await,
            Found::MovesGone(stored) => expired_json(app, stored, me).await,
        }
    }
}

/// R3934000: the read of a finished game whose log the purge took: its list row from the stored board
/// (the turn, both heroes' life, the result and when it ended) and `expired: true`, with no view, legal
/// actions or forecast (CLAUDE.md rule 7), since an empty log folds to a new game at the mulligan.
async fn expired_json(app: &Arc<App>, stored: &Stored, me: &str) -> Result<Value, ApiError> {
    let home = home_of(&stored.daily, me).ok_or_else(|| not_found("no such daily game"))?;
    let opponent_id = if home == PlayerId::P1 {
        &stored.daily.players.1
    } else {
        &stored.daily.players.0
    };
    let mut tx = app.db.begin(Some(me)).await?;
    let opponent = tx.profiles_get_by_id(opponent_id).await?;
    // R4233000: as `game_json`'s read marks, for a game whose moves are gone. A mark that changed
    // any row pushes the reader's social frame, as `game_json`'s does.
    let marked = match tx
        .notifications_mark_subject(
            me,
            &[NotificationKind::DailyMove, NotificationKind::DailyEnded],
            &stored.match_row.id,
            None,
            now_ms(),
        )
        .await
    {
        Ok(marked) => marked,
        Err(error) => {
            tracing::warn!(event = "notifications.mark_failed", matchId = %stored.match_row.id, message = %error);
            0
        }
    };
    tx.commit().await?;
    if marked > 0 {
        push_social(app, &[me]).await;
    }
    let mut game = summary_json(
        &stored.daily,
        Some(&stored.match_row),
        me,
        opponent.as_ref().map(PlayerRef::of),
    )
    .ok_or_else(|| not_found("no such daily game"))?;
    game["expired"] = json!(true);
    Ok(game)
}

/// The refusal of an action sent to a game that is over: `time_up` when the clock ended it.
fn over_refusal(board: &DailyBoard, game: Value) -> ApiResult {
    if lost_on_time(board) {
        refused_with(409, "time_up", "Time ran out on this game.", game)
    } else {
        refused_with(409, "over", "This game is over.", game)
    }
}

/// Loads the game `:matchId` for the caller, settled at now; a game the caller does not play is not
/// found (it says nothing of anyone else's games). R3934000: a finished game whose log the purge took
/// is not folded or settled.
async fn caller_game(app: &Arc<App>, req: &Req) -> Result<(String, Found), ApiError> {
    let me = caller_profile(req)?.id;
    let match_id = param(req, "matchId");
    let stored = read_stored(app, &match_id)
        .await?
        .ok_or_else(|| not_found("no such daily game"))?;
    if home_of(&stored.daily, &me).is_none() {
        return Err(not_found("no such daily game"));
    }
    if stored.moves_gone() {
        return Ok((me, Found::MovesGone(Box::new(stored))));
    }
    let loaded = fold_stored(app, stored).await?;
    let loaded = settle(app, loaded, app.daily.now()).await?;
    let seat = loaded
        .seat_of(&me)
        .ok_or_else(|| not_found("no such daily game"))?;
    Ok((me, Found::Played(Box::new(loaded), seat)))
}

/// `GET /api/daily/games/:matchId`.
pub async fn read(app: &Arc<App>, req: Req) -> ApiResult {
    let (me, found) = caller_game(app, &req).await?;
    Ok(ok(found.json(app, &me).await?))
}

/// `POST /api/daily/games/:matchId/actions` `{after, nonce, action}`.
pub async fn act(app: &Arc<App>, req: Req) -> ApiResult {
    let after = req
        .body
        .get("after")
        .and_then(Value::as_i64)
        .ok_or_else(|| bad_request("\"after\" must be the number of the log's last action"))?;
    let nonce = client_nonce(req.body.get("nonce"))
        .map_err(|error| bad_request(error.reason))?
        .to_string();
    let body = req
        .body
        .get("action")
        .and_then(Value::as_object)
        .ok_or_else(|| bad_request("\"action\" must be an action"))
        .and_then(|raw| parse_action_body(raw).map_err(|error| bad_request(error.reason)))?;
    let (me, found) = caller_game(app, &req).await?;
    let (mut loaded, seat) = match found {
        Found::Played(loaded, seat) => (*loaded, seat),
        // R3934000: never tried, never logged.
        Found::MovesGone(stored) => {
            let game = expired_json(app, &stored, &me).await?;
            return over_refusal(&stored.daily.board, game);
        }
    };

    // §9.3: a nonce already logged was applied; its retry answers the game as it stands.
    if loaded.log.iter().any(|row| row.action.nonce == nonce) {
        return Ok(ok(game_json(app, &loaded, &me, seat).await?));
    }
    if loaded.over() {
        let game = game_json(app, &loaded, &me, seat).await?;
        return over_refusal(&loaded.daily.board, game);
    }
    if after != loaded.last_seq() {
        let game = game_json(app, &loaded, &me, seat).await?;
        return refused_with(409, "stale", "The game moved on. Here it is now.", game);
    }
    // §9.13: a seat holding no move may only concede (R3727006's offerer among them), so nobody but
    // the holder appends to the log and makes the holder's next move stale.
    if !daily_allows(&loaded.state, seat, &body) {
        let game = game_json(app, &loaded, &me, seat).await?;
        return refused_with(422, "not_your_move", "Your opponent has the move.", game);
    }
    let now = app.daily.now();
    match apply(app, &mut loaded, Action::new(body, seat, nonce.clone()), now).await? {
        Applied::Done => {}
        Applied::Clash => {
            let (me, found) = caller_game(app, &req).await?;
            let game = found.json(app, &me).await?;
            // §9.3: a retry sent alongside its first try finds that one logged, as a later retry does.
            if matches!(&found, Found::Played(loaded, _) if loaded.log.iter().any(|row| row.action.nonce == nonce))
            {
                return Ok(ok(game));
            }
            return refused_with(409, "stale", "The game moved on. Here it is now.", game);
        }
        Applied::Refused(error) => {
            let game = game_json(app, &loaded, &me, seat).await?;
            return refused_with(422, "illegal", &error, game);
        }
    }
    // R3214000: both badges move; §9.13: whoever the move passed to is told.
    let (a, b) = (loaded.daily.players.0.clone(), loaded.daily.players.1.clone());
    push_social(app, &[&a, &b]).await;
    crate::daily::notify::after_move(app, &[&a, &b]);
    // R677, CLAUDE.md rule 7: the seat the caller plays after the action, which a Glitch's swap
    // changes, so the answer is never the view of the seat the opponent now plays.
    let seat = loaded.seat_of(&me).unwrap_or(seat);
    Ok(ok(game_json(app, &loaded, &me, seat).await?))
}

/// One game's row in the caller's list.
fn summary_json(
    row: &DailyGameRow,
    match_row: Option<&MatchRow>,
    me: &str,
    opponent: Option<PlayerRef>,
) -> Option<Value> {
    let seat = seat_now(row, me)?;
    let other = opponent_of(seat);
    let home = home_of(row, me)?;
    let portrait = |at: PlayerId| {
        match_row.and_then(|m| m.portraits.as_ref()).map(|portraits| {
            if at == PlayerId::P1 {
                portraits.0
            } else {
                portraits.1
            }
        })
    };
    let mine = row.clock.hold_of(seat).map(|hold| hold.deadline);
    let theirs = row.clock.hold_of(other).map(|hold| hold.deadline);
    let result = row
        .board
        .result
        .map(|result| json!({ "outcome": outcome_of(result.winner, seat), "reason": result.reason }));
    Some(json!({
        "matchId": row.match_id,
        "mode": row.mode,
        "perMoveDays": row.per_move_days,
        "seat": seat,
        "opponent": opponent,
        "portraits": { "me": portrait(home), "them": portrait(opponent_of(home)) },
        "turn": row.board.turn,
        "life": { "me": row.board.life[seat], "them": row.board.life[other] },
        "hasMove": mine.is_some(),
        "theirMove": theirs.is_some(),
        "deadline": mine.or(theirs),
        "over": row.ended_at.is_some(),
        "result": result,
        "endedAt": row.ended_at,
        "createdAt": row.created_at,
    }))
}

/// One invite as the list shows it.
fn invite_json(
    row: &DailyInviteRow,
    people: &indexmap::IndexMap<String, crate::db::store::Profile>,
) -> Value {
    let named = |id: &str| people.get(id).map(PlayerRef::of);
    json!({
        "id": row.id,
        "kind": row.kind,
        "from": named(&row.from_profile),
        "to": row.to_profile.as_deref().and_then(named),
        "mode": row.mode,
        "perMoveDays": row.per_move_days,
        "expiresAt": row.expires_at,
        "createdAt": row.created_at,
    })
}

/// `GET /api/daily`: `{games, challenges: {incoming, outgoing}, seeks, waiting}`, every running game
/// past its deadline settled first (R3727000).
pub async fn list(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let now = app.daily.now();
    let recent = i64::from(DAILY_RECENT_FINISHED);
    running_after_settling(app, &me, now).await?;

    let mut tx = app.db.begin(Some(&me)).await?;
    let rows = tx.daily_games_of(&me, recent).await?;
    let invites = tx.daily_invites_open_of(&me, now).await?;
    let mut matches = indexmap::IndexMap::new();
    for row in &rows {
        if let Some(match_row) = tx.matches_get(&row.match_id).await? {
            matches.insert(row.match_id.clone(), match_row);
        }
    }
    let mut named: Vec<String> = rows
        .iter()
        .flat_map(|row| [row.players.0.clone(), row.players.1.clone()])
        .collect();
    named.extend(
        invites
            .iter()
            .flat_map(|row| std::iter::once(row.from_profile.clone()).chain(row.to_profile.clone())),
    );
    let people = profiles_by_id(&mut tx, &named).await?;
    tx.commit().await?;

    let games: Vec<Value> = rows
        .iter()
        .filter_map(|row| {
            let home = home_of(row, &me)?;
            let opponent_id = if home == PlayerId::P1 {
                &row.players.1
            } else {
                &row.players.0
            };
            summary_json(
                row,
                matches.get(&row.match_id),
                &me,
                people.get(opponent_id).map(PlayerRef::of),
            )
        })
        .collect();
    let challenge = |row: &&DailyInviteRow| row.kind == DailyInviteKind::Challenge;
    let incoming: Vec<Value> = invites
        .iter()
        .filter(challenge)
        .filter(|row| row.to_profile.as_deref() == Some(me.as_str()))
        .map(|row| invite_json(row, &people))
        .collect();
    let outgoing: Vec<Value> = invites
        .iter()
        .filter(challenge)
        .filter(|row| row.from_profile == me)
        .map(|row| invite_json(row, &people))
        .collect();
    let seeks: Vec<Value> = invites
        .iter()
        .filter(|row| row.kind == DailyInviteKind::Seek && row.from_profile == me)
        .map(|row| invite_json(row, &people))
        .collect();
    Ok(ok(json!({
        "games": games,
        "challenges": { "incoming": incoming, "outgoing": outgoing },
        "seeks": seeks,
        "waiting": waiting_of(&rows, &me).len(),
    })))
}
