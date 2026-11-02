//! A ladder bot's sealed picks in a Conquest series (SPEC §9.5, R641000, with R330–R333): which
//! deck of its trio it plays in the next game, and when it says so.
//!
//! Pure. Reads only what the bot's own side sees (R336): its trio, which decks won, and who goes
//! first next (R335), never the opponent's trio or pick (R185). Delays and picks are drawn from
//! `{seed_base}:bot-pick-delay:{seat}:{game}` and `{seed_base}:bot-pick:{seat}:{game}`.

use jackioh_engine::{CardDefs, Rng, query_cost};

use crate::api::series_rules::{
    SeriesRefusal, SeriesRefusalReason, first_seat_of, pick_deck, seat_index, trio_deck, unwon_slots,
};
use crate::config::{
    BOT_PICK_DELAY_MAX_MS, BOT_PICK_DELAY_MIN_MS, BOT_PICK_RULE_SHARE, MS_PER_SECOND, SERIES_PICK_SECONDS,
};
use crate::db::store::{SeriesRow, SeriesSeat, SeriesSide, SeriesStatus};

/// Games are numbered from one (`SeriesGame.game_no`), as in `series_rules.rs`.
const FIRST_GAME: i32 = 1;

fn picking_for(series: &SeriesRow) -> i32 {
    series.games.len() as i32 + FIRST_GAME
}

fn side_of(series: &SeriesRow, seat: SeriesSeat) -> &SeriesSide {
    if seat_index(seat) == 0 {
        &series.sides.0
    } else {
        &series.sides.1
    }
}

/// R641000: when the bot in `seat` makes its pick: `BOT_PICK_DELAY_MIN_MS` to `BOT_PICK_DELAY_MAX_MS`
/// after the pick phase opened (its deadline less `SERIES_PICK_SECONDS`), uniformly. `None` outside a
/// pick phase.
pub fn pick_due_at(series: &SeriesRow, seat: SeriesSeat) -> Option<i64> {
    if series.status != SeriesStatus::Picking {
        return None;
    }
    let deadline = series.pick_deadline?;
    let opened = deadline - SERIES_PICK_SECONDS * MS_PER_SECOND;
    let span = BOT_PICK_DELAY_MAX_MS - BOT_PICK_DELAY_MIN_MS + 1;
    let mut rng = Rng::new(
        &format!(
            "{}:bot-pick-delay:{}:{}",
            series.seed_base,
            seat.as_str(),
            picking_for(series)
        ),
        0,
    );
    Some(opened + BOT_PICK_DELAY_MIN_MS + i64::from(rng.int(i32::try_from(span).unwrap_or(1))))
}

/// R641000: whether the bot in `seat` picks at `now`: its pick is not in (R332's automatic pick
/// counts as one), its delay has passed and the pick clock has not run out (R333).
pub fn pick_is_due(series: &SeriesRow, seat: SeriesSeat, now: i64) -> bool {
    let (Some(due), Some(deadline)) = (pick_due_at(series, seat), series.pick_deadline) else {
        return false;
    };
    side_of(series, seat).pick.is_none() && now >= due && now < deadline
}

/// R641000: the deck its rule picks. A deck's pace is the mean cost (`query_cost`, R65) of its cards;
/// on the play (its side goes first in the next game, R335) the rule takes the fastest deck that has
/// not won, and on the draw the slowest, ties going to the lower slot. `None` when every deck has won.
pub fn rule_slot(series: &SeriesRow, seat: SeriesSeat, defs: &CardDefs) -> Option<usize> {
    let trio = &side_of(series, seat).trio;
    let pace = |slot: usize| -> f64 {
        let costs: Vec<f64> = trio_deck(trio, slot)
            .map(|deck| {
                deck.cards
                    .iter()
                    .filter_map(|id| defs.get(id))
                    .map(|def| f64::from(query_cost(def)))
                    .collect()
            })
            .unwrap_or_default();
        if costs.is_empty() {
            0.0
        } else {
            costs.iter().sum::<f64>() / costs.len() as f64
        }
    };
    let on_the_play = first_seat_of(picking_for(series)) == seat;
    unwon_slots(series, seat, &series.games)
        .into_iter()
        .map(|slot| (slot, pace(slot)))
        .reduce(|best, next| {
            let better = if on_the_play {
                next.1 < best.1
            } else {
                next.1 > best.1
            };
            if better { next } else { best }
        })
        .map(|(slot, _)| slot)
}

/// R641000: the deck the bot in `seat` picks for the next game: `rule_slot` with the chance
/// `BOT_PICK_RULE_SHARE`, otherwise a uniform pick among the decks that have not won (R330).
pub fn pick_slot(series: &SeriesRow, seat: SeriesSeat, defs: &CardDefs) -> Option<usize> {
    let mut rng = Rng::new(
        &format!(
            "{}:bot-pick:{}:{}",
            series.seed_base,
            seat.as_str(),
            picking_for(series)
        ),
        0,
    );
    if rng.chance(BOT_PICK_RULE_SHARE) {
        rule_slot(series, seat, defs)
    } else {
        rng.pick(&unwon_slots(series, seat, &series.games)).copied()
    }
}

/// R641000: the bot's pick as a transition: `pick_slot` through `pick_deck` (R331), for the game the
/// pick phase is for, so it is sealed and refused exactly as a person's is.
pub fn bot_pick(
    series: &SeriesRow,
    seat: SeriesSeat,
    defs: &CardDefs,
    now: i64,
) -> Result<SeriesRow, SeriesRefusal> {
    let Some(slot) = pick_slot(series, seat, defs) else {
        return Err(SeriesRefusal::new(
            SeriesRefusalReason::PicksMissing,
            "Every deck of this trio has already won.",
        ));
    };
    pick_deck(series, seat, slot as i32, now, Some(picking_for(series)))
}
