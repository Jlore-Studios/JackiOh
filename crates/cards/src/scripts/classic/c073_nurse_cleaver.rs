//! C #73 Nurse Cleaver (SPEC §8.6 row 73). (2) Unit, Common, 3/6 → 6/12.
//!   Base:    "Rush, Cleave, Lifesteal"
//!   Radiant: "Charge, Cleave, Lifesteal"
//!   Engine:  "Keywords only. Tunes: none."
//!
//! Nothing to script: the stats and both faces' keywords are the catalog's, and the engine reads them.
//!   Rush      — §4.1: it may attack a Unit the turn it enters, never the hero (Charge: the hero too).
//!   Cleave    — §4.4 step 10: each hit of its attack also hits the defender's neighbours (§3.1), as
//!               separate damage instances; combat only, and part of the attack (R63).
//!   Lifesteal — §4.4 step 8: its controller's hero heals the amount each hit deals, the Cleave hits
//!               included.
//! Its proof: `test/classic/073-nurse-cleaver.test.ts`.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-073";

pub fn script() -> CardScripts {
    let base = Script::default();
    CardScripts {
        // The same empty script: the Radiant face's 6/12 and its Charge for Rush are catalog data.
        radiant: base.clone(),
        base,
    }
}

// C #73 Nurse Cleaver (SPEC §8.6 row 73; BUILD M9 row C 73). (2) Unit, Common, 3/6 → 6/12: Rush,
// Cleave, Lifesteal; Radiant: Charge, Cleave, Lifesteal. Keywords only.

/// `describe("C #73 Nurse Cleaver")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CLEAVER: &str = "classic-073";
    const VANILLA: &str = "core-008"; // 4/4
    const STOCKPILE: &str = "core-005";

    /// The harness registers every card first (TS's harness did on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS `{ ...base, ...over }` on two object literals.
    fn merged(mut base: Value, over: Value) -> Value {
        if let (Some(into), Some(from)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        base
    }

    /// TS `SPARE: SideSetup`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": [VANILLA, VANILLA] })
    }

    /// Three 4/4s side by side, the middle one the defender.
    fn row() -> Value {
        merged(json!({ "field": [VANILLA, VANILLA, VANILLA] }), spare())
    }

    /// The damage instances the Cleaver dealt in the last step: (target, amount), in order.
    fn cleaver_hits(s: &Scenario) -> Vec<(String, i32)> {
        let cleaver = s.card(CLEAVER).id.clone();
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id: Some(source),
                    target_id,
                    amount,
                    ..
                } if *source == cleaver => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// §4.1 Rush: it attacks a Unit the turn it enters, never the hero
        #[test]
        fn s4_1_rush_it_attacks_a_unit_the_turn_it_enters_never_the_hero() {
            let mut s = scenario(json!({ "p1": { "hand": [CLEAVER, STOCKPILE], "library": spare()["library"] }, "p2": row() }));
            s.play(CLEAVER, json!({}));
            s.expect_stats(CLEAVER, json!({ "attack": 3, "health": 6 }));
            s.expect_refused(|s| s.attack(CLEAVER, "hero"));
            let middle = s.unit(P2, 2).expect("p2's middle unit");
            s.attack(CLEAVER, &middle);
            let middle = s.unit(P2, 2).expect("p2's middle unit");
            assert_eq!(s.card(&middle).damage, 3);
        }

        /// §4.4 step 10 Cleave: the defender's neighbours are hit too, each as a separate instance
        #[test]
        fn s4_4_step_10_cleave_the_defenders_neighbours_are_hit_too_each_as_a_separate_instance() {
            let mut s = scenario(json!({ "p1": merged(json!({ "field": [CLEAVER] }), spare()), "p2": row() }));
            let left = s.unit(P2, 1).expect("p2's lane-1 unit");
            let middle = s.unit(P2, 2).expect("p2's lane-2 unit");
            let right = s.unit(P2, 3).expect("p2's lane-3 unit");
            s.attack(CLEAVER, &middle);
            let mut hits: Vec<String> = cleaver_hits(&s).into_iter().map(|(target, _)| target).collect();
            hits.sort();
            let mut expected = vec![left.id.clone(), middle.id.clone(), right.id.clone()];
            expected.sort();
            assert_eq!(hits, expected);
            let damage: Vec<i32> = [&left, &middle, &right].iter().map(|unit| s.card(*unit).damage).collect();
            assert_eq!(damage, vec![3, 3, 3]);
        }

        /// §4.4 step 8 Lifesteal: your hero heals for each hit it deals, the Cleave hits included
        #[test]
        fn s4_4_step_8_lifesteal_your_hero_heals_for_each_hit_it_deals_the_cleave_hits_included() {
            let mut s = scenario(json!({ "p1": merged(json!({ "field": [CLEAVER], "health": 10 }), spare()), "p2": row() }));
            let middle = s.unit(P2, 2).expect("p2's middle unit");
            s.attack(CLEAVER, &middle);
            // Three hits of 3; the defender's 4 back does not heal anyone.
            s.expect_health(P1, 19);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// §4.1 6/12 Charge, Cleave, Lifesteal: it may attack the hero the turn it enters
        #[test]
        fn s4_1_6_12_charge_cleave_lifesteal_it_may_attack_the_hero_the_turn_it_enters() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CLEAVER, "radiant": true }, STOCKPILE], "health": 10, "library": spare()["library"] },
                "p2": spare(),
            }));
            s.play(CLEAVER, json!({}));
            s.expect_stats(CLEAVER, json!({ "attack": 6, "health": 12 }));
            s.attack(CLEAVER, "hero");
            s.expect_health(P2, 24).expect_health(P1, 16);
        }

        /// §4.4 its Cleave and Lifesteal: three hits of 6 heal 18
        #[test]
        fn s4_4_its_cleave_and_lifesteal_three_hits_of_6_heal_18() {
            let mut s = scenario(json!({
                "p1": merged(json!({ "field": [{ "def": CLEAVER, "radiant": true }], "health": 10 }), spare()),
                "p2": merged(json!({ "field": [VANILLA, VANILLA, VANILLA] }), spare()),
            }));
            let middle = s.unit(P2, 2).expect("p2's middle unit");
            s.attack(CLEAVER, &middle);
            // Three hits of 6 (no Trample, so each deals its whole 6, R63), and each heals.
            let dealt: Vec<i32> = cleaver_hits(&s).into_iter().map(|(_, amount)| amount).collect();
            assert_eq!(dealt, vec![6, 6, 6]);
            s.expect_health(P1, 28);
        }
    }
}
