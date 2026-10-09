//! M #51 Devin Bot (SPEC §8.8 row 51, R1080): (2) Unit, Legendary, 1/1 → 2/2.
//!
//! Base:    "Reborn\nCry: Fill your board with random 1/1 Units."
//! Radiant: "Reborn\nCry: Fill your board with random Radiant 1/1 Units."
//! Engine: `fill_board_random` with `{ type: Unit, stats: {1, 1}, withTokens: true }` (R1080): every
//! empty, unlocked unit zone, left to right, gets its own pick from the match rng, repeats allowed,
//! from the Units whose printed base face is 1/1, tokens included, never Devin Bot itself (R387).
//! Summons fire no Cry (R1), and Reborn's return does not fire the Cry again.

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

// M #51 Devin Bot — SPEC §8.8 row 51, BUILD M10 row M 51: "Reborn; Cry (played or cast, R1): each
// empty, unlocked unit zone of yours, left to right, gets its own random Unit whose printed base
// face is 1/1, tokens included, never Devin Bot (R1080); a full board draws no random number
// (R129); Reborn's return does not fire the Cry; radiant 2/2 and the 1/1s are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

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
            fn reborn_s_return_fires_no_cry() {
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
                let mut before: Vec<String> =
                    units_of(&s, P1).iter().map(|unit| unit.id.clone()).collect();
                before.sort();
                assert_eq!(before.len(), 5);
                // Lunar Eclipse deals 3 to the 1/1 Bot itself: a deterministic kill (no shield on
                // the Bot to pop). Reborn returns it into its own reserved zone with no Cry — a
                // return is a summon, not a play (R1) — so the four survivors keep their ids and
                // the only new id is the returned body.
                s.end_turn();
                let me = s.card(BOT).id.clone();
                s.play(ECLIPSE, eclipse(&me));
                let mut after: Vec<String> =
                    units_of(&s, P1).iter().map(|unit| unit.id.clone()).collect();
                after.sort();
                assert_eq!(after.len(), 5);
                let survivor: Vec<String> = before.into_iter().filter(|id| id != &me).collect();
                for id in &survivor {
                    assert!(after.contains(id), "survivor {id} is still there");
                }
                let returned: Vec<String> =
                    after.into_iter().filter(|id| !survivor.contains(id)).collect();
                assert_eq!(returned.len(), 1);
                assert_eq!(s.card(returned[0].as_str()).def_id, BOT);
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
