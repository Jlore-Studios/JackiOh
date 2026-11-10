//! GIGA permanents (SPEC §3, §6.1; issue #3546, R3546000–R3546004): a permanent two lanes
//! wide in both rows. It stands in its anchor (lane L of its row) and covers the other row's lane L
//! and both rows' lane L+1, four zones nothing else enters. Lane rules read it in both its lanes.
//!
//! The geometry here never calls `zones::is_empty`: `zones` reads `covering` from its own
//! occupancy, so this module stays on `card_at` and the caller's `room` predicate.

use crate::config::GIGA_LANES;
use crate::state::{CardInstance, GameState};
use crate::wire::{PlayerId, Row, Zone};
use crate::zones::{ZoneSlot, card_at, is_open, row_size, slot_of, takes_move};

/// R3546000: whether this card fills four zones.
pub fn is_giga(card: &CardInstance) -> bool {
    card.giga == Some(true)
}

/// R3546002: whether this card enters the field GIGA: it prints the keyword where it is (§10.4),
/// granted in the hand or lent by an embiggened conditional keyword alike.
pub fn enters_giga(state: &GameState, card: &CardInstance) -> bool {
    crate::layers::unit_has(state, card, crate::wire::KeywordKind::Giga)
}

/// R3546001: the lanes this card is in for every rule that reads a lane in its row: both lanes
/// when GIGA on the field, its own lane otherwise, none off the field.
pub fn lanes_of(card: &CardInstance) -> Vec<i32> {
    let Zone::Field { lane, .. } = card.zone else {
        return Vec::new();
    };
    if is_giga(card) {
        (lane..lane + GIGA_LANES).collect()
    } else {
        vec![lane]
    }
}

fn other_row(row: Row) -> Row {
    if row == Row::Units {
        Row::Backrow
    } else {
        Row::Units
    }
}

/// R3546000: the four zones a GIGA anchored at `anchor` fills: the anchor, the other row's same
/// lane, and both rows of the next lane. `None` past the row's end.
pub fn spanned_slots(anchor: ZoneSlot) -> Option<Vec<ZoneSlot>> {
    let next = anchor.lane + 1;
    if anchor.lane < 1 || next > row_size(anchor.row) {
        return None;
    }
    let other = other_row(anchor.row);
    Some(vec![
        anchor,
        ZoneSlot {
            player: anchor.player,
            row: other,
            lane: anchor.lane,
        },
        ZoneSlot {
            player: anchor.player,
            row: anchor.row,
            lane: next,
        },
        ZoneSlot {
            player: anchor.player,
            row: other,
            lane: next,
        },
    ])
}

/// R3546000: the GIGA permanent whose span holds `slot`, if any: among the tops of `slot.player`'s
/// both rows at lanes `max(1, lane − GIGA_LANES + 1)..=lane`, the GIGA whose span holds it.
pub fn covering(state: &GameState, slot: ZoneSlot) -> Option<&CardInstance> {
    let first = (slot.lane - GIGA_LANES + 1).max(1);
    for lane in first..=slot.lane {
        for row in [Row::Units, Row::Backrow] {
            let at = ZoneSlot {
                player: slot.player,
                row,
                lane,
            };
            if let Some(top) = card_at(state, at)
                && is_giga(top)
                && let Some(anchor) = slot_of(state, top)
                && let Some(span) = spanned_slots(anchor)
                && span.contains(&slot)
            {
                return Some(top);
            }
        }
    }
    None
}

/// R3546001: the card a lane read in `slot`'s row meets there: the zone's own top, else a GIGA
/// covering it whose own row is this row (a GIGA Field Spell is no unit-zone occupant, and a GIGA
/// Unit none in the backrow).
pub fn occupant_at(state: &GameState, slot: ZoneSlot) -> Option<&CardInstance> {
    if let Some(top) = card_at(state, slot) {
        return Some(top);
    }
    covering(state, slot).filter(|found| slot_of(state, found).is_some_and(|anchor| anchor.row == slot.row))
}

/// R3546000: whether every zone a GIGA anchored at `anchor` would fill passes `room`.
pub fn fits(anchor: ZoneSlot, room: impl Fn(ZoneSlot) -> bool) -> bool {
    spanned_slots(anchor).is_some_and(|span| span.into_iter().all(room))
}

/// R3546000: the leftmost lane of `row` a GIGA fits in under `room`, if any.
pub fn first_anchor(player: PlayerId, row: Row, room: impl Fn(ZoneSlot) -> bool) -> Option<ZoneSlot> {
    (1..=row_size(row))
        .map(|lane| ZoneSlot { player, row, lane })
        .find(|anchor| fits(*anchor, &room))
}

/// R3546000: where a GIGA with no named lane enters: the leftmost fit a play may take (R64), else
/// the leftmost a move may (R688).
pub fn entry_anchor(state: &GameState, player: PlayerId, row: Row) -> Option<ZoneSlot> {
    first_anchor(player, row, |slot| is_open(state, slot))
        .or_else(|| first_anchor(player, row, |slot| takes_move(state, slot)))
}

/// R3546001: the zones beside this card in its own row: its side and row at one past either end
/// of its lanes, those on the board. A GIGA beside the target is met once, by id.
pub fn adjacent_slots(_state: &GameState, card: &CardInstance) -> Vec<ZoneSlot> {
    let Zone::Field { player, row, .. } = card.zone else {
        return Vec::new();
    };
    let lanes = lanes_of(card);
    let Some(first) = lanes.iter().min() else {
        return Vec::new();
    };
    let Some(last) = lanes.iter().max() else {
        return Vec::new();
    };
    [*first - 1, *last + 1]
        .into_iter()
        .filter(|lane| *lane >= 1 && *lane <= row_size(row))
        .map(|lane| ZoneSlot { player, row, lane })
        .collect()
}

/// R3546001: whether `a`'s lane meets `b`: some lane of `a` is in `b`'s lanes.
pub fn lanes_meet(a: &CardInstance, b: &CardInstance) -> bool {
    let lanes = lanes_of(b);
    lanes_of(a).iter().any(|lane| lanes.contains(lane))
}

/// R3546003: whether a GIGA `card` moved to `to` lands whole: every spanned slot takes a move.
pub fn lands_whole(state: &GameState, card: &CardInstance, to: ZoneSlot) -> bool {
    !is_giga(card) || fits(to, |slot| takes_move(state, slot))
}
