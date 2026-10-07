//! C+ #20 Mushroom Power (SPEC §8.7 row 20): Cry gives the Units beside it (§3.1) +{buff}/+{buff}.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-020";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|each: &mut EffectContext<'_>| {
                    adjacent_to(each, &json_as(json!({ "of": "self" })), &BoardScope::default())
                        .into_iter()
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "attack": amount,
                        "health": amount,
                    })))
                }),
            })]
        })),
        ..Script::default()
    };

    // The same script: the Radiant +4/+4 is its declared `buff`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #20 Mushroom Power — SPEC §8.7 row 20, BUILD M9 Classic+ row C+ 20: "Cry gives the Units in the
// unit zones beside it on its side (lanes N−1 and N+1) +2/+2 permanently: one neighbour in lane 1 or 5,
// nothing across the lane or in the backrow, nothing when the neighbours are empty; the buffs stay
// after it leaves; the buff reads through `param()`; radiant +4/+4".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MUSHROOM: &str = "classicplus-020";
    const BODY: &str = "core-019"; // Midrange Menace 9/9 Taunt
    const SMALL: &str = "core-012"; // Duplicating Felinors 3/4
    const FIELD_SPELL: &str = "core-075"; // Infinite Reserves, which touches no Unit
    const FILLER: &str = "core-005";

    /// The unit on top of `player`'s pile in `lane`, which the setup must have put there.
    fn unit(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane).expect("setup")
    }

    fn buffed_count(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Buffed)
            .count()
    }

    mod c_n20_mushroom_power {
        use super::*;

        #[test]
        fn is_a_1_2_2_unit_whose_cry_declares_no_choice_both_faces_run_one_script() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, MUSHROOM);
            assert_eq!(def.cost, CardCost::Fixed(1));
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(Arc::ptr_eq(
                scripts.base.cry.as_ref().expect("base cry"),
                scripts.radiant.cry.as_ref().expect("radiant cry"),
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn s3_1_gives_both_neighbours_2_2_and_leaves_lanes_further_off_the_other_side_and_the_backrow_alone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [MUSHROOM, FILLER],
                        "field": [{ "def": BODY, "lane": 1 }, { "def": SMALL, "lane": 2 }, { "def": BODY, "lane": 4 }],
                        "backrow": [{ "def": FIELD_SPELL, "lane": 4 }],
                    },
                    "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 3 }] },
                }));
                let far = unit(&s, P1, 1);
                let left = unit(&s, P1, 2);
                let right = unit(&s, P1, 4);
                let across = unit(&s, P2, 3);

                s.play(MUSHROOM, json!({ "zone": 3 }));

                s.expect_stats(&left, json!({ "attack": 5, "health": 6, "maxHealth": 6 }));
                s.expect_stats(&right, json!({ "attack": 11, "health": 11 }));
                s.expect_stats(&far, json!({ "attack": 9, "health": 9 }));
                s.expect_stats(&across, json!({ "attack": 9, "health": 9 }));
                s.expect_stats(MUSHROOM, json!({ "attack": 2, "health": 2 }));
            }

            #[test]
            fn s3_1_in_lane_1_it_has_one_neighbour_lane_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MUSHROOM, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                    "p2": { "hand": [FILLER] },
                }));
                let neighbour = unit(&s, P1, 2);
                s.play(MUSHROOM, json!({ "zone": 1 }));
                s.expect_stats(&neighbour, json!({ "attack": 11, "health": 11 }));
            }

            #[test]
            fn s3_1_in_lane_5_it_has_one_neighbour_lane_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MUSHROOM, FILLER], "field": [{ "def": BODY, "lane": 4 }] },
                    "p2": { "hand": [FILLER] },
                }));
                let neighbour = unit(&s, P1, 4);
                s.play(MUSHROOM, json!({ "zone": 5 }));
                s.expect_stats(&neighbour, json!({ "attack": 11, "health": 11 }));
            }

            #[test]
            fn with_both_neighbouring_zones_empty_nothing_is_buffed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MUSHROOM, FILLER], "field": [{ "def": BODY, "lane": 1 }] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(MUSHROOM, json!({ "zone": 3 }));
                assert_eq!(buffed_count(&s), 0);
            }

            #[test]
            fn s3_2_only_the_top_of_a_stack_pile_is_a_neighbour_the_dormant_card_beneath_is_not_buffed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [MUSHROOM, FILLER],
                        "field": [{ "def": SMALL, "lane": 2 }, { "def": "core-092", "lane": 2, "stack": true }],
                    },
                    "p2": { "hand": [FILLER] },
                }));
                let dormant = s.state().players[P1]
                    .units
                    .get(1)
                    .and_then(|pile| pile.as_ref())
                    .and_then(|pile| pile.get(1))
                    .expect("no pile")
                    .id
                    .clone();
                s.play(MUSHROOM, json!({ "zone": 3 }));
                assert_eq!(s.card(&dormant).buffs, AttackHealth { attack: 0, health: 0 });
                assert_eq!(buffed_count(&s), 1);
            }

            #[test]
            fn s10_4_the_buffs_are_permanent_they_stay_after_mushroom_power_leaves_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MUSHROOM, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                    "p2": { "hand": ["core-016", FILLER], "field": [] },
                    "active": "p1",
                }));
                let neighbour = unit(&s, P1, 2);
                s.play(MUSHROOM, json!({ "zone": 3 }));
                let mushroom = s.unit(P1, 3).expect("no mushroom");
                s.end_turn();
                s.play("core-016", json!({ "targets": [{ "pick": "instance", "instanceId": mushroom.id }] }));
                s.expect_in_zone(&mushroom, "graveyard");
                s.expect_stats(&neighbour, json!({ "attack": 11, "health": 11 }));
            }

            #[test]
            fn r386_the_buff_reads_through_param_an_upgrade_makes_it_3_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MUSHROOM, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(MUSHROOM), "buff", 1);
                let neighbour = unit(&s, P1, 2);
                s.play(MUSHROOM, json!({ "zone": 3 }));
                s.expect_stats(&neighbour, json!({ "attack": 12, "health": 12 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_4_4_and_gives_each_neighbour_4_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": MUSHROOM, "radiant": true }, FILLER],
                        "field": [{ "def": SMALL, "lane": 2 }, { "def": BODY, "lane": 4 }],
                    },
                    "p2": { "hand": [FILLER] },
                }));
                let left = unit(&s, P1, 2);
                let right = unit(&s, P1, 4);
                s.play(MUSHROOM, json!({ "zone": 3 }));
                s.expect_stats(MUSHROOM, json!({ "attack": 4, "health": 4 }));
                s.expect_stats(&left, json!({ "attack": 7, "health": 8 }));
                s.expect_stats(&right, json!({ "attack": 13, "health": 13 }));
            }
        }
    }
}
