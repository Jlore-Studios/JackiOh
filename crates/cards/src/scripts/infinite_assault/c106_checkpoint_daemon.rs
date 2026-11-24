//! SPEC §8.13 IA #106 Checkpoint Daemon ((3) Unit, Rare, 2/6 → 4/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: Take a snapshot of your hero's Health.\nDeath: Set your hero's Health to that snapshot."
//!   Radiant: "End of turn: Take a snapshot of your hero's Health.\nDeath: Set your hero's Health to that snapshot and gain {armor} Armor." (armor 5)
//!
//! `remember` keeps the controller's hero Health on the card's memory at each of its end of turns,
//! and the Death reads it back with `recalled` (the latest end of turn wins, R4589005). With no
//! snapshot the Death sets nothing, though the Radiant face still gains its Armor; setting Health
//! may lower it, as `set_health` does.

use jackioh_engine::effects::{gain_hero_armor, remember, set_health};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-106";

/// The memory key of the Health snapshot.
const SNAPSHOT_KEY: &str = "@checkpointHealth";

fn daemon(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let view: &EffectContext = ctx;
            vec![remember(json_as(json!({
                "key": SNAPSHOT_KEY,
                "value": hero_of(&*view.state, view.controller).health,
            })))]
        })),
        death: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let mut effects = Vec::new();
            if let Some(snapshot) = recalled(view, SNAPSHOT_KEY) {
                effects.push(set_health(json_as(json!({
                    "to": { "of": "selfHero" },
                    "value": snapshot,
                }))));
            }
            if radiant {
                effects.push(gain_hero_armor(json_as(json!({ "amount": param(view, "armor") }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: daemon(false),
        radiant: daemon(true),
    }
}

// IA #106 Checkpoint Daemon — SPEC §8.13 row 106 (R4589005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 plays the Daemon, snaps at its end of turn, takes a hit, then p2 kills it.
    fn snapped(radiant: bool, health: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(3),
                "mana": 10,
                "health": 30,
            },
            "p2": { "hand": [HIT_JOB], "library": crate::filler(3), "mana": 10 },
        }));
        s.play(ID, json!({ "zone": 1 }));
        // The Daemon's end of turn snapshots 30; on p2's turn it takes a hit, then p2 kills it.
        s.end_turn();
        s.state_mut().players[P1].hero.health = health;
        let daemon = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&daemon)));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4589005_death_restores_the_snapshot() {
            let mut s = snapped(false, 20);
            s.expect_health(P1, 30);
            assert_eq!(s.state().players[P1].hero.armor, 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589005_death_restores_the_snapshot_and_gains_armor() {
            let mut s = snapped(true, 20);
            s.expect_health(P1, 30);
            assert_eq!(s.state().players[P1].hero.armor, 5);
        }

        #[test]
        fn r4589005_radiant_gains_armor_without_one() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ID, "radiant": true }, HIT_JOB],
                    "library": crate::filler(3),
                    "mana": 10,
                    "health": 30,
                },
                "p2": { "library": crate::filler(3) },
            }));
            s.state_mut().players[P1].hero.health = 20;
            s.play(ID, json!({ "zone": 1 }));
            let daemon = s.unit_at(P1, 1);
            // Killed before any end of turn: no snapshot, so the Death sets nothing.
            s.play(HIT_JOB, pick::targets(pick::instance(&daemon)));
            s.expect_health(P1, 20);
            assert_eq!(s.state().players[P1].hero.armor, 5);
        }
    }
}
