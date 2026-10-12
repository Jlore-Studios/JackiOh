//! A ladder bot's aim arrow (SPEC §9.5, R738, R639000; #639): the aim a person's client would send
//! the opponent while they made the move the bot decided, so the bot's targeted plays and attacks
//! show an arrow as a person's do. The server's mirror of `apps/web/src/game/aim/aim.ts`'s
//! `aimSource` and `aimEndOf`: every end is a public handle (a hero by its seat, a field card by its
//! zone, a hand card by its position in the bot's own hand), and only a move with a declared target
//! aims, never one that only places a card. The actor's relay judges the aim again against the
//! receiver's view before it goes (`aim_is_public`), as it judges a person's.

use jackioh_engine::{
    ActionBody, Aim, AimEnd, DamageTarget, GameState, PlayerId, RANDOM_ATTACK_TARGET, Selection,
    attack_target_of, find_instance, slot_of,
};

/// Where a card stands on the field, by zone, on either side; `None` off the field.
fn field_end(state: &GameState, instance_id: &str) -> Option<AimEnd> {
    let instance = find_instance(state, instance_id)?;
    let slot = slot_of(state, instance)?;
    Some(AimEnd::Zone {
        player: slot.player,
        row: slot.row,
        lane: slot.lane,
    })
}

/// The end one declared target names: a hero, a zone, or the zone of a field card (`None` for a card
/// off the field). A mode, a recipe or an empty pick is no target at all.
fn target_end(state: &GameState, selection: &Selection) -> Option<Option<AimEnd>> {
    match selection {
        Selection::Hero { player } => Some(Some(AimEnd::Hero { player: *player })),
        Selection::Zone { player, row, lane } => Some(Some(AimEnd::Zone {
            player: *player,
            row: *row,
            lane: *lane,
        })),
        Selection::Instance { instance_id } => Some(field_end(state, instance_id)),
        Selection::Mode { .. } | Selection::Craft { .. } | Selection::None => None,
    }
}

/// The aim of a move that declares targets: its last target's end, or `None` when it declares none.
fn declared(state: &GameState, source: AimEnd, targets: Option<&Vec<Selection>>) -> Option<Aim> {
    let target = targets?
        .iter()
        .filter_map(|selection| target_end(state, selection))
        .next_back()?;
    Some(Aim { source, target })
}

/// R639000, R738: the aim `seat` shows before `body`, on `state` as the bot decided it, or `None` for
/// a move a person's client shows no aim for: an attack at a random target, a play, an Activate or a
/// hero power with no declared target, and every other move.
pub fn bot_aim(state: &GameState, seat: PlayerId, body: &ActionBody) -> Option<Aim> {
    match body {
        ActionBody::Attack {
            attacker_id,
            target_id,
        } => {
            if target_id == RANDOM_ATTACK_TARGET {
                return None;
            }
            let source = field_end(state, attacker_id)?;
            let target = match attack_target_of(state, target_id)? {
                DamageTarget::Hero { player } => AimEnd::Hero { player },
                DamageTarget::Unit { instance } => field_end(state, &instance.id)?,
            };
            Some(Aim {
                source,
                target: Some(target),
            })
        }
        ActionBody::Play {
            instance_id, targets, ..
        } => {
            let index = state.players[seat]
                .hand
                .iter()
                .position(|card| card.id == *instance_id)?;
            declared(state, AimEnd::Hand { player: seat, index }, targets.as_ref())
        }
        ActionBody::Activate {
            instance_id, targets, ..
        } => declared(state, field_end(state, instance_id)?, targets.as_ref()),
        ActionBody::ActivatePower { instance_id, targets } => {
            // A hero's power has no zone: the aim starts at the hero, as the client's does.
            let source = field_end(state, instance_id).unwrap_or(AimEnd::Hero { player: seat });
            declared(state, source, targets.as_ref())
        }
        _ => None,
    }
}
