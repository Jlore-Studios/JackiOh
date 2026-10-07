//! C+ #4 Juhan Biggest Bat (SPEC §8.7 row 4). (3) Unit, CN, Common, 9/6 → 18/12, Stack (+ First Strike).
//! Played onto a pile, every dormant card beneath becomes a copy of this on its face (E24): owner and
//! controller kept, still dormant, no Cry (R1), Immutable ones untouched (R23). Onto an empty zone it
//! does nothing. Its Cry is the arrival: only a play fires it (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-004";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![transform_beneath(Default::default())])),
        ..Script::default()
    };
    // The Radiant face adds First Strike and doubles the stats (catalog data); its copies are Radiant as it is.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #4 Juhan Biggest Bat — SPEC §8.7 row 4, BUILD M9 Classic+ row C+ 4: "Stack; played onto your
// occupied unit zone, every dormant card beneath becomes a copy of Juhan on its face, keeping each old
// card's owner and controller and staying dormant, an Immutable card staying itself (R23); no copy
// fires a Cry (R1); when the top Juhan leaves, the next Juhan resumes; played onto an empty zone nothing
// is transformed; radiant Stack, First Strike, and the copies are Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const JUHAN: &str = "classicplus-004";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FIENDER: &str = "core-092"; // (2) 5/7 Stack.
    const MENACE: &str = "core-019"; // Radiant: Taunt, Immutable.
    const MIND_CONTROL: &str = "core-049"; // (4) Steal target enemy permanent.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    const LANE_1: ZoneSlot = ZoneSlot { player: P1, row: Row::Units, lane: 1 };

    use crate::merged;

    fn setup(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": JUHAN, "radiant": radiant_face }, FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        }))
    }

    fn beneath(s: &Scenario) -> Vec<CardInstance> {
        beneath_at(s.state(), &LANE_1).to_vec()
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|k| k.kind()).collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn is_a_3_9_6_cn_unit_with_stack_radiant_18_12_with_stack_and_first_strike_one_script_runs_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(3));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(9), Some(6), Some(18), Some(12)]
        );
        assert_eq!(kinds(&def.base.keywords), vec![KeywordKind::Stack]);
        assert_eq!(kinds(&def.radiant.keywords), vec![KeywordKind::Stack, KeywordKind::FirstStrike]);
        // TS `expect(radiant).toBe(base)`: the Radiant face is the base face's very script.
        let scripts = script();
        assert!(matches!(
            (&scripts.base.cry, &scripts.radiant.cry),
            (Some(a), Some(b)) if Arc::ptr_eq(a, b)
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn e24_played_onto_a_pile_every_dormant_card_beneath_becomes_a_dormant_juhan_the_old_cards_cease_to_exist() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), false, json!({}));
            let mut old_ids = vec![s.unit(P1, 1).unwrap().id];
            old_ids.extend(beneath(&s).iter().map(|card| card.id.clone()));
            assert_eq!(old_ids.len(), 2);
            let juhan = s.card(JUHAN).clone();

            s.play(&juhan, json!({ "zone": 1 }));

            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(juhan.id.clone()));
            let under = beneath(&s);
            assert_eq!(def_ids(&under), vec![JUHAN, JUHAN]);
            assert!(under.iter().all(|card| !card.radiant && card.owner == P1 && card.controller == P1));
            assert_eq!(count(&s, "transformed"), 2);
            for id in &old_ids {
                s.expect_in_zone(id, "gone");
            }
        }

        #[test]
        fn r1_no_copy_fires_a_cry_one_play_one_cry_one_transform_per_card_beneath() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), false, json!({}));
            let juhan = s.card(JUHAN).clone();

            s.play(&juhan, json!({ "zone": 1 }));

            assert_eq!(count(&s, "cardPlayed"), 1);
            assert_eq!(count(&s, "transformed"), 2);
            // The one `summoned` is the played Juhan's own arrival; a copy is made in place, not summoned.
            let summoned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(summoned, vec![juhan.id.clone()]);
        }

        #[test]
        fn s6_3_replace_each_copy_keeps_the_old_card_s_owner_and_controller() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [MIND_CONTROL, { "def": JUHAN }], "mana": 7 }),
                false,
                json!({ "field": [VANILLA] }),
            );
            let stolen = s.unit(P2, 1).unwrap();

            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": stolen.id }] }));
            let Zone::Field { lane, .. } = s.card(&stolen).zone else {
                panic!("Mind Control did not take the Vanilla");
            };
            s.play(JUHAN, json!({ "zone": lane }));

            let copy = beneath_at(s.state(), &ZoneSlot { player: P1, row: Row::Units, lane }).first().cloned();
            assert_eq!(copy.as_ref().map(|card| card.def_id.as_str()), Some(JUHAN));
            assert_eq!(copy.as_ref().map(|card| card.owner), Some(P2));
            assert_eq!(copy.as_ref().map(|card| card.controller), Some(P1));
        }

        #[test]
        fn r23_an_immutable_card_beneath_stays_itself() {
            crate::register_all();
            let mut s = setup(
                json!({ "field": [{ "def": MENACE, "radiant": true }, { "def": FIENDER, "stack": true }] }),
                false,
                json!({}),
            );

            s.play(JUHAN, json!({ "zone": 1 }));

            assert_eq!(def_ids(&beneath(&s)), vec![JUHAN, MENACE]);
            assert_eq!(count(&s, "transformed"), 1);
        }

        #[test]
        fn s3_2_when_the_top_juhan_leaves_the_next_juhan_resumes() {
            crate::register_all();
            let mut s = setup(
                json!({
                    "hand": [{ "def": JUHAN }, HIT_JOB, FILLER],
                    "field": [VANILLA, { "def": FIENDER, "stack": true }],
                    "mana": 6,
                }),
                false,
                json!({}),
            );
            let top = s.card(JUHAN).clone();
            s.play(&top, json!({ "zone": 1 }));
            let next = beneath(&s).first().cloned().expect("a Juhan beneath");

            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": top.id }] }));

            s.expect_in_zone(&top, "graveyard");
            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(next.id.clone()));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(JUHAN.to_string()));
            assert_eq!(def_ids(&beneath(&s)), vec![JUHAN]);
            s.expect_stats(&next, json!({ "attack": 9, "health": 6 }));
        }

        #[test]
        fn played_onto_an_empty_zone_it_transforms_nothing() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA] }), false, json!({}));

            s.play(JUHAN, json!({ "zone": 2 }));

            assert_eq!(count(&s, "transformed"), 0);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            assert!(beneath_at(s.state(), &ZoneSlot { player: P1, row: Row::Units, lane: 2 }).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn has_stack_and_first_strike_and_the_copies_beneath_are_radiant_juhans() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), true, json!({}));
            let juhan = s.card(JUHAN).clone();

            s.play(&juhan, json!({ "zone": 1 }));

            let now = kinds(&s.stats(&juhan).keywords);
            assert!(now.contains(&KeywordKind::Stack));
            assert!(now.contains(&KeywordKind::FirstStrike));
            s.expect_stats(&juhan, json!({ "attack": 18, "health": 12 }));
            assert_eq!(
                beneath(&s).iter().map(|card| (card.def_id.clone(), card.radiant)).collect::<Vec<_>>(),
                vec![(JUHAN.to_string(), true), (JUHAN.to_string(), true)]
            );
        }
    }
}
