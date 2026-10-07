//! C+ #7 The House (SPEC §8.7 row 7, R406). (3) Field Spell, Rare.
//! Cry and each start of its controller's turn: summon a Right-house defender (Core #3) with chance
//! 2 in 3, else a Wrong-House Attacker (C+ #6); Radiant: one of each. Fresh base-face cards, yours, no
//! Cry (R1), R64's placement; a full board summons nothing and rolls nothing (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-007";

/// R406: "Right-House Protector" is Core #3, the Wrong-House Attacker's twin.
const RIGHT_HOUSE: &str = "core-003";
const WRONG_HOUSE: &str = "classicplus-006";
/// R406: "(2 in 3)" — the odds of the Right-house defender.
const RIGHT_HOUSE_ODDS: f64 = 2.0 / 3.0;

fn summon_one() -> Hook {
    hook(|ctx| {
        if first_free_zone(&ctx.state, ctx.controller, Row::Units).is_none() {
            vec![]
        } else {
            let def_id = if ctx.rng.chance(RIGHT_HOUSE_ODDS) { RIGHT_HOUSE } else { WRONG_HOUSE };
            vec![summon(json_as(json!({ "defId": def_id })))]
        }
    })
}

fn summon_both() -> Hook {
    hook(|_ctx| {
        vec![summon(json_as(json!({ "defId": RIGHT_HOUSE }))), summon(json_as(json!({ "defId": WRONG_HOUSE })))]
    })
}

pub fn script() -> CardScripts {
    let one = summon_one();
    let both = summon_both();
    CardScripts {
        base: Script {
            cry: Some(one.clone()),
            start_of_turn: Some(one),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(both.clone()),
            start_of_turn: Some(both),
            ..Script::default()
        },
    }
}

// C+ #7 The House — SPEC §8.7 row 7, BUILD M9 Classic+ row C+ 7: "Field Spell: its Cry and each start of
// your turn summon a Right-house defender (Core #3) with chance 2 in 3, else a Wrong-House Attacker (C+
// #6), generated from the catalog (cards, not tokens, so they go to your graveyard when they die),
// yours, no Cry (R1); 'Right-House Protector' is Core #3 (R406); a seeded roll gives a fixed unit;
// nothing at the opponent's start of turn; a full board summons nothing and rolls nothing (R129);
// radiant summons one of each, the second skipped when only one zone is open".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const HOUSE: &str = "classicplus-007";
    const RIGHT: &str = "core-003"; // Right-house defender
    const WRONG: &str = "classicplus-006"; // Wrong-House Attacker
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
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

    fn setup(p1: Value, radiant_face: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": HOUSE, "radiant": radiant_face }, FILLER],
                    "library": [STOCKPILE, STOCKPILE, STOCKPILE],
                    "mana": 8,
                }),
                p1,
            ),
            "p2": { "hand": [FILLER], "library": [STOCKPILE, STOCKPILE, STOCKPILE] },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    /// The Units summoned, in order — not The House's own arrival in its backrow zone.
    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } if def_id != HOUSE => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The defs The House summoned with its Cry, under a seed.
    fn cry_roll(seed: &str) -> Vec<String> {
        let mut s = setup(json!({}), false, Some(seed));
        s.play(HOUSE, json!({ "zone": 1 }));
        summoned(&s)
    }

    #[test]
    fn is_a_3_field_spell_naming_core_n3_and_c_n6_each_face_has_a_cry_and_a_start_of_turn_hook() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.cost, CardCost::Fixed(3));
        assert_eq!(def.refs, Some(vec![RIGHT.to_string(), WRONG.to_string()]));
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert!(face.cry.is_some());
            assert!(face.start_of_turn.is_some());
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r406_its_cry_summons_one_of_the_twins_yours_with_no_cry_a_seeded_roll_gives_a_fixed_unit() {
            crate::register_all();
            let mut s = setup(json!({}), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));

            let defs = summoned(&s);
            assert_eq!(defs.len(), 1);
            assert!([RIGHT, WRONG].contains(&defs[0].as_str()));
            let unit = s.unit(P1, 1).expect("a twin in lane 1");
            assert_eq!(
                (unit.def_id.clone(), unit.owner, unit.controller, unit.radiant),
                (defs[0].clone(), P1, P1, false)
            );
            let played: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::CardPlayed { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(played, vec![s.card(HOUSE).id.clone()]);
            assert_eq!(cry_roll("jackioh-harness"), defs);
        }

        #[test]
        fn r406_2_in_3_is_the_right_house_defender_s_chance_both_come_up_the_defender_about_twice_as_often() {
            crate::register_all();
            let mut right = 0;
            let mut wrong = 0;
            for seed in 1..=90 {
                let made = cry_roll(&format!("house-{seed}"));
                match made.first().map(String::as_str) {
                    Some(RIGHT) => right += 1,
                    Some(WRONG) => wrong += 1,
                    _ => {}
                }
            }
            assert_eq!(right + wrong, 90);
            assert!(right > 45);
            assert!(right < 75);
        }

        #[test]
        fn r11_the_twins_are_cards_not_tokens_a_wrong_house_attacker_it_made_goes_to_your_graveyard_when_it_dies() {
            crate::register_all();
            let seed = (1..=30)
                .map(|i| format!("house-{i}"))
                .find(|each| cry_roll(each).first().map(String::as_str) == Some(WRONG))
                .expect("no seed rolled the Wrong-House Attacker");
            let mut s = setup(json!({ "hand": [{ "def": HOUSE }, HIT_JOB, FILLER] }), false, Some(&seed));
            s.play(HOUSE, json!({ "zone": 1 }));
            let made = s.unit(P1, 1).expect("a twin in lane 1");
            assert_eq!(made.def_id, WRONG);

            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": made.id }] }));

            s.expect_in_zone(&made, "graveyard");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == WRONG));
        }

        #[test]
        fn s2_2_summons_again_at_the_start_of_your_turn_and_nothing_at_the_opponent_s() {
            crate::register_all();
            let mut s = setup(json!({}), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            assert_eq!(summoned(&s).len(), 1);

            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert_eq!(summoned(&s).len(), 1);

            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(summoned(&s).len(), 2);
            let second = s.unit(P1, 2).map(|unit| unit.def_id).unwrap_or_default();
            assert!(second == "core-003" || second == "classicplus-006");
        }

        #[test]
        fn r129_a_full_board_summons_nothing_and_rolls_nothing() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }), false, None);
            let cursor = s.state().rng_cursor;

            s.play(HOUSE, json!({ "zone": 1 }));

            assert_eq!(summoned(&s), Vec::<String>::new());
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r64_lands_in_the_leftmost_open_unit_zone() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, { "def": VANILLA, "lane": 3 }] }), false, None);

            s.play(HOUSE, json!({ "zone": 1 }));

            let lane_2 = s.unit(P1, 2).map(|unit| unit.def_id).unwrap_or_default();
            assert!([RIGHT, WRONG].contains(&lane_2.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_summons_a_right_house_defender_and_then_a_wrong_house_attacker_rolling_nothing() {
            crate::register_all();
            let mut s = setup(json!({}), true, None);
            let cursor = s.state().rng_cursor;

            s.play(HOUSE, json!({ "zone": 1 }));

            assert_eq!(summoned(&s), vec![RIGHT, WRONG]);
            assert_eq!(
                [s.unit(P1, 1).map(|unit| unit.def_id), s.unit(P1, 2).map(|unit| unit.def_id)],
                [Some(RIGHT.to_string()), Some(WRONG.to_string())]
            );
            // Generated on their base faces, yours.
            assert_eq!(
                [1, 2]
                    .iter()
                    .map(|&lane| s.unit(P1, lane).map(|unit| (unit.radiant, unit.owner)))
                    .collect::<Vec<_>>(),
                vec![Some((false, P1)), Some((false, P1))]
            );
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn with_one_zone_open_the_second_is_skipped() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA] }), true, None);

            s.play(HOUSE, json!({ "zone": 1 }));

            assert_eq!(summoned(&s), vec![RIGHT]);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.def_id), Some(RIGHT.to_string()));
        }

        #[test]
        fn summons_both_again_at_the_start_of_your_turn() {
            crate::register_all();
            let mut s = setup(json!({}), true, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            s.end_turn();
            s.end_turn();

            assert_eq!(summoned(&s), vec![RIGHT, WRONG, RIGHT, WRONG]);
        }
    }
}
