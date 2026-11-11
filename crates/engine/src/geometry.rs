//! Board geometry: the Black Hole and the slashed board (Stinkyscape+, issue #3554,
//! R3554000–R3554004). The Black Hole occupies the middle lane on both sides: anything standing
//! there is exiled, and at the start of each player's turn their permanents step one lane toward
//! it. The slashed board voids one lane and halves the rest, and a rule reaching across the slash
//! stops there.

use serde::{Deserialize, Serialize};

use crate::config::{BLACK_HOLE_CAPACITY, BLACK_HOLE_LANE, UNIT_ZONES};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, active_first};
use crate::wire::{PLAYER_IDS, PlayerId, Row, Zone, ZoneRef};
use crate::zones::ZoneSlot;

/// R3554003 (S+ #78 Slashed Dream Cries in Red): which half of a slashed board a card is on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum Half {
    Left,
    Right,
}

impl Half {
    /// The half as a prompt option and as stored JSON alike.
    pub fn option(self) -> &'static str {
        match self {
            Half::Left => "left",
            Half::Right => "right",
        }
    }

    /// The half an option or stored value names, if any.
    pub fn from_option(option: &str) -> Option<Half> {
        match option {
            "left" => Some(Half::Left),
            "right" => Some(Half::Right),
            _ => None,
        }
    }
}

/// R3554000 (S+ #77 Black Hole): the open hole — its lane, what it has swallowed, and whose
/// permanents it pulls (`None`: each player at the start of their own turn; `Some(p)`: only `p`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BlackHole {
    pub lane: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub swallowed: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pulls: Option<PlayerId>,
    /// R3561000: the permanents it closes after, as its card resolved with them; `None`:
    /// `BLACK_HOLE_CAPACITY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub capacity: Option<i32>,
}

impl BlackHole {
    /// What a fresh hole holds: nothing swallowed yet.
    pub fn new(lane: i32, pulls: Option<PlayerId>) -> BlackHole {
        BlackHole {
            lane,
            swallowed: None,
            pulls,
            capacity: None,
        }
    }

    /// How many permanents the hole has swallowed.
    pub fn swallowed_count(&self) -> i32 {
        self.swallowed.unwrap_or(0)
    }

    /// R3561000: the permanents it closes after swallowing.
    pub fn closes_at(&self) -> i32 {
        self.capacity.unwrap_or(BLACK_HOLE_CAPACITY)
    }
}

/// R3554007 (S+ #7 Churnwalker): which damage chain a stamp puts its Units in.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum ChainKind {
    Shared,
    Split,
}

/// R3554000: whether `slot` is the open hole's lane. Reads an `Option` only (perf budget).
pub fn in_hole(state: &GameState, slot: ZoneSlot) -> bool {
    state.black_hole.is_some_and(|hole| slot.lane == hole.lane)
}

/// R3554002: whether `slot` is the void lane the slash left. Reads an `Option` only.
pub fn is_void(state: &GameState, slot: ZoneSlot) -> bool {
    state.slashed_lane.is_some_and(|lane| slot.lane == lane)
}

/// R3554003: which half of a slashed board `lane` is on: `None` on an unslashed board and on the
/// slash lane itself, `Left` below the slash and `Right` above it.
pub fn half_of_lane(state: &GameState, lane: i32) -> Option<Half> {
    let slash = state.slashed_lane?;
    if lane == slash {
        None
    } else if lane < slash {
        Some(Half::Left)
    } else {
        Some(Half::Right)
    }
}

/// R3554003: which half `card` casts or reaches from: on the field, the half of its first lane (a
/// GIGA's anchor lane); in the resolving zone, the half its cast chose; otherwise none.
pub fn half_of_card(state: &GameState, card: &CardInstance) -> Option<Half> {
    match card.zone {
        Zone::Field { .. } => crate::giga::lanes_of(card)
            .first()
            .copied()
            .and_then(|lane| half_of_lane(state, lane)),
        Zone::Resolving { .. } => card.half,
        _ => None,
    }
}

/// R3554003: whether `lane` is on `half`: any lane is on no half (`None`).
pub fn lane_in_half(state: &GameState, lane: i32, half: Option<Half>) -> bool {
    match half {
        None => true,
        Some(half) => half_of_lane(state, lane) == Some(half),
    }
}

/// R3554003: whether an effect of `source` reaches `target` across the slash: with no slash, for a
/// target off the field, or from a source with no half, anything does; otherwise both halves must
/// match. Reads `Option`s only.
pub fn reaches(state: &GameState, source: Option<&CardInstance>, target: &CardInstance) -> bool {
    if state.slashed_lane.is_none() {
        return true;
    }
    if target.zone.z() != crate::wire::ZoneName::Field {
        return true;
    }
    let Some(source) = source else {
        return true;
    };
    let Some(from) = half_of_card(state, source) else {
        return true;
    };
    half_of_card(state, target) == Some(from)
}

/// R3554007: the Units of both sides in the damage chain `id` of `kind`: each side's acting Units
/// carrying that stamp. A Dormant card is in no chain until it awakens.
pub fn chain_members(state: &GameState, id: u32, kind: ChainKind) -> Vec<CardInstance> {
    PLAYER_IDS
        .iter()
        .flat_map(|player| crate::zones::active_units_of(state, *player))
        .filter(|unit| !crate::slumber::is_slumbering(unit))
        .filter(|unit| match kind {
            ChainKind::Shared => unit.shared_chain == Some(id),
            ChainKind::Split => unit.split_chain == Some(id),
        })
        .cloned()
        .collect()
}

/// R3554000: exile a whole field zone's pile through the exile pile, as `move_::exile_board` does.
/// True when something went.
pub fn exile_zone(sink: &mut EngineSink<'_>, slot: ZoneSlot) -> bool {
    let contents = crate::zones::zone_contents(sink.state, slot);
    if contents.is_empty() {
        return false;
    }
    for card in contents {
        // A snapshot: exiling a pile uncovers the cards beneath it.
        let Some(live) = crate::state::find_instance(sink.state, &card.id).cloned() else {
            continue;
        };
        crate::zones::exile_to_pile(sink, live);
    }
    true
}

/// Count one swallowed permanent toward the hole's capacity, closing it at its `closes_at`.
/// False once the hole is closed.
fn count_swallowed(state: &mut GameState) -> bool {
    let Some(mut hole) = state.black_hole else {
        return false;
    };
    let swallowed = hole.swallowed_count() + 1;
    if swallowed >= hole.closes_at() {
        state.black_hole = None;
    } else {
        hole.swallowed = Some(swallowed);
        state.black_hole = Some(hole);
    }
    state.black_hole.is_some()
}

/// R3554000: exile everything standing in the open hole's lane — each side's Units row, then its
/// backrow — a pile counting once, a Dormant top passed by, a GIGA met once, in `active_first`
/// order. Closes the hole at its `closes_at`. True when it swallowed anything.
pub fn swallow_standing(sink: &mut EngineSink<'_>) -> bool {
    if sink.state.black_hole.is_none() {
        return false;
    }
    let lane = sink
        .state
        .black_hole
        .map(|hole| hole.lane)
        .unwrap_or(BLACK_HOLE_LANE);
    let mut swallowed_any = false;
    let mut met: Vec<String> = Vec::new();
    for player in active_first(sink.state) {
        for row in [Row::Units, Row::Backrow] {
            let slot = ZoneSlot { player, row, lane };
            let Some(top) = crate::giga::occupant_at(sink.state, slot).cloned() else {
                continue;
            };
            // The hole passes Dormant cards by, and meets a GIGA once.
            if crate::slumber::is_slumbering(&top) || met.contains(&top.id) {
                continue;
            }
            met.push(top.id.clone());
            let Some(at) = crate::zones::slot_of(sink.state, &top) else {
                continue;
            };
            if exile_zone(sink, at) {
                swallowed_any = true;
                if !count_swallowed(sink.state) {
                    return true;
                }
            }
        }
    }
    swallowed_any
}

/// The lanes a pull walks, nearest the hole first: `hole - 1` down to 1, then `hole + 1` up to 5.
fn pull_order(hole: i32) -> Vec<i32> {
    let mut lanes: Vec<i32> = (1..hole).rev().collect();
    lanes.extend((hole + 1)..=UNIT_ZONES);
    lanes
}

/// Set a moved pile down bottom-up, as `rotation::rotate_rings` does, keeping its carried Unit.
fn place_contents(state: &mut GameState, cards: &[CardInstance], carried: Option<&str>, to: &ZoneSlot) {
    for (placed, card) in cards.iter().rev().enumerate() {
        let options = crate::zones::PlaceOnFieldOptions {
            stack: Some(placed > 0),
        };
        let landed = if carried == Some(card.id.as_str()) {
            crate::zones::place_on_field(state, &mut card.clone(), to, options)
        } else {
            crate::zones::place_standing_on_field(state, &mut card.clone(), to, options)
        };
        if !landed {
            panic!(
                "R3554001: the pull could not place {} in {} {} {}",
                card.id, to.player, to.row, to.lane
            );
        }
    }
}

/// R3554001: at the start of `player`'s turn the hole draws their permanents one lane toward it.
/// Each row walks the lanes nearest the hole first; a card whose shifted lanes reach the hole is
/// exiled and counted as in `swallow_standing`, one whose newly entered slots all take a move (or
/// are covered by the card itself) steps whole, and any other stays. Nothing here runs the state
/// check. Dormant tops and covered zones are passed by.
pub fn pull_at_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.result.is_some() {
        return;
    }
    let Some(hole) = sink.state.black_hole else {
        return;
    };
    if hole.pulls.is_some_and(|pulls| pulls != player) {
        return;
    }
    let lane = hole.lane;
    let mut met: Vec<String> = Vec::new();
    for row in [Row::Units, Row::Backrow] {
        for from in pull_order(lane) {
            if sink.state.black_hole.is_none() {
                return;
            }
            let slot = ZoneSlot {
                player,
                row,
                lane: from,
            };
            // A covered zone holds nothing of its own to pull.
            if crate::giga::covering(sink.state, slot).is_some()
                && crate::zones::card_at(sink.state, slot).is_none()
            {
                continue;
            }
            let Some(top) = crate::giga::occupant_at(sink.state, slot).cloned() else {
                continue;
            };
            if crate::slumber::is_slumbering(&top) || met.contains(&top.id) {
                continue;
            }
            met.push(top.id.clone());
            let Some(from_slot) = crate::zones::slot_of(sink.state, &top) else {
                continue;
            };
            // A GIGA steps whole from its anchor; anything else from the lane it was met in.
            let anchor = if crate::giga::is_giga(&top) {
                from_slot.lane
            } else {
                from
            };
            let step = if anchor < lane { 1 } else { -1 };
            let before: Vec<i32> = crate::giga::lanes_of(&top);
            let shifted: Vec<i32> = before.iter().map(|lane| lane + step).collect();
            if shifted.contains(&lane) {
                if exile_zone(sink, from_slot) {
                    count_swallowed(sink.state);
                }
                continue;
            }
            // The step's destination must take the move, save the slots the card covers itself.
            let to_slot = ZoneSlot {
                player,
                row: from_slot.row,
                lane: anchor + step,
            };
            let entry_ok = if crate::giga::is_giga(&top) {
                crate::giga::spanned_slots(to_slot).is_some_and(|span| {
                    span.into_iter().all(|slot| {
                        crate::zones::takes_move(sink.state, slot)
                            || crate::giga::covering(sink.state, slot).is_some_and(|found| found.id == top.id)
                    })
                })
            } else {
                crate::zones::takes_move(sink.state, to_slot)
            };
            if !entry_ok {
                continue;
            }
            // Move the pile whole, as a rotation does.
            let contents = crate::zones::zone_contents(sink.state, from_slot);
            let carried = crate::zones::carried_at(sink.state, from_slot).map(|unit| unit.id.clone());
            if contents.is_empty() {
                continue;
            }
            for card in &contents {
                crate::zones::remove_from_field(
                    sink.state,
                    card,
                    crate::zones::RemoveFromFieldOptions {
                        with_pile: Some(true),
                    },
                );
            }
            place_contents(sink.state, &contents, carried.as_deref(), &to_slot);
        }
    }
}

/// R3554005: the lattice cells on the line from `from` to `to`: with `g = gcd(|dx|, |dy|)` the
/// cells `from + k·(dx/g, dy/g)` for `k` in `1..=g` — exactly the points papaya's curve would meet,
/// and a vertical line works too. Rows read as one axis: p1's backrow 0, p1's Units 1, p2's Units 2,
/// p2's backrow 3.
pub fn line_cells(from: &ZoneRef, to: &ZoneRef) -> Vec<ZoneRef> {
    fn height(cell: &ZoneRef) -> i32 {
        match (cell.player, cell.row) {
            (PlayerId::P1, Row::Backrow) => 0,
            (PlayerId::P1, Row::Units) => 1,
            (PlayerId::P2, Row::Units) => 2,
            (PlayerId::P2, Row::Backrow) => 3,
        }
    }
    fn cell_of(lane: i32, height: i32) -> Option<ZoneRef> {
        let (player, row) = match height {
            0 => (PlayerId::P1, Row::Backrow),
            1 => (PlayerId::P1, Row::Units),
            2 => (PlayerId::P2, Row::Units),
            3 => (PlayerId::P2, Row::Backrow),
            _ => return None,
        };
        if !(1..=UNIT_ZONES).contains(&lane) {
            return None;
        }
        Some(ZoneRef { player, row, lane })
    }
    fn gcd(mut a: i32, mut b: i32) -> i32 {
        while b != 0 {
            let rest = a % b;
            a = b;
            b = rest;
        }
        a.abs()
    }
    let dx = to.lane - from.lane;
    let dy = height(to) - height(from);
    let steps = gcd(dx, dy);
    if steps <= 0 {
        return Vec::new();
    }
    (1..=steps)
        .filter_map(|k| cell_of(from.lane + k * dx / steps, height(from) + k * dy / steps))
        .collect()
}
