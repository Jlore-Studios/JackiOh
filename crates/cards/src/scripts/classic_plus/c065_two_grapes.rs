//! C+ #65 Two Grapes (SPEC §8.7 row 65). (1) Spell, Fruit, Rare.
//!   Base:    "Add {grapes|Grape|Grapes} to your hand." (exactly two, each rolled, balance patch 1;
//!            the odds stay in code, never in player text)
//!   Radiant: "Lucky 1 / Add {grapes|Radiant Grape|Radiant Grapes} to your hand."
//!   Engine:  "Two independent weighted rolls (`GRAPE_ODDS` in `config.ts`) over C+ #65.1 to C+ #65.5,
//!            a pool the text names; Lucky 1 (§6.1) rolls twice and keeps the better, in the order
//!            Rotten < Normal < Large < Golden < Mythic. The hand cap burns extras (§2.4). Tunes:
//!            grapes 2 ↑."
//!
//! The roll is the engine's `addRolledGrapes` (effects/fruit.ts, shared with C+ #66 Vine of Grapes):
//! one draw over `GRAPE_ODDS` per Grape, Lucky's extra draws keeping the later entry, each Grape added
//! before the next is rolled. The Lucky it reads is the running card's own — the Radiant face prints
//! Lucky 1, and a Degrade or an Upgrade of that number moves it (B3.4's X change) — so both faces run
//! one hook and differ by the face's keyword and by whether the Grapes are Radiant.

use jackioh_engine::effects::add_rolled_grapes;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-065";

fn two_grapes(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "count": param(ctx, "grapes") });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![add_rolled_grapes(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = two_grapes(false);

    let radiant = two_grapes(true);

    CardScripts { base, radiant }
}

// C+ #65 Two Grapes — SPEC §8.7 row 65, BUILD M9 Classic+ row C+ 65: "Add {grapes} to your hand"
// (exactly two, balance patch 1; each rolled on its own from `GRAPE_ODDS` in code, never in player
// text); a fixed seed gives fixed Grapes and many seeded rolls match the odds; a full hand burns;
// hidden from the opponent (R97); the count reads through `param()`; radiant Lucky 1, two Radiant
// Grapes, each rolled twice with the better kept in the order Rotten, Normal, Large, Golden, Mythic".
//
// The odds themselves are proved on 20,000 rolls of the verb in packages/engine/test/effects-fruit.test.ts;
// here the card is played, many times over, and its Grapes counted.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRAPES: &str = "classicplus-065";
    const FILLER: &str = "core-005";

    fn grape_ids() -> Vec<&'static str> {
        GRAPE_ODDS.iter().map(|grape| grape.def_id).collect()
    }

    /// TS `GRAPE_IDS.indexOf(id)`: −1 for an id that is no Grape.
    fn index_of(id: &str) -> i64 {
        grape_ids().iter().position(|grape| *grape == id).map_or(-1, |at| at as i64)
    }

    use crate::js;

    /// One `addedToHand` event of p1's, as TS's `Extract<GameEvent, { type: "addedToHand" }>` reads it.
    struct Added {
        instance_id: String,
        def_id: String,
    }

    fn added(s: &Scenario) -> Vec<Added> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } => Some(Added {
                    instance_id: instance_id.clone(),
                    def_id: def_id.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// TS `played({ radiant?, seed?, hand? })`.
    fn played(radiant: bool, seed: Option<&str>, hand: Option<usize>) -> Scenario {
        let mut grapes = json!({ "def": GRAPES });
        if radiant {
            grapes["radiant"] = json!(true);
        }
        let mut cards = vec![grapes];
        cards.extend((0..hand.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("two-grapes"),
            "p1": { "hand": cards },
            "p2": { "hand": [FILLER] }
        }))
    }

    /// TS `s.card(ref).tuning = …`: the live card's tuning replaced.
    fn set_tuning(s: &mut Scenario, card: &str, tuning: Value) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        instance.tuning = Some(json_as(tuning));
    }

    #[test]
    fn is_a_1_fruit_spell_with_no_refs_grape_names_the_five_grapes_r480_and_both_faces_run_one_shape_of_hook() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, GRAPES);
        assert_eq!(js(&def.cost), json!(1));
        assert!(def.tags.contains(&Tag::Fruit));
        assert!(def.refs.is_none());
        assert_eq!(
            js(&def.params),
            json!([{ "key": "grapes", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let CardScripts { base, radiant } = script();
        assert!(base.cry.is_some());
        assert!(radiant.cry.is_some());
        assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Lucky", "n": 1 }]));
    }

    mod base {
        use super::*;

        #[test]
        fn r382_adds_exactly_2_grapes_each_one_of_the_five() {
            crate::register_all();
            let mut s = played(false, None, None);
            s.play(GRAPES, json!({}));

            let grapes = added(&s);
            assert_eq!(grapes.len(), 2);
            assert!(grapes.iter().all(|event| grape_ids().contains(&event.def_id.as_str())));
            assert!(grapes.iter().all(|event| !s.card(&event.instance_id).radiant));
            assert_eq!(s.card(GRAPES).zone.z(), ZoneName::Graveyard);
        }

        #[test]
        fn r382_each_grape_is_its_own_roll_one_draw_of_the_rng_each_two_in_all() {
            crate::register_all();
            let mut s = played(false, None, None);
            let before = s.state().rng_cursor;
            s.play(GRAPES, json!({}));
            assert_eq!(s.state().rng_cursor - before, 2);
        }

        #[test]
        fn s10_7_a_fixed_seed_gives_fixed_grapes_the_very_ones_the_roll_gives_from_the_same_cursor() {
            crate::register_all();
            let mut a = played(false, Some("fixed"), None);
            let cursor = a.state().rng_cursor;
            a.play(GRAPES, json!({}));
            let mut b = played(false, Some("fixed"), None);
            b.play(GRAPES, json!({}));
            let ids = |s: &Scenario| added(s).into_iter().map(|event| event.def_id).collect::<Vec<_>>();
            assert_eq!(ids(&a), ids(&b));

            let mut rng = Rng::new("fixed", cursor);
            let first = jackioh_engine::catalog::roll_grape(&mut rng, 0);
            let second = jackioh_engine::catalog::roll_grape(&mut rng, 0);
            assert_eq!(ids(&a), vec![first, second]);
        }

        #[test]
        fn r382_many_seeded_plays_match_the_odds_normal_is_the_commonest_and_every_grape_comes_up() {
            crate::register_all();
            let mut counts: IndexMap<String, i32> = IndexMap::new();
            let plays = 250;
            for i in 0..plays {
                let seed = format!("odds-{i}");
                let mut s = played(false, Some(&seed), None);
                s.play(GRAPES, json!({}));
                for event in added(&s) {
                    *counts.entry(event.def_id).or_insert(0) += 1;
                }
            }
            let total = f64::from(plays * 2);
            let ids = grape_ids();
            let share = |at: usize| -> f64 {
                let id = ids.get(at).copied().unwrap_or("");
                f64::from(counts.get(id).copied().unwrap_or(0)) / total * 100.0
            };
            assert!((share(1) - 60.0).abs() < 6.0);
            assert!((share(0) - 12.0).abs() < 4.0);
            assert!((share(2) - 20.0).abs() < 5.0);
            assert!(share(3) > 3.0);
            assert!(share(3) < 11.0);
            assert!(share(4) < 4.0);
            assert!(ids[..4].iter().all(|id| counts.get(*id).copied().unwrap_or(0) > 0));
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_grapes_that_don_t_fit() {
            crate::register_all();
            let mut s = played(false, None, Some(9));
            s.play(GRAPES, json!({}));
            assert_eq!(s.hand(P1).len(), 10);
            let burned = s.last_events().iter().filter(|event| event.event_type().as_str() == "burned").count();
            assert_eq!(burned, 1);
            assert_eq!(added(&s).len(), 1);
        }

        #[test]
        fn r97_the_opponent_sees_two_cards_reach_a_hand_under_the_sentinel_never_which_grapes() {
            crate::register_all();
            let mut s = played(false, None, None);
            s.play(GRAPES, json!({}));
            let theirs: Vec<GameEvent> = s
                .view(P2)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert_eq!(theirs.len(), 2);
            for event in &theirs {
                let GameEvent::AddedToHand { instance_id, def_id, .. } = event else {
                    continue;
                };
                assert_eq!(def_id, "hidden");
                assert_eq!(instance_id, "hidden");
            }
            let hand = s.view(P2).opponent.hand;
            assert_eq!(js(&hand), json!({ "count": 3 }));
        }

        #[test]
        fn r386_an_upgrade_of_its_count_adds_3_grapes_a_degrade_1_never_fewer_than_1() {
            crate::register_all();
            let mut up = played(false, None, None);
            step_param(up.card_mut(GRAPES), "grapes", 1);
            up.play(GRAPES, json!({}));
            assert_eq!(added(&up).len(), 3);

            let mut down = played(false, None, None);
            step_param(down.card_mut(GRAPES), "grapes", -1);
            down.play(GRAPES, json!({}));
            assert_eq!(added(&down).len(), 1);

            let mut floor = played(false, None, None);
            step_param(floor.card_mut(GRAPES), "grapes", -5);
            floor.play(GRAPES, json!({}));
            assert_eq!(added(&floor).len(), 1);
        }
    }

    /// R1438: given Lucky 1, the base face rolls each of its two Grapes twice and keeps the better:
    /// four draws of the rng, and the Grapes the same cursor's pairs predict, none of them Radiant.
    #[test]
    fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
        crate::register_all();
        let mut s = played(false, Some("lucky"), None);
        crate::give_lucky(&mut s, GRAPES, 1);
        let cursor = s.state().rng_cursor;
        s.play(GRAPES, json!({}));
        assert_eq!(s.state().rng_cursor - cursor, 4);

        let mut rng = Rng::new("lucky", cursor);
        let ids = grape_ids();
        let expected: Vec<String> = [0, 1]
            .iter()
            .map(|_| {
                let first = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                let second = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                ids[first.max(second) as usize].to_string()
            })
            .collect();
        let grapes = added(&s);
        assert_eq!(grapes.iter().map(|event| event.def_id.clone()).collect::<Vec<String>>(), expected);
        assert!(grapes.iter().all(|event| !s.card(&event.instance_id).radiant));
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_adds_exactly_2_radiant_grapes() {
            crate::register_all();
            let mut s = played(true, None, None);
            s.play(GRAPES, json!({}));
            let grapes = added(&s);
            assert_eq!(grapes.len(), 2);
            assert!(grapes.iter().all(|event| s.card(&event.instance_id).radiant));
        }

        #[test]
        fn s6_1_lucky_1_each_grape_is_two_rolls_with_the_better_kept_four_draws_of_the_rng() {
            crate::register_all();
            let mut s = played(true, Some("lucky"), None);
            let cursor = s.state().rng_cursor;
            s.play(GRAPES, json!({}));
            assert_eq!(s.state().rng_cursor - cursor, 4);

            let mut rng = Rng::new("lucky", cursor);
            let ids = grape_ids();
            let expected: Vec<String> = [0, 1]
                .iter()
                .map(|_| {
                    let first = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                    let second = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                    ids[first.max(second) as usize].to_string()
                })
                .collect();
            let got: Vec<String> = added(&s).into_iter().map(|event| event.def_id).collect();
            assert_eq!(got, expected);
        }

        #[test]
        fn s6_1_over_many_plays_lucky_shifts_the_odds_toward_the_better_grapes() {
            crate::register_all();
            let mut rotten = 0;
            let mut better = 0;
            let plays = 200;
            let ids = grape_ids();
            for i in 0..plays {
                let seed = format!("lucky-{i}");
                let mut s = played(true, Some(&seed), None);
                s.play(GRAPES, json!({}));
                for event in added(&s) {
                    if event.def_id == ids[0] {
                        rotten += 1;
                    }
                    if index_of(&event.def_id) >= 2 {
                        better += 1;
                    }
                }
            }
            // Rotten falls from 12% to 1.44%; Large or better rises from 28% to about 48%.
            assert!(f64::from(rotten) / f64::from(plays * 2) < 0.05);
            assert!(f64::from(better) / f64::from(plays * 2) > 0.38);
        }

        #[test]
        fn r386_b3_4_its_lucky_is_a_numbered_keyword_one_x_step_up_makes_it_lucky_2_three_draws_a_grape() {
            crate::register_all();
            let mut s = played(true, Some("lucky-two"), None);
            let lucky_now = |s: &Scenario| -> Option<i64> {
                numbers_on(s.state(), s.card(GRAPES))
                    .iter()
                    .map(js)
                    .find(|number| number["ref"]["kind"] == json!("keyword") && number["ref"]["key"] == json!("Lucky"))
                    .and_then(|number| number["value"].as_i64())
            };
            assert_eq!(lucky_now(&s), Some(1));
            // An Upgrade's X change (B3.4 rule 3) is one step of `tuning.x` under the keyword's name.
            set_tuning(&mut s, GRAPES, json!({ "x": { "Lucky": 1 } }));
            assert_eq!(lucky_now(&s), Some(2));

            let cursor = s.state().rng_cursor;
            s.play(GRAPES, json!({}));
            assert_eq!(added(&s).len(), 2);
            assert_eq!(s.state().rng_cursor - cursor, 6);
        }

        #[test]
        fn r386_b3_4_a_degrade_s_x_change_never_takes_its_lucky_below_1() {
            crate::register_all();
            let mut s = played(true, Some("lucky-floor"), None);
            set_tuning(&mut s, GRAPES, json!({ "x": { "Lucky": -3 } }));
            let cursor = s.state().rng_cursor;
            s.play(GRAPES, json!({}));
            assert_eq!(s.state().rng_cursor - cursor, 4);
        }

        #[test]
        fn r386_the_radiant_count_steps_the_same_way() {
            crate::register_all();
            let mut s = played(true, None, None);
            step_param(s.card_mut(GRAPES), "grapes", 1);
            s.play(GRAPES, json!({}));
            assert_eq!(added(&s).len(), 3);
        }
    }

    #[test]
    fn the_catalog_hides_the_odds_from_both_faces_text() {
        crate::register_all();
        let def = crate::CATALOG.get(GRAPES);
        let faces = [
            def.map(|def| def.base.text.clone()).unwrap_or_default(),
            def.map(|def| def.radiant.text.clone()).unwrap_or_default(),
        ];
        for face in faces {
            assert!(!face.contains('%'));
            assert!(!face.contains("Rotten"));
            assert!(!face.contains("Mythic"));
        }
    }
}
