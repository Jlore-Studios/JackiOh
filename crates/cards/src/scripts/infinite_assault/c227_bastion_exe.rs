//! IA #227 Bastion.exe (SPEC §8.13 row 227), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Taunt, Armor 5. Aura: Your hero is immune to damage."
//!   Radiant: "Taunt, Armor 10. Aura: Your hero is immune to damage."
//!
//! Taunt and Armor are catalog data. The aura is a hero guard with a cap of 0: every damage
//! instance on its controller's hero is capped to nothing while it stands (R4594007). Fatigue goes
//! through `deal_damage`, so the cap stops it; lose-health and Set health are not damage and land.
//! The guard follows control: a stolen Bastion guards its new controller's hero.
//!
//! Engine: `heroGuard` (R4594007). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-227";

/// The guard's cap: no damage instance on its controller's hero gets through.
const IMMUNE_CAP: i32 = 0;

fn bastion() -> Script {
    Script {
        hero_guard: Some(read_hook(|_| {
            vec![HeroGuard { cap: Some(IMMUNE_CAP), ..Default::default() }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's Armor 10 is catalog data.
    let base = bastion();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #227 Bastion.exe — SPEC §8.13 row 227 (R4594007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const LUNAR: &str = "core-035"; // Lunar Eclipse: deal 3 damage.
    const VITAL_KILL: &str = "classic-029"; // Book of Vital Kill: set a hero's health to 13.
    const VANILLA: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const TECH: &str = "classic-040"; // MC Tech: steal a permanent with 5+ enemy permanents.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy the Bastion.
    const SET: &str = "stand-in (1) Trap"; // A textless Trap, for the thief's threshold.

    fn guarded(radiant: bool, seed: &str, p2: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": [STOCKPILE], "library": crate::filler(4), "health": 20 },
            "p2": p2,
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4594007_hits_attacks_and_fatigue_do_nothing() {
            let mut s = guarded(
                false,
                "bastion-guard",
                json!({ "field": [VANILLA], "hand": [LUNAR, STOCKPILE], "library": crate::filler(4) }),
            );
            s.play(LUNAR, json!({ "targets": pick::hero("p1") }));
            // The Bastion's Taunt must be answered first: the attack meets it, not the hero.
            s.attack(VANILLA, ID);
            s.expect_health(P1, 20);
        }

        #[test]
        fn r4594007_fatigue_draws_hit_nothing() {
            let mut s = scenario(json!({
                "seed": "bastion-fatigue",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": [], "health": 20 },
                "p2": { "hand": [STOCKPILE], "library": [] },
            }));
            // Both libraries are empty; the turns' draws fatigue, and p1's is capped.
            s.end_turn();
            s.end_turn();
            s.expect_health(P1, 20);
        }

        #[test]
        fn r4594007_set_health_lands() {
            let mut s = guarded(
                false,
                "bastion-vital",
                json!({ "hand": [VITAL_KILL, STOCKPILE], "library": crate::filler(4) }),
            );
            s.play(VITAL_KILL, json!({ "targets": pick::hero("p1") }));
            s.expect_health(P1, 13);
        }

        #[test]
        fn r4594007_the_guard_ends_when_it_leaves() {
            let mut s = guarded(
                false,
                "bastion-leaves",
                json!({ "field": [VANILLA], "hand": [HIT_JOB, STOCKPILE], "library": crate::filler(4) }),
            );
            s.play(HIT_JOB, json!({ "targets": pick::id(&s.unit_at(P1, 1).id) }));
            s.attack(VANILLA, "hero");
            s.expect_health(P1, 20 - 4);
        }

        #[test]
        fn r4594007_a_stolen_bastion_guards_the_thief() {
            let mut s = scenario(json!({
                "seed": "bastion-stolen",
                "active": "p2",
                "p1": {
                    "field": [{ "def": ID }, VANILLA],
                    "backrow": [SET, SET, SET],
                    "hand": [STOCKPILE],
                    "library": crate::filler(4),
                    "health": 20,
                },
                "p2": {
                    "field": [VANILLA],
                    "hand": [{ "def": TECH, "radiant": true }, STOCKPILE],
                    "library": crate::filler(4),
                    "health": 20,
                },
            }));
            s.play(TECH, json!({ "zone": 2 }));
            let bastion = s.card(ID).id.clone();
            s.answer(json!([{ "pick": "instance", "instanceId": bastion }]));
            assert_eq!(s.card(&bastion).controller, P2);
            // The thief's hero is guarded now: p1's attack meets the stolen Taunt, not the hero.
            s.end_turn();
            s.attack(VANILLA, ID);
            s.expect_health(P2, 20);
            // The victim's hero is not: p2's attack lands.
            s.end_turn();
            s.attack(VANILLA, "hero");
            s.expect_health(P1, 20 - 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594007_hits_do_nothing_on_the_radiant_face() {
            let mut s = guarded(
                true,
                "bastion-radiant",
                json!({ "hand": [LUNAR, STOCKPILE], "library": crate::filler(4) }),
            );
            s.play(LUNAR, json!({ "targets": pick::hero("p1") }));
            s.expect_health(P1, 20);
        }
    }
}
