//! M #97.1 Empty Plot (SPEC §8.8 row 97.1, R1281). (0) Unit, Token (printed Common), 0/3 → 0/8.
//!   Base:    "Can't attack. You may play a Unit on top of this, as if it had Stack."
//!   Radiant: "Can't attack. You may play a Unit on top of this, as if it had Stack. When you do, Buff that Unit."
//!   Engine:  "NEW: a pile base (STACK-BASE), a static flag: while the Plot acts (the top of its unit
//!            zone), its controller's played Units may name its zone as if they had Stack, by the same
//!            check in the legal zones and in the refusals, a Locked (R688) or reserved (R64) zone
//!            excepted; the Unit tops the pile (§3.2), the Plot dormant beneath it (R13) until the cards
//!            above it are gone; summons never stack on it, and the opponent cannot. On the Radiant face
//!            the Unit gets one Buff (Upgrade, R386) as it is placed (§10.5 step 4), before its Cry, the
//!            Plot's flag read at placement since the Plot is dormant by the time any trigger could
//!            answer (MD-F12). Tunes: buffs 1 ↑, on the Radiant face only."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-1";

/// On the Radiant face, the Unit is Buffed (Upgraded) once as it lands.
const BUFFS: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            stack_base: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            stack_base: Some(true),
            stack_base_buffs: Some(BUFFS),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// M 97.1 Empty Plot — SPEC §8.8 row 97.1, BUILD M10 row M 97.1: "Can't attack; while it tops its
// zone, you may play any Unit onto it as if the Unit had Stack (MD-F12), the Plot dormant beneath
// until the pile above is gone, then resuming; a Locked or reserved zone refuses; a summon never
// stacks onto it, and the opponent's Units cannot; radiant 0/8, and the Unit is Buffed once as it
// lands, before its Cry".
#[cfg(test)]
mod tests {
    use super::{BUFFS, ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FELINORS: &str = "core-012"; // (2) Unit: Cry summons a copy.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.

    fn slot(player: PlayerId, lane: i32) -> ZoneSlot {
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        }
    }

    #[test]
    fn is_a_0_cost_unit_token_printed_common_cant_attack() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-1");
        assert_eq!(js(&def.cost), json!(0));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Common));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(3), json!(0), json!(8)]
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Can't attack" }]));
        let s = script();
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.stack_base), Some(true));
        assert_eq!(s.radiant.static_flags.as_ref().and_then(|f| f.stack_base_buffs), Some(BUFFS));
    }

    mod base {
        use super::*;

        #[test]
        fn r1281_cant_attack() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": { "field": [{ "def": ID, "lane": 1 }], "hand": [FILLER], "library": [SPARE, SPARE] },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));
            s.expect_refused(|s| s.attack(ID, "hero"));
        }

        #[test]
        fn r1281_a_played_unit_stacks_onto_empty_plot_plot_dormant_beneath() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [VANILLA, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            // Play VANILLA specifically onto lane 1 where Plot is.
            s.play(VANILLA, json!({ "zone": 1 }));

            let pile = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 1)).unwrap();
            assert_eq!(pile.len(), 2);
            assert_eq!(pile[0].def_id, VANILLA);
            assert_eq!(pile[1].def_id, ID);

            // Empty Plot is buried / dormant beneath (R13).
            let plot = pile.iter().find(|c| c.def_id == ID).unwrap();
            assert!(jackioh_engine::zones::is_buried(s.state(), plot));
        }

        #[test]
        fn r1281_a_locked_zone_refuses_stacking() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [VANILLA, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            jackioh_engine::zones::lock_zone(s.state_mut(), &slot(P1, 1));

            s.expect_refused(|s| s.play(VANILLA, json!({ "zone": 1 })));
        }

        #[test]
        fn r1281_duplicating_felinors_summon_does_not_stack_onto_empty_plot() {
            crate::register_all();
            // P1 has Empty Plot in lane 1 and Felinors in hand.
            // Lane 2 is open.
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FELINORS, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            // Play Felinors into lane 3 (not on Plot).
            s.play(FELINORS, json!({ "zone": 3 }));

            // The summoned copy lands in the leftmost free zone (lane 2), never stacking in lane 1!
            let lane1_pile = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 1)).unwrap();
            assert_eq!(lane1_pile.len(), 1);
            assert_eq!(lane1_pile[0].def_id, ID);

            let lane2_pile = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 2)).unwrap();
            assert_eq!(lane2_pile.len(), 1);
            assert_eq!(lane2_pile[0].def_id, FELINORS);
        }

        #[test]
        fn r1281_opponent_cannot_stack_onto_your_empty_plot() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": { "field": [{ "def": ID, "lane": 1 }], "hand": [FILLER], "library": [SPARE, SPARE] },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "hand": [VANILLA, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
            }));

            s.end_turn(); // P1 passes turn, now P2's turn.
            assert_eq!(s.state().active, P2);

            // P2's lane 1 is occupied with VANILLA (not a stack base), so P2 cannot stack onto it.
            s.expect_refused(|s| s.play(VANILLA, json!({ "zone": 1 })));
        }

        #[test]
        fn r1281_empty_plot_resumes_after_hit_job() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [VANILLA, HIT_JOB, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            s.play(VANILLA, json!({ "zone": 1 }));
            let top_id = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 1)).unwrap()[0].id.clone();

            // Destroy top unit with Hit Job.
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": top_id }] }));

            // The top unit died, now Empty Plot resumes on top of lane 1!
            let pile = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 1)).unwrap();
            assert_eq!(pile.len(), 1);
            assert_eq!(pile[0].def_id, ID);
            assert!(!jackioh_engine::zones::is_buried(s.state(), &pile[0]));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1281_radiant_buffs_unit_once_before_its_cry() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "radiant": true, "lane": 1 }],
                    "hand": [FELINORS, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            s.play(FELINORS, json!({ "zone": 1 }));

            // Events verification: Upgraded must happen before Cry's summon.
            let events = s.last_events();
            let upgraded_pos = events
                .iter()
                .position(|e| e.event_type() == GameEventType::Upgraded);
            let summon_pos = events
                .iter()
                .rposition(|e| e.event_type() == GameEventType::Summoned);

            assert!(upgraded_pos.is_some(), "Unit was Upgraded as it landed");
            assert!(summon_pos.is_some(), "Cry summoned a copy");
            assert!(
                upgraded_pos.unwrap() < summon_pos.unwrap(),
                "Upgrade occurred before Cry's summon (§10.5 step 4 before step 5)"
            );

            let pile = jackioh_engine::zones::pile_at(s.state(), &slot(P1, 1)).unwrap();
            assert_eq!(pile.len(), 2);
            assert_eq!(pile[0].def_id, FELINORS);
            let orig = s.unit(P1, 1).unwrap();
            let clone = s.unit(P1, 2).unwrap();
            assert_eq!(orig.def_id, FELINORS);
            assert_eq!(clone.def_id, FELINORS);
            let upgrade_event = events
                .iter()
                .find(|e| matches!(e, GameEvent::Upgraded { instance_id, .. } if instance_id == &orig.id));
            assert!(upgrade_event.is_some(), "orig unit was Upgraded");
        }
    }
}
