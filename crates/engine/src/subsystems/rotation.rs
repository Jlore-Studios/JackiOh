//! Rotation: Silly Silas (#52) turns both rings one step (SPEC §3.1's rotation-topology ruling, R14).
//! The ten unit zones form one ring and the ten backrow zones a second: the rotating player's lanes 1
//! to 5, then the opponent's 5 down to 1 (§3.1), turned together in the declared direction (R81).
//!
//! A rotation never takes a card off the field, so R78's reset never runs (R14). Only `controller`
//! changes, across the center line, and that crossing is an entry (R171): the card is summoning sick on
//! its new side; one moving along its own side is not. The owner never changes (R12; a later bounce
//! goes to the controller's hand as theirs, R747), and a crossing face-down trap keeps `face_up` (R33).

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::config::ROTATION_RING;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId, Row, ZoneName};
use crate::zones::ZoneSlot;

pub use crate::wire::RotationDirection;

/// R14: two rings, rotated together.
pub const ROTATION_ROWS: &[Row] = &[Row::Units, Row::Backrow];

// R14: `ROTATION_RING` must name the one topology this file plays, two rings.
const _: () = assert!(
    matches!(ROTATION_RING.as_bytes(), b"two-rings"),
    "R14: subsystems/rotation.rs turns two independent rings, the units and the backrow"
);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RotationArgs {
    pub direction: RotationDirection,
    /// Whose seat "left" and "right" are read from: the rotating player (§3.1, §8 #52).
    pub perspective: PlayerId,
    /// #52 radiant: a card that would cross to the opponent of `perspective` bounces to its
    /// controller's hand at cost 0 instead; one crossing towards `perspective` still crosses (R14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RotationResult {
    /// Cards that reached a new zone, in ring order: the unit ring first, then the backrow ring.
    pub moved: Vec<String>,
    /// Cards whose new zone is on the other side (R12), each entering it this turn (R171).
    pub crossed: Vec<String>,
    /// Cards sent to their controller's hand: a reserved destination, or the radiant bounce (R14).
    pub bounced: Vec<String>,
}

/// One zone's worth of cards and where they are headed. A Stack pile travels whole (§3.2).
struct RingEntry {
    from: ZoneSlot,
    to: ZoneSlot,
    cards: Vec<CardInstance>,
    /// R446: the Unit a carrier holds in this backrow zone, which goes back on its carrier; every other
    /// card lands as it lay, standing or dormant (R1081000).
    carried: Option<String>,
}

/// Everything in a zone, top card first: the dormant cards of a Stack pile rotate with it (§3.2).
fn contents_of(state: &GameState, slot: &ZoneSlot) -> Vec<CardInstance> {
    // B5 E21, R446: a backrow zone's pile and a carrier's Unit travel whole too (`zones::zone_contents`).
    crate::zones::zone_contents(state, slot)
        .into_iter()
        .map(|card| Borrow::<CardInstance>::borrow(&card).clone())
        .collect()
}

/// Whether a destination can take a rotating card: a Locked zone does (R831000: a rotation evicts
/// nothing, so R688's Lock does not stop it), while one reserved for a dying Reborn unit counts as
/// occupied for every other card (§3.2, R64).
fn can_accept(state: &GameState, slot: &ZoneSlot) -> bool {
    !crate::zones::is_reserved(state, slot)
}

/// A pile is rebuilt from the bottom up, so the same card stays on top acting for the zone (§3.2).
fn place_contents(state: &mut GameState, cards: &[CardInstance], carried: Option<&str>, to: &ZoneSlot) {
    for (placed, card) in cards.iter().rev().enumerate() {
        // Every ring zone was emptied first and the destination accepts: a refusal is a broken invariant.
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
                "rotation could not place {} in {} {} {}",
                card.id, to.player, to.row, to.lane
            );
        }
    }
}

/// R14: the card goes to its controller's hand (R747, as §6.3 Bounce): a full hand burns it (§2.4, R4)
/// and a unit token ceases to exist on the way (R11). `cost_override` is the radiant "costing 0".
fn bounce_home(sink: &mut EngineSink<'_>, card: &CardInstance, cost_override: Option<i32>) {
    crate::effects::move_::bounce_card(sink, card);
    // §8 #52 radiant: the 0 rides only a card that reached the hand; R78 would carry it on from a burn.
    let Some(cost_override) = cost_override else {
        return;
    };
    let Some(landed) = find_instance_mut(sink.state, &card.id) else {
        return;
    };
    if landed.zone.z() != ZoneName::Hand {
        return;
    }
    landed.cost_override = Some(cost_override);
    let landed = landed.clone();
    // §10.3: like #31's +1, the new price is announced once the card has landed (R215), as what it
    // costs now (R65); a view redacts it for the other seat (R177).
    let cost = crate::mana::effective_cost(sink.state, &landed, Default::default());
    sink.events.push(GameEvent::CostChanged {
        instance_id: landed.id.clone(),
        cost,
        hidden_from: None,
    });
}

/// Rotate both rings one step (§3.1, R14, §8 #52), Silas included. Every card comes off the field
/// before any lands, so none lands on a zone whose occupant has not moved yet. Events (§10.3):
/// `rotated` once, then `controlChanged` per crossing and `bounced` per bounce, in ring order.
pub fn rotate_rings(sink: &mut EngineSink<'_>, args: &RotationArgs) -> RotationResult {
    let radiant = args.radiant == Some(true);

    let entries: Vec<RingEntry> = {
        let state: &GameState = sink.state;
        ROTATION_ROWS
            .iter()
            .flat_map(|row| {
                crate::zones::ring_order(*row, args.perspective)
                    .into_iter()
                    .map(|from| RingEntry {
                        to: crate::zones::ring_neighbor(from, args.direction, args.perspective),
                        cards: contents_of(state, &from),
                        carried: crate::zones::carried_at(state, from).map(|unit| unit.id.clone()),
                        from,
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|entry| !entry.cards.is_empty())
            .collect()
    };

    sink.events.push(GameEvent::Rotated {
        direction: args.direction,
    });

    for entry in &entries {
        for card in &entry.cards {
            let options = crate::zones::RemoveFromFieldOptions {
                with_pile: Some(true),
            };
            crate::zones::remove_from_field(sink.state, card, options);
        }
    }

    let mut result = RotationResult::default();

    for entry in &entries {
        let crosses = entry.to.player != entry.from.player;

        // #52 radiant: a card leaving the rotating player's side bounces home costing 0 instead (R747);
        // one crossing onto it crosses like any other (R14, R171).
        if radiant && crosses && entry.from.player == args.perspective {
            for card in &entry.cards {
                bounce_home(sink, card, Some(0));
                result.bounced.push(card.id.clone());
            }
            continue;
        }

        // R14, R88: a card whose destination is reserved is bounced to its controller's hand instead; a
        // Locked one takes it (R831000).
        if !can_accept(sink.state, &entry.to) {
            for card in &entry.cards {
                bounce_home(sink, card, None);
                result.bounced.push(card.id.clone());
            }
            continue;
        }

        let before: Vec<PlayerId> = entry.cards.iter().map(|card| card.controller).collect();
        place_contents(sink.state, &entry.cards, entry.carried.as_deref(), &entry.to);

        for (at, card) in entry.cards.iter().enumerate() {
            result.moved.push(card.id.clone());
            let Some(previous) = before.get(at).copied() else {
                continue;
            };
            let Some(placed) = find_instance(sink.state, &card.id).cloned() else {
                continue;
            };
            if placed.controller == previous {
                continue;
            }
            crate::combat::enter_new_side(sink, &placed, previous);
            result.crossed.push(card.id.clone());
            sink.events.push(GameEvent::ControlChanged {
                instance_id: card.id.clone(),
                controller: placed.controller,
                row: entry.to.row,
                lane: entry.to.lane,
                former_id: None,
                how: None,
            });
        }
    }

    result
}
