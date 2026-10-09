//! M #97.5 Bunker (SPEC §8.8 row 97.5, R1283). (2) Unit, Token (printed Epic), 5/10 → 10/30.
//!   Base:    "Tribute 2, Armor 3, Can't attack, First Strike\nThis strikes Units in its lane with {multiplier} times its Attack."
//!   Radiant: "Tribute 2, Armor 5, Can't attack, First Strike\nThis strikes Units in its lane with {multiplier} times its Attack."
//!   Engine:  "NEW: a lane multiplier (LANE-STRIKE): where combat reads both attacks once (R94), a
//!            combatant whose face declares it has its attack multiplied when the other combatant stands
//!            in the opposing unit zone of its own lane (§3.1), the hit then going through §4.4 as usual
//!            (Divine Shield, the target's Armor). It strikes back, first, when attacked (First Strike,
//!            §4.3) and in any forced attack, which skips Can't attack (§4.2); Cleave hits into other
//!            lanes and every non-combat damage are not multiplied (MD-F14). The view may show the in-lane
//!            number as a `preview` (R280). The designer's Radiant face printed 5/30, not doubling the
//!            attack; R275's stat half makes it 10/30 (⚠ designer), 30 in its lane. Tunes: multiplier 3 ↑
//!            (step 1); Armor 3 ↑ (Radiant 5); Tribute 2 ↓ (never below 1)."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-5";

/// §8.8: "Tribute 2".
const TRIBUTE_COST: i32 = 2;

/// Multiplies attack by 3 when striking across its own lane.
const MULTIPLIER: i32 = 3;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            lane_multiplier: Some(MULTIPLIER),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M 97.5 Bunker — SPEC §8.8 row 97.5, BUILD M10 row M 97.5: "Tribute 2, Armor 3, Can't attack, First Strike;
// attacked by the Unit across from it, it strikes first for 15 (MD-F14), through that Unit's Armor and Divine
// Shield; attacked from another lane, it strikes for 5; a forced attack (M 49.3) still multiplies in its
// lane; non-combat damage is never multiplied; multiplier reads through `param()`; radiant 10/30 and Armor 5,
// 30 in its lane".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const BIG_ATTACKER: &str = "core-019"; // (9) 9/9 Unit: survives 15 damage, so we can verify exact damage taken.
    const ARMORED: &str = "core-025"; // (4) 7/7, Armor 7 Unit.
    const MOTHS: &str = "core-009"; // (2) 1/14 Unit: forces attacks at start of turn.

    #[test]
    fn is_a_2_cost_unit_token_printed_epic() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-5");
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Epic));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(5), json!(10), json!(10), json!(30)]
        );
        assert_eq!(
            js(&def.base.keywords),
            json!([
                { "kind": "Armor", "n": 3 },
                { "kind": "Can't attack" },
                { "kind": "First Strike" }
            ])
        );
        assert_eq!(
            js(&def.radiant.keywords),
            json!([
                { "kind": "Armor", "n": 5 },
                { "kind": "Can't attack" },
                { "kind": "First Strike" }
            ])
        );
        let s = script();
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.tribute), Some(2));
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.lane_multiplier), Some(3));
    }

    mod base {
        use super::*;

        #[test]
        fn r1283_struck_back_across_its_lane_it_hits_first_for_15() {
            crate::register_all();
            // P1 has Bunker in lane 1 (5/10, Armor 3, First Strike).
            // P2 has BIG_ATTACKER (9/9) in lane 1.
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": BIG_ATTACKER, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
            }));

            s.end_turn(); // P2's turn.
            assert_eq!(s.state().active, P2);

            let big = s.unit(P2, 1).unwrap().id.clone();
            let bunker = s.unit(P1, 1).unwrap().id.clone();

            s.attack(&big, &bunker);

            // Bunker struck first for 5 * 3 = 15 damage!
            // BIG_ATTACKER had 9 health, so 15 damage dealt to it killed it before it could strike!
            assert!(s.unit(P2, 1).is_none(), "attacker dies to First Strike 15 damage");
            // Bunker took 0 damage because First Strike killed the attacker before it could hit.
            assert_eq!(s.unit(P1, 1).unwrap().damage, 0);
        }

        #[test]
        fn r1283_struck_from_another_lane_it_hits_for_its_base_attack_5() {
            crate::register_all();
            // P1 has Bunker in lane 1 (5/10, Armor 3, First Strike).
            // P2 has BIG_ATTACKER (9/9) in lane 2.
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": BIG_ATTACKER, "lane": 2 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
            }));

            s.end_turn(); // P2's turn.
            assert_eq!(s.state().active, P2);

            let big = s.unit(P2, 2).unwrap().id.clone();
            let bunker = s.unit(P1, 1).unwrap().id.clone();

            s.attack(&big, &bunker);

            // Because attack is from lane 2 into lane 1, multiplier is 1!
            // Bunker hits for 5 (not 15).
            // BIG_ATTACKER (9 health) takes 5 damage and survives with 4 health!
            let big_unit = s.unit(P2, 2).expect("big attacker survived 5 damage");
            assert_eq!(big_unit.damage, 5);

            // BIG_ATTACKER strikes back for 9. Bunker has Armor 3, so Bunker takes 9 - 3 = 6 damage.
            let bunker_unit = s.unit(P1, 1).expect("bunker survived");
            assert_eq!(bunker_unit.damage, 6);
        }

        #[test]
        fn r1283_armor_7_attacker_across_lane_takes_8() {
            crate::register_all();
            // P1 has Bunker in lane 1 (5/10, First Strike).
            // P2 has ARMORED in lane 1 (7/7, Armor 7).
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": ARMORED, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
            }));

            s.end_turn(); // P2's turn.

            let armored = s.unit(P2, 1).unwrap().id.clone();
            let bunker = s.unit(P1, 1).unwrap().id.clone();

            s.attack(&armored, &bunker);

            // Bunker strikes first across lane for 15.
            // ARMORED has Armor 7, so damage = 15 - 7 = 8!
            // 8 >= 7 health, so ARMORED dies!
            assert!(s.unit(P2, 1).is_none(), "Armor 7 attacker took 8 damage and died");
        }

        #[test]
        fn r1283_moths_to_the_flame_forced_attack_takes_15() {
            crate::register_all();
            // P1 has Bunker in lane 1.
            // P2 has Moths to the Flame in lane 1 (1/14).
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": MOTHS, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
            }));

            // Pass turn to P2. P2 starts turn, triggering Moths to force P1's Bunker in lane 1 to attack Moths in lane 1.
            s.end_turn();

            // Bunker attacks Moths across lane 1:
            // Multiplier applies: 5 * 3 = 15 damage!
            // Moths has 14 health, so 15 damage kills Moths!
            assert!(s.unit(P2, 1).is_none(), "Moths took 15 damage and died");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1283_radiant_strikes_for_30_across_lane_and_has_armor_5() {
            crate::register_all();
            // Radiant Bunker is 10/30 with Armor 5 and multiplier 3.
            // In lane, strikes for 10 * 3 = 30.
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "radiant": true, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": MOTHS, "lane": 1 }], // Moths 1/14 in lane 1
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
            }));

            let bunker = s.card(ID).clone();
            assert!(
                s.stats(&bunker)
                    .keywords
                    .iter()
                    .any(|k| k == &Keyword::Armor { n: 5 }),
                "radiant Bunker has Armor 5"
            );

            // Pass turn to trigger Moths: Bunker attacks Moths across lane 1.
            s.end_turn();

            // Moths dies to 30 damage.
            assert!(s.unit(P2, 1).is_none(), "Moths died to 30 damage");

            // Moths strikes back for 1. Bunker has Armor 5, so takes 0 damage!
            assert_eq!(s.unit(P1, 1).unwrap().damage, 0);
        }
    }
}
