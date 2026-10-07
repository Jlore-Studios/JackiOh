//! The Conquest series as pure transitions over `SeriesRow` (SPEC §9.5, R330–R337, with R262–R264).
//! Port of `apps/server/src/api/series-rules.ts`.
//!
//! Conquest is the trio mode (the queue's `bo3`, called Best of 3 before R330): a player takes the
//! series by winning one game with EACH of their three decks. A deck that has won is locked for the
//! rest of the series; a deck that lost or drew may be picked again (R330). Before each game both
//! players pick at once, from their decks that have not won yet, and a pick is sealed: final once
//! made, and hidden from the other side until both are in (R331). A player with one deck left has it
//! picked for them (R332).
//!
//! Every rule of a series lives here and nowhere else: who may pick what and when, when a game
//! begins and which seat goes first, what a finished game does to the score, when the series is
//! over and how it scores for the rating, and what each player is allowed to see of it. `series.rs` reads
//! a row, applies one of these, and writes the result back by compare-and-set; `results.rs` does the
//! same inside the game result's transaction. Nothing here reads a clock, a store or an id minter:
//! time and the next match id arrive as parameters, so the same inputs always give the same row and
//! a CAS retry can re-apply a transition to the row it re-read.
//!
//! Every exported transition returns a NEW row whose `version` is exactly one more than its input's
//! and whose `updated_at` is `now`, because `series_update` writes only over the version before
//! (R263). A transition that composes others (a pick that completes both picks begins the game) is
//! still one write, so it still moves the version by one.
//!
//! A transition that does not apply returns `SeriesRefusal`, with a sentence a player can read;
//! `series.rs` turns it into an HTTP answer.
//!
//! THE ROW IS THE ONE R259 WROTE. Which decks have won is not stored: it is read off `games` (the
//! slot each side played and who won), and `SeriesSide.wins` stays the count of games won, which a
//! locked deck makes the same number (a deck wins at most once). So a series begun before Conquest
//! shipped reads as a Conquest series with the games it has played (R337), and no migration was
//! needed.

use indexmap::IndexSet;
use jackioh_engine::{GameOverReason, PlayerId, Winner};
use serde::{Deserialize, Serialize};

use crate::config::{SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED};
use crate::db::store::{
    FrozenDeck, FrozenTrio, MatchSeat, SeriesEnd, SeriesGame, SeriesRow, SeriesSeat, SeriesSide, SeriesStatus,
};

/// Unit conversion, not configuration: the pick clock is stated in seconds, rows in epoch ms.
const MS_PER_SECOND: i64 = 1000;

/// Games are numbered from one (`SeriesGame.game_no`).
const FIRST_GAME: i32 = 1;

const SEATS: [SeriesSeat; 2] = [SeriesSeat::P1, SeriesSeat::P2];

// ---------------------------------------------------------------------------
// The projection's shape
// ---------------------------------------------------------------------------

/// What `GET /api/series/:id` answers: `SeriesView` in `apps/web/src/net/api.ts`, field for field.
/// It is restated rather than imported because the web module reads `import.meta.env`, which the
/// server's program does not compile; `tests/api/series_rules.rs` pins the key set.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesView {
    pub id: String,
    pub status: SeriesStatus,
    /// The game being picked for or played, or the last one played once the series is over.
    pub game_no: i32,
    /// R330: game wins that take the series — one with each deck of the trio.
    pub wins_needed: i32,
    /// R334: the most games a series plays.
    pub max_games: i32,
    /// Epoch ms the pick clock runs out (R333), or null outside the pick phase.
    pub pick_deadline: Option<i64>,
    /// The server's clock when it answered, so a countdown does not depend on the device's.
    pub now: i64,
    /// The match to open while `status` is `playing`.
    pub current_match_id: Option<String>,
    /// R604: the queue paired this series, so it is rated; a room's is not. Always known — the
    /// player chose the mode — so it is projected from the start, not only in `result`.
    pub ranked: bool,
    pub you: SeriesViewYou,
    pub opponent: SeriesViewOpponent,
    pub games: Vec<SeriesViewGame>,
    /// Null until the series is over. `ranked`: the series moved your rank (R604); the rating it moved
    /// is never sent (R612), and the rank it left is `GET /api/ranked`'s.
    pub result: Option<SeriesViewResult>,
}

/// `SeriesView.you`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewYou {
    pub seat: SeriesSeat,
    pub wins: i32,
    pub trio_name: String,
    /// `won`: the deck has won a game in this series and is locked (R330). `games`: games it played.
    pub decks: Vec<SeriesViewDeck>,
    /// Your sealed pick for the next game, or null (R331).
    pub pick: Option<usize>,
    /// R332: your pick was made for you, because one deck is left that has not won.
    pub auto_pick: bool,
}

/// One of `SeriesView.you.decks`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewDeck {
    pub slot: usize,
    pub name: String,
    pub cards: Vec<String>,
    pub won: bool,
    pub games: usize,
}

/// `SeriesView.opponent`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewOpponent {
    pub wins: i32,
    /// Only which slots have won: names, cards and picks stay hidden (R336).
    pub decks: Vec<SeriesViewOpponentDeck>,
    /// Whether their pick is in; never what (R331).
    pub picked: bool,
}

/// One of `SeriesView.opponent.decks`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewOpponentDeck {
    pub slot: usize,
    pub won: bool,
}

/// `"win" | "loss" | "draw"`: one game's result as the viewer reads it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}

/// `"win" | "loss" | "draw" | "abandoned"`: the series' outcome as the viewer reads it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum SeriesOutcome {
    Win,
    Loss,
    Draw,
    Abandoned,
}

/// One of `SeriesView.games`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewGame {
    pub game_no: i32,
    pub match_id: String,
    pub your_slot: usize,
    pub opponent_slot: usize,
    pub you_went_first: bool,
    pub result: Option<GameOutcome>,
    pub reason: Option<GameOverReason>,
}

/// `SeriesView.result`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesViewResult {
    pub outcome: SeriesOutcome,
    pub end_reason: SeriesEnd,
    pub ranked: bool,
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Why a transition did not apply. The first five reach a player (`series.rs` maps them to 409 or
/// 400); the rest are the server calling a transition out of turn — a CAS retry that re-read a row
/// another writer had already moved, which the caller treats as "nothing to do".
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SeriesRefusalReason {
    /// A game is being played, so there is nothing to pick and no series to forfeit.
    NotPicking,
    /// The series has ended.
    Over,
    /// The pick clock has run out; the sweeper is about to settle the picks (R333).
    PickClosed,
    /// The slot is not one of the trio's, or not a whole number.
    SlotOutOfRange,
    /// That deck has already won a game in this series, so it is locked (R330).
    SlotWon,
    /// This player's pick for the game is already in, and a pick is final (R331).
    PickSealed,
    /// The pick names a game other than the one being picked for: a late duplicate (R331).
    StalePick,
    /// The pick clock has not run out yet.
    PickOpen,
    /// There is no game in play to end or to seat.
    NotPlaying,
    /// A game cannot begin before both players have picked.
    PicksMissing,
}

impl SeriesRefusalReason {
    /// The literal TS wrote (`"not_picking"`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            SeriesRefusalReason::NotPicking => "not_picking",
            SeriesRefusalReason::Over => "over",
            SeriesRefusalReason::PickClosed => "pick_closed",
            SeriesRefusalReason::SlotOutOfRange => "slot_out_of_range",
            SeriesRefusalReason::SlotWon => "slot_won",
            SeriesRefusalReason::PickSealed => "pick_sealed",
            SeriesRefusalReason::StalePick => "stale_pick",
            SeriesRefusalReason::PickOpen => "pick_open",
            SeriesRefusalReason::NotPlaying => "not_playing",
            SeriesRefusalReason::PicksMissing => "picks_missing",
        }
    }
}

/// TS `class SeriesRefusal extends Error`: the reason and the sentence a player can read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesRefusal {
    pub reason: SeriesRefusalReason,
    pub message: String,
}

impl SeriesRefusal {
    pub fn new(reason: SeriesRefusalReason, message: impl Into<String>) -> SeriesRefusal {
        SeriesRefusal { reason, message: message.into() }
    }
}

impl std::fmt::Display for SeriesRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SeriesRefusal {}

/// TS `refuse(reason, message): never`, as the `Err` a transition returns.
fn refuse<T>(reason: SeriesRefusalReason, message: impl Into<String>) -> Result<T, SeriesRefusal> {
    Err(SeriesRefusal::new(reason, message))
}

const OVER_MESSAGE: &str = "This series is over.";

// ---------------------------------------------------------------------------
// Small reads
// ---------------------------------------------------------------------------

pub fn seat_index(seat: SeriesSeat) -> usize {
    if seat == SeriesSeat::P1 { 0 } else { 1 }
}

pub fn other_seat(seat: SeriesSeat) -> SeriesSeat {
    if seat == SeriesSeat::P1 { SeriesSeat::P2 } else { SeriesSeat::P1 }
}

/// A series seat as the `Winner` it is when it wins (`SeriesSeat | "draw"` is `Winner`'s literals).
fn winner_of_seat(seat: SeriesSeat) -> Winner {
    if seat == SeriesSeat::P1 { Winner::P1 } else { Winner::P2 }
}

/// The caller's seat in this series, or null when they are not one of its two players.
pub fn seat_of(series: &SeriesRow, profile_id: &str) -> Option<SeriesSeat> {
    if series.sides.0.profile_id == profile_id {
        return Some(SEATS[0]);
    }
    if series.sides.1.profile_id == profile_id {
        return Some(SEATS[1]);
    }
    None
}

/// `series.sides[index]` (the sides are a pair, SURFACE §4.3).
fn side_at(series: &SeriesRow, index: usize) -> &SeriesSide {
    if index == 0 { &series.sides.0 } else { &series.sides.1 }
}

fn side_at_mut(series: &mut SeriesRow, index: usize) -> &mut SeriesSide {
    if index == 0 { &mut series.sides.0 } else { &mut series.sides.1 }
}

fn side_of(series: &SeriesRow, seat: SeriesSeat) -> &SeriesSide {
    side_at(series, seat_index(seat))
}

fn side_of_mut(series: &mut SeriesRow, seat: SeriesSeat) -> &mut SeriesSide {
    side_at_mut(series, seat_index(seat))
}

/// `game.slots[index]` (the slots are a pair, SURFACE §4.3).
fn slot_at(game: &SeriesGame, index: usize) -> usize {
    (if index == 0 { game.slots.0 } else { game.slots.1 }) as usize
}

/// A frozen trio's three decks, in slot order.
fn trio_decks(trio: &FrozenTrio) -> [&FrozenDeck; 3] {
    [&trio.decks.0, &trio.decks.1, &trio.decks.2]
}

/// `trio.decks[slot]`, or `None` past the end.
fn trio_deck(trio: &FrozenTrio, slot: usize) -> Option<&FrozenDeck> {
    trio_decks(trio).get(slot).copied()
}

/// How many decks this seat's trio holds: three, by L1, frozen at queue.
fn slot_count(series: &SeriesRow, seat: SeriesSeat) -> usize {
    trio_decks(&side_of(series, seat).trio).len()
}

/// R330: the trio slots whose deck has won a game for this seat — each one locked.
pub fn won_slots(seat: SeriesSeat, games: &[SeriesGame]) -> IndexSet<usize> {
    let index = seat_index(seat);
    let won = Some(winner_of_seat(seat));
    games.iter().filter(|game| game.winner == won).map(|game| slot_at(game, index)).collect()
}

/// The slots this seat may still pick: every slot of its trio whose deck has not won (R330).
/// TS's optional `games` defaulted to `series.games`; pass `&series.games` for that.
pub fn unwon_slots(series: &SeriesRow, seat: SeriesSeat, games: &[SeriesGame]) -> Vec<usize> {
    let won = won_slots(seat, games);
    let mut open = Vec::new();
    for slot in 0..slot_count(series, seat) {
        if !won.contains(&slot) {
            open.push(slot);
        }
    }
    open
}

/// R333: the deck a player who did not pick is given — their first deck in trio order that has not
/// won — or null when every deck has won (the series is then already over).
/// TS's optional `games` defaulted to `series.games`; pass `&series.games` for that.
pub fn first_unwon(series: &SeriesRow, seat: SeriesSeat, games: &[SeriesGame]) -> Option<usize> {
    unwon_slots(series, seat, games).first().copied()
}

pub fn both_picked(series: &SeriesRow) -> bool {
    series.sides.0.pick.is_some() && series.sides.1.pick.is_some()
}

/// R335: series `p1` goes first in odd games, `p2` in even ones, drawn games counted.
fn first_seat_of(game_no: i32) -> SeriesSeat {
    if game_no % 2 == 1 { SeriesSeat::P1 } else { SeriesSeat::P2 }
}

/// The game in play: the last recorded one, while it has no result.
fn game_in_play(series: &SeriesRow) -> Option<&SeriesGame> {
    let last = series.games.last()?;
    if series.status == SeriesStatus::Playing && last.winner.is_none() { Some(last) } else { None }
}

/// R332: a seat with exactly one deck left that has not won has its pick made for it.
fn automatic_pick(series: &SeriesRow, seat: SeriesSeat) -> Option<usize> {
    let open = unwon_slots(series, seat, &series.games);
    if open.len() == 1 { open.first().copied() } else { None }
}

/// R262: series `p1`'s score once the series is over — 1 for a series win, 0 for a loss, 0.5
/// for a series draw — or null while it is not over and when it was abandoned (unrated, R333).
pub fn series_score(series: &SeriesRow) -> Option<f64> {
    if series.status != SeriesStatus::Over {
        return None;
    }
    match series.winner {
        None => None,
        Some(Winner::Draw) => Some(0.5),
        Some(Winner::P1) => Some(1.0),
        Some(Winner::P2) => Some(0.0),
    }
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

/// TS `structuredClone(series)`.
fn copy(series: &SeriesRow) -> SeriesRow {
    series.clone()
}

/// One write: the version moves by exactly one from the row the transition was applied to.
fn stamp(mut next: SeriesRow, from: &SeriesRow, now: i64) -> SeriesRow {
    next.version = from.version + 1;
    next.updated_at = now;
    next
}

fn pick_deadline_from(now: i64) -> i64 {
    now + SERIES_PICK_SECONDS * MS_PER_SECOND
}

/// Records the next game from both picks and starts playing it (R331, R335). Mutates `series`.
/// Refused when a pick is missing or names a deck that has won: both are guarded before a pick is
/// stored, so this is the last word, not the first.
fn begin(series: &mut SeriesRow) -> Result<(), SeriesRefusal> {
    let (Some(a_pick), Some(b_pick)) = (series.sides.0.pick, series.sides.1.pick) else {
        return refuse(SeriesRefusalReason::PicksMissing, "A game cannot start before both players have picked.");
    };
    for seat in SEATS {
        let pick = side_of(series, seat).pick;
        if let Some(pick) = pick
            && won_slots(seat, &series.games).contains(&(pick as usize)) {
                return refuse(
                    SeriesRefusalReason::SlotWon,
                    "That deck has already won a game in this series, so it is locked.",
                );
            }
    }
    let game_no = series.games.len() as i32 + FIRST_GAME;
    let match_id = series.next_match_id.clone();
    series.games.push(SeriesGame {
        game_no: i64::from(game_no),
        match_id,
        slots: (a_pick, b_pick),
        first: first_seat_of(game_no),
        winner: None,
        reason: None,
    });
    series.sides.0.pick = None;
    series.sides.1.pick = None;
    series.status = SeriesStatus::Playing;
    series.pick_deadline = None;
    Ok(())
}

/// Opens the pick phase for the next game under a freshly reserved match id (R263), with its clock
/// running from `now` (R333). A seat with one deck left gets it picked for it (R332); when both have,
/// there is nothing to choose and the game begins at once. Mutates `series`.
fn open_picks(series: &mut SeriesRow, match_id: &str, now: i64) -> Result<(), SeriesRefusal> {
    series.status = SeriesStatus::Picking;
    series.next_match_id = match_id.to_string();
    series.pick_deadline = Some(pick_deadline_from(now));
    for seat in SEATS {
        let pick = automatic_pick(series, seat);
        side_of_mut(series, seat).pick = pick.map(|slot| slot as i64);
    }
    if both_picked(series) {
        begin(series)?;
    }
    Ok(())
}

/// Ends the series. `winner` null is an abandoned series (R333, R263). Mutates `series`.
fn end(series: &mut SeriesRow, winner: Option<Winner>, reason: SeriesEnd, now: i64) {
    series.status = SeriesStatus::Over;
    series.winner = winner;
    series.end_reason = Some(reason);
    series.pick_deadline = None;
    series.ended_at = Some(now);
    series.sides.0.pick = None;
    series.sides.1.pick = None;
}

fn assert_picking(series: &SeriesRow, not_picking_message: &str) -> Result<(), SeriesRefusal> {
    if series.status == SeriesStatus::Over {
        return refuse(SeriesRefusalReason::Over, OVER_MESSAGE);
    }
    if series.status != SeriesStatus::Picking {
        return refuse(SeriesRefusalReason::NotPicking, not_picking_message);
    }
    Ok(())
}

/// One side of `NewSeriesInput.sides`.
#[derive(serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewSeriesSide {
    pub profile_id: String,
    pub trio: FrozenTrio,
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewSeriesInput {
    pub series_id: String,
    /// The match id `tickets_claim_pair` or `rooms_claim` reserved: game 1's (R263).
    pub first_match_id: String,
    /// Index 0 is series p1: the older ticket, or the room's host (R335).
    pub sides: (NewSeriesSide, NewSeriesSide),
    /// Each game's seed is `{seed_base}:{game_no}` (R335).
    pub seed_base: String,
    pub catalog_version: String,
    /// R604: true when the queue paired it; a room's series is unranked.
    pub ranked: bool,
}

/// R331, R333, R263: a series in game 1's pick phase, its clock running from `now`.
pub fn new_series(input: &NewSeriesInput, now: i64) -> SeriesRow {
    let side = |entry: &NewSeriesSide| SeriesSide {
        profile_id: entry.profile_id.clone(),
        trio: entry.trio.clone(),
        wins: 0,
        pick: None,
    };
    SeriesRow {
        id: input.series_id.clone(),
        sides: (side(&input.sides.0), side(&input.sides.1)),
        catalog_version: input.catalog_version.clone(),
        ranked: Some(input.ranked),
        seed_base: input.seed_base.clone(),
        status: SeriesStatus::Picking,
        games: Vec::new(),
        next_match_id: input.first_match_id.clone(),
        pick_deadline: Some(pick_deadline_from(now)),
        winner: None,
        end_reason: None,
        rating_before: None,
        rating_after: None,
        created_at: now,
        updated_at: now,
        ended_at: None,
        version: 1,
    }
}

/// R331: `seat` picks trio slot `slot` for the next game. The pick is sealed: once it is in it is
/// final (`pick_sealed`, checked first, which `series.rs` answers as a success when the same slot is
/// sent again, so a retried request is harmless), and the other side learns only that it is in. A
/// pick that names its game (`game_no`) is refused as `stale_pick` when that is not the game being
/// picked for, so a late duplicate of game n's pick can never become game n + 1's. The pick that
/// completes both begins the game at once. Refused once the pick clock has run out (R333), for a slot
/// the trio does not have, and for a deck that has already won in this series (R330).
pub fn pick_deck(
    series: &SeriesRow,
    seat: SeriesSeat,
    slot: i32,
    now: i64,
    game_no: Option<i32>,
) -> Result<SeriesRow, SeriesRefusal> {
    assert_picking(series, "A game of this series is being played; pick your next deck when it ends.")?;
    let picking_for = series.games.len() as i32 + FIRST_GAME;
    if let Some(game_no) = game_no
        && game_no != picking_for {
            return refuse(
                SeriesRefusalReason::StalePick,
                format!("That pick was for game {game_no}; this is game {picking_for}'s pick."),
            );
        }
    let side = side_of(series, seat);
    if side.pick.is_some() {
        return refuse(SeriesRefusalReason::PickSealed, "Your pick for this game is already in, and it is final.");
    }
    if let Some(deadline) = series.pick_deadline
        && now >= deadline {
            return refuse(SeriesRefusalReason::PickClosed, "The pick clock has run out for this game.");
        }
    if slot < 0 || slot as usize >= trio_decks(&side.trio).len() {
        return refuse(SeriesRefusalReason::SlotOutOfRange, "Pick one of the three decks in your trio.");
    }
    let slot = slot as usize;
    if won_slots(seat, &series.games).contains(&slot) {
        return refuse(
            SeriesRefusalReason::SlotWon,
            "That deck has already won a game in this series, so it is locked; pick another.",
        );
    }

    let mut next = copy(series);
    side_of_mut(&mut next, seat).pick = Some(slot as i64);
    if both_picked(&next) {
        begin(&mut next)?;
    }
    Ok(stamp(next, series, now))
}

/// R331: both have picked, so the game begins: recorded, seated and `playing`.
pub fn begin_game(series: &SeriesRow, now: i64) -> Result<SeriesRow, SeriesRefusal> {
    assert_picking(series, "This game has already begun.")?;
    let mut next = copy(series);
    begin(&mut next)?;
    Ok(stamp(next, series, now))
}

/// R330, R334: the game in play ended. `winner` is the series seat that won it — the deck it played
/// is locked from now on — or `Draw`, which counts for neither side and locks nothing. A side at
/// `SERIES_WINS_NEEDED` wins has won with every deck and takes the series (`decided`). After
/// `SERIES_MAX_GAMES` games without that, more wins takes it and equal wins is a series draw
/// (`exhausted`). Otherwise the next pick phase opens under `new_match_id` (R263), with the picks made
/// for a side that has one deck left (R332) — and when both have, the game begins at once.
pub fn game_ended(
    series: &SeriesRow,
    winner: Winner,
    reason: GameOverReason,
    now: i64,
    new_match_id: &str,
) -> Result<SeriesRow, SeriesRefusal> {
    if series.status == SeriesStatus::Over {
        return refuse(SeriesRefusalReason::Over, OVER_MESSAGE);
    }
    if game_in_play(series).is_none() {
        return refuse(SeriesRefusalReason::NotPlaying, "No game of this series is being played.");
    }

    let mut next = copy(series);
    let Some(game) = next.games.last_mut() else {
        return refuse(SeriesRefusalReason::NotPlaying, "No game of this series is being played.");
    };
    game.winner = Some(winner);
    game.reason = Some(reason);
    if let Some(player) = winner.player() {
        let seat = if player == PlayerId::P1 { SeriesSeat::P1 } else { SeriesSeat::P2 };
        side_of_mut(&mut next, seat).wins += 1;
    }

    let wins_needed = SERIES_WINS_NEEDED as i64;
    let p1_wins = next.sides.0.wins;
    let p2_wins = next.sides.1.wins;
    if p1_wins >= wins_needed || p2_wins >= wins_needed {
        let taker = if p1_wins >= wins_needed { Winner::P1 } else { Winner::P2 };
        end(&mut next, Some(taker), SeriesEnd::Decided, now);
    } else if next.games.len() >= SERIES_MAX_GAMES as usize {
        let taker = if p1_wins > p2_wins {
            Winner::P1
        } else if p2_wins > p1_wins {
            Winner::P2
        } else {
            Winner::Draw
        };
        end(&mut next, Some(taker), SeriesEnd::Exhausted, now);
    } else {
        open_picks(&mut next, new_match_id, now)?;
    }
    Ok(stamp(next, series, now))
}

/// R333: the pick clock ran out. A player who has not picked gets their first deck in trio order that
/// has not won, and the game begins; if neither has a pick in, the series is abandoned — no winner,
/// unrated. A pick made for a player (R332) is a pick. Refused before the deadline, so a stale sweep
/// cannot close a pick phase that opened after it read.
pub fn timeout_picks(series: &SeriesRow, now: i64) -> Result<SeriesRow, SeriesRefusal> {
    assert_picking(series, "A game of this series is being played.")?;
    match series.pick_deadline {
        Some(deadline) if now >= deadline => {}
        _ => return refuse(SeriesRefusalReason::PickOpen, "The pick clock is still running."),
    }

    let mut next = copy(series);
    if next.sides.0.pick.is_none() && next.sides.1.pick.is_none() {
        end(&mut next, None, SeriesEnd::Abandoned, now);
        return Ok(stamp(next, series, now));
    }
    for seat in SEATS {
        if side_of(&next, seat).pick.is_some() {
            continue;
        }
        let Some(slot) = first_unwon(&next, seat, &next.games) else {
            return refuse(SeriesRefusalReason::PicksMissing, "Every deck of this trio has already won.");
        };
        side_of_mut(&mut next, seat).pick = Some(slot as i64);
    }
    begin(&mut next)?;
    Ok(stamp(next, series, now))
}

/// R263: a game whose picks are in but whose match could not be started for
/// `SERIES_START_GIVE_UP_SECONDS` ends the series abandoned — no winner, unrated (R262) — so no player
/// is held in a series that cannot go on. The unplayed game is taken off the record: it never
/// happened. The sweeper decides the time has come; this only makes the change.
pub fn abandon_unstarted(series: &SeriesRow, now: i64) -> Result<SeriesRow, SeriesRefusal> {
    if series.status == SeriesStatus::Over {
        return refuse(SeriesRefusalReason::Over, OVER_MESSAGE);
    }
    if series.status != SeriesStatus::Playing {
        return refuse(SeriesRefusalReason::NotPlaying, "No game of this series is waiting to start.");
    }
    let waiting = match series.games.last() {
        Some(current) => current.match_id == series.next_match_id && current.winner.is_none(),
        None => false,
    };
    if !waiting {
        return refuse(SeriesRefusalReason::NotPlaying, "No game of this series is waiting to start.");
    }
    let mut next = copy(series);
    next.games.pop();
    end(&mut next, None, SeriesEnd::Abandoned, now);
    Ok(stamp(next, series, now))
}

/// R334: `seat` leaves the series between games and the other side wins it (`forfeit`). During a
/// game the way out is to concede that game, so a forfeit is refused while one is being played.
pub fn forfeit_series(series: &SeriesRow, seat: SeriesSeat, now: i64) -> Result<SeriesRow, SeriesRefusal> {
    assert_picking(
        series,
        "A game is being played: concede the game instead. You can forfeit the series between games.",
    )?;
    let mut next = copy(series);
    end(&mut next, Some(winner_of_seat(other_seat(seat))), SeriesEnd::Forfeit, now);
    Ok(stamp(next, series, now))
}

/// TS `{ before: [number, number]; after: [number, number] }`: R262's one rating move of a series,
/// series p1's rating then p2's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RatingMove {
    pub before: (f64, f64),
    pub after: (f64, f64),
}

/// R262: records the series' one rating move on a row that has just ended: series p1's rating, then
/// p2's, before and after (`series.rs` rates it, R603). Not a transition — it rides on the ending's
/// own write, so the version is left alone. An abandoned or unranked series keeps both fields null.
pub fn rate_series(series: &SeriesRow, rating_move: Option<RatingMove>) -> SeriesRow {
    let mut next = copy(series);
    let rating_move = match rating_move {
        Some(rating_move) if series_score(series).is_some() => rating_move,
        _ => {
            next.rating_before = None;
            next.rating_after = None;
            return next;
        }
    };
    next.rating_before = Some((rating_move.before.0, rating_move.before.1));
    next.rating_after = Some((rating_move.after.0, rating_move.after.1));
    next
}

// ---------------------------------------------------------------------------
// The game in play
// ---------------------------------------------------------------------------

/// TS `{ seats: [MatchSeat, MatchSeat]; seed: string }`: `game_seats`'s answer.
#[derive(Clone, Debug, PartialEq)]
pub struct GameSeats {
    pub seats: (MatchSeat, MatchSeat),
    pub seed: String,
}

/// R335: the two seats and the seed of the game in play. The match's `p1` is the side that goes
/// first in this game (series `p1` in odd games, `p2` in even ones), each playing the deck in the
/// trio slot they picked, and the seed is `{seed_base}:{game_no}`.
pub fn game_seats(series: &SeriesRow) -> Result<GameSeats, SeriesRefusal> {
    let Some(game) = game_in_play(series) else {
        return refuse(SeriesRefusalReason::NotPlaying, "No game of this series is being played.");
    };
    let first_index = seat_index(game.first);
    let second_index = seat_index(other_seat(game.first));
    let seat_for = |index: usize, player: PlayerId| -> MatchSeat {
        let side = side_at(series, index);
        let slot = slot_at(game, index);
        let Some(deck) = trio_deck(&side.trio, slot) else {
            panic!("series {}: slot {} is not a deck", series.id, slot);
        };
        // R642: a Conquest game plays the picked deck's portrait, frozen into the trio with it.
        MatchSeat {
            profile_id: side.profile_id.clone(),
            player,
            deck: deck.cards.clone(),
            portrait: deck.portrait.clone().flatten(),
        }
    };
    Ok(GameSeats {
        seats: (seat_for(first_index, PlayerId::P1), seat_for(second_index, PlayerId::P2)),
        seed: format!("{}:{}", series.seed_base, game.game_no),
    })
}

/// R331: whether `slot` is the pick `seat` already made — for game `game_no` when the request named
/// one, else for the game now picked for or in play — which is what a retried pick request finds
/// after its first attempt landed. `series.rs` answers such a retry as the success it was, rather
/// than as a refusal the player did nothing to earn.
pub fn already_picked(series: &SeriesRow, seat: SeriesSeat, slot: i32, game_no: Option<i32>) -> bool {
    let wanted = if slot < 0 { None } else { Some(slot as usize) };
    if let Some(game_no) = game_no {
        if let Some(begun) = series.games.iter().find(|game| game.game_no == i64::from(game_no)) {
            return wanted == Some(slot_at(begun, seat_index(seat)));
        }
        return series.status == SeriesStatus::Picking
            && game_no == series.games.len() as i32 + FIRST_GAME
            && wanted.is_some()
            && side_of(series, seat).pick.map(|pick| pick as usize) == wanted;
    }
    if series.status == SeriesStatus::Picking {
        return wanted.is_some() && side_of(series, seat).pick.map(|pick| pick as usize) == wanted;
    }
    match game_in_play(series) {
        Some(game) => wanted == Some(slot_at(game, seat_index(seat))),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// The projection
// ---------------------------------------------------------------------------

/// What one player may see of a series (R336): all of their own trio, and of the opponent's only
/// which slots have won and whether a pick is in — never its slot before both have picked (by then
/// the game has begun and it is history), never a deck name, never a card. Null for a profile that is
/// not one of the two players.
pub fn project_series(series: &SeriesRow, viewer_profile_id: &str, now: i64) -> Option<SeriesView> {
    let seat = seat_of(series, viewer_profile_id)?;
    let mine = seat_index(seat);
    let theirs = seat_index(other_seat(seat));
    let you = side_at(series, mine);
    let opponent = side_at(series, theirs);
    let my_won = won_slots(seat, &series.games);
    let their_won = won_slots(other_seat(seat), &series.games);
    let picking = series.status == SeriesStatus::Picking;

    let outcome_of = |winner: Winner| -> GameOutcome {
        if winner == Winner::Draw {
            GameOutcome::Draw
        } else if winner == winner_of_seat(seat) {
            GameOutcome::Win
        } else {
            GameOutcome::Loss
        }
    };

    let game_no = if picking {
        series.games.len() as i32 + FIRST_GAME
    } else {
        (series.games.len() as i32).max(FIRST_GAME)
    };
    let ranked = series.ranked.unwrap_or(false);

    Some(SeriesView {
        id: series.id.clone(),
        status: series.status,
        game_no,
        wins_needed: SERIES_WINS_NEEDED,
        max_games: SERIES_MAX_GAMES,
        pick_deadline: if picking { series.pick_deadline } else { None },
        now,
        current_match_id: if series.status == SeriesStatus::Playing {
            Some(series.next_match_id.clone())
        } else {
            None
        },
        ranked,
        you: SeriesViewYou {
            seat,
            wins: you.wins as i32,
            trio_name: you.trio.name.clone(),
            decks: trio_decks(&you.trio)
                .iter()
                .enumerate()
                .map(|(slot, deck)| SeriesViewDeck {
                    slot,
                    name: deck.name.clone(),
                    cards: deck.cards.clone(),
                    won: my_won.contains(&slot),
                    games: series.games.iter().filter(|game| slot_at(game, mine) == slot).count(),
                })
                .collect(),
            pick: if picking { you.pick.map(|pick| pick as usize) } else { None },
            auto_pick: picking && you.pick.is_some() && automatic_pick(series, seat) == you.pick.map(|pick| pick as usize),
        },
        opponent: SeriesViewOpponent {
            wins: opponent.wins as i32,
            decks: trio_decks(&opponent.trio)
                .iter()
                .enumerate()
                .map(|(slot, _deck)| SeriesViewOpponentDeck { slot, won: their_won.contains(&slot) })
                .collect(),
            picked: picking && opponent.pick.is_some(),
        },
        games: series
            .games
            .iter()
            .map(|game| SeriesViewGame {
                game_no: game.game_no as i32,
                match_id: game.match_id.clone(),
                your_slot: slot_at(game, mine),
                opponent_slot: slot_at(game, theirs),
                you_went_first: game.first == seat,
                result: game.winner.map(outcome_of),
                reason: game.reason,
            })
            .collect(),
        result: match (series.status, series.end_reason) {
            (SeriesStatus::Over, Some(end_reason)) => Some(SeriesViewResult {
                outcome: if end_reason == SeriesEnd::Abandoned {
                    SeriesOutcome::Abandoned
                } else {
                    match series.winner {
                        None => SeriesOutcome::Abandoned,
                        Some(winner) => match outcome_of(winner) {
                            GameOutcome::Win => SeriesOutcome::Win,
                            GameOutcome::Loss => SeriesOutcome::Loss,
                            GameOutcome::Draw => SeriesOutcome::Draw,
                        },
                    }
                },
                end_reason,
                ranked: ranked && series.rating_after.is_some(),
            }),
            _ => None,
        },
    })
}
