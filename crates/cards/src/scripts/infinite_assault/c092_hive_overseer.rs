//! SPEC §8.13 IA #92 Hive Overseer ((3) Unit, Epic, 2/8 → 4/16), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt\nWhenever a friendly Unit dies, summon a Drone."
//!   Radiant: "Taunt\nWhenever a friendly Unit dies, summon a Radiant Drone."
//!
//! TG #35 Corpse Recycler's trigger on `destroyed`, narrowed to a friendly Unit's death (R172, and a
//! Unit's by what the card was as it died, `is_unit_death`): one summon of IA #4.1 for its
//! controller. Taunt is the catalog's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-092";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

/// A friendly Unit's death (R172): the player who controlled it as it died, a Unit's by what the
/// card was as it died (`is_unit_death`), or nothing.
fn died_friendly(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Destroyed { controller, .. } = event else {
        return false;
    };
    *controller == ctx.controller && is_unit_death(&*ctx.state, event)
}

/// The condition is written twice, as Core #18's: a queued trigger takes an empty list as "not met".
fn hive_overseer(radiant: bool) -> Script {
    let trigger = TriggerDef::new("hive-overseer", &[GameEventType::Destroyed], move |ctx, event| {
        if !died_friendly(ctx, event) {
            return vec![];
        }
        vec![summon(json_as(json!({ "defId": DRONE, "player": "self", "radiant": radiant })))]
    })
    .with_when(|ctx, event| died_friendly(ctx, event));
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: hive_overseer(false),
        radiant: hive_overseer(true),
    }
}

// IA #92 Hive Overseer — SPEC §8.13 row 92.
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }, UNIT],
                "mana": 10,
                "library": crate::filler(2),
            },
            "p2": { "field": [UNIT], "library": crate::filler(2) },
        }))
    }

    fn drones(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == DRONE)
            .cloned()
            .collect()
    }

    fn kill(s: &mut Scenario, player: PlayerId, lane: i32) {
        let unit = s.unit_at(player, lane);
        s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
    }

    mod base {
        use super::*;

        #[test]
        fn taunt_and_a_friendly_death_summons_a_drone() {
            let mut s = board(false);
            let overseer = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&overseer).keywords, KeywordKind::Taunt));
            kill(&mut s, P1, 2);
            let landed = drones(&s);
            assert_eq!(landed.len(), 1);
            assert!(!landed[0].radiant);
        }

        #[test]
        fn an_enemys_death_and_its_own_summon_nothing() {
            let mut s = board(false);
            kill(&mut s, P2, 1);
            assert!(drones(&s).is_empty(), "an enemy's death summons nothing");
            kill(&mut s, P1, 1);
            assert!(drones(&s).is_empty(), "its own death summons nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_friendly_death_summons_a_radiant_drone() {
            let mut s = board(true);
            kill(&mut s, P1, 2);
            let landed = drones(&s);
            assert_eq!(landed.len(), 1);
            assert!(landed[0].radiant);
        }
    }
}
