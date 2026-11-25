//! IA #205 Doomsday Clock (SPEC §8.13 row 205), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Can't attack\nIn {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!   Radiant: "Can't attack\nIn {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!
//! Engine: Can't attack is the catalog's. "In N turns" starts when the card arrives on the field,
//! played or summoned (R4593005): the arrival's `Summoned` names it, and the trigger plants the
//! arrival controller's watched `delay` (R1181000) of the declared `turns` with its badge, so it
//! drops when the card leaves the field (R174); due, it deals the declared `damage` to the enemy
//! hero. Built by C9 (#4593).

use jackioh_engine::effects::{damage, delay};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-205";

/// The countdown's badge (R169).
const BADGE: &str = "Doomsday Clock: 10 damage to the enemy hero in 4 turns";
/// The delayed step, which the `delayed` hook runs.
const STEP: &str = "doomsday";

/// This card arrived on the field (played or summoned): its `Summoned` names its own id.
fn arrived(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn doomsday() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("doomsday", &[GameEventType::Summoned], |ctx, _event| {
                let Some(me) = ctx.live_self() else {
                    return vec![];
                };
                let id = me.id.clone();
                vec![delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "step": STEP,
                    "turns": param(&*ctx, "turns"),
                    "label": BADGE,
                    "watch": id,
                    "data": { "damage": param(&*ctx, "damage") },
                })))]
            })
            .with_when(arrived),
        ],
        delayed: Some(hook(|ctx| {
            let damage_amount = ctx.data.get("damage").and_then(Value::as_i64).unwrap_or(0) as i32;
            vec![damage(json_as(json!({
                "to": { "of": "enemyHero" },
                "amount": damage_amount,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 20 is its declared `damage`.
    let radiant = doomsday();
    CardScripts {
        base: doomsday(),
        radiant,
    }
}

// IA #205 Doomsday Clock — SPEC §8.13 row 205 (R4593005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{next_turn_of, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(8) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn hero(s: &Scenario) -> i32 {
        s.state().players[P2].hero.health
    }

    mod base {
        use super::*;

        #[test]
        fn r4593005_deals_10_at_the_4th_own_start_and_nothing_before() {
            let mut s = played(false, "doomsday-base");
            for _ in 0..3 {
                next_turn_of(&mut s, P1);
                assert_eq!(hero(&s), HERO_HEALTH, "the first three starts are quiet");
            }
            next_turn_of(&mut s, P1);
            assert_eq!(hero(&s), HERO_HEALTH - 10);
        }

        #[test]
        fn r4593005_nothing_fires_once_it_leaves_the_field() {
            let mut s = played(false, "doomsday-gone");
            let unit = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
            for _ in 0..5 {
                next_turn_of(&mut s, P1);
            }
            assert_eq!(hero(&s), HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593005_deals_20_at_the_4th_own_start() {
            let mut s = played(true, "doomsday-radiant");
            for _ in 0..3 {
                next_turn_of(&mut s, P1);
            }
            next_turn_of(&mut s, P1);
            assert_eq!(hero(&s), HERO_HEALTH - 20);
        }
    }
}
