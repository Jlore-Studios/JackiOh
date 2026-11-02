//! Play telemetry (SPEC §9.11, R1442): move durations, emotes, and endings for ladder bots (#636).
//!
//! The actor feeds `Recorder` during the match (`on_action`, `on_pushed`, `on_emote`).
//! On commit, `finish` collects rows and `record` writes them in one transaction; failures are
//! logged and dropped (like R376 game records). Keyed so repeat writes are no-ops.
//!
//! Think time runs from view push to action arrival in server time. Server actions and
//! `setAutoEndTurn` (R345) make no row. Seat names the account's initial seat (R677).
//!
//! `replay` rebuilds recorder from the log alone. `legal_actions` and evaluations read true
//! state on the server without reaching sockets.

use serde::{Deserialize, Serialize};

use jackioh_ai::{AI_EVAL, NextSwing, evaluate};
use jackioh_engine::{
    Action, ActionBody, ActionType, EmoteId, GameEventType, PLAYER_IDS, PerPlayer, PlayerId,
};

use crate::actor::contracts::one_tx;
use crate::actor::engine::{self, EngineState, MatchSnapshot, Pinned};
use crate::actor::match_actor::last_boards_of;
use crate::actor::protocol::SERVER_NONCE_PREFIX;
use crate::app::App;
use crate::db::store::{
    ActionTimingRow, EmoteEventRow, MatchActionRow, MatchRow, MatchSignalRow, Pilot, PlayTelemetry,
};
use crate::ranked::ladder::RankTier;

/// R1442: who chose the moves of a match the actor runs: both seats are people.
pub const LIVE_PILOTS: PerPlayer<Pilot> = PerPlayer {
    p1: Pilot::Human,
    p2: Pilot::Human,
};

/// Whether `seat` owes the match its next move: the open prompt's holder, else a seat still owing
/// its mulligan (R265), else the active seat. Nobody once the match has a result.
fn owes(snapshot: &MatchSnapshot, seat: PlayerId) -> bool {
    if snapshot.result.is_some() {
        return false;
    }
    if let Some(holder) = snapshot.pending_for {
        return holder == seat;
    }
    if !snapshot.mulligan_owed.is_empty() {
        return snapshot.mulligan_owed.contains(&seat);
    }
    snapshot.active == seat
}

/// The account (by the seat it began in) that plays engine seat `player` in `snapshot` (R677: a
/// swap is its own inverse).
fn home_of(snapshot: &MatchSnapshot, player: PlayerId) -> PlayerId {
    if snapshot.seats_swapped {
        player.opponent()
    } else {
        player
    }
}

fn empty_signals(match_id: &str, seat: PlayerId) -> MatchSignalRow {
    MatchSignalRow {
        match_id: match_id.to_string(),
        seat,
        conceded_turn: None,
        concede_eval_deficit: None,
        draw_offers: 0,
        draw_accepted: false,
        rematch_offered: false,
        rematch_accepted: false,
        timeouts: 0,
        pilot: Pilot::Human,
    }
}

/// One match's telemetry as it is gathered (see the module header). #1169: a checkpoint carries it
/// as JSON, all but the catalog, which the rebuild that reads it back pins again (`replay_from`).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Recorder {
    /// R388, R767000: the catalog the legal count and the concession's evaluation read.
    #[serde(skip)]
    pinned: Pinned,
    match_id: String,
    /// By engine seat: when the seat began to owe the move it owes now (the push that made it so),
    /// or `None` while it owes nothing.
    owed_since: PerPlayer<Option<i64>>,
    pushed_at: i64,
    /// The seat whose prompt was open at the last push, so a prompt newly put to a seat that already
    /// owed a move starts its think time afresh.
    prompted: Option<PlayerId>,
    pending: Option<(PlayerId, bool, Option<GameEventType>)>,
    last_event: Option<(GameEventType, i64)>,
    acted_turn: PerPlayer<Option<i32>>,
    last_emote_at: PerPlayer<Option<i64>>,
    timings: Vec<ActionTimingRow>,
    emotes: Vec<EmoteEventRow>,
    signals: PerPlayer<MatchSignalRow>,
}

impl Recorder {
    /// A match that opened at `opened_at` (epoch ms, its creation) in the state `opening` shows.
    pub fn new(pinned: Pinned, match_id: &str, opened_at: i64, opening: &MatchSnapshot) -> Recorder {
        Recorder {
            pinned,
            match_id: match_id.to_string(),
            owed_since: PerPlayer::new(
                owes(opening, PlayerId::P1).then_some(opened_at),
                owes(opening, PlayerId::P2).then_some(opened_at),
            ),
            pushed_at: opened_at,
            prompted: opening.pending_for,
            pending: None,
            last_event: None,
            acted_turn: PerPlayer::new(None, None),
            last_emote_at: PerPlayer::new(None, None),
            timings: Vec::new(),
            emotes: Vec::new(),
            signals: PerPlayer::new(
                empty_signals(match_id, PlayerId::P1),
                empty_signals(match_id, PlayerId::P2),
            ),
        }
    }

    /// An action the engine accepted and the log holds at `seq`, arriving at `at` (epoch ms) on the
    /// state `before` it. `clock_left_ms` is the acting seat's clock as it arrived; `last_event` the
    /// last event it produced.
    pub fn on_action(
        &mut self,
        before: &EngineState,
        action: &Action,
        seq: i64,
        at: i64,
        clock_left_ms: Option<i64>,
        last_event: Option<GameEventType>,
    ) {
        let pinned = self.pinned;
        let snapshot = engine::snapshot(before);
        let player = action.player_id;
        let home = home_of(&snapshot, player);
        let server = action.nonce.starts_with(SERVER_NONCE_PREFIX);
        let decision = !server && action.action_type() != ActionType::SetAutoEndTurn;

        let signals = &mut self.signals[home];
        match &action.body {
            ActionBody::Timeout if server => signals.timeouts += 1,
            ActionBody::OfferDraw => signals.draw_offers += 1,
            ActionBody::AnswerDraw { accept: true } => signals.draw_accepted = true,
            ActionBody::Concede => {
                let next = if snapshot.active == player {
                    NextSwing::Seat
                } else {
                    NextSwing::Enemy
                };
                signals.conceded_turn = Some(snapshot.turn);
                signals.concede_eval_deficit = Some(-pinned.run(|| evaluate(before, player, next, &AI_EVAL)));
            }
            _ => {}
        }

        if decision {
            let legal = engine::legal_actions(pinned, before, player).len();
            self.timings.push(ActionTimingRow {
                match_id: self.match_id.clone(),
                seat: home,
                seq,
                action_kind: action.action_type(),
                legal_count: i32::try_from(legal).unwrap_or(i32::MAX),
                turn: snapshot.turn,
                think_ms: (at - self.owed_since[player].unwrap_or(self.pushed_at)).max(0),
                clock_left_ms,
                first_in_turn: self.acted_turn[player] != Some(snapshot.turn),
                rank_bucket: None,
                pilot: Pilot::Human,
            });
            self.acted_turn[player] = Some(snapshot.turn);
        }
        self.pending = Some((player, decision, last_event));
    }

    /// The views of the state `after` the last action were pushed at `pushed_at`. A seat that owes
    /// nothing now has no think time running; one that has just come to owe a move (or a prompt), or
    /// that has just decided one and owes the next, starts it now; any other keeps its own.
    pub fn on_pushed(&mut self, after: &MatchSnapshot, pushed_at: i64) {
        let pending = self.pending.take();
        for seat in PLAYER_IDS {
            let decided = pending.is_some_and(|(player, decision, _)| decision && player == seat);
            let prompted = after.pending_for == Some(seat) && self.prompted != Some(seat);
            self.owed_since[seat] = if !owes(after, seat) {
                None
            } else if self.owed_since[seat].is_none() || decided || prompted {
                Some(pushed_at)
            } else {
                self.owed_since[seat]
            };
        }
        self.pushed_at = pushed_at;
        self.prompted = after.pending_for;
        if let Some((_, _, Some(event))) = pending {
            self.last_event = Some((event, pushed_at));
        }
    }

    /// The account that began in `home` emoted `emote` at `at`, in turn `turn`.
    pub fn on_emote(&mut self, home: PlayerId, emote: EmoteId, turn: i32, at: i64) {
        let own = self.last_emote_at[home];
        let reply_to_opponent_ms = self.last_emote_at[home.opponent()]
            .filter(|theirs| own.is_none_or(|own| *theirs > own))
            .map(|theirs| at - theirs);
        self.emotes.push(EmoteEventRow {
            match_id: self.match_id.clone(),
            seat: home,
            ordinal: i32::try_from(self.emotes.len()).unwrap_or(i32::MAX),
            emote_id: emote,
            turn,
            trigger_event: self.last_event.map(|(event, _)| event),
            ms_since_trigger: self.last_event.map(|(_, event_at)| at - event_at),
            reply_to_opponent_ms,
            pilot: Pilot::Human,
        });
        self.last_emote_at[home] = Some(at);
    }

    /// The match's rows, each stamped with its seat's pilot and ladder tier (by home seat), both
    /// seats' signals always among them.
    pub fn finish(&self, pilots: &PerPlayer<Pilot>, tiers: Option<&PerPlayer<RankTier>>) -> PlayTelemetry {
        PlayTelemetry {
            action_timings: self
                .timings
                .iter()
                .map(|row| ActionTimingRow {
                    rank_bucket: tiers.map(|tiers| tiers[row.seat]),
                    pilot: pilots[row.seat],
                    ..row.clone()
                })
                .collect(),
            emote_events: self
                .emotes
                .iter()
                .map(|row| EmoteEventRow {
                    pilot: pilots[row.seat],
                    ..row.clone()
                })
                .collect(),
            match_signals: PLAYER_IDS
                .iter()
                .map(|seat| MatchSignalRow {
                    pilot: pilots[*seat],
                    ..self.signals[*seat].clone()
                })
                .collect(),
        }
    }
}

/// Builds the recorder from the action log alone (R258). Skipped rows mirror `fold`.
/// Returns the recorder and refused row count.
pub fn replay(
    pinned: Pinned,
    match_row: &MatchRow,
    log: &[MatchActionRow],
    dealt: Option<Vec<PlayerId>>,
) -> (Recorder, usize) {
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
    let recorder = Recorder::new(
        pinned,
        &match_row.id,
        match_row.created_at,
        &engine::snapshot(&state),
    );
    replay_from(pinned, recorder, state, log)
}

/// Resumes a checkpoint's recorder (#1169) over remaining log rows.
pub fn replay_from(
    pinned: Pinned,
    mut recorder: Recorder,
    mut state: EngineState,
    log: &[MatchActionRow],
) -> (Recorder, usize) {
    recorder.pinned = pinned;
    let mut refused = 0;
    for row in log {
        let result = engine::reduce(pinned, &state, &row.action);
        if result.error.is_some() {
            refused += 1;
            continue;
        }
        recorder.on_action(
            &state,
            &row.action,
            row.seq,
            row.at,
            None,
            result.events.last().map(|event| event.event_type()),
        );
        state = result.state;
        recorder.on_pushed(&engine::snapshot(&state), row.at);
    }
    (recorder, refused)
}

/// R1442: writes one match's telemetry; a failure is logged and dropped.
pub async fn record(app: &App, telemetry: &PlayTelemetry) {
    let written = one_tx!(app.db, |t| t.play_telemetry_insert(telemetry).await?);
    if let Err(error) = written {
        let match_id = telemetry
            .match_signals
            .first()
            .map(|row| row.match_id.as_str())
            .unwrap_or_default();
        tracing::warn!(event = "telemetry.write_failed", matchId = %match_id, message = %error);
    }
}

/// R672, R1442: the account that began `match_id` in `seat` offered a rematch, and `made` when that
/// made one; a failure is logged and dropped.
pub async fn note_rematch(app: &App, match_id: &str, seat: PlayerId, made: bool) {
    let written = one_tx!(app.db, |t| t
        .play_telemetry_note_rematch(match_id, seat, made)
        .await?);
    if let Err(error) = written {
        tracing::warn!(event = "telemetry.rematch_failed", matchId = %match_id, message = %error);
    }
}

/// R1442: each player's ladder tier now (seat order, `MatchRow.players`), or `None` when the read
/// fails, which is logged.
pub async fn ladder_tiers_or_none(app: &App, players: &(String, String)) -> Option<PerPlayer<RankTier>> {
    match crate::api::ranked::ladder_tiers(app, players).await {
        Ok((p1, p2)) => Some(PerPlayer::new(p1, p2)),
        Err(error) => {
            tracing::warn!(event = "telemetry.tiers_failed", message = %error);
            None
        }
    }
}
