//! Zones, lanes, adjacency, the two rotation rings, locks and Stack piles (SPEC §3).
//! These are the only places a card changes zone; effects (M3-T1) call them and emit the events.
//!
//! Port of `packages/engine/src/zones.ts`. How TS's live objects map onto Rust ownership:
//!
//! - A reader returns references into the state (`card_at` → `Option<&CardInstance>`,
//!   `active_units_of` → `Vec<&CardInstance>`); `card_at_mut` and `pile_at_mut` are their mutable twins.
//! - A mover that takes a card off its zone (`move_to_zone`, `remove_from_any_zone`, `cease_to_exist`)
//!   takes it as `&mut CardInstance`: the value is first brought up to date with the card under its id
//!   in the state, then updated exactly as TS updated the live object, so the caller holds the card as
//!   it landed (`report_graveyard_landing` reads where). A card in no zone is taken as handed.
//! - A mover that sets a card down (`place_on_field`, `replace_in_zone`'s replacement,
//!   `fresh_face_down_id`) takes the card as `&mut CardInstance` as handed — a new card, or one already
//!   taken off its zone — and the state holds a copy of it as it then stands.
//! - A function that changes a card where it stands (`step_into_unit_zone`, `step_into_backrow`,
//!   `flicker_in_place`) takes `&CardInstance` and works on the card under that id in the state.
//! - A zone slot argument takes a `ZoneSlot` or a `&ZoneSlot` (`impl Into<ZoneSlot>`).
//! - TS's `registerGraveyardRedirect` hook is a direct call to `replacements::graveyard_redirect_for`
//!   (SURFACE §6.6: the registration hooks go).
//! - TS's optional `options` objects are `<Function>Options` structs deriving `Default`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brittle_count::{drop_spent_brittle, start_brittle_on_field};
use crate::catalog::def_of;
use crate::config::{BACKROW_ZONES, UNIT_ZONES};
use crate::faces::card_type_of;
use crate::own_library::show_to_owner;
use crate::script::EngineSink;
use crate::scripts::flags_of;
use crate::state::{
    CardInstance, EngineError, Exertion, GameState, HomeZone, Pile, PlayerState, QueuedTrigger,
    find_instance, find_instance_mut, rename_in_board_history,
};
use crate::stays::{note_field_exit, note_moved, note_uncovered};
use crate::wire::{
    AttackHealth, CardType, Counters, GameEvent, PLAYER_IDS, PlayerId, Position, RotationDirection, Row,
    Zone, ZoneName, ZoneRef, opponent_of,
};

/// `{ player, row, lane }`: a field zone (TS `ZoneSlot`), the same shape as the wire's `ZoneRef`.
pub type ZoneSlot = ZoneRef;

/// So a slot argument takes `slot` or `&slot` alike (`impl Into<ZoneSlot>`).
impl From<&ZoneRef> for ZoneRef {
    fn from(slot: &ZoneRef) -> ZoneRef {
        *slot
    }
}

/// A lane's index in its row, or `None` for a lane below 1.
fn lane_index(lane: i32) -> Option<usize> {
    usize::try_from(lane - 1).ok()
}

/// The card under `card`'s id as it stands in the state now, or `card` itself when it is in no zone
/// (TS held the live object; a caller holds a copy, which may be older than the state).
fn live_or(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone())
}

/// Make `row` long enough to hold `index` (JS grows an array assigned past its end).
fn ensure_len<T>(row: &mut Vec<Option<T>>, index: usize) {
    while row.len() <= index {
        row.push(None);
    }
}

pub fn row_size(row: Row) -> i32 {
    if row == Row::Units {
        UNIT_ZONES
    } else {
        BACKROW_ZONES
    }
}

/// §3.1, Classic #22: the midlane lanes of a board `lanes` wide — the center lane of an odd count,
/// both center lanes of an even one (R685). Locks never matter to it. The caller reads the count off
/// the board (a side's unit row length), never off `MID_LANE`, which no longer exists.
pub fn midlane_lanes(lanes: i32) -> Vec<i32> {
    if lanes <= 0 {
        return vec![];
    }
    if lanes % 2 == 1 {
        vec![(lanes + 1) / 2]
    } else {
        vec![lanes / 2, lanes / 2 + 1]
    }
}

/// R685: the midlane lanes of a player's own board, read off its unit row length. Card scripts
/// read the midlane through this (never `state.players[…]` themselves, M3-T1).
pub fn midlane_lanes_of(state: &GameState, player: PlayerId) -> Vec<i32> {
    midlane_lanes(state.players[player].units.len() as i32)
}

pub fn slots_of(player: PlayerId, row: Row) -> Vec<ZoneSlot> {
    // A loop, not `Array.from({ length })`: every unit read asks for slots through the layers, and
    // Array.from's generic path was a fifth of a long AI gate game's time (#188).
    let size = row_size(row);
    let mut slots: Vec<ZoneSlot> = Vec::with_capacity(size.max(0) as usize);
    for lane in 1..=size {
        slots.push(ZoneSlot { player, row, lane });
    }
    slots
}

/// §3.1: lane N-1 and N+1 on the same side and row, never across sides.
pub fn adjacent(slot: impl Into<ZoneSlot>) -> Vec<ZoneSlot> {
    let slot = slot.into();
    let size = row_size(slot.row);
    [slot.lane - 1, slot.lane + 1]
        .into_iter()
        .filter(|lane| *lane >= 1 && *lane <= size)
        .map(|lane| ZoneSlot {
            player: slot.player,
            row: slot.row,
            lane,
        })
        .collect()
}

/// R14: one ring per row. From the rotating player's seat it runs their lane 1 to 5, then the
/// opponent's lane 5 down to 1, and back. "Right" is one step forward along that order.
pub fn ring_order(row: Row, perspective: PlayerId) -> Vec<ZoneSlot> {
    let size = row_size(row);
    let mine = (0..size).map(|i| ZoneSlot {
        player: perspective,
        row,
        lane: i + 1,
    });
    let theirs = (0..size).map(|i| ZoneSlot {
        player: opponent_of(perspective),
        row,
        lane: size - i,
    });
    mine.chain(theirs).collect()
}

pub fn ring_neighbor(
    slot: impl Into<ZoneSlot>,
    direction: RotationDirection,
    perspective: PlayerId,
) -> ZoneSlot {
    let slot = slot.into();
    let ring = ring_order(slot.row, perspective);
    let Some(at) = ring
        .iter()
        .position(|other| other.player == slot.player && other.lane == slot.lane)
    else {
        panic!(
            "zone not on the {} ring: {} lane {}",
            slot.row, slot.player, slot.lane
        );
    };
    let step: i32 = if direction == RotationDirection::Right {
        1
    } else {
        -1
    };
    let len = ring.len() as i32;
    let next = (at as i32 + step + len) % len;
    match ring.get(next as usize) {
        Some(found) => *found,
        None => panic!("ring index out of range"),
    }
}

pub fn is_locked(state: &GameState, slot: impl Into<ZoneSlot>) -> bool {
    let slot = slot.into();
    lane_index(slot.lane)
        .and_then(|index| state.players[slot.player].locks.row(slot.row).get(index))
        .copied()
        == Some(true)
}

fn set_lock(state: &mut GameState, slot: ZoneSlot, value: bool) {
    let Some(index) = lane_index(slot.lane) else {
        return;
    };
    let flags = state.players[slot.player].locks.row_mut(slot.row);
    while flags.len() <= index {
        flags.push(false);
    }
    flags[index] = value;
}

pub fn lock_zone(state: &mut GameState, slot: impl Into<ZoneSlot>) {
    set_lock(state, slot.into(), true);
}

/// B5 E20: clear the flag. Its occupant, if any, is unaffected. R688: summons and moves enter Locked zones, so Unlock only re-opens the zone for plays.
pub fn unlock_zone(state: &mut GameState, slot: impl Into<ZoneSlot>) {
    set_lock(state, slot.into(), false);
}

pub fn pile_at(state: &GameState, slot: impl Into<ZoneSlot>) -> Option<&Pile> {
    let slot = slot.into();
    if slot.row != Row::Units {
        panic!("piles exist in the unit row only");
    }
    lane_index(slot.lane)
        .and_then(|index| state.players[slot.player].units.get(index))
        .and_then(Option::as_ref)
}

/// `pile_at`, mutably.
pub fn pile_at_mut(state: &mut GameState, slot: impl Into<ZoneSlot>) -> Option<&mut Pile> {
    let slot = slot.into();
    if slot.row != Row::Units {
        panic!("piles exist in the unit row only");
    }
    let index = lane_index(slot.lane)?;
    state.players[slot.player]
        .units
        .get_mut(index)
        .and_then(Option::as_mut)
}

/// The card that acts in this zone: the top of a Stack pile, or the backrow card (§3.2). In a backrow
/// zone that is the top of its pile (B5 E21) — and a carrier stays that card beneath the Unit it
/// carries, which stands in the zone as a Unit and never as its backrow card (R446, `carried_at`).
pub fn card_at(state: &GameState, slot: impl Into<ZoneSlot>) -> Option<&CardInstance> {
    let slot = slot.into();
    if slot.row == Row::Units {
        return pile_at(state, slot).and_then(|pile| pile.first());
    }
    lane_index(slot.lane)
        .and_then(|index| state.players[slot.player].backrow.get(index))
        .and_then(Option::as_ref)
}

/// `card_at`, mutably.
pub fn card_at_mut(state: &mut GameState, slot: impl Into<ZoneSlot>) -> Option<&mut CardInstance> {
    let slot = slot.into();
    if slot.row == Row::Units {
        return pile_at_mut(state, slot).and_then(|pile| pile.first_mut());
    }
    let index = lane_index(slot.lane)?;
    state.players[slot.player]
        .backrow
        .get_mut(index)
        .and_then(Option::as_mut)
}

pub fn is_empty(state: &GameState, slot: impl Into<ZoneSlot>) -> bool {
    let slot = slot.into();
    card_at(state, slot).is_none() && (slot.row == Row::Units || carried_at(state, slot).is_none())
}

// ---------------------------------------------------------------------------
// Backrow piles and carried Units (docs/classic-sets.md B5 E21, R446, R447)
// ---------------------------------------------------------------------------

/// B5 E21: the dormant cards beneath a backrow zone's top card, top first; empty when none.
pub fn beneath_at(state: &GameState, slot: impl Into<ZoneSlot>) -> &[CardInstance] {
    let slot = slot.into();
    if slot.row != Row::Backrow {
        return pile_at(state, slot).and_then(|pile| pile.get(1..)).unwrap_or(&[]);
    }
    lane_index(slot.lane)
        .and_then(|index| {
            state.players[slot.player]
                .backrow_piles
                .as_ref()
                .and_then(|piles| piles.get(index))
        })
        .map_or(&[], Vec::as_slice)
}

/// R446: the Unit a carrier in this backrow zone holds, or `None`.
pub fn carried_at(state: &GameState, slot: impl Into<ZoneSlot>) -> Option<&CardInstance> {
    let slot = slot.into();
    if slot.row != Row::Backrow {
        return None;
    }
    let index = lane_index(slot.lane)?;
    state.players[slot.player]
        .carried
        .as_ref()
        .and_then(|row| row.get(index))
        .and_then(Option::as_ref)
}

/// B5 E21, R446: a backrow card whose text lets a Unit be played on top of it (`staticFlags.carrier`,
/// or Classic+ #33 Ivory Tower's `fusesCarried`, R653). The flag is the card's text, so a Vanilla
/// carrier carries nothing more (§6.3, R115: `flags_of` reads nothing off a Vanilla instance).
pub fn is_carrier(state: &GameState, card: &CardInstance) -> bool {
    let flags = flags_of(state, card);
    flags.carrier == Some(true) || flags.fuses_carried == Some(true)
}

/// R653: where a carrier that fuses its Unit (`fusesCarried`) notes the Unit stacked onto it, by id, for
/// the rest of its stay. Memory, so R78 clears it when the card leaves the field, and a Fuse that keeps
/// the carrier keeps it (R77: it is the engine's entry, not a text's).
const STACKED_KEY: &str = "__stacked";

/// R653: the id of the Unit stacked onto this `fusesCarried` carrier on this stay, or `None` if none yet.
pub fn stacked_onto(card: &CardInstance) -> Option<&str> {
    card.memory.get(STACKED_KEY).and_then(Value::as_str)
}

/// R446: whether this card is a Unit standing on a carrier in a backrow zone.
pub fn is_carried(state: &GameState, card: &CardInstance) -> bool {
    let Zone::Field {
        player,
        row: Row::Backrow,
        lane,
    } = card.zone
    else {
        return false;
    };
    carried_at(
        state,
        ZoneSlot {
            player,
            row: Row::Backrow,
            lane,
        },
    )
    .is_some_and(|carried| carried.id == card.id)
}

/// R446: every Unit a carrier of this player's holds, in lane order.
pub fn carried_units_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    state.players[player]
        .carried
        .iter()
        .flatten()
        .filter_map(Option::as_ref)
        .collect()
}

/// R446: the backrow zones of `player`'s side a Unit they play may name — each zone whose acting card
/// is a carrier holding no Unit yet, and which takes a card at all: not Locked, not held for a card's
/// return (R64, B3.1 rule 6). `play_choices` offers and checks exactly these (`legal_zones_for`,
/// `refuse_zone`), so the list and the refusal cannot disagree.
pub fn carrier_zones_for(state: &GameState, player: PlayerId) -> Vec<ZoneSlot> {
    slots_of(player, Row::Backrow)
        .into_iter()
        .filter(|slot| why_cannot_carry(state, slot).is_ok())
        .collect()
}

/// R446: why a Unit played now could not name this backrow zone, or `Ok` when it can (SURFACE §4.4.9).
pub fn why_cannot_carry(state: &GameState, slot: impl Into<ZoneSlot>) -> Result<(), EngineError> {
    let slot = slot.into();
    if slot.row != Row::Backrow {
        return Err(EngineError::new("only a backrow zone carries a Unit"));
    }
    let top = match card_at(state, slot) {
        Some(top) if is_carrier(state, top) => top,
        _ => {
            return Err(EngineError::new(
                "that zone holds no card a Unit may be played on top of",
            ));
        }
    };
    if carried_at(state, slot).is_some() {
        return Err(EngineError::new("that card already carries a Unit"));
    }
    if flags_of(state, top).fuses_carried == Some(true) && stacked_onto(top).is_some() {
        return Err(EngineError::new("that card has taken its one Unit"));
    }
    if is_locked(state, slot) {
        return Err(EngineError::new("that zone is Locked"));
    }
    if is_reserved(state, slot) {
        return Err(EngineError::new("that zone is held for a card's return"));
    }
    Ok(())
}

/// `accepts_stack_card`'s options: `move_` is TS's `{ move: true }`, a move rather than a play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcceptsStackCardOptions {
    pub move_: Option<bool>,
}

/// §3.2, B5 E21: a zone a Stack card may enter although it is occupied. Occupancy is exactly what
/// Stack lifts, so what is left is what occupancy never covered — a zone held for a card's return
/// (R64, B3.1 rule 6) takes no Stack card either — plus, in a backrow zone, a carrier's Unit: a zone
/// carrying one takes nothing more (R446). A play's Stack entry still honors a Lock (R688: only plays
/// are refused one); a move's does not, via `{ move: true }`.
pub fn accepts_stack_card(
    state: &GameState,
    slot: impl Into<ZoneSlot>,
    options: AcceptsStackCardOptions,
) -> bool {
    let slot = slot.into();
    if is_reserved(state, slot) {
        return false;
    }
    if options.move_ != Some(true) && is_locked(state, slot) {
        return false;
    }
    slot.row == Row::Units || carried_at(state, slot).is_none()
}

/// MD-F12, R1281: whether this slot is an active stack base for `player`.
pub fn stack_base_at(state: &GameState, player: PlayerId, slot: &ZoneSlot) -> bool {
    slot.row == Row::Units
        && slot.player == player
        && card_at(state, *slot).is_some_and(|card| flags_of(state, card).stack_base == Some(true))
        && accepts_stack_card(state, slot, Default::default())
}

/// Everything in a zone, top card first, so a move that lifts whole zones (#52's rotation, #87's board
/// swap) sets each down whole (§3.2): a unit zone's pile, or a backrow zone's carried Unit, its top
/// card and the dormant cards beneath (B5 E21, R446). Copies, for the mover to set down.
pub fn zone_contents(state: &GameState, slot: impl Into<ZoneSlot>) -> Vec<CardInstance> {
    let slot = slot.into();
    if slot.row == Row::Units {
        return pile_at(state, slot).cloned().unwrap_or_default();
    }
    let mut out: Vec<CardInstance> = Vec::new();
    if let Some(carried) = carried_at(state, slot) {
        out.push(carried.clone());
    }
    if let Some(top) = card_at(state, slot) {
        out.push(top.clone());
    }
    out.extend(beneath_at(state, slot).iter().cloned());
    out
}

fn set_beneath(side: &mut PlayerState, lane: i32, cards: Vec<CardInstance>) {
    let Some(index) = lane_index(lane) else {
        return;
    };
    let mut piles = side
        .backrow_piles
        .take()
        .unwrap_or_else(|| vec![Vec::new(); BACKROW_ZONES.max(0) as usize]);
    while piles.len() <= index {
        piles.push(Vec::new());
    }
    piles[index] = cards;
    if piles.iter().all(Vec::is_empty) {
        side.backrow_piles = None;
    } else {
        side.backrow_piles = Some(piles);
    }
}

fn set_carried(side: &mut PlayerState, lane: i32, card: Option<CardInstance>) {
    let Some(index) = lane_index(lane) else {
        return;
    };
    let mut row = side
        .carried
        .take()
        .unwrap_or_else(|| vec![None; BACKROW_ZONES.max(0) as usize]);
    ensure_len(&mut row, index);
    row[index] = card;
    if row.iter().all(Option::is_none) {
        side.carried = None;
    } else {
        side.carried = Some(row);
    }
}

/// Whether a card's own face is a Unit's (§5.2, B2.7) — the one kind of card that stands on a carrier
/// (R446). Its face, not where it stands: an animated card in a unit zone is a Unit there (R383) but
/// never a Unit face, so it never lands on a carrier.
fn is_unit_face(state: &GameState, instance: &CardInstance) -> bool {
    // R1040: a Unit set face-down under Knowledge Breaker's Aura stands in the backrow as a Field Trap.
    if instance.set_as.is_some() {
        return false;
    }
    let def = def_of(Some(state), &instance.def_id);
    let face = if instance.radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_) == CardType::Unit
}

/// A zone a play, or a summon with no named zone, may take: empty and unlocked (§3.2). R688: a named
/// summon or a move may enter a Locked zone instead (`takes_move`, `place_on_field`); only plays refuse one.
pub fn is_open(state: &GameState, slot: impl Into<ZoneSlot>) -> bool {
    let slot = slot.into();
    is_empty(state, slot) && !is_locked(state, slot) && !is_reserved(state, slot)
}

/// R688: a zone a move may enter — empty and unreserved. Locked zones take moves; only plays refuse one.
pub fn takes_move(state: &GameState, slot: impl Into<ZoneSlot>) -> bool {
    let slot = slot.into();
    is_empty(state, slot) && !is_reserved(state, slot)
}

/// R64: a dying Reborn unit holds its zone until it comes back. B3.1 rule 6: so does an animated
/// "Animated on your turn" card its backrow zone, for its return at its controller's cleanup.
pub fn is_reserved(state: &GameState, slot: impl Into<ZoneSlot>) -> bool {
    let slot = slot.into();
    if state
        .reserved
        .iter()
        .any(|r| r.player == slot.player && r.row == slot.row && r.lane == slot.lane)
    {
        return true;
    }
    state.homes.iter().flatten().any(|home| {
        home.zone.player == slot.player && home.zone.row == slot.row && home.zone.lane == slot.lane
    })
}

/// B3.1 rule 6: the home zone held for this animated card, if any.
pub fn home_of<'a>(state: &'a GameState, instance_id: &str) -> Option<&'a HomeZone> {
    state
        .homes
        .iter()
        .flatten()
        .find(|home| home.instance_id == instance_id)
}

/// B3.1 rule 6: hold a backrow zone for an animated card's return. One home per card.
pub fn reserve_home(state: &mut GameState, zone: impl Into<ZoneSlot>, instance_id: &str) {
    let zone = zone.into();
    let mut homes: Vec<HomeZone> = state
        .homes
        .iter()
        .flatten()
        .filter(|home| home.instance_id != instance_id)
        .cloned()
        .collect();
    homes.push(HomeZone {
        instance_id: instance_id.to_string(),
        zone: ZoneRef {
            player: zone.player,
            row: zone.row,
            lane: zone.lane,
        },
    });
    state.homes = Some(homes);
}

/// B3.1 rule 6: the card's home is no longer held — it returned, or it left the field.
pub fn release_home(state: &mut GameState, instance_id: &str) {
    let Some(homes) = state.homes.as_ref() else {
        return;
    };
    let kept: Vec<HomeZone> = homes
        .iter()
        .filter(|home| home.instance_id != instance_id)
        .cloned()
        .collect();
    if kept.is_empty() {
        state.homes = None;
    } else {
        state.homes = Some(kept);
    }
}

pub fn reserve_zone(state: &mut GameState, slot: impl Into<ZoneSlot>) {
    let slot = slot.into();
    if !is_reserved(state, slot) {
        state.reserved.push(slot);
    }
}

pub fn release_zone(state: &mut GameState, slot: impl Into<ZoneSlot>) {
    let slot = slot.into();
    state
        .reserved
        .retain(|r| !(r.player == slot.player && r.row == slot.row && r.lane == slot.lane));
}

pub fn open_zones(state: &GameState, player: PlayerId, row: Row) -> Vec<ZoneSlot> {
    slots_of(player, row)
        .into_iter()
        .filter(|slot| is_open(state, slot))
        .collect()
}

/// R64: the leftmost open zone, or `None` when the row is full.
pub fn first_free_zone(state: &GameState, player: PlayerId, row: Row) -> Option<ZoneSlot> {
    open_zones(state, player, row).into_iter().next()
}

/// R64, R688: where a summon, a recruit or a move with no named zone enters — the leftmost open zone,
/// else the leftmost empty Locked one (a Lock refuses only plays), or `None` when the row is full. Plays
/// and casts keep `first_free_zone`.
pub fn first_entry_zone(state: &GameState, player: PlayerId, row: Row) -> Option<ZoneSlot> {
    first_free_zone(state, player, row).or_else(|| {
        slots_of(player, row)
            .into_iter()
            .find(|slot| takes_move(state, slot))
    })
}

pub fn zone_of(slot: impl Into<ZoneSlot>) -> Zone {
    let slot = slot.into();
    Zone::Field {
        player: slot.player,
        row: slot.row,
        lane: slot.lane,
    }
}

/// True when this def is a unit token, which ceases to exist off the field (R11).
pub fn is_unit_token(state: &GameState, instance: &CardInstance) -> bool {
    let def = def_of(Some(state), &instance.def_id);
    def.token && card_type_of(state, instance) == CardType::Unit
}

/// `place_on_field`'s options: `stack` lets a Stack card top an occupied zone (TS `{ stack? }`, which a
/// test builds with `json_as`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOnFieldOptions {
    pub stack: Option<bool>,
}

/// Where `place_on_field` sets the card down, decided before anything changes.
enum Landing {
    Units,
    Carried { stamp: bool },
    Backrow,
}

/// Put a card on the field. A Stack card may enter an occupied unit zone and becomes the top of the
/// pile; the card beneath keeps its damage and stops acting (§3.2). Returns false when the zone
/// cannot take it, leaving the state (and the card) untouched.
pub fn place_on_field(
    state: &mut GameState,
    instance: &mut CardInstance,
    slot: impl Into<ZoneSlot>,
    options: PlaceOnFieldOptions,
) -> bool {
    let slot = slot.into();
    let stack = options.stack == Some(true);
    // R688: a Lock refuses plays, never placements — summons, moves, rotations and restores all land.
    // Plays never reach here unvetted (`play_choices` offers and `play_steps` assigns unlocked zones only).
    if is_reserved(state, slot) {
        return false;
    }
    let Some(index) = lane_index(slot.lane) else {
        return false;
    };

    let landing = if slot.row == Row::Units {
        let existing = state.players[slot.player]
            .units
            .get(index)
            .is_some_and(Option::is_some);
        if existing && !stack {
            return false;
        }
        Landing::Units
    } else if is_unit_face(state, instance) {
        // R446: a Unit enters a backrow zone only on top of a carrier, which stays beneath it and keeps
        // acting there. A move that sets a whole zone down (`zone_contents`: a rotation, a board swap)
        // puts the carrier down first and its Unit back on it, whatever the carrier's text says by then.
        let Some(top) = state.players[slot.player]
            .backrow
            .get(index)
            .and_then(Option::as_ref)
        else {
            return false;
        };
        if carried_at(state, slot).is_some() {
            return false;
        }
        if !is_carrier(state, top) && !stack {
            return false;
        }
        // R653: the first Unit to stand on a carrier that fuses its Unit is the one it takes this stay.
        let stamp = flags_of(state, top).fuses_carried == Some(true) && stacked_onto(top).is_none();
        Landing::Carried { stamp }
    } else {
        // B5 E21: a Stack card may top an occupied backrow zone as it may a unit zone; the card beneath
        // goes dormant (§3.2). A zone carrying a Unit takes nothing more (R446).
        let existing = state.players[slot.player]
            .backrow
            .get(index)
            .is_some_and(Option::is_some);
        if existing && (!stack || carried_at(state, slot).is_some()) {
            return false;
        }
        Landing::Backrow
    };

    // R638: a move from one field zone to another (a steal, a swap, a rotation) is no arrival.
    let from_off_field = instance.zone.z() != ZoneName::Field;
    instance.controller = slot.player;
    instance.zone = zone_of(slot);
    if (slot.row == Row::Units || is_unit_face(state, instance)) && instance.position.is_none() {
        instance.position = Some(Position::Atk);
    }
    // B3.3 rule 1, R385, R638: a printed Brittle starts as its card enters the field, and a held count starts ticking.
    start_brittle_on_field(state, instance, from_off_field);

    let placed = instance.clone();
    match landing {
        Landing::Units => {
            let side = &mut state.players[slot.player];
            ensure_len(&mut side.units, index);
            let pile = match side.units[index].take() {
                None => vec![placed],
                Some(existing) => {
                    let mut pile = Vec::with_capacity(existing.len() + 1);
                    pile.push(placed);
                    pile.extend(existing);
                    pile
                }
            };
            side.units[index] = Some(pile);
        }
        Landing::Carried { stamp } => {
            let id = placed.id.clone();
            let side = &mut state.players[slot.player];
            set_carried(side, slot.lane, Some(placed));
            if stamp && let Some(Some(top)) = side.backrow.get_mut(index) {
                top.memory.insert(STACKED_KEY.to_string(), Value::String(id));
            }
        }
        Landing::Backrow => {
            let beneath: Vec<CardInstance> = beneath_at(state, slot).to_vec();
            let side = &mut state.players[slot.player];
            ensure_len(&mut side.backrow, index);
            if let Some(existing) = side.backrow[index].take() {
                let mut pile = Vec::with_capacity(beneath.len() + 1);
                pile.push(existing);
                pile.extend(beneath);
                set_beneath(side, slot.lane, pile);
            }
            side.backrow[index] = Some(placed);
        }
    }
    true
}

/// R638, R13: places a card beneath the top card of a unit zone pile.
/// This is no arrival (R638), and the card lies dormant (R13).
pub fn place_beneath_top(state: &mut GameState, instance: &mut CardInstance, slot: ZoneSlot) -> bool {
    if slot.row != Row::Units {
        return false;
    }
    let Some(pile) = pile_at_mut(state, slot) else {
        return false;
    };
    if pile.is_empty() {
        return false;
    }
    instance.controller = slot.player;
    instance.zone = zone_of(slot);
    pile.insert(1, instance.clone());
    true
}

/// R227: whether a card placed in this row lands face-down — a Trap or a Field Trap in a backrow
/// (§3.2, R33). A Field Spell lands face-up, and a Unit never reaches the backrow.
pub fn lands_face_down(state: &GameState, instance: &CardInstance, row: Row) -> bool {
    if row != Row::Backrow {
        return false;
    }
    let kind = card_type_of(state, instance);
    kind == CardType::Trap || kind == CardType::FieldTrap
}

/// R227: a card going face-down takes a fresh instance id, so the one handle the action protocol has
/// for a face-down card — a play's target, a prompt option's answer (R177) — is an id no player has
/// seen before, and an id seen while the card was public never names it again. The id is the next
/// number, as a new card's is; whether a card goes face-down is public (§10.8 shows the zone
/// occupied), so the number it takes says nothing either. Called on a card that is in no pile, just
/// before it is placed. Returns the id the card had, which the `cardPlayed` or `summoned` that
/// places it carries as `formerId` for the views to follow (R97).
pub fn fresh_face_down_id(state: &mut GameState, instance: &mut CardInstance) -> String {
    let former = instance.id.clone();
    instance.id = format!("c{}", state.next_id);
    state.next_id += 1;
    // R419: C+ #35's history names the card by the id it has now.
    rename_in_board_history(state, &former, &instance.id);
    former
}

/// Where `replace_in_zone` finds the old card.
enum OldPlace {
    Units,
    Backrow,
    Carried,
    Beneath,
}

/// §6.3 Replace on the field: the new card takes the old one's place — the same zone, and the same
/// place in a Stack pile — under the same controller. That is no summon, so §3.2's Lock ("the zone
/// accepts no summons … the current occupant is unaffected") and R64's reservation do not refuse it:
/// the zone was occupied before and is occupied after. Returns false, changing nothing, when the old
/// card is not on the field. The old card is left pointing at its zone for the caller to retire.
pub fn replace_in_zone(state: &mut GameState, old: &CardInstance, replacement: &mut CardInstance) -> bool {
    let old = &live_or(state, old);
    let Zone::Field { player, row, lane } = old.zone else {
        return false;
    };
    let Some(index) = lane_index(lane) else {
        return false;
    };
    let slot = ZoneSlot {
        player,
        row: Row::Backrow,
        lane,
    };
    let place = {
        let side = &state.players[player];
        if row == Row::Units {
            let Some(pile) = side.units.get(index).and_then(Option::as_ref) else {
                return false;
            };
            if !pile.iter().any(|card| card.id == old.id) {
                return false;
            }
            OldPlace::Units
        } else if side
            .backrow
            .get(index)
            .and_then(Option::as_ref)
            .is_some_and(|card| card.id == old.id)
        {
            OldPlace::Backrow
        } else if side
            .carried
            .as_ref()
            .and_then(|carried| carried.get(index))
            .and_then(Option::as_ref)
            .is_some_and(|card| card.id == old.id)
        {
            // R446: the carried Unit's place on its carrier.
            OldPlace::Carried
        } else {
            // B5 E21: a dormant card's place in a backrow pile.
            if !beneath_at(state, slot).iter().any(|card| card.id == old.id) {
                return false;
            }
            OldPlace::Beneath
        }
    };

    replacement.controller = player;
    replacement.zone = old.zone.clone();
    if (row == Row::Units || is_unit_face(state, replacement)) && replacement.position.is_none() {
        replacement.position = Some(Position::Atk);
    }
    // B3.3 rule 1, R385: the new card has entered the field, so its printed Brittle starts.
    start_brittle_on_field(state, replacement, true);

    let placed = replacement.clone();
    match place {
        OldPlace::Units => {
            if let Some(Some(pile)) = state.players[player].units.get_mut(index) {
                for card in pile.iter_mut() {
                    if card.id == old.id {
                        *card = placed.clone();
                    }
                }
            }
        }
        OldPlace::Backrow => {
            state.players[player].backrow[index] = Some(placed);
        }
        OldPlace::Carried => {
            set_carried(&mut state.players[player], lane, Some(placed));
        }
        OldPlace::Beneath => {
            let beneath: Vec<CardInstance> = beneath_at(state, slot)
                .iter()
                .map(|card| {
                    if card.id == old.id {
                        placed.clone()
                    } else {
                        card.clone()
                    }
                })
                .collect();
            set_beneath(&mut state.players[player], lane, beneath);
        }
    }
    true
}

/// `remove_from_field`'s options: `with_pile` is a move that lifts whole piles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemoveFromFieldOptions {
    pub with_pile: Option<bool>,
}

/// Take a card off the field; the card beneath a Stack resumes acting (§3.2), which is noted against
/// the card that left (`stays::note_uncovered`, R212): no event reports a resume. `with_pile` is for a
/// move that lifts whole piles and sets each down whole elsewhere (#87's board swap, #52's rotation):
/// its cards come off one at a time, but nothing beneath any of them resumes, so no resume is noted.
pub fn remove_from_field(
    state: &mut GameState,
    instance: &CardInstance,
    options: RemoveFromFieldOptions,
) -> bool {
    let with_pile = options.with_pile == Some(true);
    for player in PLAYER_IDS {
        let units = state.players[player].units.len();
        for i in 0..units {
            let Some(pile) = state.players[player].units[i].as_ref() else {
                continue;
            };
            let Some(at) = pile.iter().position(|card| card.id == instance.id) else {
                continue;
            };
            let rest: Vec<CardInstance> = pile
                .iter()
                .filter(|card| card.id != instance.id)
                .cloned()
                .collect();
            let resumed = if at == 0 && !with_pile {
                rest.first().map(|card| card.id.clone())
            } else {
                None
            };
            state.players[player].units[i] = if rest.is_empty() { None } else { Some(rest) };
            note_uncovered(state, &instance.id, resumed.as_deref());
            return true;
        }
        let backrow = state.players[player].backrow.len();
        for i in 0..backrow {
            let lane = i as i32 + 1;
            let slot = ZoneSlot {
                player,
                row: Row::Backrow,
                lane,
            };
            if state.players[player].backrow[i]
                .as_ref()
                .is_some_and(|card| card.id == instance.id)
            {
                // B5 E21: the card beneath a backrow pile's top resumes, as in a unit pile (§3.2, R212).
                let mut beneath: Vec<CardInstance> = beneath_at(state, slot).to_vec();
                let resumed = if beneath.is_empty() {
                    None
                } else {
                    Some(beneath.remove(0))
                };
                let resumed_id = resumed.as_ref().map(|card| card.id.clone());
                let side = &mut state.players[player];
                side.backrow[i] = resumed;
                set_beneath(side, lane, beneath);
                let noted = if with_pile { None } else { resumed_id };
                note_uncovered(state, &instance.id, noted.as_deref());
                return true;
            }
            if state.players[player]
                .carried
                .as_ref()
                .and_then(|carried| carried.get(i))
                .and_then(Option::as_ref)
                .is_some_and(|card| card.id == instance.id)
            {
                set_carried(&mut state.players[player], lane, None);
                note_uncovered(state, &instance.id, None);
                return true;
            }
            let beneath = beneath_at(state, slot);
            if beneath.iter().any(|card| card.id == instance.id) {
                let kept: Vec<CardInstance> = beneath
                    .iter()
                    .filter(|card| card.id != instance.id)
                    .cloned()
                    .collect();
                set_beneath(&mut state.players[player], lane, kept);
                note_uncovered(state, &instance.id, None);
                return true;
            }
        }
    }
    false
}

/// The off-field zones a card can be moved to (TS `OffFieldZone`: "hand" | "library" | "graveyard" |
/// "exile"). Written by hand rather than with `wire::string_union!`: it is no wire type, and the macro
/// would export it to the web's generated types (SURFACE §5.1).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum OffFieldZone {
    Hand,
    Library,
    Graveyard,
    Exile,
}

impl OffFieldZone {
    /// Every off-field zone, in TS's order (`removeFromAnyZone` searches them so).
    pub const ALL: &'static [OffFieldZone] = &[
        OffFieldZone::Hand,
        OffFieldZone::Library,
        OffFieldZone::Graveyard,
        OffFieldZone::Exile,
    ];

    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            OffFieldZone::Hand => "hand",
            OffFieldZone::Library => "library",
            OffFieldZone::Graveyard => "graveyard",
            OffFieldZone::Exile => "exile",
        }
    }

    /// The zone's name, as `Zone::z` reads it.
    pub fn zone_name(self) -> ZoneName {
        match self {
            OffFieldZone::Hand => ZoneName::Hand,
            OffFieldZone::Library => ZoneName::Library,
            OffFieldZone::Graveyard => ZoneName::Graveyard,
            OffFieldZone::Exile => ZoneName::Exile,
        }
    }

    /// The zone itself, on `player`'s side.
    pub fn zone_for(self, player: PlayerId) -> Zone {
        match self {
            OffFieldZone::Hand => Zone::Hand { player },
            OffFieldZone::Library => Zone::Library { player },
            OffFieldZone::Graveyard => Zone::Graveyard { player },
            OffFieldZone::Exile => Zone::Exile { player },
        }
    }
}

/// The pile a card lands in; off the field it always belongs to its owner (R12).
fn pile_for(side: &mut PlayerState, zone: OffFieldZone) -> &mut Vec<CardInstance> {
    match zone {
        OffFieldZone::Hand => &mut side.hand,
        OffFieldZone::Library => &mut side.library,
        OffFieldZone::Graveyard => &mut side.graveyard,
        OffFieldZone::Exile => &mut side.exile,
    }
}

/// Take a card out of wherever it is. `instance` is first brought up to date with the card under its id
/// in the state (TS's live object), and TS's `delete instance.returnToHandAtEndOfTurn` on a card leaving
/// a graveyard is made on it, so the caller holds the card as it left.
pub fn remove_from_any_zone(state: &mut GameState, instance: &mut CardInstance) {
    *instance = live_or(state, instance);
    if remove_from_field(state, instance, RemoveFromFieldOptions::default()) {
        return;
    }
    for player in PLAYER_IDS {
        for zone in OffFieldZone::ALL.iter().copied() {
            let pile = pile_for(&mut state.players[player], zone);
            let Some(at) = pile.iter().position(|card| card.id == instance.id) else {
                continue;
            };
            pile.remove(at);
            // R212: a move of a card that left a pile's top ends that removal's Stack note.
            note_moved(state, &instance.id);
            // R1140: and a card leaving a hand ends its stay there, which a hand watch was aimed at.
            if zone == OffFieldZone::Hand {
                forget_hand_watch(state, &instance.id);
            }
            // R155: §5.1's end-of-turn return belongs to the Spell its own play landed in the graveyard
            // (§10.5 step 7). A card that leaves the graveyard has spent that landing, so whatever puts
            // it back there this turn — a discard (#76), a burn — is no play of its, and it stays (R153).
            if zone == OffFieldZone::Graveyard {
                instance.return_to_hand_at_end_of_turn = None;
                // R429, R766: and the price noted for that return goes with it.
                crate::resolve::forget_return_price(instance);
            }
            return;
        }
        // §10.5 step 4 parks a card here between its play and its destination. Without this a Spell
        // moved from `resolving` to the graveyard would be left in both piles, i.e. two live copies
        // of one instance — so the resolving pile is searched like any other.
        let resolving = &mut state.players[player].resolving;
        if let Some(at) = resolving.iter().position(|card| card.id == instance.id) {
            resolving.remove(at);
            note_moved(state, &instance.id);
            return;
        }
    }
}

/// R78: leaving the field resets an instance, while costMod, costOverride and radiant persist. R215
/// applies the same reset to a hand or library card that reaches a graveyard or exile, and to a card
/// leaving the resolving zone once its play is over. The price is not touched here: a card bounced to a
/// hand or shuffled into a library keeps it, and R766 takes it off only a card that reaches a graveyard
/// or an exile pile (`reset_price`).
///
/// Patch v0.2.0 adds three more that persist in every zone (R385, R386, B5 E39): `tuning` (what
/// Degrade, Upgrade and KY's Constant changed), `brittle` (the Brittle count) and `enchantments` —
/// none of them is touched here. The one exception is a Brittle count that has crumbled its card, which
/// is spent and goes (R441, `brittle_count::drop_spent_brittle`). ME-CN's `chinese` flag persists the
/// same way: nothing removes it (R1300). So do the tags an effect granted (R923).
pub fn reset_instance(instance: &mut CardInstance) {
    drop_spent_brittle(instance);
    instance.damage = 0;
    instance.buffs = AttackHealth { attack: 0, health: 0 };
    instance.granted_keywords = Vec::new();
    // ME-GRANT (MD-D13): granted Death abilities are lost with the card leaving the field (R78).
    instance.grants = None;
    instance.vanilla = false;
    instance.counters = Counters::default();
    instance.memory = Default::default();
    instance.exertion = Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    };
    instance.controller = instance.owner;
    instance.position = None;
    instance.summoned_turn = None;
    instance.stats_override = None;
    instance.armor_override = None;
    instance.taunt_suppressed_turn = None;
    instance.face_up = None;
    instance.revealed = None;
    instance.last_damaged_by = None;
    instance.x = None;
    instance.embiggened = None;
    instance.divine_shield_spent = None;
    instance.marked_destroyed = None;
    instance.marked_exiled = None;
    instance.reborn_spent = None;
    // B5 E35: Berserk is a status of the unit on the field, lost as it leaves (R78).
    instance.berserk = None;
    // R1040, R1045: a card set face-down as a Trap is its printed card again once it leaves the field.
    instance.set_as = None;
}

/// R766 (#473): a card that reaches a graveyard or an exile pile is its printed card again, its price
/// included — `costMod` (a discount, a surcharge, a Degrade's or Upgrade's cost step, KY's Constant's
/// cost) and `costOverride` (a "(0)" given in a hand) both go, so it costs its printed cost (R65), or a
/// fused card its fused definition's (R77). Its Radiant face, its `tuning`, its Brittle and times-played
/// counts, its enchantments, its `chinese` flag (R1300) and its granted tags (R923) are what the card
/// is or what happened to it, not a price, and stay.
pub fn reset_price(instance: &mut CardInstance) {
    instance.cost_mod = 0;
    instance.cost_override = None;
}

/// R174: drop the delayed effects aimed at a card that is leaving the field (`DelayedEffect.watch`).
/// The card that may later stand in the same zone under the same id — bounced and replayed, or back
/// through Reborn — is a new arrival (R78, R83), and an effect aimed at the old one fizzles (R76).
fn forget_watchers(state: &mut GameState, instance_id: &str) {
    if !state
        .delayed
        .iter()
        .any(|effect| effect.watch.as_deref() == Some(instance_id))
    {
        return;
    }
    state
        .delayed
        .retain(|effect| effect.watch.as_deref() != Some(instance_id));
}

/// R1140: a card leaving a hand ends its stay there. Every delayed effect watching it there
/// (`DelayedEffect.handWatch`, Meditative #76) stops watching it, and one left watching nothing is
/// dropped, so a card that comes back to a hand is a new stay nobody watches (R174) and its mark goes
/// at the next sweep (`marks::sweep_marks`). Called by every removal from a hand: `remove_from_any_zone`
/// and a play taking the card out of the hand (`play_steps::take_from_play_source`).
pub fn forget_hand_watch(state: &mut GameState, instance_id: &str) {
    if !state.delayed.iter().any(|effect| {
        effect
            .hand_watch
            .as_ref()
            .is_some_and(|ids| ids.iter().any(|id| id == instance_id))
    }) {
        return;
    }
    for effect in &mut state.delayed {
        if let Some(ids) = effect.hand_watch.as_mut() {
            ids.retain(|id| id != instance_id);
        }
    }
    state
        .delayed
        .retain(|effect| effect.hand_watch.as_ref().is_none_or(|ids| !ids.is_empty()));
}

/// The zones a queue entry names when the card answered from the field (`triggers::queue_trigger`).
const FIELD_TRIGGER_ZONES: &[&str] = &["field", "backrow"];

/// R174: drop the triggers and turn hooks this card queued while it stood on the field. They belong
/// to that stay: a Reborn body or a replayed card under the same id is a reset instance that has
/// entered the field again (R78, R83), so an entry queued before it left — #91's Plague Counter for the
/// hit that killed it, #37's start-of-turn hook queued before it died — never acts on what came
/// back. Without Reborn the entry already fizzled, because a card in a graveyard answers none of
/// its field triggers (R153); this makes the card that returns answer none of them either.
fn forget_queued_triggers(state: &mut GameState, instance_id: &str) {
    let owned = |entry: &QueuedTrigger| {
        entry.instance_id == instance_id
            && entry
                .resume
                .data
                .get("zone")
                .and_then(Value::as_str)
                .is_some_and(|zone| FIELD_TRIGGER_ZONES.contains(&zone))
    };
    if !state.trigger_queue.iter().any(owned) {
        return;
    }
    state.trigger_queue.retain(|entry| !owned(entry));
}

/// R86: a card ceases to exist — replaced by a Transform (R35), fused away (R77), or a unit token
/// leaving the field (R11) — and is in no pile afterwards, which is the `{ z: "gone" }` zone. One
/// that ceases to exist ON the field has left it, as a destroyed or bounced card has (R174): the
/// departure is counted, so a trap later in the same dispatch meets a play the first Sheepish turned
/// into a Sheep as a card no longer in play (`traps::standing_event`), and the delayed effects and
/// queued triggers aimed at that stay end with it, as `move_to_zone` ends them for a card that lands.
pub fn cease_to_exist(state: &mut GameState, instance: &mut CardInstance) {
    *instance = live_or(state, instance);
    let was_on_field = instance.zone.z() == ZoneName::Field;
    remove_from_any_zone(state, instance);
    if was_on_field {
        left_the_field(state, &instance.id);
    }
    instance.zone = Zone::Gone {
        player: instance.owner,
    };
}

/// R174: what a card leaving the field ends — the stay every effect aimed at it was aimed at, the
/// delayed effects watching it, the triggers it queued there — and, B3.1 rule 6, the home zone an
/// animated card held for its return: it will not return from a graveyard, a hand or exile.
fn left_the_field(state: &mut GameState, instance_id: &str) {
    note_field_exit(state, instance_id);
    forget_watchers(state, instance_id);
    forget_queued_triggers(state, instance_id);
    release_home(state, instance_id);
}

/// §2.3: X and the embiggen price are chosen at play time and stored on the played instance, and
/// R65 has an X-cost card cost 0 and an embiggen card its base price everywhere outside play. So the
/// choice ends with the play: a Spell that leaves the resolving zone (to its graveyard, to exile, or
/// straight to a hand) drops it, as R78's reset drops it from a permanent leaving the field. Without
/// this #24 Efficiency Dividend returned to hand at the X it was last played for.
fn end_play_choices(instance: &mut CardInstance) {
    instance.x = None;
    instance.embiggened = None;
}

/// What `move_to_zone` did (TS `MoveResult`: "moved" | "vanished" | "replaced"; by hand, as
/// `OffFieldZone` is). `Replaced` (B5 E5, R460): the card was on its way to a graveyard and a
/// replacement sent it elsewhere — its exile pile, or the bottom of its library — so it is not in the
/// graveyard, and `report_graveyard_landing` names where it went.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum MoveResult {
    Moved,
    Vanished,
    Replaced,
}

impl MoveResult {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            MoveResult::Moved => "moved",
            MoveResult::Vanished => "vanished",
            MoveResult::Replaced => "replaced",
        }
    }
}

// ---- B5 E5: "would go to a graveyard" (damage and combat) ----

/// Where a replacement sends a card that would go to a graveyard (B5 E5; Classic #28, #50, #60): TS's
/// `{ to: "exile" } | { to: "library"; position: "bottom" }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraveyardRedirect {
    /// `{ to: "exile" }`.
    Exile,
    /// `{ to: "library", position: "bottom" }`.
    LibraryBottom,
}

/// B5 E5: the event a move toward a graveyard reports once the card has landed, wherever that was —
/// `enteredGraveyard` in the graveyard, `exiled` when a replacement exiled it, `shuffledIn` at the
/// bottom of its library — and nothing for a unit token that ceased to exist (R11). Every engine path
/// that sends a card to a graveyard reports its landing here rather than assuming the graveyard.
pub fn report_graveyard_landing(sink: &mut EngineSink<'_>, instance: &CardInstance, result: MoveResult) {
    if result == MoveResult::Vanished {
        return;
    }
    let instance_id = instance.id.clone();
    let def_id = instance.def_id.clone();
    let owner = instance.owner;
    match instance.zone.z() {
        ZoneName::Graveyard => sink.events.push(GameEvent::EnteredGraveyard {
            instance_id,
            def_id,
            owner,
        }),
        ZoneName::Exile => sink.events.push(GameEvent::Exiled {
            instance_id,
            def_id,
            owner,
        }),
        ZoneName::Library => {
            let position = sink.state.players[owner]
                .library
                .iter()
                .position(|card| card.id == instance.id)
                .map_or(-1, |at| at as i32);
            sink.events.push(GameEvent::ShuffledIn {
                player: owner,
                instance_id,
                def_id,
                position,
            });
        }
        _ => {}
    }
}

/// Where `move_to_zone` puts a card in a library (TS `"top" | "bottom" | number`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryPosition {
    Top,
    Bottom,
    /// An index from the top, clamped to the library.
    At(i32),
}

/// `move_to_zone`'s options: where in a library, and `keep_state` to skip R78's reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveToZoneOptions {
    pub position: Option<LibraryPosition>,
    pub keep_state: Option<bool>,
}

/// Move a card to one of its owner's off-field zones. Unit tokens cease to exist instead (R11),
/// and a unit-token card leaving hand or library other than by being drawn or played does too.
///
/// R151's arrival hook is deliberately NOT here, although this is the single point every zone change
/// goes through: the roll a card makes as it arrives needs the match rng, and this function takes a
/// `GameState`, which holds only the seed and the cursor `reduce` stores between actions. Building an
/// rng from those mid-action would repeat draws the action's own rng has already taken and would have
/// its advanced cursor thrown away by `reduce`'s `next.rng_cursor = sink.rng.cursor()`. The hook lives
/// one layer up, on the sink-holding funnels every hand and library arrival passes through —
/// `draw::add_to_hand` and `draw::shuffle_into_library` (`run_arrival_hooks` in `draw.rs`).
pub fn move_to_zone(
    state: &mut GameState,
    instance: &mut CardInstance,
    zone: OffFieldZone,
    options: MoveToZoneOptions,
) -> MoveResult {
    *instance = live_or(state, instance);
    let from = instance.zone.z();
    let was_on_field = from == ZoneName::Field;
    let token = is_unit_token(state, instance);
    remove_from_any_zone(state, instance);
    // R174: leaving the field ends every delayed effect aimed at this card and every trigger it
    // queued there, whatever comes back, and ends the stay every effect aimed at it was aimed at.
    if was_on_field {
        left_the_field(state, &instance.id);
    }
    if from == ZoneName::Resolving {
        end_play_choices(instance);
    }

    // R11: a unit token ceases to exist when it leaves the field, and a unit-token card ceases to
    // exist when it would reach a graveyard or exile. One may live in a hand or library (#75) and
    // "ceases to exist if it leaves that zone other than by being drawn or played, burning
    // included" — a draw is this call moving it from the library to the hand, and a play never comes
    // through here (`play_steps` puts the card on the field or in `resolving` itself), so every other
    // move out of a hand or a library ends it: discarded, exiled, burned, shuffled back.
    // Staying put is not leaving, so a copy shuffled into the library it was made in lives (R34).
    let left_hand_or_library = (from == ZoneName::Hand || from == ZoneName::Library)
        && zone.zone_name() != from
        && !(from == ZoneName::Library && zone == OffFieldZone::Hand);
    if token
        && (was_on_field
            || from == ZoneName::Resolving
            || zone == OffFieldZone::Graveyard
            || zone == OffFieldZone::Exile
            || left_hand_or_library)
    {
        // R86: it ceased to exist, so it goes to "gone" rather than looking like an exiled card.
        instance.zone = Zone::Gone {
            player: instance.owner,
        };
        return MoveResult::Vanished;
    }

    // R215: a card that reaches a graveyard or an exile pile from a hand or a library is reset too, so
    // what comes back from there is the printed card (#89's hand buffs, #98's rolled power, R151) —
    // R78's reset, with `radiant` and the rest of what R78 keeps in every zone left alone.
    // So is a card that lands from the resolving zone (§10.5 step 7): its play is over, and a #95 an
    // earlier Call to Chaos cast (R87) carries no link of that chain (R28) back into a play of its own.
    let to_graveyard_or_exile = zone == OffFieldZone::Graveyard || zone == OffFieldZone::Exile;
    let pile_to_pile = (from == ZoneName::Hand || from == ZoneName::Library) && to_graveyard_or_exile;
    let landed = from == ZoneName::Resolving;
    if (was_on_field || pile_to_pile || landed) && options.keep_state != Some(true) {
        reset_instance(instance);
    }
    // R766 (#473): and a card that reaches a graveyard or an exile pile, from wherever it came, costs its
    // printed cost again: both cost layers go with it (`reset_price`). A price given on the way to a
    // hand or a library stays (R215's hand prices, R742's gift, R78's bounce). A card a replacement
    // sends elsewhere instead (below) lands as the graveyard would have had it land, price included.
    if to_graveyard_or_exile && from != zone.zone_name() && options.keep_state != Some(true) {
        reset_price(instance);
    }

    let owner = instance.owner;
    // B5 E5, R460: a card that would go to a graveyard may be sent elsewhere instead — asked now that it
    // has left the zone it was in. It lands the way the graveyard would have had it land (the reset
    // above), and an exile counts like any other (R55). A unit token never gets here (R11).
    let redirect = if zone == OffFieldZone::Graveyard {
        crate::replacements::graveyard_redirect_for(state, instance)
    } else {
        None
    };
    if let Some(redirect) = redirect {
        match redirect {
            GraveyardRedirect::Exile => {
                instance.zone = Zone::Exile { player: owner };
                state.players[owner].exile.push(instance.clone());
                state.counters.exiled += 1;
            }
            GraveyardRedirect::LibraryBottom => {
                instance.zone = Zone::Library { player: owner };
                // R311: it goes in openly, on its way to a pile both players read.
                show_to_owner(instance);
                state.players[owner].library.push(instance.clone());
            }
        }
        return MoveResult::Replaced;
    }

    instance.zone = zone.zone_for(owner);
    let placed = instance.clone();
    let pile = pile_for(&mut state.players[owner], zone);
    match (zone, options.position) {
        (OffFieldZone::Library, Some(at)) if at != LibraryPosition::Top => {
            let index = match at {
                LibraryPosition::At(n) => n.clamp(0, pile.len() as i32) as usize,
                _ => pile.len(),
            };
            pile.insert(index, placed);
        }
        (OffFieldZone::Library, _) => pile.insert(0, placed),
        _ => pile.push(placed),
    }
    MoveResult::Moved
}

/// Every unit that acts for this player: each unit zone's top card in lane order (§3.2), then the
/// Units its carriers hold, in backrow lane order — a carried Unit is a Unit for every rule (R446).
pub fn active_units_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    // Read straight off the rows, not through `slots_of` and `card_at`: the auras ask for this on every
    // unit read (`layers::aura_sources`), so it builds nothing it does not return (#188).
    let side = &state.players[player];
    let mut units: Vec<&CardInstance> = Vec::new();
    for lane in 1..=UNIT_ZONES {
        let top = lane_index(lane)
            .and_then(|index| side.units.get(index))
            .and_then(Option::as_ref)
            .and_then(|pile| pile.first());
        if let Some(top) = top {
            units.push(top);
        }
    }
    units.extend(side.carried.iter().flatten().filter_map(Option::as_ref));
    units
}

pub fn dormant_units_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    state.players[player]
        .units
        .iter()
        .flatten()
        .flat_map(|pile| pile.iter().skip(1))
        .collect()
}

/// §3.2, R13: a card dormant under a Stack pile — in a unit zone and not the top of its pile. It is
/// "not on the field for effects": nothing targets it, and an effect aimed at it fizzles (R174).
pub fn is_buried(state: &GameState, instance: &CardInstance) -> bool {
    let Zone::Field { player, row, lane } = instance.zone else {
        return false;
    };
    let slot = ZoneSlot { player, row, lane };
    // B5 E21: a backrow pile's dormant cards are buried the same way; the top and a carrier's Unit act.
    if row == Row::Backrow {
        return beneath_at(state, slot).iter().any(|card| card.id == instance.id);
    }
    card_at(state, slot).is_none_or(|top| top.id != instance.id)
}

/// §3.2, R446: whether a card acts on the field — the top of a unit pile, the top of a backrow zone, or
/// a Unit a carrier holds. A dormant card under either kind of pile does not.
pub fn acts_on_field(state: &GameState, instance: &CardInstance) -> bool {
    instance.zone.z() == ZoneName::Field && !is_buried(state, instance)
}

/// R64: "fill your board" takes every empty, unlocked unit zone, left to right. The caller makes
/// each card; this returns the zones to fill, in order.
pub fn fill_board_zones(state: &GameState, player: PlayerId) -> Vec<ZoneSlot> {
    open_zones(state, player, Row::Units)
}

pub fn slot_of(_state: &GameState, instance: &CardInstance) -> Option<ZoneSlot> {
    match instance.zone {
        Zone::Field { player, row, lane } => Some(ZoneSlot { player, row, lane }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Moves inside the field: Animated (B3.1, R383), a carrier's Unit (R446), Flicker (B5 E22, R444)
// ---------------------------------------------------------------------------

/// B3.1 rules 2 and 5, R446: move a card acting in one of its side's backrow zones — an Animated card,
/// or a Unit a carrier held — into an open unit zone of that side, without leaving the field: no R78
/// reset, no departure counted (R174), nothing that watches it or that it queued forgotten. A card
/// dormant beneath it in its backrow pile resumes (§3.2). False, changing nothing, when the card is
/// not acting in a backrow zone or `to` is not an open unit zone of its side.
///
/// `card` names the card; the one that moves is the card under that id as it stands in the state.
pub fn step_into_unit_zone(state: &mut GameState, card: &CardInstance, to: impl Into<ZoneSlot>) -> bool {
    let to = to.into();
    let mut moving = live_or(state, card);
    let Some(from) = slot_of(state, &moving) else {
        return false;
    };
    if from.row != Row::Backrow || to.row != Row::Units {
        return false;
    }
    if from.player != to.player || !acts_on_field(state, &moving) || !takes_move(state, to) {
        return false;
    }
    remove_from_field(
        state,
        &moving,
        RemoveFromFieldOptions {
            with_pile: Some(true),
        },
    );
    place_on_field(state, &mut moving, to, PlaceOnFieldOptions::default())
}

/// B3.1 rule 6: move a card acting in a unit zone into a backrow zone of its side, without leaving the
/// field. `to` must take it: not held for another card, and empty — or, for a card that
/// has Stack, a zone a Stack card may top (B5 E21). A Lock never stops a move (R688). The caller releases the card's own home first. False,
/// changing nothing, when it cannot go.
///
/// `card` names the card, as in `step_into_unit_zone`; `stack` is TS's `{ stack?: boolean }`.
pub fn step_into_backrow(
    state: &mut GameState,
    card: &CardInstance,
    to: impl Into<ZoneSlot>,
    stack: bool,
) -> bool {
    let to = to.into();
    let mut moving = live_or(state, card);
    let Some(from) = slot_of(state, &moving) else {
        return false;
    };
    if from.row != Row::Units || to.row != Row::Backrow {
        return false;
    }
    if from.player != to.player || !acts_on_field(state, &moving) {
        return false;
    }
    let takes = if is_empty(state, to) {
        takes_move(state, to)
    } else {
        stack && accepts_stack_card(state, to, AcceptsStackCardOptions { move_: Some(true) })
    };
    if !takes {
        return false;
    }
    remove_from_field(
        state,
        &moving,
        RemoveFromFieldOptions {
            with_pile: Some(true),
        },
    );
    place_on_field(state, &mut moving, to, PlaceOnFieldOptions { stack: Some(stack) })
}

/// B5 E22: the card leaves the field and re-enters the same zone at once — the same place in its pile,
/// on the same side. Leaving is leaving (R174: every effect aimed at it and every trigger it queued
/// ends, and an animated card's home is released), and R78 resets it; re-entering is entering, so it is
/// summoning sick (R83) and in Attack Position. It never passes through another pile, so a unit token
/// comes back like any other card (R444, as R175 brings one back through Reborn). The caller emits
/// the events. False, changing nothing, for a card that is not acting on the field.
///
/// `card` names the card; the one reset is the card under that id as it stands in the state.
pub fn flicker_in_place(state: &mut GameState, card: &CardInstance) -> bool {
    let current = live_or(state, card);
    let Zone::Field { player, row, .. } = current.zone else {
        return false;
    };
    if !acts_on_field(state, &current) {
        return false;
    }
    left_the_field(state, &current.id);
    let turn = state.turn;
    let unit_face = row == Row::Units || is_unit_face(state, &current);
    // R1040: a card set face-down as a Trap re-enters its backrow zone the Trap it was set as, since
    // a Unit or a Spell face never stands there on its own.
    let set_as = current.set_as.filter(|_| row == Row::Backrow);
    if let Some(live) = find_instance_mut(state, &current.id) {
        reset_instance(live);
        live.set_as = set_as;
        live.controller = player;
        live.summoned_turn = Some(turn);
        if unit_face {
            live.position = Some(Position::Atk);
        }
    }
    true
}
