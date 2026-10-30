//! Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; docs/classic-sets.md B5 E32).
//!
//! The grid is the board's 20 cells from the caster's seat: x the lane less one (§3.1), y 0 to 3 from
//! the caster's backrow out. The curve is the Lagrange polynomial through 1 to `PAPAYA_MAX_CELLS` cells
//! in different lanes, in exact rationals so "y = p(x) exactly" is an integer test (R422).
//!
//! Cells are Discovered one `mode` prompt at a time (R831003) from random cells of the unused lanes, so
//! `legal_actions`, the fuzz suite and the AI reach every answer while the player's control stays
//! limited; options are cells, never cards (R177). The curve's pile tops are destroyed (R1704000),
//! face-down cards too, a dormant card beneath resuming (§3.2, R13); the Radiant face only the enemy's.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{PAPAYA_BASE_OFFERS, PAPAYA_MAX_CELLS, PAPAYA_OFFERS, PAPAYA_ROWS, UNIT_ZONES};
use crate::effects::choose::{choose_mode, chosen_options};
use crate::effects::destroy::destroy;
use crate::effects::move_::exile;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::state::GameState;
use crate::wire::{GameEvent, PlayerId, Row, ZoneRef, opponent_of};
use crate::zones::{ZoneSlot, zone_contents};

/// R422: a cell of the grid from the caster's seat.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PapayaPoint {
    pub x: i32,
    pub y: i32,
}

/// R422: the lanes the curve is read at (§3.1: both rows have as many).
pub const PAPAYA_LANES: i32 = UNIT_ZONES;

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

/// R422: the curve through the cells, read at x exactly; panics on cells that make no curve.
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

/// R422: the ids of the tops of the piles on the curve.
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
/// R831003: the option that ends the picks (through v0.4.0f, R1704000).
pub const PAPAYA_DONE: &str = "done";

/// R1704000: the rules a Papaya's curve runs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PapayaRules {
    pub base_offers: i32,
    pub done: bool,
    pub destroys: bool,
}

/// R1704000: four Discovers, each of `PAPAYA_BASE_OFFERS` cells on the base face, then the destroys.
pub const PAPAYA_RULES: PapayaRules = PapayaRules {
    base_offers: PAPAYA_BASE_OFFERS,
    done: false,
    destroys: true,
};

/// R831003, R767000: the curve through v0.4.0f: 1 to 4 Discovers of `PAPAYA_OFFERS` cells on both
/// faces, then the exiles (openly, once in exile, R97; tokens ceasing to exist, R11).
pub const PAPAYA_RULES_V0_4_0F: PapayaRules = PapayaRules {
    base_offers: PAPAYA_OFFERS,
    done: true,
    destroys: false,
};

/// R831003: the option a cell is, `"x,y"`.
fn cell_option(point: &PapayaPoint) -> String {
    format!("{},{}", point.x, point.y)
}

/// R831003: the cell an option names.
fn option_cell(option: &str) -> Option<PapayaPoint> {
    let (x, y) = option.split_once(',')?;
    let point = PapayaPoint {
        x: x.parse().ok()?,
        y: y.parse().ok()?,
    };
    is_cell(&point).then_some(point)
}

/// R831003: a cell as the prompt names it, from the caster's seat.
fn cell_label(point: &PapayaPoint) -> String {
    let side = match point.y {
        0 => "your backrow",
        1 => "your Units",
        2 => "enemy Units",
        _ => "enemy backrow",
    };
    format!("Lane {}, {side}", point.x + 1)
}

/// R831003, R1704000: the Discover of the next cell, drawn as the prompt opens, then "done" once a cell
/// is picked where the rules allow it.
fn ask_cell(points: Vec<PapayaPoint>, rules: PapayaRules) -> Effect {
    Effect::new("papayaCell", move |ctx| {
        let offers = if ctx.radiant {
            PAPAYA_OFFERS
        } else {
            rules.base_offers
        };
        let used: Vec<i32> = points.iter().map(|point| point.x).collect();
        let free: Vec<PapayaPoint> = (0..PAPAYA_LANES)
            .filter(|x| !used.contains(x))
            .flat_map(|x| (0..PAPAYA_ROWS).map(move |y| PapayaPoint { x, y }))
            .collect();
        let mut offered: Vec<PapayaPoint> = ctx
            .sink
            .rng
            .shuffle(&free)
            .into_iter()
            .take(offers.max(0) as usize)
            .collect();
        offered.sort_by_key(|point| (point.x, point.y));
        let mut options: Vec<String> = offered.iter().map(cell_option).collect();
        let mut labels: serde_json::Map<String, Value> = offered
            .iter()
            .map(|point| (cell_option(point), json!(cell_label(point))))
            .collect();
        if rules.done && !points.is_empty() {
            options.push(PAPAYA_DONE.to_string());
            labels.insert(PAPAYA_DONE.to_string(), json!("Done"));
        }
        let prompt = if points.is_empty() {
            "KY's Papaya: Discover a cell for the curve"
        } else if rules.done {
            "KY's Papaya: Discover another cell, or done"
        } else {
            "KY's Papaya: Discover another cell"
        };
        let ask = choose_mode(json_as(json!({
            "step": PAPAYA_STEP,
            "options": options,
            "labels": labels,
            "prompt": prompt,
            "data": { PAPAYA_POINTS_KEY: points },
        })));
        (ask.apply)(ctx);
    })
}

/// R422: the card's on-resolve hook — the first cell prompt.
pub fn papaya_begin() -> Vec<Effect> {
    papaya_begin_with(PAPAYA_RULES)
}

/// R422, R1704000: `papaya_begin` under `rules`.
pub fn papaya_begin_with(rules: PapayaRules) -> Vec<Effect> {
    vec![ask_cell(Vec::new(), rules)]
}

/// The cells a resume carried, read back defensively.
fn held_points(data: Option<&Value>) -> Vec<PapayaPoint> {
    data.and_then(|value| serde_json::from_value::<Vec<PapayaPoint>>(value.clone()).ok())
        .unwrap_or_default()
}

/// R422, R831003, R1704000: an answer: the next Discover while cells are being added, else the curve.
pub fn papaya_answered(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    papaya_answered_with(PAPAYA_RULES, ctx)
}

/// `papaya_answered` under `rules`.
pub fn papaya_answered_with(rules: PapayaRules, ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let held = held_points(ctx.data.get(PAPAYA_POINTS_KEY));
    let used: Vec<i32> = held.iter().map(|point| point.x).collect();
    let picked: Vec<PapayaPoint> = chosen_options(ctx)
        .iter()
        .filter_map(|option| option_cell(option))
        .filter(|point| !used.contains(&point.x))
        .collect();
    let mut points = held;
    points.extend(picked.iter().copied());
    if !picked.is_empty() && (points.len() as i32) < PAPAYA_MAX_CELLS {
        return vec![ask_cell(points, rules)];
    }
    if points.is_empty() {
        return Vec::new();
    }
    let player = ctx.controller;
    let cells: Vec<ZoneRef> = cells_on_curve(&points)
        .iter()
        .map(|cell| zone_of_point(player, cell))
        .collect();
    let mut effects = vec![Effect::new("curveDrawn", move |ctx| {
        ctx.events.push(GameEvent::CurveDrawn {
            player,
            cells: cells.clone(),
        });
    })];
    effects.extend(
        cards_on_curve(ctx.sink.state, ctx.controller, &points, ctx.radiant)
            .into_iter()
            .map(|instance_id| {
                let target = json!({ "target": { "of": "instance", "instanceId": instance_id } });
                if rules.destroys {
                    destroy(json_as(target))
                } else {
                    exile(json_as(target))
                }
            }),
    );
    effects
}
