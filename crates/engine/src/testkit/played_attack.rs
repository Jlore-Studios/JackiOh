//! What one attack did, read off the step's events and the state it left (R3100000, #4111): the one
//! reading `tests/rules/combat_forecast.rs` and the fuzz's I18 (`invariants.rs`) both hold
//! `query::combat_forecast` to, so the two judges cannot drift apart. `PlayedAttack::forecast` puts a
//! forecast entry in the same terms. R4290000: a Cleave attack's splash and a Trample attack's overflow
//! are read off the damage events the engine tags with their route (R2783001).

use crate::query::{AttackForecast, SplashHit, TrampleOverflow};
use crate::state::{GameState, find_instance};
use crate::wire::{GameEvent, KeywordKind, PlayerId, Zone};

use super::invariants::hero_seat_of;

/// The facts a forecast entry states about an attack (R3100000): the damage into the target and into
/// the attacker, whether the target was destroyed (the game won on it, for a hero), whether the
/// attacker is still on the field, and whether the target's Divine Shield broke; and (R4290000) each
/// Cleave hit beside the target and the Trample excess onto the target's hero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayedAttack {
    pub dealt: i32,
    pub taken: i32,
    pub kills: bool,
    pub survives: bool,
    pub pops: bool,
    pub splash: Vec<SplashHit>,
    pub overflow: Option<TrampleOverflow>,
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
            splash: entry.splash.clone(),
            overflow: entry.overflow.clone(),
        }
    }

    /// What `attacker_id`'s attack on `target_id` did: `events` are the step's, `after` the state it
    /// left. A unit is gone when the step destroyed it or it is off the field after: Reborn returns a
    /// destroyed unit with its own id (R83), and an exile on damage leaves it with no `destroyed` (R1124).
    /// `splashed` are the ids the forecast's splash names, in its order (R4290000): a Cleave hit a Divine
    /// Shield, Armor or Indestructible stops leaves no `damage` event to find it by, so each is read
    /// first, then any other unit a Cleave hit damaged, in the events' order.
    pub fn read(
        events: &[GameEvent],
        after: &GameState,
        attacker_id: &str,
        target_id: &str,
        splashed: &[String],
    ) -> PlayedAttack {
        let damage = || {
            events.iter().filter_map(|event| match event {
                GameEvent::Damage {
                    target_id,
                    amount,
                    via,
                    ..
                } => Some((target_id.as_str(), *amount, *via)),
                _ => None,
            })
        };
        let sum_into = |id: &str| -> i32 { damage().filter(|hit| hit.0 == id).map(|hit| hit.1).sum() };
        let sum_via = |id: &str, route: KeywordKind| -> i32 {
            damage()
                .filter(|hit| hit.0 == id && hit.2 == Some(route))
                .map(|hit| hit.1)
                .sum()
        };
        let gone = |id: &str| -> bool {
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == id))
                || !find_instance(after, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
        };
        let popped = |id: &str| -> bool {
            events.iter().any(
                |event| matches!(event, GameEvent::DivineShieldLost { instance_id } if instance_id == id),
            )
        };
        // An alternative win (R850) over a hero still standing is no kill of it.
        let hero_falls = |seat: PlayerId| after.result.is_some() && after.players[seat].hero.health <= 0;
        let kills = match hero_seat_of(target_id) {
            Some(seat) => hero_falls(seat),
            None => gone(target_id),
        };
        let mut cleaved: Vec<String> = splashed.to_vec();
        for (id, _, via) in damage() {
            if via == Some(KeywordKind::Cleave) && !cleaved.iter().any(|met| met == id) {
                cleaved.push(id.to_string());
            }
        }
        let splash = cleaved
            .into_iter()
            .map(|id| SplashHit {
                dealt: sum_via(&id, KeywordKind::Cleave),
                kills: gone(&id),
                pops: popped(&id),
                instance_id: id,
            })
            .collect();
        // A hero target's `dealt` holds what tramples onto it already (R4221003).
        let overflow = if hero_seat_of(target_id).is_some() {
            None
        } else {
            damage()
                .find_map(|(id, _, via)| {
                    let seat = hero_seat_of(id)?;
                    (via == Some(KeywordKind::Trample)).then_some((id, seat))
                })
                .map(|(id, seat)| TrampleOverflow {
                    hero_id: id.to_string(),
                    dealt: sum_via(id, KeywordKind::Trample),
                    kills: hero_falls(seat),
                })
        };
        PlayedAttack {
            dealt: sum_into(target_id),
            taken: sum_into(attacker_id),
            kills,
            survives: !gone(attacker_id),
            pops: popped(target_id),
            splash,
            overflow,
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
        )?;
        for hit in &self.splash {
            write!(
                f,
                ", splash {} on {}{}{}",
                hit.dealt,
                hit.instance_id,
                if hit.kills { " destroyed" } else { "" },
                if hit.pops { " shield popped" } else { "" },
            )?;
        }
        if let Some(overflow) = &self.overflow {
            write!(
                f,
                ", overflow {} onto {}{}",
                overflow.dealt,
                overflow.hero_id,
                if overflow.kills { " ends the game" } else { "" },
            )?;
        }
        Ok(())
    }
}
