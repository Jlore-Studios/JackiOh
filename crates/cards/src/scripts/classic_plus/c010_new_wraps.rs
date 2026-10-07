//! C+ #10 New Wraps (SPEC §8.7 row 10). (0) Spell, Common.
//! A target Unit on either side gains Reborn as a granted keyword (§10.4, R78). Radiant: then the same
//! Unit is made Radiant (§5.2: its face swaps at once, damage and buffs kept, no Cry).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-010";

// R656: Reborn helps, so a random cast that targets enemies aims this at friends.
fn a_unit() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))
    }]
}

fn give_reborn() -> Effect {
    grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Reborn" } })))
}

pub fn script() -> CardScripts {
    // One effect, as TS's module constant `giveReborn`, shared by both faces' Cries.
    let give = give_reborn();
    let on_base = give.clone();
    let on_radiant = give;
    CardScripts {
        base: Script {
            targets: a_unit(),
            cry: Some(hook(move |_ctx| vec![on_base.clone()])),
            ..Script::default()
        },
        radiant: Script {
            targets: a_unit(),
            cry: Some(hook(move |_ctx| vec![on_radiant.clone(), set_radiant(Default::default())])),
            ..Script::default()
        },
    }
}

// C+ #10 New Wraps — SPEC §8.7 row 10, BUILD M9 Classic+ row C+ 10: "A target Unit gains Reborn as a
// granted keyword (kept by a Vanilla, lost on leaving the field, R78), so its next death returns it at
// 1 health without Reborn; a Unit that has Reborn gains nothing; radiant also makes it Radiant, its
// Radiant face applying at once with damage and buffs kept and no Cry (§5.2)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    const WRAPS: &str = "classicplus-010";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const DEFENDER: &str = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const FLOOD: &str = "core-017"; // (4) Bounce all Units.
    const SILENCE: &str = "classicplus-009"; // (0) Vanilla a Unit.
    const SURGERY: &str = "core-063"; // (1) +3/+3 and 1 random keyword.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    use crate::merged;

    fn setup(p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(json!({ "library": [STOCKPILE, STOCKPILE], "mana": 10 }), p1),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        }))
    }

    fn at(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    fn reborn_count(s: &Scenario, id: &str) -> usize {
        s.stats(id).keywords.iter().filter(|k| k.kind() == KeywordKind::Reborn).count()
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    #[test]
    fn is_a_0_spell_targeting_a_unit_on_either_side_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.cost, CardCost::Fixed(0));
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                serde_json::to_value(&face.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "aim": "help", "filter": { "side": "any", "of": ["unit"] } }])
            );
        }
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_r64_the_unit_gains_reborn_as_a_granted_keyword_its_next_death_returns_it_at_1_health_without_reborn() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, HIT_JOB, HIT_JOB, FILLER], "field": [VANILLA] }), json!({}));
            let vanilla = s.card(VANILLA).clone();

            s.play(WRAPS, json!({ "targets": at(&vanilla.id) }));
            assert!(s.card(&vanilla).granted_keywords.contains(&Keyword::Reborn));
            assert_eq!(reborn_count(&s, &vanilla.id), 1);

            s.play(HIT_JOB, json!({ "targets": at(&vanilla.id) }));
            let back = s.unit(P1, 1).expect("the Reborn body");
            assert_eq!(back.def_id, VANILLA);
            s.expect_stats(&back, json!({ "attack": 4, "health": 1 }));
            assert_eq!(reborn_count(&s, &back.id), 0);

            s.play(HIT_JOB, json!({ "targets": at(&back.id) }));
            assert!(s.unit(P1, 1).is_none());
            s.expect_in_zone(&vanilla, "graveyard");
        }

        #[test]
        fn reaches_an_enemy_unit_too() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, FILLER] }), json!({ "field": [VANILLA] }));

            let vanilla = s.card(VANILLA).id.clone();
            s.play(WRAPS, json!({ "targets": at(&vanilla) }));

            assert_eq!(reborn_count(&s, VANILLA), 1);
        }

        #[test]
        fn s10_4_a_vanilla_keeps_the_granted_reborn() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, SILENCE, HIT_JOB, FILLER], "field": [VANILLA] }), json!({}));
            let vanilla = s.card(VANILLA).clone();

            s.play(WRAPS, json!({ "targets": at(&vanilla.id) }));
            s.play(SILENCE, json!({ "targets": at(&vanilla.id) }));
            assert_eq!(reborn_count(&s, &vanilla.id), 1);

            s.play(HIT_JOB, json!({ "targets": at(&vanilla.id) }));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
        }

        #[test]
        fn r78_leaving_the_field_clears_it_a_bounced_unit_comes_back_to_hand_without_it() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, FLOOD, FILLER], "field": [VANILLA] }), json!({}));
            let vanilla = s.card(VANILLA).clone();

            s.play(WRAPS, json!({ "targets": at(&vanilla.id) }));
            s.play(FLOOD, json!({}));

            assert_eq!(s.card(&vanilla).zone.z(), ZoneName::Hand);
            assert!(s.card(&vanilla).granted_keywords.is_empty());
        }

        #[test]
        fn a_unit_that_has_reborn_gains_no_second_one_it_still_returns_once() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, HIT_JOB, HIT_JOB, FILLER], "field": [DEFENDER] }), json!({}));
            let defender = s.card(DEFENDER).clone();

            s.play(WRAPS, json!({ "targets": at(&defender.id) }));
            assert_eq!(reborn_count(&s, &defender.id), 1);

            s.play(HIT_JOB, json!({ "targets": at(&defender.id) }));
            let back = s.unit(P1, 1).expect("the Reborn body");
            assert_eq!(reborn_count(&s, &back.id), 0);
            s.play(HIT_JOB, json!({ "targets": at(&back.id) }));
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r570_on_a_unit_that_prints_reborn_the_grant_is_still_recorded_a_later_vanilla_leaves_it_the_granted_reborn() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, SILENCE, HIT_JOB, FILLER], "field": [DEFENDER] }), json!({}));
            let defender = s.card(DEFENDER).clone();
            s.play(WRAPS, json!({ "targets": at(&defender.id) }));
            assert!(s.card(&defender).granted_keywords.contains(&Keyword::Reborn));

            s.play(SILENCE, json!({ "targets": at(&defender.id) }));
            assert_eq!(reborn_count(&s, &defender.id), 1);
            s.play(HIT_JOB, json!({ "targets": at(&defender.id) }));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(DEFENDER.to_string()));

            // Without New Wraps a Vanilla Right-house defender has no Reborn left, and stays dead.
            let mut bare = setup(json!({ "hand": [SILENCE, HIT_JOB, FILLER], "field": [DEFENDER] }), json!({}));
            let target = bare.card(DEFENDER).id.clone();
            bare.play(SILENCE, json!({ "targets": at(&target) }));
            let target = bare.card(DEFENDER).id.clone();
            bare.play(HIT_JOB, json!({ "targets": at(&target) }));
            assert!(bare.unit(P1, 1).is_none());
        }

        #[test]
        fn a_reborn_body_that_spent_its_reborn_gains_it_again_and_returns_once_more() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [WRAPS, HIT_JOB, HIT_JOB, FILLER], "field": [DEFENDER] }), json!({}));
            let defender = s.card(DEFENDER).id.clone();
            s.play(HIT_JOB, json!({ "targets": at(&defender) }));
            let body = s.unit(P1, 1).expect("the Reborn body");
            assert_eq!(reborn_count(&s, &body.id), 0);

            s.play(WRAPS, json!({ "targets": at(&body.id) }));
            assert_eq!(reborn_count(&s, &body.id), 1);
            s.play(HIT_JOB, json!({ "targets": at(&body.id) }));

            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(DEFENDER.to_string()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s5_2_also_makes_it_radiant_at_once_the_radiant_stats_apply_with_its_damage_and_buffs_kept_and_no_cry() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": WRAPS, "radiant": true }, SURGERY, FILLER], "field": [{ "def": MR_TOKEN, "damage": 0 }] }),
                json!({}),
            );
            let unit = s.card(MR_TOKEN).clone();
            s.play(SURGERY, json!({ "targets": at(&unit.id) })); // +3/+3: a 4/4 Mr Token

            s.play(WRAPS, json!({ "targets": at(&unit.id) }));

            assert!(s.card(&unit).radiant);
            s.expect_stats(&unit, json!({ "attack": 2 + 3, "maxHealth": 2 + 3 }));
            assert_eq!(reborn_count(&s, &unit.id), 1);
            s.expect_events(json!(["keywordGranted", "radiantSet"]));
            assert_eq!(count(&s, "summoned"), 0);
        }

        #[test]
        fn s5_2_damage_is_kept_through_the_face_swap() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": WRAPS, "radiant": true }, FILLER], "field": [{ "def": VANILLA, "damage": 3 }] }),
                json!({}),
            );

            let vanilla = s.card(VANILLA).id.clone();
            s.play(WRAPS, json!({ "targets": at(&vanilla) }));

            s.expect_stats(VANILLA, json!({ "attack": 12, "maxHealth": 12, "health": 9 })); // Radiant Mr. Vanilla is 12/12
        }
    }
}
