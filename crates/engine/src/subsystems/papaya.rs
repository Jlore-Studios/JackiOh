//! Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; docs/classic-sets.md B5 E32).
//!
//! The grid is the board's 20 cells from the caster's seat: x the lane less one on both sides (§3.1),
//! y 0 the caster's backrow, 1 their units, 2 the opponent's units, 3 the opponent's backrow. The curve
//! is the Lagrange polynomial through 1 to `PAPAYA_MAX_CELLS` cells in different lanes, in exact
//! rationals so "y = p(x) exactly" is an integer test; a cell is on it when p(x) is a whole row 0 to 3 (R422).
//!
//! Cells are asked one `cell` prompt at a time (E18, `choose_cell`): every cell of the unused lanes and,
//! after the first, "done", at most 21 answers, so `legal_actions`, the fuzz suite and the AI reach every
//! curve. The cells so far ride the prompt's resume data as plain points (R113); options are cells, never
//! cards (R177). Then the board is read once and the top of the pile at each cell on the curve is exiled:
//! face-down cards too (openly, once in exile, R97), tokens ceasing to exist (R11), a dormant card beneath
//! resuming and not exiled again (§3.2, R13); the Radiant face only the enemy's rows 2 and 3.
//!
//! A card script: `cry: hook(|_| subsystems::papaya_begin())`,
//! `resume: [(subsystems::PAPAYA_STEP, hook(subsystems::papaya_answered))]`.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{PAPAYA_MAX_CELLS, PAPAYA_ROWS, UNIT_ZONES};
use crate::effects::choose::{choose_cell, chosen_cells};
use crate::effects::move_::exile;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::state::GameState;
use crate::wire::{PlayerId, Row, ZoneRef, opponent_of};
use crate::zones::{ZoneSlot, zone_contents};

/// R422: a cell of the grid from the caster's seat — x the lane less one, y the row.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PapayaPoint {
    pub x: i32,
    pub y: i32,
}

/// R422: the lanes the curve is read at (§3.1: both rows have as many).
pub const PAPAYA_LANES: i32 = UNIT_ZONES;

/// One row of the grid: whose side it is on, and which row of that side.
struct GridRow {
    enemy: bool,
    row: Row,
}

/// R422: each row number, from the caster's hero outward. (Immutable data: SURFACE §3 bans only
/// mutable statics.)
static GRID_ROWS: [GridRow; 4] = [
    GridRow {
        enemy: false,
        row: Row::Backrow,
    },
    GridRow {
        enemy: false,
        row: Row::Units,
    },
    GridRow {
        enemy: true,
        row: Row::Units,
    },
    GridRow {
        enemy: true,
        row: Row::Backrow,
    },
];

fn is_cell(point: &PapayaPoint) -> bool {
    let PapayaPoint { x, y } = *point;
    (0..PAPAYA_LANES).contains(&x) && (0..PAPAYA_ROWS).contains(&y)
}

/// The grid row a `y` names, or `None` off the grid.
fn grid_row(y: i32) -> Option<&'static GridRow> {
    usize::try_from(y).ok().and_then(|at| GRID_ROWS.get(at))
}

/// R422: the zone a cell is, from `caster`'s seat. Panics on a point off the grid.
pub fn zone_of_point(caster: PlayerId, point: &PapayaPoint) -> ZoneSlot {
    let grid = match grid_row(point.y) {
        Some(grid) if is_cell(point) => grid,
        _ => panic!("R422: ({}, {}) is not a cell of the grid", point.x, point.y),
    };
    ZoneSlot {
        player: if grid.enemy { opponent_of(caster) } else { caster },
        row: grid.row,
        lane: point.x + 1,
    }
}

/// R422: the cell a zone is, from `caster`'s seat.
pub fn point_of_zone(caster: PlayerId, zone: &ZoneRef) -> PapayaPoint {
    let enemy = zone.player != caster;
    let y = GRID_ROWS
        .iter()
        .position(|grid| grid.enemy == enemy && grid.row == zone.row)
        .map_or(-1, |at| at as i32);
    PapayaPoint { x: zone.lane - 1, y }
}

/// A reduced fraction: integer numerator over a positive integer denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn rational(num: i64, den: i64) -> Rational {
    if num == 0 {
        return Rational { num: 0, den: 1 };
    }
    let divisor = gcd(num, den) * den.signum();
    Rational {
        num: num / divisor,
        den: den / divisor,
    }
}

/// R422: the curve through the cells (Lagrange), read at x exactly. Panics on points that are not 1
/// to `PAPAYA_MAX_CELLS` grid cells in different lanes.
pub fn curve_at(points: &[PapayaPoint], x: i32) -> Rational {
    let lanes: IndexSet<i32> = points.iter().map(|point| point.x).collect();
    if points.is_empty()
        || points.len() as i32 > PAPAYA_MAX_CELLS
        || lanes.len() != points.len()
        || !points.iter().all(is_cell)
    {
        panic!("R422: a curve goes through 1 to {PAPAYA_MAX_CELLS} grid cells in different lanes");
    }
    let x = i64::from(x);
    let mut sum = Rational { num: 0, den: 1 };
    for (i, pi) in points.iter().enumerate() {
        let mut term = Rational {
            num: i64::from(pi.y),
            den: 1,
        };
        for (j, pj) in points.iter().enumerate() {
            if j != i {
                term = rational(
                    term.num * (x - i64::from(pj.x)),
                    term.den * (i64::from(pi.x) - i64::from(pj.x)),
                );
            }
        }
        sum = rational(sum.num * term.den + term.num * sum.den, sum.den * term.den);
    }
    sum
}

/// R422: every cell on the curve, one per lane at most, in lane order.
pub fn cells_on_curve(points: &[PapayaPoint]) -> Vec<PapayaPoint> {
    (0..PAPAYA_LANES)
        .map(|x| (x, curve_at(points, x)))
        .filter(|(_, value)| value.den == 1 && value.num >= 0 && value.num < i64::from(PAPAYA_ROWS))
        .map(|(x, value)| PapayaPoint {
            x,
            y: value.num as i32,
        })
        .collect()
}

/// R422: the ids of the cards the curve exiles — the top of each pile on it, the enemy's rows only when
/// `enemy_only`.
pub fn cards_on_curve(
    state: &GameState,
    controller: PlayerId,
    points: &[PapayaPoint],
    enemy_only: bool,
) -> Vec<String> {
    if points.is_empty() {
        return Vec::new();
    }
    cells_on_curve(points)
        .iter()
        .filter_map(|cell| {
            if enemy_only && !grid_row(cell.y).is_some_and(|grid| grid.enemy) {
                return None;
            }
            zone_contents(state, zone_of_point(controller, cell))
                .first()
                .map(|top| top.id.clone())
        })
        .collect()
}

/// R422: the resume step each cell prompt's answer re-enters.
pub const PAPAYA_STEP: &str = "papayaCell";
/// R113: where the cells picked so far ride the resume data.
pub const PAPAYA_POINTS_KEY: &str = "papayaPoints";

fn ask_cell(points: &[PapayaPoint]) -> Effect {
    let except_lanes: Vec<i32> = points.iter().map(|point| point.x + 1).collect();
    let prompt = if points.is_empty() {
        "KY's Papaya: choose a cell for the curve"
    } else {
        "KY's Papaya: choose another cell, or done"
    };
    choose_cell(json_as(json!({
        "step": PAPAYA_STEP,
        "done": !points.is_empty(),
        "cells": { "exceptLanes": except_lanes },
        "prompt": prompt,
        "data": { PAPAYA_POINTS_KEY: points },
    })))
}

/// R422: the card's on-resolve hook — the first cell prompt.
pub fn papaya_begin() -> Vec<Effect> {
    vec![ask_cell(&[])]
}

/// The cells a resume carried, read back defensively.
fn held_points(data: Option<&Value>) -> Vec<PapayaPoint> {
    data.and_then(|value| serde_json::from_value::<Vec<PapayaPoint>>(value.clone()).ok())
        .unwrap_or_default()
}

/// R422: an answer — the next prompt while cells are being added, else the curve's exiles.
pub fn papaya_answered(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let held = held_points(ctx.data.get(PAPAYA_POINTS_KEY));
    let picked: Vec<PapayaPoint> = chosen_cells(ctx)
        .iter()
        .map(|zone| point_of_zone(ctx.controller, zone))
        .collect();
    let mut points = held;
    points.extend(picked.iter().copied());
    if !picked.is_empty() && (points.len() as i32) < PAPAYA_MAX_CELLS {
        return vec![ask_cell(&points)];
    }
    cards_on_curve(ctx.sink.state, ctx.controller, &points, ctx.radiant)
        .into_iter()
        .map(|instance_id| {
            exile(json_as(
                json!({ "target": { "of": "instance", "instanceId": instance_id } }),
            ))
        })
        .collect()
}
