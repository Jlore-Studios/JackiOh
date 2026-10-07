//! C #16 Book of Flame (SPEC §8.6 row 16). (1) Spell, Book, Epic.
//!   Base:    "Deal {damage} damage." — damage 4
//!   Radiant: "Deal {damage} damage." — damage 8
//!   Engine:  "One targeted hit ("deal N damage" with no target named is targeted, as #68 Twisted
//!            Sorcerer's is). This is the Book of Flame that C #23 Devil's Pact and C #29 Book of
//!            Vital Kill name; C #55 Book of Wildfire is a different card with its own name (R381).
//!            Tunes: damage 4 ↑."
//!
//! §8's Conventions: "target" is any unit or hero on either side, and a bare `target` declaration is
//! units only (R90), so the heroes are named. The pick travels in the play action (R81). The hit is
//! one §4.4 instance, so Armor, Divine Shield, the anti-oneshot cap and Indestructible all apply.
//!
//! The amount is the declared number `damage` (R386), read through `param`: 4 on the base face, 8 on
//! the Radiant one. Both faces run this one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-016";

/// §8 Conventions: any unit or hero, either side. (TS `const targets: TargetDecl[]`.)
fn targets() -> Vec<TargetDecl> {
    vec![json_as(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 8 is its declared `damage`, which `param` reads off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #16 Book of Flame — SPEC §8.6 row 16, BUILD M9 Classic row C 16: "One targeted hit of 4 on any
// Unit or hero, either side; it is the Book of Flame C #23 and C #29 make (`classic-016`, R381);
// radiant 8; its tuned number (damage) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-016";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
    const FILLER: &str = "core-005";

    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {player:?} lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    /// TS `hits(s)`: each `damage` event's `{ targetId, amount }`.
    fn hits(s: &Scenario) -> Vec<(String, i32)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn refs_of(id: &str) -> Vec<String> {
        crate::CATALOG.get(id).and_then(|card| card.refs.clone()).unwrap_or_default()
    }

    #[test]
    fn declares_one_target_any_unit_or_hero_on_either_side_and_runs_one_script_on_both_faces() {
        assert_eq!(crate::card_def(ID).id, BOOK);
        let CardScripts { base, radiant } = script();
        assert_eq!(
            serde_json::to_value(&base.targets).unwrap(),
            json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }])
        );
        // TS `expect(radiant).toBe(base)`: one script, so one hook.
        assert!(Arc::ptr_eq(base.cry.as_ref().unwrap(), radiant.cry.as_ref().unwrap()));
        assert_eq!(radiant.targets, base.targets);
    }

    #[test]
    fn r381_it_is_the_one_card_named_book_of_flame_which_c_23_and_c_29_name_c_55_is_book_of_wildfire() {
        let named: Vec<String> = crate::CATALOG
            .values()
            .filter(|card| card.name == "Book of Flame")
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(named, [BOOK]);
        assert_eq!(crate::CATALOG.get("classic-055").map(|card| card.name.as_str()), Some("Book of Wildfire"));
    }

    #[test]
    fn r279_r381_c_23_devil_s_pact_and_c_29_book_of_vital_kill_name_this_card_and_no_entry_names_book_of_wildfire_for_it() {
        assert!(refs_of("classic-023").contains(&BOOK.to_string()));
        assert!(refs_of("classic-029").contains(&BOOK.to_string()));
        assert!(!refs_of("classic-023").contains(&"classic-055".to_string()));
        assert!(!refs_of("classic-029").contains(&"classic-055".to_string()));
    }

    mod base {
        use super::*;

        #[test]
        fn deals_4_to_an_enemy_unit_in_one_hit() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 5, "maxHealth": 9 }));
            assert_eq!(hits(&s).len(), 1);
        }

        #[test]
        fn deals_4_to_your_own_unit_either_side_is_legal() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "field": [MENACE] }, "p2": { "hand": [FILLER] } }));

            let targets = at(&s, P1, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 5 }));
        }

        #[test]
        fn deals_4_to_the_enemy_hero_and_to_your_own_if_you_pick_it() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, BOOK, FILLER], "mana": 3 }, "p2": { "hand": [FILLER] } }));
            let hand = s.hand(P1);
            let (Some(first), Some(second)) = (hand.first().cloned(), hand.get(1).cloned()) else {
                panic!("two Books in hand");
            };

            s.play(&first, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.play(&second, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

            s.expect_health(P2, 26).expect_health(P1, 26);
        }

        #[test]
        fn sec4_4_the_hit_goes_through_the_pipeline_armor_7_takes_all_4() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "field": [ARMORED] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(ARMORED, json!({ "health": 7 }));
        }

        #[test]
        fn r386_an_upgrade_makes_it_deal_5_a_degrade_3() {
            let mut up = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(up.card_mut(BOOK), "damage", 1);
            up.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            up.expect_health(P2, 25);

            let mut down = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(down.card_mut(BOOK), "damage", -1);
            down.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            down.expect_health(P2, 27);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_8_in_one_hit() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 1, "maxHealth": 9 }));
            assert_eq!(hits(&s).into_iter().map(|(_, amount)| amount).collect::<Vec<_>>(), vec![8]);
        }

        #[test]
        fn deals_8_to_a_hero() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_health(P2, 22);
        }

        #[test]
        fn r386_an_upgrade_steps_the_radiant_8_to_9() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(BOOK), "damage", 1);

            s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_health(P2, 21);
        }
    }
}
