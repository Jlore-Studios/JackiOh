//! C+ #19 League of Losers (SPEC §8.7 row 19): summons the five Losers into unit zones 1–5, each aimed at
//! its own zone (Radiant: all Radiant); the Mid Loser it summons has its Cry triggered (R411).
//! An occupied or Locked zone is skipped: occupancy fizzles in the engine, and the Lock is this
//! card's own override of R688, so the script skips a Locked lane itself.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019";

/// The five-stack, in the order they are summoned; each goes to the unit zone of its lane (index + 1).
const FIVE_STACK: [&str; 5] = [
    "classicplus-019-1", // Top Loser
    "classicplus-019-2", // Jungle Loser
    "classicplus-019-3", // Mid Loser
    "classicplus-019-4", // Support Loser
    "classicplus-019-5", // Bot Loser
];

const MID_LOSER: &str = "classicplus-019-3";

/// R411: the Mid Loser this list summoned into `lane`, if it is there, has its Cry triggered.
fn mid_loser_cry(lane: i32) -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(move |ctx: &mut EffectContext<'_>| {
            let unit = card_at(&*ctx.state, &ZoneRef { player: ctx.controller, row: Row::Units, lane }).cloned();
            match unit {
                Some(unit) if unit.def_id == MID_LOSER && summoned_so_far(ctx).contains(&unit.id) => vec![unit.id],
                _ => Vec::new(),
            }
        }),
        each: Arc::new(|instance_id: &str| trigger_cry(json_as(json!({ "instanceId": instance_id })))),
    })
}

fn league(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            FIVE_STACK
                .iter()
                .zip(1..)
                .flat_map(|(&def_id, lane): (&&str, i32)| {
                    // R688's card-specific override: a Locked lane is skipped (§8.7 row 19).
                    if is_locked(&*ctx.state, &ZoneRef { player: ctx.controller, row: Row::Units, lane }) {
                        return Vec::new();
                    }
                    let summoned = summon(json_as(json!({ "defId": def_id, "lane": lane, "radiant": radiant })));
                    if def_id == MID_LOSER {
                        vec![summoned, mid_loser_cry(lane)]
                    } else {
                        vec![summoned]
                    }
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: league(false),
        radiant: league(true),
    }
}

// C+ #19 League of Losers — SPEC §8.7 row 19, BUILD M9 Classic+ row C+ 19: "Summons Top, Jungle, Mid,
// Support and Bot Loser (C+ #19.1–#19.5) into your unit zones 1 to 5 in that order, each aimed at its
// own zone: an occupied, Locked or reserved zone is skipped and that Loser is not summoned elsewhere;
// Mid Loser's Cry fires when this summons it (R411); a full board summons nothing; a Loser summoned by
// any other effect (a copy, a Rollback's recreation, Frostspatula's copies) fires no Cry (R1); radiant
// all five Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LEAGUE: &str = "classicplus-019";
    const FIVE: [&str; 5] = [
        "classicplus-019-1",
        "classicplus-019-2",
        "classicplus-019-3",
        "classicplus-019-4",
        "classicplus-019-5",
    ];
    const MID: &str = "classicplus-019-3";
    const BODY: &str = "core-008"; // 4/4
    const CUBE: &str = "core-022";
    const HIT_JOB: &str = "core-016";
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

    /// TS `{ ...defaults, ...overrides }` on a side setup: every key of `overrides` replaces the default's.
    fn spread(defaults: Value, overrides: &Value) -> Value {
        let mut out = defaults;
        if let (Some(into), Some(from)) = (out.as_object_mut(), overrides.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn league(p1: Value, radiant_face: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut side1 = spread(json!({ "library": DECK }), &p1);
        let mut hand = vec![json!({ "def": LEAGUE, "radiant": radiant_face }), json!(FILLER)];
        hand.extend(p1.get("hand").and_then(Value::as_array).cloned().unwrap_or_default());
        side1["hand"] = Value::Array(hand);
        let mut opts = json!({ "p1": side1, "p2": { "hand": [FILLER], "library": DECK } });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|unit| unit.def_id)).collect()
    }

    /// The row TS spelled out as ids (`null` for an empty zone).
    fn ids(row: &[Option<&str>]) -> Vec<Option<String>> {
        row.iter().map(|id| id.map(str::to_string)).collect()
    }

    /// The Mid Loser's Cry is the first draw the play makes: heads wanted or tails.
    fn seeded_for(heads: bool, p1: Value, radiant_face: bool) -> Scenario {
        for n in 1..=200 {
            let s = league(p1.clone(), radiant_face, Some(&format!("league-{n}")));
            if Rng::new(&s.state().seed, s.state().rng_cursor).coin() == heads {
                return s;
            }
        }
        panic!("no seed");
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    /// JS `indexOf`: the first position of `item`, or −1.
    fn index_of(items: &[String], item: &str) -> i64 {
        items
            .iter()
            .position(|each| each == item)
            .map_or(-1, |at| i64::try_from(at).unwrap_or(i64::MAX))
    }

    /// TS `s.hand("p1").find((card) => card.defId === HIT_JOB) ?? HIT_JOB`.
    fn a_hit_job(s: &Scenario) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == HIT_JOB)
            .map_or_else(|| HIT_JOB.to_string(), |card| card.id)
    }

    mod c_n19_league_of_losers {
        use super::*;

        #[test]
        fn is_a_4_legendary_spell_with_no_play_time_choice() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(4));
            assert_eq!(def.rarity, Rarity::Legendary);
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
            assert_eq!(def.refs, Some(FIVE.iter().map(|id| id.to_string()).collect::<Vec<String>>()));
        }

        mod base {
            use super::*;

            #[test]
            fn summons_the_five_stack_into_unit_zones_1_to_5_in_order_each_on_its_base_face_summoning_sick() {
                let mut s = league(json!({}), false, None);
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&FIVE.map(Some)));
                for lane in 1..=5 {
                    let unit = s.unit(P1, lane);
                    assert_eq!(unit.as_ref().map(|unit| unit.radiant), Some(false));
                    assert_eq!(unit.as_ref().and_then(|unit| unit.summoned_turn), Some(s.state().turn));
                }
                let order: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(order, FIVE.map(str::to_string).to_vec());
            }

            #[test]
            fn r411_mid_loser_s_cry_fires_when_this_summons_it_heads() {
                let mut s = seeded_for(true, json!({}), false);
                s.play(LEAGUE, json!({}));
                let third = unit_or_blank(&s, P1, 3);
                s.expect_stats(&third, json!({ "attack": 10, "health": 10 }));
                // Only the Mid Loser was buffed: the other four print no Cry.
                for lane in [1, 2, 4, 5] {
                    assert_eq!(s.card(unit_or_blank(&s, P1, lane)).buffs, AttackHealth { attack: 0, health: 0 });
                }
            }

            #[test]
            fn r411_mid_loser_s_cry_fires_when_this_summons_it_tails_and_the_opponent_s_next_refresh_is_1_higher() {
                let mut s = seeded_for(false, json!({}), false);
                s.play(LEAGUE, json!({}));
                let third = unit_or_blank(&s, P1, 3);
                s.expect_stats(&third, json!({ "attack": 2, "health": 2 }));
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 1);
            }

            #[test]
            fn r411_the_cry_runs_right_after_the_mid_loser_s_own_summon_before_support_loser_s() {
                let mut s = league(json!({}), false, None);
                s.play(LEAGUE, json!({}));
                let types: Vec<String> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Summoned { def_id, .. } => Some(format!("summoned:{def_id}")),
                        GameEvent::Buffed { .. } => Some("buffed".to_string()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(index_of(&types, "buffed"), index_of(&types, &format!("summoned:{MID}")) + 1);
            }

            #[test]
            fn s3_2_an_occupied_zone_is_skipped_and_its_loser_is_not_summoned_elsewhere() {
                let mut s = league(json!({ "field": [{ "def": BODY, "lane": 2 }] }), false, None);
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&[Some(FIVE[0]), Some(BODY), Some(FIVE[2]), Some(FIVE[3]), Some(FIVE[4])]));
            }

            #[test]
            fn r688_a_locked_zone_is_skipped_the_card_s_own_override_of_the_locked_takes_summons_rule() {
                let mut s = league(json!({}), false, None);
                lock_zone(s.state_mut(), &ZoneRef { player: P1, row: Row::Units, lane: 4 });
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&[Some(FIVE[0]), Some(FIVE[1]), Some(FIVE[2]), None, Some(FIVE[4])]));
            }

            #[test]
            fn r64_a_reserved_zone_is_skipped() {
                let mut s = league(json!({}), false, None);
                reserve_zone(s.state_mut(), &ZoneRef { player: P1, row: Row::Units, lane: 5 });
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&[Some(FIVE[0]), Some(FIVE[1]), Some(FIVE[2]), Some(FIVE[3]), None]));
            }

            #[test]
            fn r411_with_mid_loser_s_zone_taken_there_is_no_mid_loser_and_no_cry() {
                let mut s = league(json!({ "field": [{ "def": MID, "lane": 3 }] }), false, None);
                let standing = s.unit(P1, 3);
                let cursor = s.state().rng_cursor;
                s.play(LEAGUE, json!({}));
                assert_eq!(
                    s.unit(P1, 3).map(|unit| unit.id),
                    standing.as_ref().map(|unit| unit.id.clone())
                );
                let standing = standing.map(|unit| unit.id).unwrap_or_default();
                s.expect_stats(&standing, json!({ "attack": 5, "health": 5 }));
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn a_full_board_summons_nothing() {
                let mut s = league(json!({ "field": [BODY, BODY, BODY, BODY, BODY] }), false, None);
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&[Some(BODY), Some(BODY), Some(BODY), Some(BODY), Some(BODY)]));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Summoned));
            }

            #[test]
            fn r1_a_loser_summoned_by_any_other_effect_fires_no_cry_carnivorous_cube_s_copies_of_a_mid_loser() {
                let mut s = league(json!({ "hand": [CUBE, HIT_JOB, HIT_JOB], "mana": 20 }), false, None);
                s.play(LEAGUE, json!({}));
                let mid = s.unit(P1, 3).expect("setup");
                let top = s.unit(P1, 1).expect("setup");
                // Free lane 1 for the Cube, which eats the Mid Loser; then destroy the Cube, whose Death summons
                // two copies of it.
                let hit = a_hit_job(&s);
                s.play(hit, json!({ "targets": [{ "pick": "instance", "instanceId": top.id }] }));
                s.play(CUBE, json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": mid.id }] }));
                let cube = s.unit(P1, 1).expect("no Cube");
                let cursor = s.state().rng_cursor;
                let refresh = s.state().players[P2].mana.next_turn_mod;
                let hit = a_hit_job(&s);
                s.play(hit, json!({ "targets": [{ "pick": "instance", "instanceId": cube.id }] }));
                let copies: Vec<CardInstance> =
                    (1..=5).filter_map(|lane| s.unit(P1, lane).filter(|unit| unit.def_id == MID)).collect();
                assert_eq!(copies.len(), 2);
                // No coin was flipped, nothing was buffed, and the opponent's refresh is as it was.
                assert_eq!(s.state().rng_cursor, cursor);
                for copy in &copies {
                    assert_eq!(s.card(copy).buffs, AttackHealth { attack: 0, health: 0 });
                }
                assert_eq!(s.state().players[P2].mana.next_turn_mod, refresh);
            }

            #[test]
            fn r11_the_losers_are_tokens_one_that_leaves_the_field_ceases_to_exist() {
                let mut s = league(json!({ "hand": [HIT_JOB], "mana": 20 }), false, None);
                s.play(LEAGUE, json!({}));
                let top = unit_or_blank(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": top }] }));
                s.expect_in_zone(&top, "gone");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn summons_all_five_radiant() {
                let mut s = league(json!({}), true, None);
                s.play(LEAGUE, json!({}));
                assert_eq!(row(&s), ids(&FIVE.map(Some)));
                for lane in 1..=5 {
                    assert_eq!(s.unit(P1, lane).map(|unit| unit.radiant), Some(true));
                }
                let first = unit_or_blank(&s, P1, 1);
                s.expect_stats(&first, json!({ "attack": 10, "health": 10 }));
            }

            #[test]
            fn r411_the_radiant_mid_loser_s_cry_fires_with_its_lucky_1_two_flips() {
                let mut s = league(json!({}), true, None);
                let cursor = s.state().rng_cursor;
                s.play(LEAGUE, json!({}));
                assert_eq!(s.state().rng_cursor - cursor, 2);
            }
        }
    }
}
