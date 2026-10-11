//! M #97.4 Mega Church (SPEC §8.8 row 97.4, R1282). (3) Unit, Token (printed Rare), 0/5 → 0/10.
//!   Base:    "Tribute 5, Can't attack\nThis may Tribute any permanent on either side that costs ({cheap}) or less.\nActivate: Take control of an enemy permanent."
//!   Radiant: "Tribute 5, Can't attack, Divine Shield\nThis may Tribute any permanent on either side that costs ({cheap}) or less.\nActivate: Take control of an enemy permanent. It becomes Radiant."
//!   Engine:  "NEW: a wider Tribute (WIDE-TRIBUTE), a static flag the legal Tribute sets read: besides
//!            your own Units, every permanent acting on the field on either side (unit tops, backrow
//!            cards, face-down ones included, their cost public, R33; never a dormant card, R13) whose
//!            cost is 1 or less (R65; an X card its X, R396) may pay, each worth 1 (a Sheep Token its
//!            2 or 3); the set is minimal (R101) and travels in the play (R81), and the payment is §10.5
//!            step 2's Sacrifice (a Death fires, the card going to its owner's graveyard); hand cards
//!            never pay, and the Church stays with its player, with no hand-over to the opponent as
//!            R360's (MD-F13). R101 lists every minimal set (R90), up to 15,504 of them with twenty
//!            permanents out, so the builder measures the legal actions and the AI's budget on a full
//!            board. Activate (R384): one declared enemy permanent acting on the field, stolen (§6.3; the
//!            same lane if free, else the first free zone, else it stays, R15; R171's entry), then made
//!            Radiant on the Radiant face. Tunes: cheap 1 ↑; Tribute 5 ↓ (never below 1)."

use jackioh_engine::effects::{set_radiant, steal};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-4";

/// §8.8: "Tribute 5".
const TRIBUTE_COST: i32 = 5;

/// Costs (1) or less on either side.
const TRIBUTE_CHEAP: i32 = 1;

/// Activate is once a turn.
const USES: i32 = 1;

fn church(radiant: bool) -> Script {
    let activation = ActivationDecl {
        id: "church-steal".to_string(),
        label: "Take control of an enemy permanent".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({
                "side": "enemy",
                "of": ["unit", "backrow"]
            }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            let mut effects = vec![steal(json_as(json!({ "target": { "of": "chosen" } })))];
            if radiant {
                effects.push(set_radiant(json_as(json!({ "target": { "of": "chosen" } }))));
            }
            effects
        }),
    };

    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            tribute_cheap: Some(TRIBUTE_CHEAP),
            ..StaticFlags::default()
        }),
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: church(false),
        radiant: church(true),
    }
}

// M 97.4 Mega Church — SPEC §8.8 row 97.4, BUILD M10 row M 97.4: "Tribute 5, paid by any minimal set
// of your Units and of permanents on either side costing (1) or less, face-down ones included, each
// worth 1 (a Sheep 2 or 3), as Sacrifices (MD-F13); a dormant card or a hand card never pays;
// `legalActions` stays within the AI's budget on a full board; Activate, once a turn: a declared
// enemy permanent becomes yours by R15, staying put with no free zone; cheap and Tribute read through
// `param()`; radiant 0/10, Divine Shield, and the stolen permanent is made Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const CHEAP_UNIT: &str = "core-008"; // (1) Unit.
    const DEAR_UNIT: &str = "core-025"; // (4) Unit.
    const CHEAP_SPELL: &str = "core-041"; // (1) Trap.
    const VANILLA: &str = "core-008";

    #[test]
    fn is_a_3_cost_unit_token_printed_rare() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-4");
        assert_eq!(js(&def.cost), json!(3));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Rare));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(5), json!(0), json!(10)]
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(
            js(&def.radiant.keywords),
            json!([{ "kind": "Can't attack" }, { "kind": "Divine Shield" }])
        );
        let s = script();
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.tribute), Some(5));
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.tribute_cheap), Some(1));
    }

    mod base {
        use super::*;

        #[test]
        fn r1282_pays_mixed_5_card_payment_to_each_owner_s_graveyard_and_church_stays_its_player_s() {
            crate::register_all();
            // P1 has 3 cheap units on field, P2 has 2 cheap permanents (1 unit, 1 trap).
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [
                        { "def": CHEAP_UNIT, "lane": 1 },
                        { "def": CHEAP_UNIT, "lane": 2 },
                        { "def": CHEAP_UNIT, "lane": 3 },
                    ],
                    "hand": [ID, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": {
                    "field": [
                        { "def": CHEAP_UNIT, "lane": 1 },
                    ],
                    "backrow": [
                        { "def": CHEAP_SPELL, "lane": 1 },
                    ],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
            }));

            // Collect the 5 instance IDs to pay.
            let p1_u1 = s.unit(P1, 1).unwrap().id.clone();
            let p1_u2 = s.unit(P1, 2).unwrap().id.clone();
            let p1_u3 = s.unit(P1, 3).unwrap().id.clone();
            let p2_u1 = s.unit(P2, 1).unwrap().id.clone();
            let p2_b1 = s.backrow(P2, 1).unwrap().id.clone();

            let tributes = vec![
                p1_u1.clone(),
                p1_u2.clone(),
                p1_u3.clone(),
                p2_u1.clone(),
                p2_b1.clone(),
            ];

            s.play(ID, json!({ "zone": 1, "tributes": tributes }));

            // Church is in P1 lane 1!
            assert_eq!(s.unit(P1, 1).unwrap().def_id, ID);
            assert_eq!(s.unit(P1, 1).unwrap().controller, P1);

            // P1's 3 cards are in P1's graveyard.
            let p1_gy: Vec<String> = s.state().players.p1.graveyard.iter().map(|c| c.id.clone()).collect();
            assert!(p1_gy.contains(&p1_u1));
            assert!(p1_gy.contains(&p1_u2));
            assert!(p1_gy.contains(&p1_u3));

            // P2's 2 cards are in P2's graveyard.
            let p2_gy: Vec<String> = s.state().players.p2.graveyard.iter().map(|c| c.id.clone()).collect();
            assert!(p2_gy.contains(&p2_u1));
            assert!(p2_gy.contains(&p2_b1));
        }

        #[test]
        fn r1282_dear_or_dormant_card_is_neither_offered_nor_accepted() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [
                        { "def": CHEAP_UNIT, "lane": 1 },
                        { "def": CHEAP_UNIT, "lane": 2 },
                        { "def": CHEAP_UNIT, "lane": 3 },
                        { "def": CHEAP_UNIT, "lane": 4 },
                    ],
                    "hand": [ID, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": {
                    "field": [
                        { "def": DEAR_UNIT, "lane": 1 }, // cost 4 > 1!
                    ],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
            }));

            let p2_dear = s.unit(P2, 1).unwrap().id.clone();
            let p1_u1 = s.unit(P1, 1).unwrap().id.clone();
            let p1_u2 = s.unit(P1, 2).unwrap().id.clone();
            let p1_u3 = s.unit(P1, 3).unwrap().id.clone();
            let p1_u4 = s.unit(P1, 4).unwrap().id.clone();

            // Refuses payment containing dear card.
            s.expect_refused(|s| {
                s.play(
                    ID,
                    json!({ "lane": 5, "tributes": [p1_u1, p1_u2, p1_u3, p1_u4, p2_dear] }),
                )
            });
        }

        #[test]
        fn r1282_activate_steals_once_a_turn() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 2 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
            }));

            let enemy_unit = s.unit(P2, 2).unwrap().id.clone();

            s.activate(ID, json!({ "targets": [{ "pick": "instance", "instanceId": enemy_unit }] }));

            // Unit is now controlled by P1!
            let stolen = s.unit(P1, 2).expect("unit stole into lane 2");
            assert_eq!(stolen.id, enemy_unit);
            assert_eq!(stolen.controller, P1);

            // Second activation in the same turn is refused.
            s.expect_refused(|s| {
                s.activate(ID, json!({ "targets": [{ "pick": "instance", "instanceId": enemy_unit }] }))
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1282_radiant_face_has_divine_shield_and_makes_stolen_card_radiant() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "radiant": true, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 2 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE],
                },
            }));

            // Verify Divine Shield on radiant face.
            let church = s.card(ID).clone();
            assert!(
                s.stats(&church)
                    .keywords
                    .iter()
                    .any(|k| k.kind() == KeywordKind::DivineShield),
                "Radiant church has Divine Shield"
            );

            let enemy_unit = s.unit(P2, 2).unwrap().id.clone();
            assert!(!s.unit(P2, 2).unwrap().radiant, "initially not radiant");

            s.activate(ID, json!({ "targets": [{ "pick": "instance", "instanceId": enemy_unit }] }));

            let stolen = s.unit(P1, 2).expect("unit stole into lane 2");
            assert_eq!(stolen.id, enemy_unit);
            assert_eq!(stolen.controller, P1);
            assert!(stolen.radiant, "stolen card became Radiant");
        }
    }
}
