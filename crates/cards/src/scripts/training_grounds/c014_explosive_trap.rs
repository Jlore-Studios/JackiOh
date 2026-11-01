//! TG #14 Explosive Trap (SPEC §8.11 row 14). (1) Trap, Common. "Trample / Reveals when your opponent
//! plays a Unit: After it resolves, deal {damage} damage to it." (Radiant: Trample, Pierce, 12.)
//! Core #41 trigger: §10.5 step 7 `cardResolved` (after Cry/Echo, R427; never token/Recruit/Reborn, R1, R61; R174, R99).
//! C #5 hit states Trample and Pierce (R2199001; §4.4 step 9, R1750000 excess to hero).

use jackioh_engine::catalog::def_of;
use jackioh_engine::prelude::*;
use jackioh_engine::query::left_field_since_resolved;

pub const ID: &str = "traininggrounds-014";

fn explosive_trap(pierce: bool) -> Script {
    let trigger = TriggerDef::new("explosive-trap", &[GameEventType::CardResolved], move |ctx, event| {
        let GameEvent::CardResolved { instance_id, .. } = event else {
            return vec![];
        };
        let mut args = json!({
            "to": { "of": "instance", "instanceId": instance_id },
            "amount": param(&*ctx, "damage"),
            "trample": true,
        });
        if pierce {
            args["ignoreArmor"] = json!(true);
        }
        vec![damage(json_as(args))]
    })
    .with_when(|ctx, event| {
        let GameEvent::CardResolved { player, def_id, permanent, .. } = event else {
            return false;
        };
        // R2199003: the opponent's played Unit, still on the field from that play.
        *player != ctx.controller
            && def_of(Some(&*ctx.state), def_id).type_ == CardType::Unit
            && *permanent
            && !left_field_since_resolved(&*ctx.state, event)
    });
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: explosive_trap(false),
        radiant: explosive_trap(true),
    }
}

// TG #14 Explosive Trap — SPEC §8.11 row 14, BUILD M13 row TG 14.
// lane 1; p1 is active and plays.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::upgrade_number;

    const TRAP: &str = super::ID;
    const RENO: &str = "core-053"; // (3) 4/6, Cry: heal your hero up to 30.
    const SAINTESS: &str = "core-081"; // (1) 1/2.
    const ARMORED: &str = "core-025"; // (4) 7/7, Armor 7.
    const DEFENDER: &str = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
    const TOKEN_MAKER: &str = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
    const TRUE_STRIKE: &str = "core-044"; // a Spell.
    const MANA_WELL: &str = "core-006"; // a Field Spell.
    const HIT_JOB: &str = "core-016"; // rides along so the turn does not auto-end (R82).
    const PEA: &str = "meditative-013"; // (2) 2/6, Armor 2.
    const BOOTS: &str = "classic-013"; // Radiant (1) 4/2 Charge: when this attacks, Recruit a card.
    const RUSH_TOKEN: &str = "core-t-rush";

    fn trap_set(hand: Value, radiant_trap: bool) -> Scenario {
        crate::register_all();
        let mut cards = hand.as_array().cloned().unwrap_or_default();
        cards.push(json!(HIT_JOB));
        scenario(json!({
            "seed": "explosive-trap",
            "p1": { "hand": cards, "health": 10, "mana": 10 },
            "p2": { "backrow": [{ "def": TRAP, "radiant": radiant_trap, "faceUp": false, "lane": 1 }], "health": 30 },
        }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    fn damage_to(s: &Scenario, id: &str) -> i32 {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == id => Some(*amount),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn r33_it_is_set_face_down_until_it_fires() {
        let s = trap_set(json!([RENO]), false);
        assert_eq!(s.backrow(P2, 1).unwrap().face_up, Some(false));
        let seen = serde_json::to_string(&view_for(s.state(), P1)).unwrap();
        assert!(!seen.contains(TRAP), "p1's view names the face-down trap");
    }

    mod base {
        use super::*;

        #[test]
        fn r427_after_the_cry_resolves_the_unit_takes_6_and_the_trap_is_consumed() {
            let mut s = trap_set(json!([RENO]), false);
            s.play(RENO, json!({ "zone": 1 }));

            s.expect_events(json!(["cardPlayed", "healed", "cardResolved", "trapFired", "damage", "destroyed"]));
            s.expect_health(P1, 30);
            s.expect_in_zone(RENO, "graveyard");
            s.expect_in_zone(TRAP, "graveyard");
        }

        #[test]
        fn r1750000_a_2_health_unit_takes_2_and_its_controllers_hero_4() {
            let mut s = trap_set(json!([SAINTESS]), false);
            s.play(SAINTESS, json!({ "zone": 1 }));

            let saintess = s.card(SAINTESS).id.clone();
            assert_eq!(damage_to(&s, &saintess), 2);
            s.expect_health(P1, 6);
        }

        #[test]
        fn r1750000_armor_takes_its_share_first_and_only_the_rest_tramples() {
            // 7 Armor absorbs 6: the Unit takes nothing, so nothing is left over to trample.
            let mut s = trap_set(json!([ARMORED]), false);
            s.play(ARMORED, json!({ "zone": 1 }));

            s.expect_stats(ARMORED, json!({ "health": 7 }));
            s.expect_health(P1, 10);
        }

        #[test]
        fn r1750000_a_3_health_armor_2_unit_takes_3_and_the_hero_1_armor_first() {
            // A Gatling Pea at 2/3 keeps its printed Armor 2: of the 6, Armor takes 2, the Unit its 3
            // Health and the 1 left over tramples to p1's hero. Without Armor first the hero would take 3.
            let mut s = trap_set(json!([PEA]), false);
            s.card_mut(PEA).stats_override = Some(AttackHealth { attack: 2, health: 3 });
            s.play(PEA, json!({ "zone": 1 }));

            assert!(fired(&s));
            let pea = s.card(PEA).id.clone();
            assert_eq!(damage_to(&s, &pea), 3);
            s.expect_in_zone(PEA, "graveyard");
            s.expect_health(P1, 9);
        }

        #[test]
        fn r1750000_a_divine_shield_unit_loses_the_shield_and_nothing_tramples() {
            let mut s = trap_set(json!([DEFENDER]), false);
            s.play(DEFENDER, json!({ "zone": 1 }));

            assert!(s.events().iter().any(|e| matches!(e, GameEvent::DivineShieldLost { .. })));
            s.expect_in_zone(DEFENDER, "field");
            s.expect_health(P1, 10);
        }

        #[test]
        fn r2199003_a_summoned_token_is_no_play_and_leaves_it_set_for_the_next_played_unit() {
            // A Rush Token summoned by p1's effect, with no play around it: the trap stays set.
            let mut s = trap_set(json!([TOKEN_MAKER]), false);
            let events = crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({
                    "defId": RUSH_TOKEN,
                    "player": "self",
                    "lane": 3,
                }))),
                P1,
            );
            assert_eq!(s.unit(P1, 3).map(|card| card.def_id).as_deref(), Some(RUSH_TOKEN));
            assert!(!events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            assert_eq!(s.backrow(P2, 1).map(|card| card.def_id).as_deref(), Some(TRAP));

            // Still armed: the next played Unit is hit, never the token its Cry makes.
            s.play(TOKEN_MAKER, json!({ "zone": 1 }));
            assert_eq!(s.events().iter().filter(|e| matches!(e, GameEvent::TrapFired { .. })).count(), 1);
            s.expect_in_zone(TOKEN_MAKER, "graveyard");
            s.expect_in_zone(TRAP, "graveyard");
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id).as_deref(), Some(RUSH_TOKEN));
        }

        #[test]
        fn r2199003_a_recruit_is_no_play_and_leaves_it_set() {
            // A Radiant Boots on the Ground attacks and Recruits Reno from the top of p1's deck (§6.3, R1).
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "explosive-trap",
                "p1": { "hand": [HIT_JOB], "library": [RENO, HIT_JOB], "mana": 10,
                        "field": [{ "def": BOOTS, "radiant": true, "lane": 1 }] },
                "p2": { "backrow": [{ "def": TRAP, "faceUp": false, "lane": 1 }], "health": 30 },
            }));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 26);
            s.expect_in_zone(RENO, "field");
            assert!(!fired(&s));
            s.expect_stats(RENO, json!({ "health": 6 }));
            assert_eq!(s.backrow(P2, 1).map(|card| card.def_id).as_deref(), Some(TRAP));
        }

        #[test]
        fn r61_a_spell_or_a_backrow_card_leaves_it_set() {
            let mut s = trap_set(json!([TRUE_STRIKE, MANA_WELL]), false);
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.play(MANA_WELL, json!({}));

            assert!(!fired(&s));
            assert_eq!(s.backrow(P2, 1).unwrap().def_id, TRAP);
        }

        #[test]
        fn r61_its_controllers_own_unit_leaves_it_set() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [RENO, HIT_JOB], "mana": 10,
                        "backrow": [{ "def": TRAP, "faceUp": false, "lane": 1 }] },
            }));
            s.play(RENO, json!({ "zone": 1 }));

            assert!(!fired(&s));
            s.expect_in_zone(RENO, "field");
        }

        #[test]
        fn r174_a_played_unit_gone_before_it_resolves_leaves_it_set() {
            // S #135 Sacked, set in lane 1, answers the same `cardResolved` first (R68) and takes the Unit
            // off the field beneath its Sack, so no Unit of that play is left to hit.
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "explosive-trap",
                "p1": { "hand": [RENO, HIT_JOB], "mana": 10 },
                "p2": { "backrow": [
                    { "def": "stinkyscape-135", "faceUp": false, "lane": 1 },
                    { "def": TRAP, "faceUp": false, "lane": 2 },
                ] },
            }));
            s.play(RENO, json!({ "zone": 1 }));

            assert_eq!(s.backrow(P2, 2).map(|card| card.def_id).as_deref(), Some(TRAP));
        }

        #[test]
        fn r386_damage_reads_through_param() {
            let mut s = trap_set(json!([RENO]), false);
            assert_eq!(upgrade_number(&mut s, TRAP, "damage"), 8);
            s.play(RENO, json!({ "zone": 1 }));

            let reno = s.card(RENO).id.clone();
            assert_eq!(damage_to(&s, &reno), 6);
            s.expect_health(P1, 28);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1750000_it_deals_12_the_excess_trampling() {
            let mut s = trap_set(json!([RENO]), true);
            s.play(RENO, json!({ "zone": 1 }));

            s.expect_in_zone(RENO, "graveyard");
            s.expect_health(P1, 24);
        }

        #[test]
        fn r2199001_pierce_an_armored_unit_takes_its_health_and_the_hero_the_rest() {
            // 7 Armor would take 7 of the 12 and leave 5 for its 7 Health: Pierce skips it, so the Unit
            // takes its 7 and the hero the other 5.
            let mut s = trap_set(json!([ARMORED]), true);
            s.play(ARMORED, json!({ "zone": 1 }));

            let armored = s.card(ARMORED).id.clone();
            assert_eq!(damage_to(&s, &armored), 7);
            s.expect_in_zone(ARMORED, "graveyard");
            s.expect_health(P1, 5);
        }
    }
}
