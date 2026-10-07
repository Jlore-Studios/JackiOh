//! C+ #5 Guy Att (SPEC §8.7 row 5). (2) Unit, Human, Common, 6/8 → 12/16.
//! Cry: destroy every backrow card you control (Radiant: every backrow card), face-down ones included,
//! tops of backrow piles only; one state check (R59), Indestructible ones staying (R46). An Animated
//! card standing in a unit zone is a Unit and is not hit (R383); an Ivory Tower is, whatever it has fused
//! (R418), and a Unit standing on it while its play resolves is not (R446, R653).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-005";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "any", "rows": ["backrow"] })))])),
            ..Script::default()
        },
    }
}

// C+ #5 Guy Att — SPEC §8.7 row 5, BUILD M9 Classic+ row C+ 5: "Cry destroys every backrow card you
// control, face-down traps and Field Spells alike, an Indestructible one staying (R46); a destroyed
// backrow card that prints Death fires it (§4.5, C+ #61); an 'Animated on your turn' card animated as a
// Unit is not a backrow card and stays; no Cry when copied or recruited (R1); radiant destroys every
// backrow card on both sides, the opponent's face-down cards reaching their graveyard openly (R97)".
//
// The "prints Death" clause is proved with C+ #12.8 Frostspatula, a Field Spell whose Death fires when
// it is destroyed in the backrow (C+ #61 proves it again in its own file).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const GUY: &str = "classicplus-005";
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const MY_PAWN: &str = "core-096"; // (1) Trap: fires only on a lethal attack, so a play never sets it off.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const HEROIC_POWER: &str = "core-098"; // Indestructible Field Spell.
    const FROSTSPATULA: &str = "classicplus-012-8"; // Field Spell token: Animated on your turn, Rush.
    const TOWER: &str = "classicplus-033"; // Ivory Tower: the first Unit stacked onto it is fused into it.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const COOKIE_GUILD: &str = "classic-031"; // (2) Cry: Recruit 1 Unit of (2) Cost or less.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    /// TS `{ ...base, ...extra }` on two object literals (a shallow merge; `extra`'s keys win).
    fn merged(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(into), Value::Object(from)) = (out.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        out
    }

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": GUY, "radiant": radiant_face }, FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "mana": 8,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        }))
    }

    fn my_backrow() -> Value {
        json!([{ "def": SHEEPISH, "faceUp": false, "lane": 1 }, { "def": MANA_WELL, "lane": 2 }])
    }

    fn their_backrow() -> Value {
        json!([{ "def": MY_PAWN, "faceUp": false, "lane": 1 }, { "def": MANA_WELL, "lane": 3 }])
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    #[test]
    fn is_a_2_6_8_human_unit_radiant_12_16_with_a_cry_on_each_face() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(6), Some(8), Some(12), Some(16)]
        );
        assert_eq!(def.tags, vec![Tag::Human]);
        let scripts = script();
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_destroys_every_backrow_card_you_control_face_down_traps_and_field_spells_alike_theirs_stay() {
            crate::register_all();
            let mut s = setup(json!({ "backrow": my_backrow() }), json!({ "backrow": their_backrow() }), false);
            let mine = [s.backrow(P1, 1).unwrap(), s.backrow(P1, 2).unwrap()];
            let theirs = [s.backrow(P2, 1).unwrap(), s.backrow(P2, 3).unwrap()];

            s.play(GUY, json!({ "zone": 1 }));

            for card in &mine {
                s.expect_in_zone(card, "graveyard");
            }
            for card in &theirs {
                assert_eq!(s.card(card).zone.z(), ZoneName::Field);
            }
            assert_eq!(count(&s, "destroyed"), 2);
        }

        #[test]
        fn r46_an_indestructible_backrow_card_stays() {
            crate::register_all();
            let mut s = setup(
                json!({ "backrow": [{ "def": HEROIC_POWER, "lane": 3 }, { "def": MANA_WELL, "lane": 4 }] }),
                json!({}),
                false,
            );

            s.play(GUY, json!({ "zone": 1 }));

            assert_eq!(s.backrow(P1, 3).map(|card| card.def_id), Some(HEROIC_POWER.to_string()));
            s.expect_in_zone(MANA_WELL, "graveyard");
        }

        #[test]
        fn r383_an_animated_on_your_turn_card_standing_animated_in_a_unit_zone_is_a_unit_and_stays() {
            crate::register_all();
            let mut s = setup(
                json!({ "backrow": [{ "def": FROSTSPATULA, "lane": 3 }, { "def": MANA_WELL, "lane": 4 }] }),
                json!({}),
                false,
            );
            let spatula = s.card(FROSTSPATULA).clone();
            s.start_turn(); // B3.1: it animates at its controller's start of turn, into its lane's unit zone.
            assert_eq!(s.unit(P1, 3).map(|unit| unit.id), Some(spatula.id.clone()));

            s.play(GUY, json!({ "zone": 1 }));

            assert_eq!(s.unit(P1, 3).map(|unit| unit.id), Some(spatula.id.clone()));
            s.expect_in_zone(MANA_WELL, "graveyard");
        }

        #[test]
        fn r418_an_ivory_tower_is_a_backrow_card_and_is_destroyed_the_unit_it_fused_in_with_it() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [VANILLA, { "def": GUY }, FILLER], "backrow": [{ "def": TOWER, "lane": 2 }] }),
                json!({}),
                false,
            );
            let tower = s.card(TOWER).id.clone();
            let rider = s.card(VANILLA).clone();
            s.play(&rider, json!({ "zone": 2, "row": "backrow" }));
            // R653: once its play resolved, the Vanilla was fused into the Tower.
            s.expect_in_zone(&rider, "gone");
            assert!(carried_at(s.state(), &ZoneSlot { player: P1, row: Row::Backrow, lane: 2 }).is_none());

            s.play(GUY, json!({ "zone": 1 }));

            s.expect_in_zone(&tower, "graveyard");
        }

        #[test]
        fn r418_r653_a_guy_att_stacked_onto_your_ivory_tower_destroys_it_with_its_own_cry_and_steps_down_unfused() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": GUY }, FILLER], "backrow": [{ "def": TOWER, "lane": 2 }] }),
                json!({}),
                false,
            );
            let tower = s.card(TOWER).id.clone();
            let guy = s.card(GUY).id.clone();

            s.play(&guy, json!({ "zone": 2, "row": "backrow" }));

            s.expect_in_zone(&tower, "graveyard");
            assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(guy.clone()));
            assert_eq!(s.card(&guy).def_id, GUY);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Fused));
        }

        #[test]
        fn r1_a_recruited_guy_att_fires_no_cry_your_backrow_stays() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [COOKIE_GUILD, FILLER], "library": [GUY, STOCKPILE], "backrow": my_backrow(), "mana": 8 },
                "p2": { "hand": [FILLER] },
            }));

            s.play(COOKIE_GUILD, json!({ "zone": 1 }));

            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(GUY.to_string()));
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(SHEEPISH.to_string()));
            assert_eq!(s.backrow(P1, 2).map(|card| card.def_id), Some(MANA_WELL.to_string()));
            assert_eq!(count(&s, "destroyed"), 0);
        }

        #[test]
        fn with_an_empty_backrow_it_destroys_nothing() {
            crate::register_all();
            let mut s = setup(json!({}), json!({ "backrow": their_backrow() }), false);

            s.play(GUY, json!({ "zone": 1 }));

            assert_eq!(count(&s, "destroyed"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_every_backrow_card_on_both_sides() {
            crate::register_all();
            let mut s = setup(json!({ "backrow": my_backrow() }), json!({ "backrow": their_backrow() }), true);
            let all = [
                s.backrow(P1, 1).unwrap(),
                s.backrow(P1, 2).unwrap(),
                s.backrow(P2, 1).unwrap(),
                s.backrow(P2, 3).unwrap(),
            ];

            s.play(GUY, json!({ "zone": 1 }));

            for card in &all {
                s.expect_in_zone(card, "graveyard");
            }
            s.expect_stats(GUY, json!({ "attack": 12, "health": 16 }));
        }

        #[test]
        fn r97_the_opponent_s_face_down_trap_reaches_their_graveyard_openly_both_players_read_it_there() {
            crate::register_all();
            let mut s = setup(json!({}), json!({ "backrow": their_backrow() }), true);
            assert!(!serde_json::to_string(&s.view(P1).opponent.backrow).unwrap().contains(MY_PAWN));

            s.play(GUY, json!({ "zone": 1 }));

            assert_eq!(count(&s, "trapFired"), 0);
            assert!(s.view(P1).opponent.graveyard.iter().any(|card| card.def_id == MY_PAWN));
            assert!(s.view(P2).you.graveyard.iter().any(|card| card.def_id == MY_PAWN));
        }

        #[test]
        fn s4_5_a_destroyed_backrow_card_that_prints_death_fires_it_the_opponent_s_frostspatula_resummons_its_kill() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [{ "def": GUY, "radiant": true }, FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "field": [MR_TOKEN],
                    "mana": 8,
                },
                "p2": { "hand": [FROSTSPATULA, FILLER], "library": [STOCKPILE, STOCKPILE], "mana": 8 },
            }));
            let victim = s.card(MR_TOKEN).clone();
            s.play(FROSTSPATULA, json!({ "zone": 2 })); // R383: it animates into p2's unit zone 2 at once
            s.attack(FROSTSPATULA, &victim); // its Rush reaches units: the 1/1 dies, remembered (R42)
            s.end_turn(); // it returns to p2's backrow zone 2 at their cleanup
            assert_eq!(s.backrow(P2, 2).map(|card| card.def_id), Some(FROSTSPATULA.to_string()));

            s.play(GUY, json!({ "zone": 2 }));

            s.expect_in_zone(FROSTSPATULA, "graveyard");
            let copy = s.unit(P2, 1);
            assert_eq!(copy.as_ref().map(|card| card.def_id.as_str()), Some(MR_TOKEN));
            assert_ne!(copy.as_ref().map(|card| card.id.clone()), Some(victim.id.clone()));
            assert_eq!(copy.as_ref().map(|card| card.controller), Some(P2));
            // A summon, not a play: the copy's Cry (a Rush Token) never runs (R1).
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn r46_an_indestructible_backrow_card_on_either_side_stays() {
            crate::register_all();
            let mut s = setup(
                json!({}),
                json!({ "backrow": [{ "def": HEROIC_POWER, "lane": 2 }, { "def": MY_PAWN, "faceUp": false, "lane": 4 }] }),
                true,
            );

            s.play(GUY, json!({ "zone": 1 }));

            assert_eq!(s.backrow(P2, 2).map(|card| card.def_id), Some(HEROIC_POWER.to_string()));
            s.expect_in_zone(MY_PAWN, "graveyard");
        }
    }
}
