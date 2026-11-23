//! R4552000–R4552002 (SPEC §9.11): how far a card's plays move its player's chance of winning.
//!
//! A record written since R4552000 carries, for each seat, one row per turn it took (`SwingTurn`): the
//! turn counter, the AI's evaluation of the seat less its opponent's at the turn's start (the true, whole
//! state: nothing is hidden from a statistic), and the cards acting on each side's field then. The AI's
//! fold writes them (`crates/ai/src/swing_turns.rs`), since the engine cannot call the AI.
//!
//! The rows are read through a calibrated win probability (`win_probability`, its weights fit by `cargo
//! jackioh stats calibrate`): a turn's change, `Δ`, is the seat's win probability at the start of its
//! next turn less at the start of this one (the last turn ends at the result), so it holds the
//! opponent's reply and one end-of-turn and start-of-turn cycle, and a card is never credited for being
//! played from ahead. Each `Δ` is folded, as whole thousandths, into the totals of the cards played in
//! the turn, the cards acting for the seat, and (as the opponent's) the cards acting against it, beside
//! an ordinary turn's under the records key (`GameTotal.swing`). Every tally is an integer sum, so
//! records folded in any order make the same totals, and every figure is read off finished totals in a
//! fixed order, so they are the same bit for bit.
//!
//! The figures (`swing_report`): a card's play term `b` and board term `a` (Board swing) are each the
//! mean of what an ordinary turn (`α`, per own turn and seat) leaves of its turns, shrunk as R2777000
//! shrinks a lift: a diagonal ridge, the prior's spread read off the field as R3022000 reads it.
//! Swing is `b` plus the board terms of the turns it and the cards it made then act. Decisiveness is
//! the root mean square of the part of its play turns its mean does not explain plus `b`, shrunk toward
//! the typical card's by its own sampling noise. Only Decisiveness is shown (R4552001, R4552003).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::{
    CardStatsFilter, CardTotal, GameRecord, GameSummary, Outcome, RecordsKey, StatsTotals, TotalsKey,
    made_by_the_rules, outcome_for, pilot_counts, random_effects, sources_of, table, to_fixed,
};
use crate::config::{
    ADJUSTED_FIELD_MIN_CARDS, DECISIVENESS_PRIOR_CENTER, DECISIVENESS_PRIOR_SPREAD, SWING_EVAL_SCALE,
    SWING_FIELD_MIN_ROWS, SWING_PERMILLE, SWING_PRIOR_BOARD_SPREAD, SWING_PRIOR_PLAY_SPREAD,
    SWING_TURN_BUCKETS, WIN_PROBABILITY_EVAL, WIN_PROBABILITY_FIRST, WIN_PROBABILITY_INTERCEPT,
    WIN_PROBABILITY_TURN,
};
use crate::wire::catalog_types::{PLAYER_IDS, PerPlayer, PlayerId};

// ---------------------------------------------------------------------------
// The record (R4552000)
// ---------------------------------------------------------------------------

/// R4552000: one turn of a seat's, read at its start: the turn counter (§2.5), the AI's evaluation of the
/// seat less its opponent's in thousandths (`SWING_EVAL_SCALE`), and the cards acting on each field.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwingTurn {
    pub turn: i32,
    /// The raw evaluation difference, never a probability, so a refit of the weights rewrites no record.
    pub eval: i64,
    /// The seat's cards acting on its field: Units, Field Spells (a Law among them) and face-up backrow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acting: Vec<ActingCard>,
    /// The opponent's acting cards at the same moment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facing: Vec<ActingCard>,
}

/// R4552000, R4552002: one acting card: a dealt card by its catalog id, or a Created one (R943) with the
/// card whose play or trigger made it, when an effect put it onto the field or cast it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum ActingCard {
    Dealt(String),
    Created(CreatedActing),
}

/// R4552002: a Created card acting. `maker` is absent when a hand held it first (it was played: The
/// Coin, Rosemary Triscuit), since its own play is then the seat's decision.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatedActing {
    pub card: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker: Option<String>,
}

impl ActingCard {
    /// Its catalog id.
    pub fn card(&self) -> &str {
        match self {
            ActingCard::Dealt(card) => card,
            ActingCard::Created(made) => &made.card,
        }
    }
}

// ---------------------------------------------------------------------------
// The tallies (R4552001)
// ---------------------------------------------------------------------------

/// R4552001: the turns of one kind a card took part in, as whole sums of their `q`, each turn's `Δ` in
/// thousandths (`SWING_PERMILLE`): per bucket (`swing_bucket`) the turns and `Σq`, then `Σq²`, `Σq³` and
/// `Σq⁴` over all of them, and `Σ` over games of the turns that game gave squared (the games cluster
/// the turns, `n_eff = n² / cluster`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwingSums {
    pub buckets: IndexMap<i32, [i64; 2]>,
    pub moments: [i64; 3],
    pub cluster: i64,
}

impl SwingSums {
    /// No turns.
    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    /// The turns it counts.
    pub fn rows(&self) -> i64 {
        self.buckets.values().map(|[rows, _]| rows).sum()
    }

    fn count(&mut self, bucket: i32, q: i64) {
        let held = self.buckets.entry(bucket).or_default();
        held[0] += 1;
        held[1] += q;
        let square = q.saturating_mul(q);
        self.moments[0] = self.moments[0].saturating_add(square);
        self.moments[1] = self.moments[1].saturating_add(square.saturating_mul(q));
        self.moments[2] = self.moments[2].saturating_add(square.saturating_mul(square));
    }

    fn add(&mut self, other: &SwingSums) {
        for (bucket, [rows, sum]) in &other.buckets {
            let held = self.buckets.entry(*bucket).or_default();
            held[0] += rows;
            held[1] += sum;
        }
        for (mine, theirs) in self.moments.iter_mut().zip(other.moments) {
            *mine = mine.saturating_add(theirs);
        }
        self.cluster = self.cluster.saturating_add(other.cluster);
    }

    /// One game's sums closed: its turns squared are its cluster.
    fn close_game(&mut self) {
        let rows = self.rows();
        self.cluster = rows.saturating_mul(rows);
    }
}

/// R4552001, R4552002: one card's turns under one key: the turns it was played on (each once however
/// many copies), the turns it acted for its seat, the turns it acted against the other seat (that seat's
/// `Δ`), and per Created card it made the turns that card then acted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwingTally {
    #[serde(default, skip_serializing_if = "SwingSums::is_empty")]
    pub played: SwingSums,
    #[serde(default, skip_serializing_if = "SwingSums::is_empty")]
    pub acting: SwingSums,
    #[serde(default, skip_serializing_if = "SwingSums::is_empty")]
    pub facing: SwingSums,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub made_acting: IndexMap<String, i64>,
    /// R4552002: of a Created card's acting turns, those of a copy a hand held (and so was played, not
    /// made onto the field): what its own Swing counts per play. A deck card's acting turns all follow
    /// its plays, so it keeps none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub held_acting: i64,
}

fn is_zero(turns: &i64) -> bool {
    *turns == 0
}

impl SwingTally {
    /// No turns of any kind.
    pub fn is_empty(&self) -> bool {
        self.played.is_empty()
            && self.acting.is_empty()
            && self.facing.is_empty()
            && self.made_acting.is_empty()
            && self.held_acting == 0
    }

    /// Adds `other`'s sums into these.
    pub fn add(&mut self, other: &SwingTally) {
        self.played.add(&other.played);
        self.acting.add(&other.acting);
        self.facing.add(&other.facing);
        for (card, turns) in &other.made_acting {
            *self.made_acting.entry(card.clone()).or_default() += turns;
        }
        self.held_acting += other.held_acting;
    }

    fn close_game(&mut self) {
        self.played.close_game();
        self.acting.close_game();
        self.facing.close_game();
    }
}

/// R4552001: every turn of a records key, per bucket: the turns, `Σq` and `Σq²`. An ordinary turn's gain
/// (`α`) and the turns' spread about it are read off these.
pub type SwingRows = IndexMap<i32, [i64; 3]>;

/// Adds `other`'s turns into `into`, bucket by bucket.
pub fn add_swing_rows(into: &mut SwingRows, other: &SwingRows) {
    for (bucket, sums) in other {
        let held = into.entry(*bucket).or_default();
        for (mine, theirs) in held.iter_mut().zip(sums) {
            *mine = mine.saturating_add(*theirs);
        }
    }
}

// ---------------------------------------------------------------------------
// The fold (R4552000, R4552001, R4552002)
// ---------------------------------------------------------------------------

/// R4552000: the seat's chance of winning at the start of a turn, `σ(β₀ + β₁·eval + β₂·turn +
/// β₃·[went first])`, `eval` a row's thousandths read back to the AI's units.
pub fn win_probability(eval: i64, turn: i32, first: bool) -> f64 {
    let z = WIN_PROBABILITY_INTERCEPT
        + WIN_PROBABILITY_EVAL * (eval as f64 / SWING_EVAL_SCALE)
        + WIN_PROBABILITY_TURN * f64::from(turn)
        + WIN_PROBABILITY_FIRST * f64::from(u8::from(first));
    1.0 / (1.0 + (-z).exp())
}

/// R4552001: the bucket of a seat's `index`th turn (from 0): its own turn number, capped at
/// `SWING_TURN_BUCKETS`, positive for the seat that went first and negative for the other.
pub fn swing_bucket(index: usize, first: bool) -> i32 {
    let own = i32::try_from(index + 1)
        .unwrap_or(i32::MAX)
        .min(SWING_TURN_BUCKETS);
    if first { own } else { -own }
}

/// R4552001: a turn's change in win probability as whole thousandths.
fn quantum(delta: f64) -> i64 {
    (delta * SWING_PERMILLE).round() as i64
}

/// R4552000, R4552001: what the seat's result is worth at its last turn's end, and what the win
/// probability is fit to (`cargo jackioh stats calibrate`): a win 1, a loss 0, a draw ½ (R1843004's
/// reading: a fled game is the fleer's win).
pub fn seat_result(game: &GameSummary, seat: PlayerId) -> f64 {
    match outcome_for(game, seat) {
        Outcome::Win => 1.0,
        Outcome::Draw => 0.5,
        Outcome::Loss => 0.0,
    }
}

/// One seat's tallies of one game, before they are closed and added: its deck cards' and its Created
/// cards' apart, as `CardStats.swing` and `created_swing` keep them.
#[derive(Default)]
struct SeatSwing {
    deck: IndexMap<String, SwingTally>,
    created: IndexMap<String, SwingTally>,
}

impl SeatSwing {
    fn deck(&mut self, card: &str) -> &mut SwingTally {
        self.deck.entry(card.to_string()).or_default()
    }

    fn created(&mut self, card: &str) -> &mut SwingTally {
        self.created.entry(card.to_string()).or_default()
    }
}

impl StatsTotals {
    /// R4552000–R4552002: folds a record's turn rows, when it carries them: each turn's `q` under its
    /// records key's `GameTotal.swing`, and into each card's `swing` (dealt) or `created_swing` (Created)
    /// of the seat whose turn it was:
    ///  - `played`: the cards played in the turn, from the hand or (R4279000) cast or summoned by an
    ///    effect, each once; a Created card played from the hand (`createdPlayed`) as its own;
    ///  - `acting`: the seat's acting cards at the turn's start. A Created card an effect put there
    ///    adds the turn to its maker's `made_acting`, and has no play of its own;
    ///  - `facing`, under the opponent's key: the opponent's acting cards.
    ///
    /// A play goes to the seat's last turn begun by the play's turn, so a cast on the opponent's turn
    /// counts with the turn whose reply it was.
    pub(super) fn fold_swing(&mut self, record: &GameRecord) {
        let seats = &record.game.seats;
        let mut game: PerPlayer<SeatSwing> = PerPlayer::new(SeatSwing::default(), SeatSwing::default());
        let mut rows_total = SwingRows::new();
        for seat in PLAYER_IDS {
            let Some(rows) = seats[seat].swing_turns.as_deref().filter(|rows| !rows.is_empty()) else {
                continue;
            };
            let summary = &seats[seat];
            let opponent = seat.opponent();
            let first = record.game.first == seat;
            let end = seat_result(&record.game, seat);
            let deck: IndexSet<&str> = summary.deck.iter().map(String::as_str).collect();
            let opponent_deck: IndexSet<&str> = seats[opponent].deck.iter().map(String::as_str).collect();

            let row_of = |turn: i32| rows.iter().rposition(|row| row.turn <= turn);
            let mut plays: Vec<IndexSet<&str>> = vec![IndexSet::new(); rows.len()];
            let mut made_plays: Vec<IndexSet<&str>> = vec![IndexSet::new(); rows.len()];
            if let Some(turns) = summary
                .played_turns
                .as_ref()
                .filter(|turns| turns.len() == summary.played.len())
            {
                for (card, turn) in summary.played.iter().zip(turns) {
                    if let Some(row) = row_of(*turn) {
                        plays[row].insert(card.as_str());
                    }
                }
            }
            for cast in summary.cast_or_summoned.iter().flatten() {
                if let Some(row) = row_of(cast.turn) {
                    plays[row].insert(cast.card.as_str());
                }
            }
            for used in summary.created_played.iter().flatten() {
                if let Some(row) = row_of(used.turn) {
                    made_plays[row].insert(used.card.as_str());
                }
            }

            for (index, row) in rows.iter().enumerate() {
                let now = win_probability(row.eval, row.turn, first);
                let next = rows
                    .get(index + 1)
                    .map_or(end, |next| win_probability(next.eval, next.turn, first));
                let q = quantum(next - now);
                let bucket = swing_bucket(index, first);
                let total = rows_total.entry(bucket).or_default();
                total[0] += 1;
                total[1] += q;
                total[2] = total[2].saturating_add(q.saturating_mul(q));

                let mine = &mut game[seat];
                for &card in plays[index].iter().filter(|card| deck.contains(*card)) {
                    mine.deck(card).played.count(bucket, q);
                }
                for &card in &made_plays[index] {
                    mine.created(card).played.count(bucket, q);
                }
                let mut seen: IndexSet<(bool, &str)> = IndexSet::new();
                let mut made: IndexSet<(&str, &str)> = IndexSet::new();
                let mut held: IndexSet<&str> = IndexSet::new();
                for acting in &row.acting {
                    match acting {
                        ActingCard::Dealt(card) => {
                            if deck.contains(card.as_str()) && seen.insert((false, card.as_str())) {
                                mine.deck(card).acting.count(bucket, q);
                            }
                        }
                        ActingCard::Created(created) => {
                            if seen.insert((true, created.card.as_str())) {
                                mine.created(&created.card).acting.count(bucket, q);
                            }
                            match created.maker.as_deref() {
                                Some(maker) => {
                                    if made.insert((maker, created.card.as_str())) {
                                        let tally = if deck.contains(maker) {
                                            mine.deck(maker)
                                        } else {
                                            mine.created(maker)
                                        };
                                        *tally.made_acting.entry(created.card.clone()).or_default() += 1;
                                    }
                                }
                                None => {
                                    if held.insert(created.card.as_str()) {
                                        mine.created(&created.card).held_acting += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                let theirs = &mut game[opponent];
                let mut seen: IndexSet<(bool, &str)> = IndexSet::new();
                for facing in &row.facing {
                    match facing {
                        ActingCard::Dealt(card) => {
                            if opponent_deck.contains(card.as_str()) && seen.insert((false, card.as_str())) {
                                theirs.deck(card).facing.count(bucket, q);
                            }
                        }
                        ActingCard::Created(created) => {
                            if seen.insert((true, created.card.as_str())) {
                                theirs.created(&created.card).facing.count(bucket, q);
                            }
                        }
                    }
                }
            }
        }
        if rows_total.is_empty() {
            return;
        }
        let records_key = RecordsKey {
            patch: record.patch.clone(),
            source: record.source,
            mode: record.mode,
            pilots: (record.pilots.p1, record.pilots.p2),
        };
        add_swing_rows(
            &mut self.game_totals.entry(records_key).or_default().swing,
            &rows_total,
        );
        for seat in PLAYER_IDS {
            let key = TotalsKey {
                patch: record.patch.clone(),
                source: record.source,
                mode: record.mode,
                pilot: record.pilots[seat],
            };
            let SeatSwing { deck, created } = std::mem::take(&mut game[seat]);
            if deck.is_empty() && created.is_empty() {
                continue;
            }
            let cards = self.cards.entry(key).or_default();
            for (card, mut tally) in deck {
                tally.close_game();
                cards
                    .entry(card.clone())
                    .or_insert_with(|| CardTotal::new(&card))
                    .stats
                    .swing
                    .add(&tally);
            }
            for (card, mut tally) in created {
                tally.close_game();
                cards
                    .entry(card.clone())
                    .or_insert_with(|| CardTotal::new(&card))
                    .stats
                    .created_swing
                    .add(&tally);
            }
        }
    }
}

/// R4552001: every turn of the totals a filter reads, selected as `game_figures_of` selects games: its
/// sources, mode and patch, and a key one of whose seats' pilots it counts.
pub fn swing_rows_of(totals: &StatsTotals, filter: &CardStatsFilter) -> SwingRows {
    let sources = sources_of(filter.source);
    let mut rows = SwingRows::new();
    for (key, total) in &totals.game_totals {
        if sources.contains(&key.source)
            && filter.mode.is_none_or(|mode| key.mode == mode)
            && filter.patch.as_ref().is_none_or(|patch| key.patch == *patch)
            && (pilot_counts(filter.pilot, key.pilots.0) || pilot_counts(filter.pilot, key.pilots.1))
        {
            add_swing_rows(&mut rows, &total.swing);
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// The figures (R4552001, R4552002)
// ---------------------------------------------------------------------------

/// R4552001: what an ordinary turn gains, `α` per bucket, and `σ²`, the turns' variance about it pooled
/// over the buckets: the noise of one turn, which every card's term is shrunk by.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SwingBaseline {
    pub alpha: IndexMap<i32, f64>,
    pub row_variance: f64,
    pub rows: i64,
}

/// The buckets of a map in their order, so a sum over them is the same whatever order they were met in.
fn sorted<T>(map: &IndexMap<i32, T>) -> Vec<(i32, &T)> {
    let mut entries: Vec<(i32, &T)> = map.iter().map(|(key, value)| (*key, value)).collect();
    entries.sort_unstable_by_key(|(key, _)| *key);
    entries
}

/// R4552001: the baseline of some turns (`swing_rows_of`, `windowed_swing_rows`).
pub fn swing_baseline(rows: &SwingRows) -> SwingBaseline {
    let mut alpha = IndexMap::new();
    let (mut within, mut count) = (0.0, 0i64);
    for (bucket, [rows, sum, squares]) in sorted(rows) {
        if *rows <= 0 {
            continue;
        }
        let n = *rows as f64;
        let mean = *sum as f64 / n / SWING_PERMILLE;
        within += *squares as f64 / (SWING_PERMILLE * SWING_PERMILLE) - n * mean * mean;
        count += rows;
        alpha.insert(bucket, mean);
    }
    let freedom = count - alpha.len() as i64;
    SwingBaseline {
        row_variance: if freedom > 0 {
            (within / freedom as f64).max(0.0)
        } else {
            0.0
        },
        alpha,
        rows: count,
    }
}

/// One kind of a card's turns read against the baseline: `n`, the mean of `y = Δ − α`, the mean of `y²`,
/// the raw moments of `Δ` (`E[Δ]` to `E[Δ⁴]`) and the effective turns once the games cluster them.
#[derive(Clone, Copy, Debug)]
struct Read {
    rows: i64,
    sum: f64,
    squares: f64,
    raw: [f64; 4],
    effective: f64,
}

impl Read {
    fn of(sums: &SwingSums, baseline: &SwingBaseline) -> Option<Read> {
        let rows = sums.rows();
        if rows <= 0 {
            return None;
        }
        let n = rows as f64;
        let scale = |power: i32| SWING_PERMILLE.powi(power);
        let mut sum = 0.0;
        let mut total = 0.0;
        let mut squares = sums.moments[0] as f64 / scale(2);
        for (bucket, [count, bucket_sum]) in sorted(&sums.buckets) {
            let alpha = baseline.alpha.get(&bucket).copied().unwrap_or(0.0);
            let t = *bucket_sum as f64 / SWING_PERMILLE;
            let count = *count as f64;
            total += t;
            sum += t - count * alpha;
            squares += -2.0 * alpha * t + count * alpha * alpha;
        }
        let raw = [
            total / n,
            sums.moments[0] as f64 / scale(2) / n,
            sums.moments[1] as f64 / scale(3) / n,
            sums.moments[2] as f64 / scale(4) / n,
        ];
        let effective = if sums.cluster > 0 {
            (n * n / sums.cluster as f64).min(n)
        } else {
            n
        };
        Some(Read {
            rows,
            sum,
            squares,
            raw,
            effective,
        })
    }

    fn mean(&self) -> f64 {
        self.sum / self.rows as f64
    }

    /// The variance of `y` over its turns.
    fn variance(&self) -> f64 {
        let mean = self.mean();
        (self.squares / self.rows as f64 - mean * mean).max(0.0)
    }
}

/// R4552001: a card's board turns, its own field's (`acting`) counted as they are and the other
/// field's (`facing`) with the sign turned: `(turns, Σ signed y)`.
fn board_read(tally: &SwingTally, baseline: &SwingBaseline) -> Option<(i64, f64)> {
    let acting = Read::of(&tally.acting, baseline);
    let facing = Read::of(&tally.facing, baseline);
    let rows = acting.map_or(0, |read| read.rows) + facing.map_or(0, |read| read.rows);
    (rows > 0).then(|| {
        (
            rows,
            acting.map_or(0.0, |read| read.sum) - facing.map_or(0.0, |read| read.sum),
        )
    })
}

/// R4552001: the prior spreads of one field's play and board terms: R3022000's estimate over the cards
/// with `SWING_FIELD_MIN_ROWS` turns, each giving its mean and that mean's noise `σ² / n`, or the
/// named priors below `ADJUSTED_FIELD_MIN_CARDS` such cards.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TermSpreads {
    play: f64,
    board: f64,
}

fn term_spread(means: Vec<(i64, f64)>, noise: f64, prior: f64) -> f64 {
    let lifts: Vec<(f64, f64)> = means
        .into_iter()
        .filter(|(rows, _)| *rows >= SWING_FIELD_MIN_ROWS)
        .map(|(rows, mean)| (mean, noise / rows as f64))
        .collect();
    if noise <= 0.0 || lifts.len() < ADJUSTED_FIELD_MIN_CARDS {
        return prior;
    }
    random_effects(&lifts).1.sqrt()
}

fn term_spreads<'a>(tallies: impl Iterator<Item = &'a SwingTally>, baseline: &SwingBaseline) -> TermSpreads {
    let (mut plays, mut boards) = (Vec::new(), Vec::new());
    for tally in tallies {
        if let Some(read) = Read::of(&tally.played, baseline) {
            plays.push((read.rows, read.mean()));
        }
        if let Some((rows, sum)) = board_read(tally, baseline) {
            boards.push((rows, sum / rows as f64));
        }
    }
    TermSpreads {
        play: term_spread(plays, baseline.row_variance, SWING_PRIOR_PLAY_SPREAD),
        board: term_spread(boards, baseline.row_variance, SWING_PRIOR_BOARD_SPREAD),
    }
}

/// R2777000's shrinkage toward 0: `mean · τ² / (τ² + σ²/n)`.
fn shrunk(mean: f64, rows: i64, spread: f64, noise: f64) -> f64 {
    let prior = spread * spread;
    let sampling = noise / rows as f64;
    if prior + sampling <= 0.0 {
        return mean;
    }
    mean * prior / (prior + sampling)
}

/// R4552001: what Decisiveness is read against: the typical card's (`typical`, the field's centre), the
/// spread real cards' show about it, and the cards they were read from (0 for the named prior).
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecisivenessField {
    pub typical: f64,
    pub spread: f64,
    pub cards: usize,
}

impl DecisivenessField {
    /// Below `ADJUSTED_FIELD_MIN_CARDS` read cards.
    pub const PRIOR: DecisivenessField = DecisivenessField {
        typical: DECISIVENESS_PRIOR_CENTER,
        spread: DECISIVENESS_PRIOR_SPREAD,
        cards: 0,
    };
}

/// R4552001: one card's figures. Only `decisiveness` is shown (R4552003); the rest are for `stats
/// report --swing` and `stats-cards --swing`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwingFigures {
    /// The turns it was played on.
    pub plays: i64,
    /// The turns it acted on its own field.
    pub acting: i64,
    /// `b`: what a turn it is played on gains beyond an ordinary turn, shrunk.
    pub play: Option<f64>,
    /// `a`, Board swing: what a turn it acts on its field gains, shrunk.
    pub board: Option<f64>,
    /// Swing: `b` and the board terms of the turns it and the cards it made then act, per play.
    pub swing: Option<f64>,
    /// The raw root mean square of its turns' unexplained swing plus `b`: its play turns, or for a
    /// Created card no hand held its acting turns.
    pub rms: Option<f64>,
    /// `rms`'s sampling variance (the delta method, the turns clustered by game).
    pub noise: Option<f64>,
    /// Decisiveness: `rms` shrunk toward the typical card's.
    pub decisiveness: Option<f64>,
}

/// R4552001: every card's figures, and the fields they were read against.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwingReport {
    pub deck_field: DecisivenessField,
    pub created_field: DecisivenessField,
    pub cards: IndexMap<String, SwingFigures>,
    pub created: IndexMap<String, SwingFigures>,
}

impl SwingReport {
    /// A deck card's figures, or for a card only a rule makes (R3070000) its Created ones.
    pub fn of(&self, card: &str) -> Option<&SwingFigures> {
        self.cards.get(card)
    }
}

/// One card's terms before the fields of Decisiveness are read.
struct Terms {
    plays: i64,
    acting: i64,
    play: Option<f64>,
    board: Option<f64>,
    rms: Option<f64>,
    /// The turns `rms` was read over.
    rms_rows: i64,
    noise: Option<f64>,
}

/// R4552001: the root mean square of `r = y − ȳ + term` over a kind's turns, `√(Var y + term²)`, and its
/// sampling variance by the delta method, `Var(r²) / (4 · mean r² · n_eff)`, read off the raw moments of
/// `d = Δ − E[Δ] + term` (`α` varies little across buckets, so `d` is `r` but for it). `Var(r²)` is at
/// least a normal distribution's, `2 · RMS⁴`: a few turns that happen to swing alike (two plays, each
/// ±40 points) measure no spread of `r²` at all, and are not read as certain for it. It is at least an
/// ordinary turn's too, `2 · σ⁴` (`row_variance`, the baseline's): one turn has no spread, so its RMS is
/// its shrunk term, near 0, and would read as certainly near 0 (S+ #61.1 Shawarma Law's one play among
/// its 152 acting turns read ±0.3%). Infinite at 0.
fn rms_of(read: &Read, term: f64, row_variance: f64) -> (f64, f64) {
    let rms = (read.variance() + term * term).sqrt();
    let shift = read.raw[0] - term;
    let [m1, m2, m3, m4] = read.raw;
    let second = m2 - 2.0 * shift * m1 + shift * shift;
    let fourth = m4 - 4.0 * shift * m3 + 6.0 * shift * shift * m2 - 4.0 * shift.powi(3) * m1 + shift.powi(4);
    let spread = (fourth - second * second)
        .max(2.0 * rms.powi(4))
        .max(2.0 * row_variance * row_variance);
    let noise = if rms > 0.0 {
        spread / (4.0 * rms * rms * read.effective)
    } else {
        f64::INFINITY
    };
    (rms, noise)
}

fn terms_of(tally: &SwingTally, baseline: &SwingBaseline, spreads: &TermSpreads, created: bool) -> Terms {
    let played = Read::of(&tally.played, baseline);
    let play = played.map(|read| shrunk(read.mean(), read.rows, spreads.play, baseline.row_variance));
    let board = board_read(tally, baseline)
        .map(|(rows, sum)| shrunk(sum / rows as f64, rows, spreads.board, baseline.row_variance));
    // R4552002: a Created card no hand held reads the turns it acts, its board term in place of a play's.
    let (rms, noise, rms_rows) = match (played, play) {
        (Some(read), Some(play)) => {
            let (rms, noise) = rms_of(&read, play, baseline.row_variance);
            (Some(rms), Some(noise), read.rows)
        }
        _ if created => match (Read::of(&tally.acting, baseline), board) {
            (Some(read), Some(board)) => {
                let (rms, noise) = rms_of(&read, board, baseline.row_variance);
                (Some(rms), Some(noise), read.rows)
            }
            _ => (None, None, 0),
        },
        _ => (None, None, 0),
    };
    Terms {
        plays: tally.played.rows(),
        acting: tally.acting.rows(),
        play,
        board,
        rms,
        rms_rows,
        noise,
    }
}

/// R4552001: a field of Decisiveness, R3022000's estimate over the cards with `SWING_FIELD_MIN_ROWS`
/// turns and a finite, positive noise, each giving `(rms, noise)`; `DecisivenessField::PRIOR` below
/// `ADJUSTED_FIELD_MIN_CARDS` such cards.
fn decisiveness_field<'a>(terms: impl Iterator<Item = &'a Terms>) -> DecisivenessField {
    let read: Vec<(f64, f64)> = terms
        .filter(|terms| terms.rms_rows >= SWING_FIELD_MIN_ROWS)
        .filter_map(|terms| Some((terms.rms?, terms.noise?)))
        .filter(|(_, noise)| noise.is_finite() && *noise > 0.0)
        .collect();
    if read.len() < ADJUSTED_FIELD_MIN_CARDS {
        return DecisivenessField::PRIOR;
    }
    let (typical, spread_squared) = random_effects(&read);
    DecisivenessField {
        typical,
        spread: spread_squared.sqrt(),
        cards: read.len(),
    }
}

/// R4552001: `rms` kept in the share `τ_D² / (τ_D² + SE²)` of its distance from the typical card's.
fn decisiveness_of(terms: &Terms, field: &DecisivenessField) -> Option<f64> {
    let rms = terms.rms?;
    let noise = terms.noise?;
    let prior = field.spread * field.spread;
    let weight = if noise.is_finite() {
        if prior + noise > 0.0 {
            prior / (prior + noise)
        } else {
            1.0
        }
    } else {
        0.0
    };
    Some(field.typical + weight * (rms - field.typical))
}

/// R4552001, R4552002: Swing, what a play is worth: `b`, plus per play the turns it then acts times its
/// board term (a Created card's, only the turns of its copies a hand held: the others follow their
/// makers' plays), plus per play the turns each Created card it made acts times that card's (its Created
/// figure). `None` with no play.
fn swing_of(
    terms: &Terms,
    tally: &SwingTally,
    created: bool,
    made_board: &IndexMap<&str, f64>,
) -> Option<f64> {
    let play = terms.play?;
    let plays = terms.plays as f64;
    let own = if created { tally.held_acting } else { terms.acting };
    let mut swing = play + terms.board.unwrap_or(0.0) * own as f64 / plays;
    let mut made: Vec<(&String, &i64)> = tally.made_acting.iter().collect();
    made.sort_unstable();
    for (card, turns) in made {
        swing += made_board.get(card.as_str()).copied().unwrap_or(0.0) * *turns as f64 / plays;
    }
    Some(swing)
}

/// R4552001, R4552002: every card's figures. `deck` holds the deck cards' tallies (`CardStats.swing`),
/// `created` the Created cards' (`CreatedStats.swing`), `baseline` the turns they were read beside. The
/// deck cards and the Created ones are separate fields, each its own term spreads and Decisiveness field;
/// a card a rule makes (R4082000's `made_by_the_rules`: The Coin, Glitch, the Dormant Starship) is read
/// against the deck cards' and counts toward neither. A card with no turns of a kind has no figure of
/// it.
pub fn swing_report<'a>(
    deck: impl IntoIterator<Item = (&'a str, &'a SwingTally)>,
    created: impl IntoIterator<Item = (&'a str, &'a SwingTally)>,
    baseline: &SwingBaseline,
) -> SwingReport {
    let deck: Vec<(&str, &SwingTally)> = deck.into_iter().collect();
    let created: Vec<(&str, &SwingTally)> = created.into_iter().collect();
    let deck_spreads = term_spreads(deck.iter().map(|(_, tally)| *tally), baseline);
    let created_spreads = term_spreads(
        created
            .iter()
            .filter(|(card, _)| !made_by_the_rules(card))
            .map(|(_, tally)| *tally),
        baseline,
    );
    let deck_terms: Vec<(&str, &SwingTally, Terms)> = deck
        .iter()
        .map(|(card, tally)| (*card, *tally, terms_of(tally, baseline, &deck_spreads, false)))
        .collect();
    let created_terms: Vec<(&str, &SwingTally, Terms)> = created
        .iter()
        .map(|(card, tally)| {
            let spreads = if made_by_the_rules(card) {
                &deck_spreads
            } else {
                &created_spreads
            };
            (*card, *tally, terms_of(tally, baseline, spreads, true))
        })
        .collect();
    let deck_field = decisiveness_field(deck_terms.iter().map(|(_, _, terms)| terms));
    let created_field = decisiveness_field(
        created_terms
            .iter()
            .filter(|(card, _, _)| !made_by_the_rules(card))
            .map(|(_, _, terms)| terms),
    );
    let made_board: IndexMap<&str, f64> = created_terms
        .iter()
        .filter_map(|(card, _, terms)| Some((*card, terms.board?)))
        .collect();
    let figures =
        |tally: &SwingTally, terms: &Terms, field: &DecisivenessField, created: bool| SwingFigures {
            plays: terms.plays,
            acting: terms.acting,
            play: terms.play,
            board: terms.board,
            swing: swing_of(terms, tally, created, &made_board),
            rms: terms.rms,
            noise: terms.noise,
            decisiveness: decisiveness_of(terms, field),
        };
    SwingReport {
        cards: deck_terms
            .iter()
            .map(|(card, tally, terms)| (card.to_string(), figures(tally, terms, &deck_field, false)))
            .collect(),
        created: created_terms
            .iter()
            .map(|(card, tally, terms)| {
                let field = if made_by_the_rules(card) {
                    &deck_field
                } else {
                    &created_field
                };
                (card.to_string(), figures(tally, terms, field, true))
            })
            .collect(),
        deck_field,
        created_field,
    }
}

/// A fraction of win chance in points with its sign, "+3.2", or "—".
fn points(value: Option<f64>) -> String {
    match value {
        None => "—".to_string(),
        Some(value) => {
            let shown = to_fixed(value * 100.0, 1);
            if value > 0.0 { format!("+{shown}") } else { shown }
        }
    }
}

/// R4552001: Decisiveness as the page shows it, "±14.2%", or "—".
pub fn format_decisiveness(value: Option<f64>) -> String {
    value.map_or_else(
        || "—".to_string(),
        |value| format!("±{}%", to_fixed(value * 100.0, 1)),
    )
}

/// R4552001: the report as plain text, `stats report --swing` and `stats-cards --swing`: each field's
/// typical Decisiveness, then the deck cards and the Created ones, most decisive first, each with its
/// plays, play term, Board swing, Swing, raw RMS and Decisiveness. `name_of` names an id.
pub fn format_swing(report: &SwingReport, name_of: impl Fn(&str) -> Option<String>) -> String {
    let named = |card: &str| match name_of(card) {
        None => card.to_string(),
        Some(name) => format!("{card} {name}"),
    };
    let header: Vec<String> = [
        "Card",
        "Plays",
        "Acting",
        "Play term",
        "Board swing",
        "Swing",
        "RMS",
        "Decisiveness",
    ]
    .map(str::to_string)
    .to_vec();
    let rows = |figures: &IndexMap<String, SwingFigures>| -> Vec<Vec<String>> {
        let mut sorted: Vec<(&String, &SwingFigures)> = figures.iter().collect();
        sorted.sort_by(|a, b| {
            b.1.decisiveness
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&a.1.decisiveness.unwrap_or(f64::NEG_INFINITY))
                .then_with(|| a.0.cmp(b.0))
        });
        sorted
            .into_iter()
            .map(|(card, figures)| {
                vec![
                    named(card),
                    figures.plays.to_string(),
                    figures.acting.to_string(),
                    points(figures.play),
                    points(figures.board),
                    points(figures.swing),
                    format_decisiveness(figures.rms),
                    format_decisiveness(figures.decisiveness),
                ]
            })
            .collect()
    };
    let field = |name: &str, field: &DecisivenessField| {
        format!(
            "{name}: a typical card {}, real cards spread {} about it, read from {} cards.",
            format_decisiveness(Some(field.typical)),
            to_fixed(field.spread * 100.0, 1),
            field.cards
        )
    };
    let mut lines = vec![
        "Decisiveness: how far a card's play typically moves its player's chance of winning, either way (R4552001).".to_string(),
        "Play term and Board swing are what a turn it is played on, or acts on its field, gains beyond an ordinary turn, in".to_string(),
        "points; Swing is the play term and the board terms of the turns it and the cards it made then act, per play.".to_string(),
        field("Deck cards", &report.deck_field),
        field("Created cards", &report.created_field),
        String::new(),
    ];
    lines.extend(table(&header, &rows(&report.cards)));
    if !report.created.is_empty() {
        lines.push(String::new());
        lines.push("Created cards:".to_string());
        lines.extend(table(&header, &rows(&report.created)));
    }
    lines.join("\n")
}

// R4552000–R4552002: records written by hand, each turn's win probability set by the evaluation that
// reads it (`eval_for`), so every turn's change is known.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::events::{GameOverReason, Winner};
    use crate::wire::stats::{
        CardStatsReport, CreatedUse, GameMode, GameSource, GameSummary, Pilot, PilotFilter, SeatSummary,
        SourceFilter, card_stats_of, parse_game_record, played_delta,
    };
    use serde_json::json;

    const EVERY: CardStatsFilter = CardStatsFilter {
        source: SourceFilter::All,
        mode: None,
        patch: None,
        pilot: PilotFilter::Unified,
    };

    /// The evaluation a row needs for its seat's win probability to read `p`.
    fn eval_for(p: f64, turn: i32, first: bool) -> i64 {
        let logit = (p / (1.0 - p)).ln();
        let rest = logit
            - WIN_PROBABILITY_INTERCEPT
            - WIN_PROBABILITY_TURN * f64::from(turn)
            - WIN_PROBABILITY_FIRST * f64::from(u8::from(first));
        (rest / WIN_PROBABILITY_EVAL * SWING_EVAL_SCALE).round() as i64
    }

    /// The seat's `index`th turn's counter, p1 going first.
    fn turn_of(index: usize, first: bool) -> i32 {
        2 * i32::try_from(index).unwrap() + if first { 1 } else { 2 }
    }

    /// A seat holding `deck` whose turns begin at the win probabilities given, each with the cards it
    /// played and the rows' acting cards.
    fn seat(deck: &[&str], first: bool, turns: &[(f64, Vec<&str>, Vec<ActingCard>)]) -> SeatSummary {
        let mut seat = SeatSummary {
            deck: deck.iter().map(|card| card.to_string()).collect(),
            played_turns: Some(Vec::new()),
            swing_turns: Some(Vec::new()),
            ..SeatSummary::default()
        };
        for (index, (p, played, acting)) in turns.iter().enumerate() {
            let turn = turn_of(index, first);
            for card in played {
                seat.played.push(card.to_string());
                seat.played_turns.as_mut().unwrap().push(turn);
            }
            seat.swing_turns.as_mut().unwrap().push(SwingTurn {
                turn,
                eval: eval_for(*p, turn, first),
                acting: acting.clone(),
                facing: Vec::new(),
            });
        }
        seat
    }

    fn record(id: &str, winner: Winner, p1: SeatSummary, p2: SeatSummary) -> GameRecord {
        GameRecord {
            id: id.to_string(),
            source: GameSource::Dev,
            mode: GameMode::Random,
            patch: "v0.1.1".to_string(),
            pilots: PerPlayer::new(Pilot::Ai, Pilot::Ai),
            game: GameSummary {
                first: PlayerId::P1,
                winner,
                reason: GameOverReason::HeroDeath,
                turns: 20,
                seats: PerPlayer::new(p1, p2),
            },
            build: None,
        }
    }

    fn totals_of<'a>(records: impl IntoIterator<Item = &'a GameRecord>) -> StatsTotals {
        let mut totals = StatsTotals::default();
        for record in records {
            totals.fold(record, false);
        }
        totals
    }

    fn report_of(totals: &StatsTotals) -> (CardStatsReport, SwingReport) {
        let report = card_stats_of(totals, &EVERY);
        let baseline = swing_baseline(&swing_rows_of(totals, &EVERY));
        let swing = swing_report(
            report
                .cards
                .iter()
                .map(|stats| (stats.card.as_str(), &stats.swing)),
            report
                .created
                .iter()
                .map(|stats| (stats.card.as_str(), &stats.swing)),
            &baseline,
        );
        (report, swing)
    }

    const FILLERS: usize = 32;

    fn filler(index: usize) -> String {
        format!("filler-{index:02}")
    }

    /// Game `n`: p1 plays `quiet` on its first turn, which changes nothing, and `wild` on its second,
    /// which moves its chance 40 points one way or the other. p2 plays sixteen of `FILLERS` cards, each
    /// on a turn of its own that moves p2's chance by the filler's own size (2 to 16 points) one way or
    /// the other, a quiet turn after it bringing it back, so the field's typical card and spread are
    /// read off real spread.
    fn swing_game(n: usize, wild: &str, quiet: &str) -> GameRecord {
        let won = n.is_multiple_of(2);
        let after = if won { 0.9 } else { 0.1 };
        let p1 = seat(
            &[wild, quiet],
            true,
            &[
                (0.5, vec![quiet], vec![]),
                (0.5, vec![wild], vec![]),
                (after, vec![], vec![]),
            ],
        );
        let fillers: Vec<usize> = (0..16).map(|i| (n * 16 + i) % FILLERS).collect();
        let names: Vec<String> = fillers.iter().map(|index| filler(*index)).collect();
        let mut turns = Vec::new();
        for (i, (index, card)) in fillers.iter().zip(&names).enumerate() {
            let size = 0.02 * (1 + index % 8) as f64;
            let sign = if (n + i).is_multiple_of(2) { 1.0 } else { -1.0 };
            turns.push((0.5, vec![card.as_str()], vec![]));
            turns.push((0.5 + sign * size, vec![], vec![]));
        }
        // A last, empty turn ends at the result.
        turns.push((0.5, vec![], vec![]));
        let deck: Vec<&str> = names.iter().map(String::as_str).collect();
        let p2 = seat(&deck, false, &turns);
        record(
            &format!("g{n:03}"),
            if won { Winner::P1 } else { Winner::P2 },
            p1,
            p2,
        )
    }

    fn figures<'a>(report: &'a SwingReport, card: &str) -> &'a SwingFigures {
        report
            .cards
            .get(card)
            .unwrap_or_else(|| panic!("no figures for {card}"))
    }

    #[test]
    fn r4552000_win_probability_reads_the_constants() {
        let sigmoid = |z: f64| 1.0 / (1.0 + (-z).exp());
        for (eval, turn, first) in [(0, 1, true), (12_500, 7, false), (-40_000, 22, true)] {
            let z = WIN_PROBABILITY_INTERCEPT
                + WIN_PROBABILITY_EVAL * eval as f64 / SWING_EVAL_SCALE
                + WIN_PROBABILITY_TURN * f64::from(turn)
                + if first { WIN_PROBABILITY_FIRST } else { 0.0 };
            assert_eq!(win_probability(eval, turn, first).to_bits(), sigmoid(z).to_bits());
        }
        assert!((win_probability(eval_for(0.85, 9, true), 9, true) - 0.85).abs() < 1e-4);
        assert_eq!(swing_bucket(0, true), 1);
        assert_eq!(swing_bucket(3, false), -4);
        assert_eq!(swing_bucket(40, true), SWING_TURN_BUCKETS);
    }

    #[test]
    fn r4552000_a_record_with_swing_turns_parses_and_round_trips() {
        let mut p1 = seat(
            &["core-015"],
            true,
            &[
                (0.5, vec!["core-015"], vec![]),
                (
                    0.6,
                    vec![],
                    vec![
                        ActingCard::Dealt("core-015".to_string()),
                        ActingCard::Created(CreatedActing {
                            card: "core-t-rush".to_string(),
                            maker: Some("core-015".to_string()),
                        }),
                    ],
                ),
            ],
        );
        p1.swing_turns.as_mut().unwrap()[1].facing = vec![ActingCard::Created(CreatedActing {
            card: "core-coin".to_string(),
            maker: None,
        })];
        let written = record("m1", Winner::P1, p1, SeatSummary::default());
        let value = serde_json::to_value(&written).unwrap();
        let row = &value["game"]["seats"]["p1"]["swingTurns"][1];
        assert_eq!(row["acting"][0], json!("core-015"));
        assert_eq!(
            row["acting"][1],
            json!({ "card": "core-t-rush", "maker": "core-015" })
        );
        assert_eq!(row["facing"][0], json!({ "card": "core-coin" }));
        assert_eq!(parse_game_record(&value).unwrap(), written);
        // A record written before the rows reads none, and a malformed one is refused by its path.
        assert_eq!(value["game"]["seats"]["p2"].get("swingTurns"), None);
        let mut bad = value.clone();
        bad["game"]["seats"]["p1"]["swingTurns"] = json!([{ "turn": "one", "eval": 0 }]);
        assert_eq!(
            parse_game_record(&bad).unwrap_err().message,
            "game.seats.p1.swingTurns is not a list of turn rows"
        );
    }

    #[test]
    fn r4552001_a_card_swinging_either_way_reads_well_above_the_typical_card() {
        let records: Vec<GameRecord> = (0..80).map(|n| swing_game(n, "wild", "quiet")).collect();
        let (_, report) = report_of(&totals_of(&records));
        // The fillers make a field of its own, not the named prior.
        assert!(
            report.deck_field.cards >= ADJUSTED_FIELD_MIN_CARDS,
            "{:?}",
            report.deck_field
        );
        let typical = report.deck_field.typical;
        // Within the fillers' own swings, 2 to 16 points (`swing_game`).
        assert!(
            typical > 0.02 && typical < 0.16 && typical != DECISIVENESS_PRIOR_CENTER,
            "typical {typical}"
        );
        let wild = figures(&report, "wild");
        assert_eq!(wild.plays, 80);
        // Its raw RMS is its 40 points; shrunk by 80 plays' noise, it still reads twice the typical card.
        assert!((wild.rms.unwrap() - 0.4).abs() < 0.01, "{wild:?}");
        let decisive = wild.decisiveness.unwrap();
        assert!(
            decisive > 2.0 * typical && decisive > typical + 0.1,
            "{decisive} against {typical}"
        );
        assert!(wild.swing.unwrap().abs() < 0.02, "{:?}", wild.swing);
        let quiet = figures(&report, "quiet");
        assert!(quiet.decisiveness.unwrap() <= typical + 1e-12, "{quiet:?}");
        assert!(quiet.swing.unwrap().abs() < 0.01);
    }

    #[test]
    fn r4552001_a_card_played_from_ahead_that_changes_nothing_reads_no_swing() {
        // Game n: p1 holds `ahead`. In even games it is ahead at its second turn (85%), plays it, stays
        // there and wins; in odd games it draws it, never plays it, and loses from an even game.
        let records: Vec<GameRecord> = (0..60)
            .map(|n| {
                let played = n % 2 == 0;
                let mut p1 = if played {
                    seat(
                        &["ahead"],
                        true,
                        &[
                            (0.5, vec![], vec![]),
                            (0.85, vec!["ahead"], vec![]),
                            (0.85, vec![], vec![]),
                        ],
                    )
                } else {
                    seat(
                        &["ahead"],
                        true,
                        &[
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                        ],
                    )
                };
                p1.drawn = vec!["ahead".to_string()];
                let p2 = seat(&["other"], false, &[(0.5, vec![], vec![]), (0.5, vec![], vec![])]);
                record(
                    &format!("a{n:03}"),
                    if played { Winner::P1 } else { Winner::P2 },
                    p1,
                    p2,
                )
            })
            .collect();
        let totals = totals_of(&records);
        let (cards, report) = report_of(&totals);
        let stats = cards.cards.iter().find(|stats| stats.card == "ahead").unwrap();
        assert!(played_delta(stats).unwrap() > 0.2, "{:?}", played_delta(stats));
        let ahead = figures(&report, "ahead");
        assert_eq!(ahead.plays, 30);
        assert!(ahead.swing.unwrap().abs() < 0.01, "{:?}", ahead.swing);
        assert!(
            ahead.decisiveness.unwrap() <= report.deck_field.typical + 1e-12,
            "{ahead:?}"
        );
    }

    /// A seat that plays `card` on its only turn and moves its chance by `delta` from an even game.
    fn one_swing(n: usize, card: &str, delta: f64) -> GameRecord {
        let p1 = seat(
            &[card],
            true,
            &[(0.5, vec![card], vec![]), (0.5 + delta, vec![], vec![])],
        );
        let p2 = seat(&["other"], false, &[(0.5, vec![], vec![])]);
        let winner = if delta > 0.0 { Winner::P1 } else { Winner::P2 };
        // The last turn ends at the result, from 0.5 + delta: its move is no part of `card`'s.
        record(&format!("{card}-{n:04}"), winner, p1, p2)
    }

    #[test]
    fn r4552001_a_thin_sample_is_shrunk_toward_the_typical_card_and_a_thick_one_is_not() {
        let moves = [0.05, -0.45, 0.25, -0.35, 0.15, -0.3, 0.4];
        let mut records: Vec<GameRecord> = (0..3).map(|n| one_swing(n, "thin", moves[n])).collect();
        records.extend((0..8000).map(|n| one_swing(n, "thick", moves[n % moves.len()])));
        let (_, report) = report_of(&totals_of(&records));
        let typical = report.deck_field.typical;
        assert_eq!(report.deck_field, DecisivenessField::PRIOR);
        let kept = |card: &str| {
            let figures = figures(&report, card);
            let rms = figures.rms.unwrap();
            assert!(rms > 2.0 * typical, "{card}: {figures:?}");
            (figures.decisiveness.unwrap() - typical) / (rms - typical)
        };
        let (thin, thick) = (kept("thin"), kept("thick"));
        assert!(thin < 0.5, "thin keeps {thin}");
        assert!(thick > 0.95, "thick keeps {thick}");
    }

    #[test]
    fn r4552001_a_dozen_plays_keep_little_of_their_distance_from_the_typical_card() {
        // A plain stat-stick's dozen plays read 17 points raw against the typical card's 12: real cards'
        // RMS spread about 2 points, so a dozen plays' noise is most of what they show (issue #4552's
        // 4-mana 7/7 kept 0.6 of it under a 4.5-point prior spread, and read clearly decisive).
        let records: Vec<GameRecord> = (0..12)
            .map(|n| {
                let delta = if n % 2 == 0 { 0.17 } else { -0.17 };
                let p1 = seat(
                    &["dozen"],
                    true,
                    &[(0.5, vec!["dozen"], vec![]), (0.5 + delta, vec![], vec![])],
                );
                let p2 = seat(&["other"], false, &[(0.5, vec![], vec![])]);
                record(&format!("dozen-{n:04}"), Winner::Draw, p1, p2)
            })
            .collect();
        let (_, report) = report_of(&totals_of(&records));
        assert_eq!(report.deck_field, DecisivenessField::PRIOR);
        let typical = report.deck_field.typical;
        let dozen = figures(&report, "dozen");
        let rms = dozen.rms.unwrap();
        assert!((rms - 0.17).abs() < 0.005, "{dozen:?}");
        let kept = (dozen.decisiveness.unwrap() - typical) / (rms - typical);
        assert!(kept > 0.0 && kept < 0.35, "keeps {kept}: {dozen:?}");
    }

    #[test]
    fn r4552001_one_play_is_read_as_the_typical_card_not_as_certainly_quiet() {
        // One turn has no spread, so its RMS is its play term, shrunk near 0: the 40 points it moved
        // must not read as a certainly quiet card. The thick card gives the turns their spread.
        let moves = [0.05, -0.45, 0.25, -0.35, 0.15, -0.3, 0.4];
        let mut records = vec![one_swing(0, "once", 0.4)];
        records.extend((0..2000).map(|n| one_swing(n, "thick", moves[n % moves.len()])));
        let (_, report) = report_of(&totals_of(&records));
        let typical = report.deck_field.typical;
        let once = figures(&report, "once");
        assert!(once.rms.unwrap() < 0.01, "{once:?}");
        let decisiveness = once.decisiveness.unwrap();
        assert!(
            decisiveness <= typical && typical - decisiveness < 0.001,
            "{once:?} against {typical}"
        );
    }

    fn bits(figures: &SwingFigures) -> Vec<Option<u64>> {
        [
            figures.play,
            figures.board,
            figures.swing,
            figures.rms,
            figures.noise,
            figures.decisiveness,
        ]
        .iter()
        .map(|value| value.map(f64::to_bits))
        .collect()
    }

    #[test]
    fn r4552001_any_fold_order_gives_the_same_figures_bit_for_bit() {
        let records: Vec<GameRecord> = (0..40).map(|n| swing_game(n, "wild", "quiet")).collect();
        let forward = totals_of(&records);
        let reverse = totals_of(records.iter().rev());
        let mut halves = totals_of(records.iter().skip(20));
        halves.add(&totals_of(records.iter().take(20)));
        assert_eq!(forward, reverse);
        assert_eq!(forward, halves);
        let (_, first) = report_of(&forward);
        for other in [&reverse, &halves] {
            let (_, again) = report_of(other);
            assert_eq!(
                first.deck_field.typical.to_bits(),
                again.deck_field.typical.to_bits()
            );
            assert_eq!(first.cards.len(), again.cards.len());
            for (card, figures) in &first.cards {
                assert_eq!(bits(figures), bits(&again.cards[card]), "{card}");
            }
        }
    }

    #[test]
    fn r4552002_a_token_put_onto_the_field_is_credited_to_its_maker() {
        // In even games p1 plays core-015, whose Cry puts a core-t-rush onto the field; the token acts on
        // p1's next two turns, each of which gains 20 points. In odd games p1 plays nothing and its
        // chance stands still, so an ordinary second and third turn gain 10 points and the token's 20.
        let rush = || {
            ActingCard::Created(CreatedActing {
                card: "core-t-rush".to_string(),
                maker: Some("core-015".to_string()),
            })
        };
        let records: Vec<GameRecord> = (0..60)
            .map(|n| {
                let mut p1 = if n % 2 == 0 {
                    seat(
                        &["core-015"],
                        true,
                        &[
                            (0.5, vec!["core-015"], vec![]),
                            (0.5, vec![], vec![rush()]),
                            (0.7, vec![], vec![rush()]),
                            (0.9, vec![], vec![]),
                        ],
                    )
                } else {
                    seat(
                        &["core-015"],
                        true,
                        &[
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                        ],
                    )
                };
                if n % 2 == 0 {
                    p1.created_used = Some(vec![CreatedUse {
                        card: "core-t-rush".to_string(),
                        turn: 1,
                        radiant: false,
                    }]);
                }
                let p2 = seat(&["other"], false, &[(0.5, vec![], vec![]), (0.5, vec![], vec![])]);
                record(
                    &format!("t{n:03}"),
                    if n % 2 == 0 { Winner::P1 } else { Winner::P2 },
                    p1,
                    p2,
                )
            })
            .collect();
        let totals = totals_of(&records);
        let (cards, report) = report_of(&totals);
        let token = cards
            .created
            .iter()
            .find(|made| made.card == "core-t-rush")
            .unwrap();
        assert!(
            token.swing.played.is_empty(),
            "a token put onto the field has no play of its own"
        );
        assert_eq!(token.swing.acting.rows(), 60);
        let maker = cards.cards.iter().find(|stats| stats.card == "core-015").unwrap();
        assert_eq!(maker.swing.played.rows(), 30);
        assert_eq!(maker.swing.made_acting.get("core-t-rush"), Some(&60));
        let token = &report.created["core-t-rush"];
        let maker = figures(&report, "core-015");
        assert_eq!(token.plays, 0);
        assert_eq!(token.swing, None);
        let board = token.board.unwrap();
        assert!(board > 0.0);
        // The maker's Swing: its own play term and two turns of its token's board term per play.
        let from_token = maker.swing.unwrap() - maker.play.unwrap();
        assert!(
            (from_token - 2.0 * board).abs() < 1e-12,
            "{from_token} against {board}"
        );
        // A token no hand held reads the turns it acts.
        assert!(token.decisiveness.is_some());
        assert_eq!(
            cards
                .created
                .iter()
                .find(|made| made.card == "core-t-rush")
                .unwrap()
                .swing
                .held_acting,
            0
        );
    }

    #[test]
    fn r4552002_a_created_card_a_hand_held_counts_its_own_acting_turns_in_its_swing() {
        // In even games p1 plays a Created copy from its hand on its first turn, which then acts on its
        // next two turns, each gaining 20 points; in odd games p1 plays nothing and stands still.
        let held = || {
            ActingCard::Created(CreatedActing {
                card: "core-008".to_string(),
                maker: None,
            })
        };
        let records: Vec<GameRecord> = (0..60)
            .map(|n| {
                let played = n % 2 == 0;
                let mut p1 = if played {
                    seat(
                        &["other"],
                        true,
                        &[
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![held()]),
                            (0.7, vec![], vec![held()]),
                            (0.9, vec![], vec![]),
                        ],
                    )
                } else {
                    seat(
                        &["other"],
                        true,
                        &[
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                            (0.5, vec![], vec![]),
                        ],
                    )
                };
                if played {
                    let used = CreatedUse {
                        card: "core-008".to_string(),
                        turn: 1,
                        radiant: false,
                    };
                    p1.created_used = Some(vec![used.clone()]);
                    p1.created_played = Some(vec![used]);
                }
                let p2 = seat(&["other"], false, &[(0.5, vec![], vec![]), (0.5, vec![], vec![])]);
                record(
                    &format!("h{n:03}"),
                    if played { Winner::P1 } else { Winner::P2 },
                    p1,
                    p2,
                )
            })
            .collect();
        let (cards, report) = report_of(&totals_of(&records));
        let tally = &cards
            .created
            .iter()
            .find(|made| made.card == "core-008")
            .unwrap()
            .swing;
        assert_eq!(
            (tally.played.rows(), tally.acting.rows(), tally.held_acting),
            (30, 60, 60)
        );
        let copy = &report.created["core-008"];
        let from_board = copy.swing.unwrap() - copy.play.unwrap();
        assert!(copy.board.unwrap() > 0.0);
        assert!((from_board - 2.0 * copy.board.unwrap()).abs() < 1e-12, "{copy:?}");
    }
}
