//! Board snapshots (docs/classic-sets.md B5 E29; SPEC §2.2, §10.1, §8.7 C+ #35; R419, R562, R563, R566):
//! the history C+ #35 Rollback returns the field to.
//!   - `record_board_snapshot` starts every turn, before the mana refresh (R62): both sides' zones as
//!     whole instances, their Locks and the homes held then (R383), the last BOARD_HISTORY_DEPTH kept
//!     in `state.board_history`. Plain data (§9.3), never in `view_for` (§10.8), renamed in it when a
//!     card takes a fresh id (`state::rename_in_board_history`, R227).
//!   - `restore_board` is R419's three steps, each on every restored side before the next: 1. a card the
//!     snapshot does not hold goes to its controller's hand (R78, §2.4, R11, R747); 2. each card it holds
//!     goes back to its zone and place (a fresh id going face-down, R227); 3. the Locks become the snapshot's.
//!
//! Not plays, summons or deaths: `rolledBack`, `bounced`/`burned` (step 1), `controlChanged` for a card
//! that entered the side (R73, R171) and `locked`/`unlocked` (step 3).

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::combat::enter_new_side;
use crate::config::BOARD_HISTORY_DEPTH;
use crate::effects::move_::bounce_card;
use crate::preview::is_face_down;
use crate::script::{Effect, EngineSink};
use crate::state::{
    BoardSnapshot, CardInstance, GameState, HomeZone, SideSnapshot, find_instance, find_instance_mut,
    side_snapshot_instances,
};
use crate::wire::{GameEvent, PLAYER_IDS, PerPlayer, PlayerId, Row, RowFlags, Zone, ZoneName};
use crate::zones::{
    PlaceOnFieldOptions, RemoveFromFieldOptions, ZoneSlot, fresh_face_down_id, place_on_field, release_home,
    remove_from_any_zone, remove_from_field, slots_of, zone_contents,
};

const ROWS: [Row; 2] = [Row::Units, Row::Backrow];

/// R419, §2.2: the field as the turn that has just begun finds it; the last BOARD_HISTORY_DEPTH are kept, oldest first.
pub fn record_board_snapshot(state: &mut GameState) {
    let side = |player: PlayerId| -> SideSnapshot {
        let held = &state.players[player];
        let homes: Vec<HomeZone> = state
            .homes
            .iter()
            .flatten()
            .filter(|home| home.zone.player == player)
            .cloned()
            .collect();
        SideSnapshot {
            units: held.units.clone(),
            backrow: held.backrow.clone(),
            locks: held.locks.clone(),
            backrow_piles: held.backrow_piles.clone(),
            carried: held.carried.clone(),
            homes: if homes.is_empty() { None } else { Some(homes) },
        }
    };
    let snapshot = BoardSnapshot {
        turn: state.turn,
        sides: PerPlayer {
            p1: side(PlayerId::P1),
            p2: side(PlayerId::P2),
        },
    };
    let mut history = state.board_history.take().unwrap_or_default();
    history.push(snapshot);
    let depth = BOARD_HISTORY_DEPTH.max(0) as usize;
    if history.len() > depth {
        history.drain(..history.len() - depth);
    }
    state.board_history = Some(history);
}

/// R419, R562: the snapshot of the start of player-turn (this turn − N), or the oldest the history holds
/// when it holds none that old — at most the game's first turn's. `None` with no history (before turn 1).
pub fn snapshot_for(state: &GameState, turns_ago: i32) -> Option<&BoardSnapshot> {
    state
        .board_history
        .iter()
        .flatten()
        .find(|snapshot| snapshot.turn >= state.turn - turns_ago)
}

/// One zone as a snapshot holds it, in `zones::zone_contents`'s order: a pile top first; a carried Unit, top, dormant cards.
fn snapshot_zone(side: &SideSnapshot, slot: &ZoneSlot) -> Vec<CardInstance> {
    let Ok(i) = usize::try_from(slot.lane - 1) else {
        return Vec::new();
    };
    if slot.row == Row::Units {
        return side.units.get(i).cloned().flatten().unwrap_or_default();
    }
    let mut out: Vec<CardInstance> = Vec::new();
    if let Some(carried) = side
        .carried
        .as_ref()
        .and_then(|carried| carried.get(i))
        .cloned()
        .flatten()
    {
        out.push(carried);
    }
    if let Some(top) = side.backrow.get(i).cloned().flatten() {
        out.push(top);
    }
    if let Some(beneath) = side.backrow_piles.as_ref().and_then(|piles| piles.get(i)) {
        out.extend(beneath.iter().cloned());
    }
    out
}

fn zones_of(player: PlayerId) -> Vec<ZoneSlot> {
    ROWS.iter().flat_map(|row| slots_of(player, *row)).collect()
}

fn same_slot(a: &ZoneSlot, b: &ZoneSlot) -> bool {
    a.player == b.player && a.row == b.row && a.lane == b.lane
}

fn row_flags(locks: &RowFlags, row: Row) -> &Vec<bool> {
    match row {
        Row::Units => &locks.units,
        Row::Backrow => &locks.backrow,
    }
}

fn locked_at(locks: &RowFlags, slot: &ZoneSlot) -> bool {
    usize::try_from(slot.lane - 1)
        .ok()
        .and_then(|at| row_flags(locks, slot.row).get(at))
        .copied()
        == Some(true)
}

/// A card the snapshot holds (`card`, a copy, becomes the card on the field) and where it stood just before.
#[derive(Clone, Debug)]
struct Placement {
    card: CardInstance,
    slot: ZoneSlot,
    live: Option<CardInstance>,
    from: Option<PlayerId>,
    was_face_down: bool,
}

/// R566: a card that stood on the same side of the field just before, under the same id, entered nothing
/// (R171) and keeps its sickness and exertion; any other — a fresh id included — entered the field on
/// this turn (R83), crossing sides with what it installed (`combat::enter_new_side`). Returns whether it
/// entered. `card_id` names the placed card in the state (its fresh id when it took one).
fn stamp_turn_state(sink: &mut EngineSink<'_>, placement: &Placement, card_id: &str, renamed: bool) -> bool {
    let Some(card) = find_instance_mut(sink.state, card_id) else {
        return false;
    };
    card.summoned_turn = None;
    card.taunt_suppressed_turn = None;
    if let Some(live) = &placement.live
        && placement.from == Some(placement.slot.player)
        && !renamed
    {
        card.exertion = live.exertion;
        if live.summoned_turn.is_some() {
            card.summoned_turn = live.summoned_turn;
        }
        if live.taunt_suppressed_turn.is_some() {
            card.taunt_suppressed_turn = live.taunt_suppressed_turn;
        }
        return false;
    }
    let card = card.clone();
    enter_new_side(sink, &card, placement.from.unwrap_or(placement.slot.player));
    true
}

/// R419: return `only`'s sides of the field to the snapshot `turns_ago` names (R562). Returns how many
/// turns it went back, or `None` with no history, when nothing happens.
pub fn restore_board(
    sink: &mut EngineSink<'_>,
    by: PlayerId,
    turns_ago: i32,
    only: &[PlayerId],
) -> Option<i32> {
    let sides: Vec<PlayerId> = PLAYER_IDS
        .into_iter()
        .filter(|player| only.contains(player))
        .collect();
    // A copy (SURFACE §4.4.8): the fresh ids handed out below rename the stored history's cards (R227), not these.
    let snapshot: BoardSnapshot = snapshot_for(sink.state, turns_ago)?.clone();
    if sides.is_empty() {
        return None;
    }
    let back = sink.state.turn - snapshot.turn;
    sink.events.push(GameEvent::RolledBack {
        player: by,
        turns_ago: back,
        sides: sides.clone(),
    });

    // Step 1.
    let held: IndexSet<String> = sides
        .iter()
        .flat_map(|player| side_snapshot_instances(&snapshot.sides[*player]))
        .map(|card| card.id.clone())
        .collect();
    for player in &sides {
        let standing: Vec<String> = zones_of(*player)
            .iter()
            .flat_map(|slot| {
                zone_contents(sink.state, slot)
                    .iter()
                    .map(|card| card.id.clone())
                    .collect::<Vec<String>>()
            })
            .collect();
        for id in standing {
            if held.contains(&id) {
                continue;
            }
            if let Some(card) = find_instance(sink.state, &id).cloned() {
                bounce_card(sink, &card);
            }
        }
    }

    // Step 2: read where every held card stands, lift them all out, then rebuild each zone bottom first.
    let mut placements: Vec<Placement> = Vec::new();
    for player in &sides {
        for slot in zones_of(*player) {
            for card in snapshot_zone(&snapshot.sides[*player], &slot) {
                let live = find_instance(sink.state, &card.id).cloned();
                // R566: a card mid-play stays its play's (§10.5); its place in the snapshot is left empty.
                if live
                    .as_ref()
                    .is_some_and(|live| live.zone.z() == ZoneName::Resolving)
                {
                    continue;
                }
                let (from, was_face_down) = match &live {
                    Some(
                        standing @ CardInstance {
                            zone: Zone::Field { player, row, .. },
                            ..
                        },
                    ) => (
                        Some(*player),
                        *row == Row::Backrow && is_face_down(sink.state, standing),
                    ),
                    _ => (None, false),
                };
                placements.push(Placement {
                    card,
                    slot,
                    live,
                    from,
                    was_face_down,
                });
            }
        }
    }
    for placement in &placements {
        let Some(live) = &placement.live else {
            continue;
        };
        let Some(from) = placement.from else {
            remove_from_any_zone(sink.state, &mut live.clone());
            continue;
        };
        // A restored side's piles are rebuilt whole, so nothing there resumes; on the other side the card
        // beneath does (§3.2). A move along the field is no departure (R174); a moved card's own home goes (R563).
        remove_from_field(
            sink.state,
            live,
            RemoveFromFieldOptions {
                with_pile: Some(sides.contains(&from)),
            },
        );
        release_home(sink.state, &live.id);
    }
    // R563: what is held now on a restored side is let go; the cards go back whatever is Locked now.
    sink.state.reserved.retain(|zone| !sides.contains(&zone.player));
    let homes_now: Vec<HomeZone> = sink
        .state
        .homes
        .iter()
        .flatten()
        .filter(|home| !sides.contains(&home.zone.player))
        .cloned()
        .collect();
    sink.state.homes = Some(homes_now);
    let locks_before: Vec<RowFlags> = sides
        .iter()
        .map(|player| sink.state.players[*player].locks.clone())
        .collect();
    for player in &sides {
        let locks = &mut sink.state.players[*player].locks;
        locks.units = locks.units.iter().map(|_| false).collect();
        locks.backrow = locks.backrow.iter().map(|_| false).collect();
    }
    for player in &sides {
        for slot in zones_of(*player) {
            let mut zone: Vec<&Placement> = placements
                .iter()
                .filter(|placement| same_slot(&placement.slot, &slot))
                .collect();
            zone.reverse();
            let mut entered: Vec<GameEvent> = Vec::new();
            for (at, placement) in zone.into_iter().enumerate() {
                let mut card = placement.card.clone();
                let brittle = card.brittle;
                // R227: going face-down from anywhere but a face-down zone, it takes a fresh id as it goes.
                let former_id = if slot.row == Row::Backrow
                    && is_face_down(sink.state, &card)
                    && !placement.was_face_down
                {
                    Some(fresh_face_down_id(sink.state, &mut card))
                } else {
                    None
                };
                // Every restored zone is empty and open now; only a carried Unit whose carrier is mid-play
                // (left out above) can find no place, and it goes to its controller's hand as step 1's cards do.
                if !place_on_field(
                    sink.state,
                    &mut card,
                    slot,
                    PlaceOnFieldOptions { stack: Some(at > 0) },
                ) {
                    bounce_card(sink, &card);
                    continue;
                }
                // The snapshot's Brittle count exactly: placement starts a printed one on a card with none (R385).
                if brittle.is_none()
                    && let Some(placed) = find_instance_mut(sink.state, &card.id)
                {
                    placed.brittle = None;
                }
                if !stamp_turn_state(sink, placement, &card.id, former_id.is_some()) {
                    continue;
                }
                entered.insert(
                    0,
                    GameEvent::ControlChanged {
                        instance_id: card.id.clone(),
                        controller: slot.player,
                        row: slot.row,
                        lane: slot.lane,
                        former_id,
                        how: None,
                    },
                );
            }
            sink.events.extend(entered);
        }
    }
    // R563: the snapshot's own holds come back for the cards standing there again. Read `state.homes`
    // afresh: the hand bounce above releases homes, which deletes the field when none is left.
    let regained: Vec<HomeZone> = sides
        .iter()
        .flat_map(|player| snapshot.sides[*player].homes.clone().unwrap_or_default())
        .filter(|home| {
            find_instance(sink.state, &home.instance_id).is_some_and(|card| card.zone.z() == ZoneName::Field)
        })
        .collect();
    let mut homes: Vec<HomeZone> = sink.state.homes.clone().unwrap_or_default();
    homes.extend(regained);
    sink.state.homes = if homes.is_empty() { None } else { Some(homes) };

    // Step 3.
    for (at, player) in sides.iter().enumerate() {
        let after = snapshot.sides[*player].locks.clone();
        sink.state.players[*player].locks = after.clone();
        for slot in zones_of(*player) {
            let now = locked_at(&after, &slot);
            if now
                != locks_before
                    .get(at)
                    .is_some_and(|before| locked_at(before, &slot))
            {
                sink.events.push(if now {
                    GameEvent::Locked {
                        player: *player,
                        row: slot.row,
                        lane: slot.lane,
                    }
                } else {
                    GameEvent::Unlocked {
                        player: *player,
                        row: slot.row,
                        lane: slot.lane,
                    }
                });
            }
        }
    }
    Some(back)
}

/// `roll_back`'s "self" | "enemy" | "both": the sides of the field, seen from the card's controller.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RollBackSides {
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
    #[serde(rename = "both")]
    Both,
}

/// `roll_back`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct RollBackArgs {
    pub turns_ago: i32,
    pub sides: RollBackSides,
}

/// The verb C+ #35 casts (R419): return the board to the snapshot `turns_ago` names, on the side of the
/// card's controller ("self"), the other ("enemy") or both.
pub fn roll_back(args: RollBackArgs) -> Effect {
    Effect::new("rollBack", move |ctx| {
        let controller = ctx.controller;
        let sides: Vec<PlayerId> = PLAYER_IDS
            .into_iter()
            .filter(|player| {
                args.sides == RollBackSides::Both
                    || (*player == controller) == (args.sides == RollBackSides::SelfSide)
            })
            .collect();
        restore_board(ctx, controller, args.turns_ago, &sides);
    })
}
