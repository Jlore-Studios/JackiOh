//! M #51 Devin Bot (SPEC §8.8 row 51, R1080): (2) Unit, Legendary, 1/1 → 2/2.
//!
//! Base:    "Cry: Fill your board with random 1/1 Units."
//! Radiant: "Cry: Fill your board with random Radiant 1/1 Units."
//! Engine: `fill_board_random` with `{ type: Unit, stats: {1, 1}, withTokens: true }` (R1080): every
//! empty unit zone (R1136000), left to right, gets its own pick from the match rng, repeats allowed,
//! from the Units whose printed base face is 1/1, tokens included, never Devin Bot itself (R387).
//! Summons fire no Cry (R1). It has had no Reborn since patch v0.3.10.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-051";

fn devin_bot(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![fill_board_random(json_as(json!({
                "query": {
                    "type": "Unit",
                    "stats": { "attack": 1, "health": 1 },
                    "withTokens": true,
                },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: devin_bot(false),
        radiant: devin_bot(true),
    }
}

// M #51 Devin Bot — SPEC §8.8 row 51, BUILD M10 row M 51: "No Reborn (patch v0.3.10); Cry (played or
// cast, R1): each empty unit zone of yours, a Locked one included (R1136000), left to right, gets its
// own random Unit whose printed base face is 1/1, tokens included, never Devin Bot (R1080); a full
// board draws no random number (R129); it dies for good; radiant 2/2 and the 1/1s are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::effects::cast_new;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{ZoneSlot, lock_zone};

    const P1: PlayerId = PlayerId::P1;

    const BOT: &str = "meditative-051";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 plays Devin Bot (`radiant_face`) with an otherwise empty board.
    fn played(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        let _guard = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": BOT, "radiant": radiant_face }, FILLER],
                "library": filler(3),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(BOT, json!({}));
        s
    }

    fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        active_units_of(s.state(), player).into_iter().cloned().collect()
    }

    /// Lunar Eclipse (Core #35): (1) Spell, deals 3 to a target.
    const ECLIPSE: &str = "core-035";

    fn eclipse(target: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": target }] })
    }

    /// p1 with `field` already on the board and Devin Bot in hand.
    fn board(seed: &str, field: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [BOT, FILLER], "field": field, "library": filler(3) },
            "p2": { "hand": [ECLIPSE, FILLER], "library": filler(4) },
        }))
    }

    /// The units p1's Cry summoned (every one but the Bot).
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        let me = s.card(BOT).id.clone();
        units_of(s, P1).into_iter().filter(|unit| unit.id != me).collect()
    }

    mod m51_devin_bot {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn cry_fills_each_open_zone_left_to_right_with_1_1s_never_itself() {
                let s = played("devin-fill", false);

                let units = units_of(&s, P1);
                // Five unit zones: Devin Bot plus four summons.
                assert_eq!(units.len(), 5);
                let lanes: Vec<i32> = units
                    .iter()
                    .map(|unit| slot_of(s.state(), unit).map(|slot| slot.lane).unwrap_or(0))
                    .collect();
                assert_eq!(lanes, vec![1, 2, 3, 4, 5]);
                for unit in &units {
                    let def = crate::card_def(&unit.def_id);
                    assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
                    assert!(!unit.radiant);
                }
                assert_eq!(
                    units.iter().filter(|unit| unit.def_id == BOT).count(),
                    1,
                    "Devin Bot never generates itself (R387)"
                );
            }

            #[test]
            fn has_no_reborn_since_patch_v0_3_10_so_it_dies_for_good() {
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = board(
                    "devin-reborn-keyword",
                    json!(["core-008", "core-008", "core-008", "core-008"]),
                );
                s.play(BOT, json!({}));
                let me = s.card(BOT).id.clone();
                let keywords = serde_json::to_string(&s.stats(me.as_str()).keywords).expect("JSON");
                assert!(!keywords.contains("Reborn"), "{keywords}");
                s.end_turn();
                s.play(ECLIPSE, eclipse(&me));
                // Nothing comes back into its zone, lane 5.
                assert!(s.unit(P1, 5).is_none());
                s.expect_in_zone(&me, "graveyard");
            }

            #[test]
            fn r1_a_cast_fires_the_cry_too() {
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = board("devin-cast", json!([]));
                let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
                let mut rng = Rng::new(&seed, cursor);
                let mut events: Vec<GameEvent> = Vec::new();
                {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    {
                        let mut ctx = make_context(
                            &mut sink,
                            None,
                            HookOptions {
                                controller: Some(P1),
                                ..HookOptions::default()
                            },
                        );
                        apply_effects(&[cast_new(json_as(json!({ "def": BOT })))], &mut ctx);
                    }
                    settle(&mut sink, SettleOptions::default());
                }
                s.state_mut().rng_cursor = rng.cursor();
                // Cast, it enters and its Cry fills the other four zones.
                assert_eq!(units_of(&s, P1).len(), 5);
            }

            #[test]
            fn r1136000_r1080_a_locked_zone_gets_its_own_pick_too() {
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = board("devin-locked", json!([]));
                lock_zone(
                    s.state_mut(),
                    ZoneSlot {
                        player: P1,
                        row: Row::Units,
                        lane: 3,
                    },
                );
                s.play(BOT, json!({ "zone": 1 }));
                let lanes: Vec<i32> = units_of(&s, P1)
                    .iter()
                    .map(|unit| slot_of(s.state(), unit).map(|slot| slot.lane).unwrap_or(0))
                    .collect();
                assert_eq!(lanes, vec![1, 2, 3, 4, 5]);
            }

            #[test]
            fn r1080_each_zone_its_own_pick_tokens_included() {
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut mixed = false;
                let mut token = false;
                for n in 0..40 {
                    let mut s = board(&format!("devin-picks-{n}"), json!([]));
                    s.play(BOT, json!({}));
                    let defs: Vec<String> = summoned(&s).into_iter().map(|unit| unit.def_id).collect();
                    assert_eq!(defs.len(), 4, "{n}");
                    let mut distinct = defs.clone();
                    distinct.sort();
                    distinct.dedup();
                    mixed |= distinct.len() > 1;
                    token |= defs.iter().any(|def| crate::card_def(def).token);
                }
                assert!(mixed, "each zone draws its own pick");
                assert!(token, "the pool holds tokens");
            }

            #[test]
            fn r1_the_summoned_1_1s_fire_no_cry() {
                let _guard = preview_sets(&[SetName::Meditative]);
                for n in 0..10 {
                    let mut s = board(&format!("devin-no-cry-{n}"), json!([]));
                    s.play(BOT, json!({}));
                    // Only the Bot was played, and no summoned Cry (a Discover, a summon) ran.
                    let plays = s
                        .events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::CardPlayed { .. }))
                        .count();
                    assert_eq!(plays, 1, "{n}");
                    assert!(s.state().pending.is_none(), "{n}");
                    assert_eq!(units_of(&s, P1).len(), 5, "{n}");
                }
            }

            #[test]
            fn r129_a_board_it_fills_itself_draws_no_random_number() {
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = board(
                    "devin-last-zone",
                    json!(["core-008", "core-008", "core-008", "core-008"]),
                );
                let cursor = s.state().rng_cursor;
                s.play(BOT, json!({}));
                // It took the last open zone: its Cry finds none and draws nothing.
                assert_eq!(units_of(&s, P1).len(), 5);
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn r1080_r129_a_full_board_refuses_the_play() {
                crate::register_all();
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = scenario(json!({
                    "seed": "devin-full-board",
                    "p1": {
                        "hand": [BOT, FILLER],
                        "field": ["core-008", "core-008", "core-008", "core-008", "core-008"],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                // No open zone: the play itself is refused, so the Cry never runs (R129's draw
                // count is proved at the verb level in the engine's `effects_summon.rs`).
                s.expect_refused(|s| s.play(BOT, json!({})));
            }

            #[test]
            fn its_death_brings_nothing_back_and_fires_no_cry() {
                crate::register_all();
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = scenario(json!({
                    "seed": "devin-reborn",
                    "p1": {
                        "hand": [BOT, FILLER],
                        "library": filler(3),
                    },
                    "p2": {
                        "hand": [ECLIPSE, FILLER],
                        "library": filler(4),
                    },
                }));
                s.play(BOT, json!({}));
                let mut before: Vec<String> = units_of(&s, P1).iter().map(|unit| unit.id.clone()).collect();
                before.sort();
                assert_eq!(before.len(), 5);
                // Lunar Eclipse deals 3 to the 1/1 Bot itself: a deterministic kill. With no Reborn
                // (patch v0.3.10) the four it summoned stay and nothing new arrives.
                s.end_turn();
                let me = s.card(BOT).id.clone();
                s.play(ECLIPSE, eclipse(&me));
                let mut after: Vec<String> = units_of(&s, P1).iter().map(|unit| unit.id.clone()).collect();
                after.sort();
                let survivors: Vec<String> = before.into_iter().filter(|id| id != &me).collect();
                assert_eq!(after, survivors);
                s.expect_in_zone(&me, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn cry_summons_radiant_1_1s() {
                let s = played("devin-radiant", true);

                let units = units_of(&s, P1);
                assert_eq!(units.len(), 5);
                for unit in units.iter().filter(|unit| unit.id != s.card(BOT).id) {
                    assert!(unit.radiant);
                    let def = crate::card_def(&unit.def_id);
                    assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
                }
                let stats = s.stats(BOT);
                assert_eq!((stats.attack, stats.health), (2, 2));
            }
        }
    }
}
