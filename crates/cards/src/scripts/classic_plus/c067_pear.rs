//! C+ #67 Pear (SPEC §8.7 row 67). (2) Spell, Fruit, Rare.
//!   Base:    "Summon {units|random (1) Cost Common Unit|random (1) Cost Common Units}."
//!   Radiant: "Summon {units|random Radiant (1) Cost Common Unit|random Radiant (1) Cost Common Units}."
//!   Engine:  "Non-token Units printed at (1) Cost (R65) with rarity Common, every set (R380); repeats
//!            allowed (R60); summoned, so no Cry (R1), placed per R64; a full board takes fewer.
//!            Tunes: units 2 ↑."
//!
//! Each Unit is its own `summonRandom`: one pick of the pool per summon (R60's repeats), placed in the
//! leftmost open, unlocked, unreserved zone (R64), and no pick at all when no zone is open (R129), so a
//! nearly full board takes one and a full one none. The pool's cost is R65's out-of-play cost, which is
//! what `cost` in a catalog query reads (an X-cost card reads 0, never 1).

use jackioh_engine::effects::summon_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-067";

/// §8.7: the printed cost of the Units it summons.
const UNIT_COST: i32 = 1;

fn pear(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(ctx, "units"))
                .map(|_| {
                    let mut args = json!({ "query": { "type": "Unit", "cost": UNIT_COST, "rarity": "Common" } });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    summon_random(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = pear(false);

    let radiant = pear(true);

    CardScripts { base, radiant }
}

// C+ #67 Pear — SPEC §8.7 row 67, BUILD M9 Classic+ row C+ 67: "Summons 2 random non-token Units of any
// set printed at (1) Cost and Common rarity (R380; out-of-play cost, R65), no Cry (R1), repeats allowed;
// one on a nearly full board, none on a full one and no random draw (R129); the count reads through
// `param()`; radiant the Units are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const PEAR: &str = "classicplus-067";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019";
    const ME_AND_MR_TOKEN: &str = "core-015"; // (1) Common Unit, "Cry: Summon a Rush Token."

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn summoned(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// TS `pear({ radiant?, seed?, field? })`.
    fn pear(radiant: bool, seed: Option<&str>, field: Option<usize>) -> Scenario {
        let mut card = json!({ "def": PEAR });
        if radiant {
            card["radiant"] = json!(true);
        }
        let field: Vec<Value> = (0..field.unwrap_or(0)).map(|_| json!(MENACE)).collect();
        scenario(json!({
            "seed": seed.unwrap_or("pear"),
            "p1": { "hand": [card, FILLER], "field": field },
            "p2": { "hand": [FILLER] }
        }))
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the live card in the state, tuned in place.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(instance, key, steps);
    }

    fn values(entries: &[(&str, i32)]) -> IndexMap<String, i32> {
        entries.iter().map(|(key, value)| ((*key).to_string(), *value)).collect()
    }

    #[test]
    fn is_a_2_fruit_spell_both_faces_one_shape_of_hook() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, PEAR);
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.tags.contains(&Tag::Fruit));
        let CardScripts { base, radiant } = script();
        assert!(base.cry.is_some());
        assert!(radiant.cry.is_some());
    }

    #[test]
    fn r482_the_text_agrees_with_its_count_at_every_value_one_unit_two_units() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(
            fill_params(&def, FaceKind::Base, Some(&values(&[("units", 1)]))),
            "Summon 1 random (1) Cost Common Unit."
        );
        assert_eq!(fill_params(&def, FaceKind::Base, None), "Summon 2 random (1) Cost Common Units.");
        assert_eq!(
            fill_params(&def, FaceKind::Radiant, Some(&values(&[("units", 1)]))),
            "Summon 1 random Radiant (1) Cost Common Unit."
        );
        assert_eq!(
            fill_params(&def, FaceKind::Radiant, Some(&values(&[("units", 3)]))),
            "Summon 3 random Radiant (1) Cost Common Units."
        );
    }

    mod base {
        use super::*;

        #[test]
        fn r64_summons_2_random_1_cost_common_units_into_your_leftmost_open_zones() {
            crate::register_all();
            let mut s = pear(false, None, None);
            s.play(PEAR, json!({}));
            let ids = summoned(&s);
            assert_eq!(ids.len(), 2);
            for id in &ids {
                let card = def_of(Some(s.state()), &s.card(id).def_id);
                assert_eq!(card.type_, CardType::Unit);
                assert_eq!(js(&card.cost), json!(1));
                assert_eq!(card.rarity, Rarity::Common);
                assert!(!card.token);
                assert!(!s.card(id).radiant);
            }
            let lanes = vec![s.unit(P1, 1).map(|unit| unit.id.clone()), s.unit(P1, 2).map(|unit| unit.id.clone())];
            let expected: Vec<Option<String>> = ids.iter().cloned().map(Some).collect();
            assert_eq!(lanes, expected);
        }

        #[test]
        fn r380_r65_over_many_seeds_every_unit_is_in_the_pool_of_non_token_1_cost_commons_of_every_set_and_more_than_one_set_shows() {
            crate::register_all();
            let pool: IndexSet<String> =
                jackioh_engine::catalog::query(&json_as(json!({ "type": "Unit", "cost": 1, "rarity": "Common" })))
                    .iter()
                    .map(|card| card.id.clone())
                    .collect();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let seed = format!("pear-{i}");
                let mut s = pear(false, Some(&seed), None);
                s.play(PEAR, json!({}));
                for id in summoned(&s) {
                    seen.insert(s.card(&id).def_id.clone());
                }
            }
            assert!(seen.iter().all(|id| pool.contains(id)));
            let sets: IndexSet<&str> = seen.iter().map(|id| id.split('-').next().unwrap_or("")).collect();
            assert!(sets.len() > 1);
        }

        #[test]
        fn r1_a_summoned_unit_fires_no_cry_me_and_mr_token_arrives_without_its_rush_token() {
            crate::register_all();
            let mut checked = false;
            let mut i = 0;
            while i < 200 && !checked {
                let seed = format!("pear-cry-{i}");
                i += 1;
                let mut s = pear(false, Some(&seed), None);
                s.play(PEAR, json!({}));
                let defs: Vec<String> = summoned(&s).iter().map(|id| s.card(id).def_id.clone()).collect();
                if !defs.iter().any(|id| id == ME_AND_MR_TOKEN) {
                    continue;
                }
                checked = true;
                assert_eq!(summoned(&s).len(), 2);
                assert!(!active_units_of(s.state(), P1).iter().any(|unit| unit.def_id == "core-t-rush"));
            }
            assert!(checked);
        }

        #[test]
        fn r60_repeats_are_allowed_some_seed_summons_the_same_unit_twice() {
            crate::register_all();
            let mut twice = false;
            let mut i = 0;
            while i < 200 && !twice {
                let seed = format!("pear-twice-{i}");
                i += 1;
                let mut s = pear(false, Some(&seed), None);
                s.play(PEAR, json!({}));
                let defs: Vec<String> = summoned(&s).iter().map(|id| s.card(id).def_id.clone()).collect();
                twice = defs.len() == 2 && defs[0] == defs[1];
            }
            assert!(twice);
        }

        #[test]
        fn r64_a_nearly_full_board_takes_one() {
            crate::register_all();
            let mut s = pear(false, None, Some(4));
            s.play(PEAR, json!({}));
            assert_eq!(summoned(&s).len(), 1);
            assert!(s.unit(P1, 5).is_some());
        }

        #[test]
        fn r129_a_full_board_summons_nothing_and_draws_nothing_from_the_rng() {
            crate::register_all();
            let mut s = pear(false, None, Some(5));
            let cursor = s.state().rng_cursor;
            s.play(PEAR, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r386_an_upgrade_summons_3_a_degrade_1() {
            crate::register_all();
            let mut up = pear(false, None, None);
            step(&mut up, PEAR, "units", 1);
            up.play(PEAR, json!({}));
            assert_eq!(summoned(&up).len(), 3);

            let mut down = pear(false, None, None);
            step(&mut down, PEAR, "units", -1);
            down.play(PEAR, json!({}));
            assert_eq!(summoned(&down).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_units_are_radiant() {
            crate::register_all();
            let mut s = pear(true, None, None);
            s.play(PEAR, json!({}));
            let ids = summoned(&s);
            assert_eq!(ids.len(), 2);
            for id in &ids {
                let unit = s.card(id).clone();
                assert!(unit.radiant);
                let printed = def_of(Some(s.state()), &unit.def_id).radiant.clone();
                s.expect_stats(
                    &unit,
                    json!({ "attack": printed.attack.unwrap_or(0), "maxHealth": printed.health.unwrap_or(0) }),
                );
            }
        }

        #[test]
        fn r129_a_full_board_summons_nothing_on_the_radiant_face_either() {
            crate::register_all();
            let mut s = pear(true, None, Some(5));
            let cursor = s.state().rng_cursor;
            s.play(PEAR, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
        }
    }
}
