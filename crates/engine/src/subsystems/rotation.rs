//! Rotation: Silly Silas (#52) turns both rings one step (SPEC §3.1's rotation-topology ruling, R14).
//!
//! The ten unit zones form one ring and the ten backrow zones a second, independent one: the
//! rotating player's lanes 1 to 5, then the opponent's lanes 5 down to 1, and back (§3.1). Both
//! rings turn together, one step, in the direction the play declared (R81).
//!
//! What travels with a card: its instance. A rotation never takes a card off the field, so R78's
//! reset never runs and its damage, buffs, counters and position all come along (R14). What
//! changes is `controller`, and only when the card's new zone is on the other side of the centre
//! line. That crossing is an entry (R171): the card takes this turn as its
//! `summoned_turn` and a fresh exertion, so it is summoning sick on its new side for the rest of the
//! turn. A card that moves along its own side has entered nothing and keeps both. The owner never
//! changes on a crossing; a bounce takes the card to its controller's hand as theirs (R747), and it
//! goes to its owner's library, graveyard or exile (R12) whenever it later leaves the field. A face-down trap that crosses is read by its new controller and no
//! longer by the old one, which follows from `controller` alone, so `face_up` is deliberately
//! untouched here (R33).
//!
//! Port of `packages/engine/src/subsystems/rotation.ts`. `RotationDirection` is the wire's
//! (`rotated.direction`, part 1's freeze), re-exported here under TS's module path.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId, Row, ZoneName};
use crate::zones::ZoneSlot;

pub use crate::wire::RotationDirection;

/// R14: two rings, rotated together.
pub const ROTATION_ROWS: &[Row] = &[Row::Units, Row::Backrow];

/// TS `RotationArgs`.
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

/// TS `RotationResult`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RotationResult {
    /// Cards that reached a new zone, in ring order: the unit ring first, then the backrow ring.
    pub moved: Vec<String>,
    /// Cards whose controller changed because their new zone is on the other side (R12). Each one has
    /// entered its new side on this turn (R171).
    pub crossed: Vec<String>,
    /// Cards sent to their controller's hand: a Locked destination, or the radiant bounce (R14).
    pub bounced: Vec<String>,
}

/// One zone's worth of cards and where they are headed. A Stack pile travels whole (§3.2).
struct RingEntry {
    from: ZoneSlot,
    to: ZoneSlot,
    cards: Vec<CardInstance>,
}

/// Everything in a zone, top card first. A unit zone may hold a Stack pile, and the dormant cards
/// under the top are in the zone too, so they rotate with it (§3.2).
fn contents_of(state: &GameState, slot: &ZoneSlot) -> Vec<CardInstance> {
    // B5 E21, R446: a backrow zone's pile and a carrier's Unit travel whole too (`zones::zone_contents`).
    crate::zones::zone_contents(state, slot)
        .into_iter()
        .map(|card| Borrow::<CardInstance>::borrow(&card).clone())
        .collect()
}

/// Whether a destination can take a rotating card. A Locked zone never accepts one (#52's
/// card-specific override of R688, R14) and
/// a zone reserved for a dying Reborn unit counts as occupied for every other card (§3.2, R64).
fn can_accept(state: &GameState, slot: &ZoneSlot) -> bool {
    !crate::zones::is_locked(state, slot) && !crate::zones::is_reserved(state, slot)
}

/// Put a zone's cards down in their new zone. A pile is rebuilt from the bottom up so the card that
/// was on top is on top again, which keeps the same card acting for the zone (§3.2).
fn place_contents(state: &mut GameState, cards: &[CardInstance], to: &ZoneSlot) {
    let mut placed = 0;
    for card in cards.iter().rev() {
        // Every ring zone was emptied before anything was placed and the destination accepts cards,
        // so a refusal here is a broken invariant, not a game rule; `zones.rs` says so the same way.
        let options = crate::zones::PlaceOnFieldOptions {
            stack: Some(placed > 0),
            ..Default::default()
        };
        if !crate::zones::place_on_field(state, card, to, options) {
            panic!(
                "rotation could not place {} in {} {} {}",
                card.id, to.player, to.row, to.lane
            );
        }
        placed += 1;
    }
}

/// R14: the card goes to its controller's hand (R747, shared with §6.3 Bounce). The hand cap
/// applies, so a full hand burns it (§2.4, R4), and a unit token ceases to exist on the way and
/// never reaches a hand (R11).
/// `cost_override` is the radiant variant's "costing 0"; R78 keeps it while the card waits in hand.
fn bounce_home(sink: &mut EngineSink<'_>, card: &CardInstance, cost_override: Option<i32>) {
    crate::effects::move_::bounce_card(sink, card);
    // §8 #52 radiant: "bounced to their controller's hand costing 0" is a rider on a card that reaches
    // the hand. A full hand burns it instead (§2.4, R4), and a burned card is an ordinary graveyard card
    // that R78 would otherwise have carry the 0 into every later zone.
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
    // §10.3: the new price is a visible change, announced as #31's +1 and #72r's 0 are once the card has
    // landed (R215), with what the card costs now (R65). A view redacts it for the other seat (R177).
    let cost = crate::mana::effective_cost(sink.state, &landed, Default::default());
    sink.events.push(GameEvent::CostChanged {
        instance_id: landed.id.clone(),
        cost,
        hidden_from: None,
    });
}

/// Rotate both rings one step (§3.1, R14, §8 #52). Silas is on the field when his Cry resolves, so
/// he is in the snapshot and rotates with everything else.
///
/// The whole board is read before anything is placed, so one rotation is a single atomic step: no
/// card can land on a zone whose occupant has not moved yet, and a full ring keeps every card.
///
/// Events (§10.3): `rotated` once for the rotation, then `controlChanged` per card that crossed and
/// `bounced` per card that was bounced, in ring order.
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
                        to: crate::zones::ring_neighbor(&from, args.direction, args.perspective),
                        cards: contents_of(state, &from),
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

    // Read first, then place: every card comes off the field before any card lands.
    for entry in &entries {
        for card in &entry.cards {
            let options = crate::zones::RemoveFromFieldOptions {
                with_pile: Some(true),
                ..Default::default()
            };
            crate::zones::remove_from_field(sink.state, card, options);
        }
    }

    let mut result = RotationResult::default();

    for entry in &entries {
        let crosses = entry.to.player != entry.from.player;

        // #52 radiant: "cards that would move to the opponent are bounced to their controller's hand
        // costing 0 instead" — the cards the rotating player would lose, which are the ones leaving
        // their side. "The opponent" is the rotating player's (§8 Conventions: "your" is the
        // controller), so a card crossing the other way, onto the rotating player's side, is not one of
        // them: the base clause the radiant cell does not restate still holds for it, and it crosses and
        // changes control like any other (R14, R171). The bounce goes to the card's controller's hand (R747).
        // A Locked destination would have bounced an outbound card anyway, so this also covers that case.
        if radiant && crosses && entry.from.player == args.perspective {
            for card in &entry.cards {
                bounce_home(sink, card, Some(0));
                result.bounced.push(card.id.clone());
            }
            continue;
        }

        // R14: a card whose destination is Locked is bounced to its controller's hand instead.
        if !can_accept(sink.state, &entry.to) {
            for card in &entry.cards {
                bounce_home(sink, card, None);
                result.bounced.push(card.id.clone());
            }
            continue;
        }

        let before: Vec<PlayerId> = entry.cards.iter().map(|card| card.controller).collect();
        place_contents(sink.state, &entry.cards, &entry.to);

        for (at, card) in entry.cards.iter().enumerate() {
            result.moved.push(card.id.clone());
            let Some(previous) = before.get(at).copied() else {
                continue;
            };
            // TS read the live instance, which the placement had moved to its new side.
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
            });
        }
    }

    result
}
