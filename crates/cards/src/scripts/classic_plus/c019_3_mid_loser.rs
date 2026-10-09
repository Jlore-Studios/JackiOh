//! C+ #19.3 Mid Loser (SPEC §8.7 row 19.3): Cry flips a coin (Lucky X: X more flips, heads kept): heads
//! +{heads}/+{heads}; tails −{tails}/−{tails} and the opponent's next refresh is 1 higher.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-3";

// "your opponent gains 1 mana next turn": the declared number `oppMana` (R386), less being better.

/// One coin, Lucky X times more, heads kept if any of them lands heads.
fn lands_heads(ctx: &mut EffectContext<'_>) -> bool {
    // R987: the controller's Luck flips extra times beside the card's own Lucky.
    let lucky = match ctx.live_self() {
        Some(me) if me.zone.z() == ZoneName::Field => {
            numbered_sum(&unit_view(&*ctx.state, me).keywords, KeywordKind::Lucky).unwrap_or(0)
        }
        _ => 0,
    } + luck_of(&*ctx.state, ctx.controller);
    let flip = |rng: &mut Rng| -> bool { rng.coin() };
    if lucky > 0 {
        ctx.rng.lucky(lucky, flip, |a, b| a || b)
    } else {
        flip(&mut *ctx.rng)
    }
}

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    if lands_heads(ctx) {
        let heads = param(&*ctx, "heads");
        return vec![buff(json_as(
            json!({ "target": { "of": "self" }, "attack": heads, "health": heads }),
        ))];
    }
    let tails = param(&*ctx, "tails");
    vec![
        buff(json_as(
            json!({ "target": { "of": "self" }, "attack": -tails, "health": -tails }),
        )),
        next_turn_mana(json_as(
            json!({ "amount": param(&*ctx, "oppMana"), "player": "enemy" }),
        )),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };

    // The same script: the Radiant face's Lucky 1 and its heads 10 are catalog data the script reads.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #19.3 Mid Loser — SPEC §8.7 row 19.3, BUILD M9 Classic+ row C+ 19.3: "Cry, on a play from hand, a
// cast or League of Losers' summon (R1, R411), flips a seeded coin: heads +5/+5 permanently; tails
// −3/−3 permanently (a damaged Mid Loser can die at the state check) and the opponent's next mana
// refresh is 1 higher (`nextTurnMod`); a copy or a Rollback's recreation flips nothing; heads and tails
// amounts read through `param()`; radiant Lucky 1 (two flips, heads kept if either lands) and heads
// +10/+10".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MID: &str = "classicplus-019-3";
    const LEAGUE: &str = "classicplus-019";
    const CUBE: &str = "core-022"; // Cry: Tribute one of your other Units and remember it. Death: summon 2 copies of it.
    const HIT_JOB: &str = "core-016";
    const TESLA: &str = "classic-005"; // Field Trap: when your opponent summons a Unit, deal 4 damage to it
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Flip {
        Heads,
        Tails,
    }

    /// The coin the next draw of the match rng lands, and with Lucky X the best of X + 1.
    fn next_flip(s: &Scenario, lucky: i32) -> Flip {
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let mut heads = rng.coin();
        for _ in 0..lucky {
            heads = rng.coin() || heads;
        }
        if heads { Flip::Heads } else { Flip::Tails }
    }

    /// A seed whose next flips land as wanted, for a scenario built by `make`.
    fn seeded(make: impl Fn(&str) -> Scenario, want: impl Fn(&Scenario) -> bool) -> Scenario {
        for n in 1..=200 {
            let s = make(&format!("mid-{n}"));
            if want(&s) {
                return s;
            }
        }
        panic!("no seed lands the flips wanted");
    }

    fn from_hand(radiant_face: bool) -> impl Fn(&str) -> Scenario {
        move |seed: &str| {
            crate::register_all();
            scenario(json!({
                "seed": seed,
                "p1": { "hand": [{ "def": MID, "radiant": radiant_face }, FILLER], "library": DECK },
                "p2": { "hand": [FILLER], "library": DECK },
            }))
        }
    }

    fn mid(s: &Scenario) -> CardInstance {
        for lane in 1..=5 {
            if let Some(unit) = s.unit(P1, lane)
                && unit.def_id == MID
            {
                return unit;
            }
        }
        panic!("no Mid Loser on the field");
    }

    use crate::unit_or_blank;

    mod c_n19_3_mid_loser {
        use super::*;

        #[test]
        fn is_a_2_5_5_unit_token_printed_legendary_whose_radiant_face_prints_lucky_1_one_script_for_both() {
            let def = crate::card_def(ID);
            assert!(def.token);
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
            assert_eq!(def.radiant.keywords, vec![Keyword::Lucky { n: 1 }]);
            let scripts = script();
            assert!(Arc::ptr_eq(
                scripts.base.cry.as_ref().expect("base cry"),
                scripts.radiant.cry.as_ref().expect("radiant cry"),
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn r1_played_from_a_hand_heads_5_5_permanently_and_the_opponent_gains_nothing() {
                let mut s = seeded(from_hand(false), |each| next_flip(each, 0) == Flip::Heads);
                s.play(MID, json!({}));
                let me = mid(&s);
                s.expect_stats(&me, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 0);
            }

            #[test]
            fn r987_feng_shui_s_luck_adds_a_roll() {
                crate::register_all();
                let _open = preview_sets(&[SetName::Meditative]);
                let flips = |judge: bool| -> u32 {
                    let mut p1 = json!({ "hand": [MID, FILLER], "library": DECK });
                    if judge {
                        p1["backrow"] = json!(["meditative-040"]);
                    }
                    let mut s = scenario(json!({
                        "seed": "mid-loser-luck",
                        "p1": p1,
                        "p2": { "hand": [FILLER], "library": DECK },
                    }));
                    let cursor = s.state().rng_cursor;
                    s.play(MID, json!({}));
                    s.state().rng_cursor - cursor
                };
                assert_eq!(flips(false), 1);
                assert_eq!(flips(true), 2);
            }

            #[test]
            fn r1_played_from_a_hand_tails_3_3_permanently_and_the_opponent_s_next_refresh_is_1_higher() {
                let mut s = seeded(from_hand(false), |each| next_flip(each, 0) == Flip::Tails);
                let before = s.state().players[P2].mana.max;
                s.play(MID, json!({}));
                let me = mid(&s);
                s.expect_stats(&me, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 1);
                s.end_turn();
                assert_eq!(s.state().players[P2].mana.current, before + 1);
            }

            #[test]
            fn s4_5_tails_on_a_damaged_mid_loser_can_kill_it_at_the_state_check_heads_saves_it() {
                // The opponent's Tesla answers the summon with 4 damage: a 5/5 left at 1 health.
                let tesla = |seed: &str| -> Scenario {
                    crate::register_all();
                    scenario(json!({
                        "seed": seed,
                        "p1": { "hand": [MID, FILLER], "library": DECK },
                        "p2": { "hand": [FILLER], "library": DECK, "backrow": [{ "def": TESLA, "lane": 1, "faceUp": false }] },
                    }))
                };
                let mut tails = seeded(tesla, |each| next_flip(each, 0) == Flip::Tails);
                let doomed = tails.card(MID).clone();
                tails.play(MID, json!({}));
                assert!(
                    tails
                        .events()
                        .iter()
                        .any(|event| event.event_type() == GameEventType::TrapFired)
                );
                tails.expect_in_zone(&doomed, "gone");
                assert_eq!(tails.state().players[P2].mana.next_turn_mod, 1);

                let mut heads = seeded(tesla, |each| next_flip(each, 0) == Flip::Heads);
                heads.play(MID, json!({}));
                let me = mid(&heads);
                heads.expect_stats(&me, json!({ "attack": 10, "health": 6, "maxHealth": 10 }));
            }

            #[test]
            fn r1_a_copy_flips_nothing_carnivorous_cube_s_copies_of_an_eaten_mid_loser_are_plain_5_5s() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CUBE, HIT_JOB, FILLER], "field": [{ "def": MID, "lane": 1 }], "mana": 6, "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let eaten = s.unit(P1, 1).expect("setup");
                s.play(
                    CUBE,
                    json!({ "zone": 2, "targets": [{ "pick": "instance", "instanceId": eaten.id }] }),
                );
                let cube = s.unit(P1, 2).expect("no cube");
                let cursor = s.state().rng_cursor;
                s.play(
                    HIT_JOB,
                    json!({ "targets": [{ "pick": "instance", "instanceId": cube.id }] }),
                );
                let copies: Vec<CardInstance> = [1, 2, 3, 4, 5]
                    .into_iter()
                    .filter_map(|lane| s.unit(P1, lane).filter(|unit| unit.def_id == MID))
                    .collect();
                assert_eq!(copies.len(), 2);
                for copy in &copies {
                    s.expect_stats(copy, json!({ "attack": 5, "health": 5 }));
                }
                assert_eq!(s.state().rng_cursor, cursor);
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 0);
            }

            #[test]
            fn r411_league_of_losers_summon_fires_it() {
                let mut s = seeded(
                    |seed| {
                        crate::register_all();
                        scenario(json!({
                            "seed": seed,
                            "p1": { "hand": [LEAGUE, FILLER], "library": DECK },
                            "p2": { "hand": [FILLER], "library": DECK },
                        }))
                    },
                    |each| next_flip(each, 0) == Flip::Heads,
                );
                s.play(LEAGUE, json!({}));
                let third = unit_or_blank(&s, P1, 3);
                s.expect_stats(&third, json!({ "attack": 10, "health": 10 }));
            }

            #[test]
            fn r386_heads_and_tails_read_through_param_an_upgrade_makes_heads_6_6_and_tails_2_2() {
                let mut heads = seeded(from_hand(false), |each| next_flip(each, 0) == Flip::Heads);
                step_param(heads.card_mut(MID), "heads", 1);
                heads.play(MID, json!({}));
                let me = mid(&heads);
                heads.expect_stats(&me, json!({ "attack": 11, "health": 11 }));

                let mut tails = seeded(from_hand(false), |each| next_flip(each, 0) == Flip::Tails);
                step_param(tails.card_mut(MID), "tails", -1);
                tails.play(MID, json!({}));
                let me = mid(&tails);
                tails.expect_stats(&me, json!({ "attack": 3, "health": 3 }));
            }

            #[test]
            fn s9_3_the_coin_replays_from_a_json_copy_to_the_same_hash() {
                let s = from_hand(true)("mid-replay");
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(MID).id,
                    "zone": { "row": "units", "lane": 1 },
                    "playerId": "p1",
                    "nonce": "mid-replay",
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
            fn draws_one_coin_from_the_match_rng() {
                let mut s = from_hand(false)("mid-one-coin");
                let cursor = s.state().rng_cursor;
                s.play(MID, json!({}));
                assert_eq!(s.state().rng_cursor - cursor, 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s6_1_lucky_1_tails_then_heads_is_heads_10_10() {
                let mut s = seeded(from_hand(true), |each| {
                    let mut rng = Rng::new(&each.state().seed, each.state().rng_cursor);
                    !rng.coin() && rng.coin()
                });
                s.play(MID, json!({}));
                let me = mid(&s);
                s.expect_stats(&me, json!({ "attack": 20, "health": 20 }));
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 0);
            }

            #[test]
            fn s6_1_lucky_1_two_tails_is_tails_3_3_and_a_mana_for_the_opponent() {
                let mut s = seeded(from_hand(true), |each| next_flip(each, 1) == Flip::Tails);
                s.play(MID, json!({}));
                let me = mid(&s);
                s.expect_stats(&me, json!({ "attack": 7, "health": 7 }));
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 1);
            }

            #[test]
            fn s6_1_lucky_1_flips_twice() {
                let mut s = from_hand(true)("mid-two-coins");
                let cursor = s.state().rng_cursor;
                s.play(MID, json!({}));
                assert_eq!(s.state().rng_cursor - cursor, 2);
            }

            #[test]
            fn r386_lucky_is_tuned_like_any_numbered_keyword_lucky_2_flips_three_times() {
                let mut s = from_hand(true)("mid-three-coins");
                let tuning = tuning_of(s.card_mut(MID));
                tuning.x = Some(add_step(tuning.x.as_ref(), "Lucky", 1));
                let cursor = s.state().rng_cursor;
                s.play(MID, json!({}));
                assert_eq!(s.state().rng_cursor - cursor, 3);
            }
        }

        /// R386 the opponent's mana on tails is declared, less being better: a Degrade gives them 2, and an
        /// Upgrade finds it at its floor of 1
        #[test]
        fn r386_a_degrade_gives_the_opponent_2_mana_on_tails_and_an_upgrade_finds_it_at_its_floor_of_1() {
            let mut s = seeded(from_hand(false), |each| next_flip(each, 0) == Flip::Tails);
            assert!(!crate::can_upgrade_number(&s, MID, "oppMana"));
            assert_eq!(crate::degrade_number(&mut s, MID, "oppMana"), 2);
            s.play(MID, json!({}));
            assert_eq!(s.state().players[P2].mana.next_turn_mod, 2);
        }
    }
}
