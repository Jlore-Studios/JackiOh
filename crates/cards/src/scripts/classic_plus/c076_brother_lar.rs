//! C+ #76 Brother Lar (SPEC §8.7 row 76). (1) Unit, CN, Human, Rare, 1/1 → 2/2.
//!   Base:    "Death: Summon a Brother Ping."
//!   Radiant: "Death: Summon a Radiant Brother Ping."
//!   Engine:  "Brother Ping (C+ #76.1) goes into your leftmost open unit zone (R64): Brother Lar's own zone
//!            is free again unless a Reborn reserves it, and a full row summons nothing. Summoned, so no
//!            Cry. Tunes: none."
//!
//! Death fires when Brother Lar goes from the field to a graveyard (§6.2): destroyed or tributed — not
//! when it is exiled or bounced. By then its zone is empty, so R64's leftmost open zone may be its own,
//! unless a granted Reborn has reserved that zone for Lar's return (R64), when Ping goes elsewhere.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-076";

/// §7, §8.7: the token Brother Lar leaves behind (TS `cardDef("classicplus-076-1").id`, read off the
/// catalog).
fn ping_id() -> String {
    crate::card_def("classicplus-076-1").id
}

pub fn script() -> CardScripts {
    let ping = ping_id();
    let radiant_ping = ping.clone();
    CardScripts {
        base: Script {
            death: Some(hook(move |_ctx| vec![summon(json_as(json!({ "defId": ping.as_str() })))])),
            ..Script::default()
        },
        radiant: Script {
            death: Some(hook(move |_ctx| {
                vec![summon(json_as(json!({ "defId": radiant_ping.as_str(), "radiant": true })))]
            })),
            ..Script::default()
        },
    }
}

// C+ #76 Brother Lar — SPEC §8.7 row 76, BUILD M9 Classic+ row C+ 76: "Death summons a Brother Ping
// (C+ #76.1) into your leftmost open unit zone, Lar's own zone included unless a granted Reborn reserves
// it (R64); exile or a bounce summons nothing; radiant the Ping is Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const LAR: &str = "classicplus-076";
    const PING: &str = "classicplus-076-1";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const TIMMY: &str = "core-011";
    const HIT_JOB: &str = "core-016"; // (3) "Destroy target Unit."
    const COLLATERAL: &str = "core-034"; // (4) "Exile target permanent and a random card from your opponent's deck."
    const FLOOD: &str = "core-017"; // (4) "Bounce all Units."

    const P1: PlayerId = PlayerId::P1;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn target(s: &Scenario, card: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": s.card(card).id }])
    }

    fn pings(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == PING)
            .collect()
    }

    fn reborn() -> Vec<Keyword> {
        vec![json_as(json!({ "kind": "Reborn" }))]
    }

    mod c_n76_brother_lar {
        use super::*;

        #[test]
        fn is_a_1_1_1_cn_human_unit_2_2_radiant_naming_brother_ping() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, LAR);
            assert_eq!(
                [
                    js(&def.cost),
                    js(&def.base.attack),
                    js(&def.base.health),
                    js(&def.radiant.attack),
                    js(&def.radiant.health)
                ],
                [json!(1), json!(1), json!(1), json!(2), json!(2)]
            );
            assert_eq!(js(&def.tags), json!(["CN", "Human"]));
            assert_eq!(def.refs, Some(vec![PING.to_string()]));
            let scripts = super::super::script();
            assert!(scripts.base.death.is_some());
            assert!(scripts.radiant.death.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn s6_2_dying_in_combat_it_leaves_a_brother_ping_in_its_own_zone_the_leftmost_open_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [LAR] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.attack(LAR, MENACE);
                s.expect_in_zone(LAR, "graveyard");
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(PING.to_string()));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.radiant), Some(false));
                s.expect_events(json!(["destroyed", "summoned"]));
            }

            #[test]
            fn r64_destroyed_by_an_effect_in_lane_3_ping_takes_the_leftmost_open_zone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [{ "def": LAR, "lane": 3 }] },
                    "p2": { "hand": [FILLER] },
                }));
                let targets = target(&s, LAR);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(PING.to_string()));
                assert!(s.unit(P1, 3).is_none());
            }

            #[test]
            fn r1_ping_is_summoned_so_it_fires_no_cry_and_is_not_a_play() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [LAR] }, "p2": { "hand": [FILLER] } }));
                let targets = target(&s, LAR);
                s.play(HIT_JOB, json!({ "targets": targets }));
                let played: Vec<String> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::CardPlayed { def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(played, vec![HIT_JOB]);
            }

            #[test]
            fn r64_a_granted_reborn_reserves_lars_zone_for_its_return_so_ping_goes_to_the_next_open_zone() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [LAR] }, "p2": { "hand": [FILLER] } }));
                s.card_mut(LAR).granted_keywords = reborn();
                let targets = target(&s, LAR);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(PING.to_string()));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(LAR.to_string()));
            }

            #[test]
            fn r64_a_full_row_summons_nothing_the_ping_has_nowhere_to_go() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [LAR, TIMMY, TIMMY, TIMMY, TIMMY] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.card_mut(LAR).granted_keywords = reborn();
                s.attack(LAR, MENACE);
                assert!(pings(&s).is_empty());
            }

            #[test]
            fn s6_2_exiled_it_has_no_death_no_ping() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [COLLATERAL, FILLER], "field": [LAR] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                let targets = target(&s, LAR);
                s.play(COLLATERAL, json!({ "targets": targets }));
                s.expect_in_zone(LAR, "exile");
                assert!(pings(&s).is_empty());
            }

            #[test]
            fn s6_2_bounced_it_has_no_death_no_ping() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FLOOD, FILLER], "field": [LAR] }, "p2": { "hand": [FILLER] } }));
                s.play(FLOOD, json!({}));
                s.expect_in_zone(LAR, "hand");
                assert!(pings(&s).is_empty());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r74_its_death_summons_a_radiant_brother_ping_an_8_8() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LAR, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "radiant": true }] },
                }));
                s.attack(LAR, MENACE);
                let ping = s.unit(P1, 1);
                assert_eq!(ping.as_ref().map(|unit| unit.def_id.clone()), Some(PING.to_string()));
                assert_eq!(ping.as_ref().map(|unit| unit.radiant), Some(true));
                let reference = ping.map(|unit| unit.id).unwrap_or_else(|| PING.to_string());
                s.expect_stats(reference.as_str(), json!({ "attack": 8, "maxHealth": 8 }));
            }

            #[test]
            fn s6_2_exiled_the_radiant_face_summons_nothing_either() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [COLLATERAL, FILLER], "field": [{ "def": LAR, "radiant": true }] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                let targets = target(&s, LAR);
                s.play(COLLATERAL, json!({ "targets": targets }));
                assert!(pings(&s).is_empty());
            }
        }
    }
}
