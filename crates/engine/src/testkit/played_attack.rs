//! What one attack did, read off the step's events and the state it left (R3100000, #4111): the one
//! reading `tests/rules/combat_forecast.rs` and the fuzz's I18 (`invariants.rs`) both hold
//! `query::combat_forecast` to, so the two judges cannot drift apart. `PlayedAttack::forecast` puts a
//! forecast entry in the same terms.

use crate::query::AttackForecast;
use crate::state::{GameState, find_instance};
use crate::wire::{GameEvent, Zone};

use super::invariants::hero_seat_of;

/// The five facts a forecast entry states about an attack (R3100000): the damage into the target and
/// into the attacker, whether the target was destroyed (the game won on it, for a hero), whether the
/// attacker is still on the field, and whether the target's Divine Shield broke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayedAttack {
    pub dealt: i32,
    pub taken: i32,
    pub kills: bool,
    pub survives: bool,
    pub pops: bool,
}

impl PlayedAttack {
    /// What `entry` says its attack will do.
    pub fn forecast(entry: &AttackForecast) -> PlayedAttack {
        PlayedAttack {
            dealt: entry.dealt,
            taken: entry.taken,
            kills: entry.kills,
            survives: entry.survives,
            pops: entry.pops,
        }
    }

    /// What `attacker_id`'s attack on `target_id` did: `events` are the step's, `after` the state it
    /// left. A unit is gone when the step destroyed it or it is off the field after: Reborn returns a
    /// destroyed unit with its own id (R83), and an exile on damage leaves it with no `destroyed` (R1124).
    pub fn read(events: &[GameEvent], after: &GameState, attacker_id: &str, target_id: &str) -> PlayedAttack {
        let sum_into = |id: &str| -> i32 {
            events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Damage {
                        target_id, amount, ..
                    } if target_id == id => Some(*amount),
                    _ => None,
                })
                .sum()
        };
        let gone = |id: &str| -> bool {
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == id))
                || !find_instance(after, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
        };
        let pops = events.iter().any(
            |event| matches!(event, GameEvent::DivineShieldLost { instance_id } if instance_id == target_id),
        );
        let kills = match hero_seat_of(target_id) {
            // An alternative win (R850) over a hero still standing is no kill of it.
            Some(seat) => after.result.is_some() && after.players[seat].hero.health <= 0,
            None => gone(target_id),
        };
        PlayedAttack {
            dealt: sum_into(target_id),
            taken: sum_into(attacker_id),
            kills,
            survives: !gone(attacker_id),
            pops,
        }
    }
}

impl std::fmt::Display for PlayedAttack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dealt {}, taken {}, {}, {}, {}",
            self.dealt,
            self.taken,
            if self.kills { "kills" } else { "no kill" },
            if self.survives {
                "attacker survives"
            } else {
                "attacker falls"
            },
            if self.pops {
                "pops the shield"
            } else {
                "no shield pop"
            },
        )
    }
}
