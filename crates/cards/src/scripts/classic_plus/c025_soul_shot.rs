//! C+ #25 Soul Shot (SPEC §8.7 row 25): destroy a random enemy Unit a Spell may affect; Lucky X picks X
//! more and keeps the best by R414's comparator.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-025";

/// R414: higher attack plus current health, then higher cost, then lower lane.
fn better(state: &GameState) -> impl Fn(CardInstance, CardInstance) -> CardInstance + '_ {
    let worth = move |card: &CardInstance| -> i32 {
        let view = unit_view(state, card);
        view.attack + view.health
    };
    // TS `?? Number.MAX_SAFE_INTEGER`: a card on no lane sorts after every lane.
    let lane = move |card: &CardInstance| -> i32 { slot_of(state, card).map_or(i32::MAX, |at| at.lane) };
    move |a, b| {
        if worth(&a) != worth(&b) {
            return if worth(&a) > worth(&b) { a } else { b };
        }
        if cost_now(state, &a) != cost_now(state, &b) {
            return if cost_now(state, &a) > cost_now(state, &b) {
                a
            } else {
                b
            };
        }
        if lane(&b) < lane(&a) { b } else { a }
    }
}

/// One random enemy Unit a Spell may affect, Lucky X picks keeping the best; none on an empty board.
fn pick(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let pool = cards_in_scope(ctx, &json_as(json!({ "side": "enemy" })));
    if pool.is_empty() {
        return Vec::new();
    }
    // R987: the controller's Luck rolls extra times beside the card's own Lucky.
    let lucky = match ctx.live_self() {
        None => 0,
        Some(me) => numbered_sum(&card_keywords(&*ctx.state, me), KeywordKind::Lucky).unwrap_or(0),
    } + luck_of(&*ctx.state, ctx.controller);
    // The state and the rng are two fields of the sink: the comparator reads the one while the rolls draw
    // from the other.
    let sink = &mut ctx.sink;
    let state: &GameState = &*sink.state;
    let rng: &mut Rng = &mut *sink.rng;
    let roll = |rng: &mut Rng| -> Option<CardInstance> { rng.pick(&pool).cloned() };
    let chosen = if lucky > 0 {
        rng.lucky(lucky, roll, |a, b| match (a, b) {
            (None, b) => b,
            (a, None) => a,
            (Some(a), Some(b)) => Some(better(state)(a, b)),
        })
    } else {
        roll(rng)
    };
    chosen.map(|card| vec![card.id]).unwrap_or_default()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(pick),
                each: Arc::new(|instance_id: &str| {
                    destroy(json_as(
                        json!({ "target": { "of": "instance", "instanceId": instance_id } }),
                    ))
                }),
            })]
        })),
        ..Script::default()
    };

    // The same script: Lucky 1 is the Radiant face's catalog keyword, which `pick` reads.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #25 Soul Shot — SPEC §8.7 row 25, BUILD M9 Classic+ row C+ 25: "Destroys one random enemy Unit
// (R60), an Indestructible one knocked to Attack Position instead (R46); no enemy Unit, nothing and no
// random draw (R129); an Immune to Spells Unit is never destroyed by it; radiant Lucky 1: two picks, the
// better destroyed, better being the higher attack plus current health, then the higher cost, then the
// lower lane (R414)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHOT: &str = "classicplus-025";
    const SMALL: &str = "core-012"; // 3/4, (2)
    const BIG: &str = "core-019"; // 9/9, (3)
    const ROCK: &str = "core-066"; // 10/10 Indestructible
    const TOP_LOSER: &str = "classicplus-019-1"; // Radiant: Immune to Spells
    const MOTHS: &str = "core-009"; // Moths to the Flame, (2) 1/14
    const VANILLA: &str = "core-008"; // Mr. Vanilla, (1) 4/4
    const SILAS: &str = "core-052"; // Silly Silas, (3) 4/4
    const FILLER: &str = "core-005";

    /// TS `{ hand: [FILLER], ...p2 }`.
    fn spread(defaults: Value, overrides: &Value) -> Value {
        let mut out = defaults;
        if let (Some(into), Some(from)) = (out.as_object_mut(), overrides.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn shot(p2: Value, radiant_face: bool, seed: &str) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": SHOT, "radiant": radiant_face }, FILLER], "field": [{ "def": BIG, "lane": 3 }] },
            "p2": spread(json!({ "hand": [FILLER] }), &p2),
        }))
    }

    fn destroyed_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod c_n25_soul_shot {
        use super::*;

        #[test]
        fn is_a_2_spell_with_no_play_time_choice_the_radiant_face_prints_lucky_1() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(2));
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(Arc::ptr_eq(
                scripts.base.cry.as_ref().expect("base cry"),
                scripts.radiant.cry.as_ref().expect("radiant cry"),
            ));
            assert_eq!(def.radiant.keywords, vec![Keyword::Lucky { n: 1 }]);
        }

        mod base {
            use super::*;

            #[test]
            fn r60_destroys_exactly_one_enemy_unit_never_one_of_yours_and_the_pick_varies_by_seed() {
                let mut picked: IndexSet<String> = IndexSet::new();
                for seed in 1..=16 {
                    let mut s = shot(
                        json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }, { "def": SMALL, "lane": 4 }] }),
                        false,
                        &format!("shot-{seed}"),
                    );
                    let enemies: Vec<Option<String>> = [1, 2, 4]
                        .iter()
                        .map(|lane| s.unit(P2, *lane).map(|unit| unit.id))
                        .collect();
                    s.play(SHOT, json!({}));
                    let dead = destroyed_ids(&s);
                    assert_eq!(dead.len(), 1);
                    let first = dead.first().cloned();
                    assert!(enemies.contains(&first));
                    assert_eq!(s.unit(P1, 3).map(|unit| unit.def_id), Some(BIG.to_string()));
                    // TS `String(enemies.indexOf(dead[0]))`.
                    let at = enemies
                        .iter()
                        .position(|enemy| *enemy == first)
                        .map_or(-1, |at| i64::try_from(at).unwrap_or(i64::MAX));
                    picked.insert(at.to_string());
                }
                assert!(picked.len() > 1);
            }

            #[test]
            fn r129_with_no_enemy_unit_nothing_happens_and_no_random_draw_is_made() {
                let mut s = shot(json!({}), false, "soul-shot");
                let cursor = s.state().rng_cursor;
                s.play(SHOT, json!({}));
                assert_eq!(destroyed_ids(&s), Vec::<String>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn r987_feng_shui_s_luck_adds_a_roll() {
                crate::register_all();
                let _open = preview_sets(&[SetName::Meditative]);
                let draws = |judge: bool| -> u32 {
                    let mut p1 = json!({
                        "hand": [{ "def": SHOT }, FILLER],
                        "field": [{ "def": BIG, "lane": 3 }],
                    });
                    if judge {
                        p1["backrow"] = json!(["meditative-040"]);
                    }
                    let mut s = scenario(json!({
                        "seed": "soul-shot-luck",
                        "p1": p1,
                        "p2": {
                            "hand": [FILLER],
                            "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }],
                        },
                    }));
                    let cursor = s.state().rng_cursor;
                    s.play(SHOT, json!({}));
                    s.state().rng_cursor - cursor
                };
                assert_eq!(draws(true), draws(false) + 1);
            }

            #[test]
            fn r46_an_indestructible_pick_survives_knocked_to_attack_position() {
                let mut s = shot(
                    json!({ "field": [{ "def": ROCK, "lane": 1, "position": "DEF" }] }),
                    false,
                    "soul-shot",
                );
                s.play(SHOT, json!({}));
                let rock = s.unit(P2, 1);
                assert_eq!(rock.as_ref().map(|unit| unit.def_id.as_str()), Some(ROCK));
                let rock = rock.map(|unit| unit.id).unwrap_or_default();
                assert_eq!(s.stats(&rock).position, Position::Atk);
            }

            #[test]
            fn s6_1_an_immune_to_spells_unit_is_never_picked() {
                for seed in 1..=12 {
                    let mut s = shot(
                        json!({ "field": [{ "def": TOP_LOSER, "lane": 1, "radiant": true }, { "def": SMALL, "lane": 2 }] }),
                        false,
                        &format!("shot-immune-{seed}"),
                    );
                    s.play(SHOT, json!({}));
                    assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(TOP_LOSER.to_string()));
                    assert!(s.unit(P2, 2).is_none());
                }
            }

            #[test]
            fn s6_1_with_only_an_immune_to_spells_unit_nothing_is_destroyed_and_nothing_drawn() {
                let mut s = shot(
                    json!({ "field": [{ "def": TOP_LOSER, "lane": 1, "radiant": true }] }),
                    false,
                    "soul-shot",
                );
                let cursor = s.state().rng_cursor;
                s.play(SHOT, json!({}));
                assert_eq!(destroyed_ids(&s), Vec::<String>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            }
        }

        /// R1438: given Lucky 1, the base face picks twice and keeps the better: two draws where the plain
        /// base face makes one, and of a 9/9 and a 3/4 the 9/9 dies more often than one pick kills it.
        #[test]
        fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
            let board = json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }] });
            let cast = |seed: i32, lucky: bool| -> (bool, u32) {
                let mut s = shot(board.clone(), false, &format!("lucky-base-{seed}"));
                if lucky {
                    crate::give_lucky(&mut s, SHOT, 1);
                }
                let before = s.state().rng_cursor;
                s.play(SHOT, json!({}));
                assert_eq!(destroyed_ids(&s).len(), 1);
                (s.unit(P2, 2).is_none(), s.state().rng_cursor - before)
            };
            let (mut plain_big, mut lucky_big) = (0, 0);
            for seed in 1..=24 {
                let ((plain, plain_draws), (lucky, lucky_draws)) = (cast(seed, false), cast(seed, true));
                assert_eq!(lucky_draws, 2 * plain_draws);
                plain_big += i32::from(plain);
                lucky_big += i32::from(lucky);
            }
            assert!(lucky_big > plain_big);
        }

        mod radiant {
            use super::*;

            #[test]
            fn r414_lucky_1_of_two_picks_the_better_dies_with_a_9_9_and_a_3_4_the_9_9_dies_far_more_often_than_one_pick_would()
             {
                let mut big: i32 = 0;
                let tries: i32 = 24;
                for seed in 1..=tries {
                    let mut s = shot(
                        json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
                        true,
                        &format!("lucky-{seed}"),
                    );
                    s.play(SHOT, json!({}));
                    if s.unit(P2, 2).is_none() {
                        big += 1;
                    }
                    assert_eq!(destroyed_ids(&s).len(), 1);
                }
                // Two picks keep the 9/9 unless both land on the 3/4: three in four on average.
                assert!(f64::from(big) > f64::from(tries) / 2.0);
            }

            #[test]
            fn r414_each_lucky_pick_is_a_draw_two_draws_where_the_base_face_makes_one() {
                let mut once = shot(
                    json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
                    false,
                    "soul-shot",
                );
                let c1 = once.state().rng_cursor;
                once.play(SHOT, json!({}));
                let mut twice = shot(
                    json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
                    true,
                    "soul-shot",
                );
                let c2 = twice.state().rng_cursor;
                twice.play(SHOT, json!({}));
                assert_eq!(twice.state().rng_cursor - c2, 2 * (once.state().rng_cursor - c1));
            }

            /// R414 by the rng itself: the two picks are the next two draws of the match rng over the enemy Units
            /// in lane order, so a test can name them. Of two different picks `winner` must die; of one Unit
            /// picked twice, that one. Both cases must come up across the seeds.
            fn expect_lucky_keeps(p2: Value, lanes: &[i32], winner_lane: i32) {
                let (mut split, mut same) = (0, 0);
                for seed in 1..=24 {
                    let mut s = shot(p2.clone(), true, &format!("r414-{seed}"));
                    let pool: Vec<String> = lanes
                        .iter()
                        .map(|lane| s.unit(P2, *lane).map(|unit| unit.id).unwrap_or_default())
                        .collect();
                    let winner = s.unit(P2, winner_lane).map(|unit| unit.id).unwrap_or_default();
                    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let first = rng.pick(&pool).cloned();
                    let second = rng.pick(&pool).cloned();
                    s.play(SHOT, json!({}));
                    if first == second {
                        same += 1;
                    } else {
                        split += 1;
                    }
                    let expected = if first == second {
                        first.unwrap_or_default()
                    } else {
                        winner
                    };
                    assert_eq!(destroyed_ids(&s), vec![expected]);
                }
                assert!(split > 0);
                assert!(same > 0);
            }

            #[test]
            fn r414_the_better_is_the_higher_attack_plus_current_health_a_1_14_damaged_to_1_4_loses_to_a_4_4()
            {
                expect_lucky_keeps(
                    json!({ "field": [{ "def": MOTHS, "lane": 1, "damage": 10 }, { "def": VANILLA, "lane": 2 }] }),
                    &[1, 2],
                    2,
                );
            }

            #[test]
            fn r414_a_tie_on_attack_plus_health_goes_to_the_higher_cost_a_3_4_4_over_a_1_4_4() {
                expect_lucky_keeps(
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }, { "def": SILAS, "lane": 3 }] }),
                    &[1, 3],
                    3,
                );
            }

            #[test]
            fn r414_then_to_the_lower_lane_of_two_1_4_4s_the_one_in_lane_2_over_lane_4() {
                expect_lucky_keeps(
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }] }),
                    &[2, 4],
                    2,
                );
            }

            #[test]
            fn s9_3_the_two_lucky_picks_replay_from_a_json_copy_to_the_same_hash() {
                let s = shot(
                    json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }, { "def": VANILLA, "lane": 4 }] }),
                    true,
                    "shot-replay",
                );
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(SHOT).id,
                    "playerId": "p1",
                    "nonce": "shot-replay",
                }));
                let thawed: GameState =
                    serde_json::from_value(serde_json::to_value(s.state()).expect("JSON")).expect("a state");
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert_eq!(
                    hash_state(&reduce(&thawed, &action).state),
                    hash_state(&live.state)
                );
            }

            #[test]
            fn r386_lucky_is_tuned_like_any_numbered_keyword_lucky_2_makes_three_picks() {
                let mut s = shot(
                    json!({ "field": [{ "def": SMALL, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
                    true,
                    "soul-shot",
                );
                let tuning = tuning_of(s.card_mut(SHOT));
                tuning.x = Some(add_step(tuning.x.as_ref(), "Lucky", 1));
                let cursor = s.state().rng_cursor;
                s.play(SHOT, json!({}));
                assert_eq!(s.state().rng_cursor - cursor, 3);
            }
        }
    }
}
