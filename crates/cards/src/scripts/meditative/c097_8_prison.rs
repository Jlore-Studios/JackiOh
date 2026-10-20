//! M #97.8 Prison (SPEC §8.8 row 97.8, R1284). (4) Unit, Token (printed Legendary), 0/16 → 0/32.
//!   Base:    "Tribute 2\nAfter your opponent plays a Unit: {chance}% chance to stack it under this; you control it."
//!   Radiant: "Tribute 2, Lucky 1\nAfter your opponent plays a Unit: {chance}% chance to stack it under this. You control it."
//!   Engine:  "NEW: a capture (CAPTURE): a trigger on the resolution of a Unit its controller's opponent
//!            played or cast (R70), in the window #41 Sheepish uses (after the Unit resolves, its Cry
//!            included, §10.5 step 7, R427), while Prison acts: one `rng.chance`, the Radiant face's
//!            Lucky 1 and its controller's Luck (R987) rolling it again. On a success the Unit, if still
//!            on the field on that stay (R174), moves from the top of its zone to directly beneath Prison
//!            in Prison's pile without leaving the field (no R78 reset, its damage kept), dormant (R13),
//!            its controller becoming Prison's (control only, its owner unchanged, R12; an entry, R171),
//!            with a public `controlChanged` (how: steal); a card it uncovered on the opponent's side
//!            resumes there; several captures lie newest first beneath Prison, and when Prison leaves the
//!            field the newest resumes under Prison's controller (MD-F16). Tunes: chance 50 ↑; Tribute 2 ↓
//!            (never below 1); Lucky 1 ↑, on the Radiant face only."

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::steal::{CaptureArgs, capture};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-8";

/// §8.8: "Tribute 2".
const TRIBUTE_COST: i32 = 2;

/// Declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// Which Lucky the capture rolls with: `lucky_of` today, `printed_lucky_of` in the script v0.3.7b kept.
type LuckyOf = fn(&EffectContext<'_>) -> i32;

/// §6.1, R1438, R987: Prison's Lucky X, printed (as Nerf and Buff have moved it) plus given
/// (`lucky_on`), and its controller's Luck.
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    ctx.self_.as_ref().map_or(0, |own| lucky_on(ctx.sink.state, own)) + luck_of(ctx.sink.state, ctx.controller)
}

/// R767000: the face's printed Lucky X keyword alone: what the capture rolled with through v0.3.7b
/// (`script_v0_3_7b`).
fn printed_lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(own) = ctx.self_.as_ref() else {
        return 0;
    };
    numbered_keywords_on(ctx.sink.state, own)
        .into_iter()
        .find(|keyword| keyword.key == NumberedKey::Lucky)
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

fn prison_trigger(read_lucky: LuckyOf) -> TriggerDef {
    TriggerDef::new(
        "prison-capture",
        &[GameEventType::CardResolved],
        move |ctx, event| {
            let GameEvent::CardResolved {
                instance_id,
                permanent,
                ..
            } = event
            else {
                return vec![];
            };
            if !*permanent {
                return vec![];
            }
            let chance = f64::from(param(&*ctx, "chance")) / f64::from(PERCENT);
            let lucky = read_lucky(ctx);
            if ctx.rng.lucky(lucky, |rng| rng.chance(chance), |a, b| a || b) {
                vec![capture(CaptureArgs {
                    instance_id: instance_id.clone(),
                })]
            } else {
                vec![]
            }
        },
    )
    .with_when(|ctx, event| {
        let GameEvent::CardResolved {
            player,
            def_id,
            permanent,
            ..
        } = event
        else {
            return false;
        };
        // §8: "your opponent".
        if *player == ctx.controller {
            return false;
        }
        // §5.1: a Unit.
        if def_of(Some(&*ctx.state), def_id).type_ != CardType::Unit {
            return false;
        }
        *permanent
    })
}

/// One face: Tribute 2 and the capture, its roll's Lucky read by `read_lucky`.
fn face(read_lucky: LuckyOf) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        triggers: vec![prison_trigger(read_lucky)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = face(lucky_of);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: the scripts v0.3.7b and every version before it ran, whose capture read the printed Lucky
/// alone, without its controller's Luck (R987) or a given Lucky (R1438); `KEPT_SCRIPTS` folds those
/// versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    let base = face(printed_lucky_of);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M 97.8 Prison — SPEC §8.8 row 97.8, BUILD M10 row M 97.8: "Tribute 2; after each Unit the opponent plays
// or casts has resolved, its Cry included, a 50% roll moves it, still on the field, beneath Prison under your
// control, dormant, its damage kept and its owner unchanged (MD-F16), `controlChanged` public, a card it
// uncovered resuming on their side; a failure leaves it; summons are not captured; when Prison leaves, the
// newest captive resumes under your control; chance reads through `param()`; radiant 0/32 and Lucky 1: 75%".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit.
    const FIENDER: &str = "core-092"; // (2) Unit: Stack.
    const FELINORS: &str = "core-012"; // (2) Unit: Cry summons a copy.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.

    fn slot(player: PlayerId, lane: i32) -> ZoneSlot {
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        }
    }

    #[test]
    fn is_a_4_cost_unit_token_printed_legendary() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-8");
        assert_eq!(js(&def.cost), json!(4));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(16), json!(0), json!(32)]
        );
        assert_eq!(
            js(&def.radiant.keywords),
            json!([{ "kind": "Lucky", "n": 1 }])
        );
        let s = script();
        assert_eq!(s.base.static_flags.as_ref().and_then(|f| f.tribute), Some(2));
    }

    mod base {
        use super::*;

        #[test]
        fn r1284_captures_at_chance_100() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "hand": [VANILLA, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
            }));

            // Set chance to 100 on Prison.
            set_param(s.card_mut(ID), "chance", 100);

            s.end_turn(); // P2's turn.

            s.play(VANILLA, json!({ "zone": 2 }));

            // VANILLA resolved and was captured beneath Prison in P1 lane 1!
            let pile = jackioh_engine::zones::pile_at(s.state(), slot(P1, 1)).unwrap();
            assert_eq!(pile.len(), 2);
            assert_eq!(pile[0].def_id, ID);
            assert_eq!(pile[1].def_id, VANILLA);
            assert_eq!(pile[1].controller, P1);
            assert!(jackioh_engine::zones::is_buried(s.state(), &pile[1]));

            // P2 lane 2 is empty!
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn r1284_seed_sweep_at_chance_1_misses_at_least_once() {
            crate::register_all();
            let mut missed = false;

            for n in 0..100 {
                let mut s = crate::scenario(json!({
                    "seed": format!("prison-sweep-{n}"),
                    "p1": {
                        "field": [{ "def": ID, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [SPARE, SPARE]
                    },
                    "p2": {
                        "hand": [VANILLA, FILLER],
                        "library": [SPARE, SPARE],
                        "mana": 10
                    },
                }));

                set_param(s.card_mut(ID), "chance", 1);
                s.end_turn();
                s.play(VANILLA, json!({ "zone": 2 }));

                // If not captured, VANILLA remains in P2 lane 2!
                if s.unit(P2, 2).is_some() {
                    missed = true;
                    break;
                }
            }

            assert!(missed, "at chance 1, Prison misses capture at least once");
        }

        #[test]
        fn r1284_summon_is_not_captured() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "hand": [FELINORS, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
            }));

            // Even at chance 100...
            set_param(s.card_mut(ID), "chance", 100);

            s.end_turn(); // P2's turn.

            // P2 plays Felinors into lane 2.
            s.play(FELINORS, json!({ "zone": 2 }));

            // The played Felinors in lane 2 is captured.
            // But the SUMMONED copy in lane 1 of P2 is NOT captured!
            let p2_summon = s.unit(P2, 1);
            assert!(p2_summon.is_some(), "summoned token is not captured");
            assert_eq!(p2_summon.unwrap().controller, P2);
        }

        #[test]
        fn r1284_capturing_felinor_fiender_uncovers_p2_unit_beneath() {
            crate::register_all();
            // P2 has VANILLA in lane 1.
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "hand": [FIENDER, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
            }));

            set_param(s.card_mut(ID), "chance", 100);

            s.end_turn(); // P2's turn.

            // P2 plays Fiender stacking onto lane 1!
            s.play(FIENDER, json!({ "zone": 1 }));

            // Fiender is captured beneath Prison in P1 lane 1!
            let p1_pile = jackioh_engine::zones::pile_at(s.state(), slot(P1, 1)).unwrap();
            assert_eq!(p1_pile.len(), 2);
            assert_eq!(p1_pile[1].def_id, FIENDER);

            // VANILLA on P2 lane 1 is uncovered and resumes!
            let p2_unit = s.unit(P2, 1).expect("uncovered unit in lane 1");
            assert_eq!(p2_unit.def_id, VANILLA);
            assert_eq!(p2_unit.controller, P2);
            assert!(!jackioh_engine::zones::is_buried(s.state(), &p2_unit));
        }

        #[test]
        fn r1284_hit_job_on_prison_resumes_captive_under_p1() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [HIT_JOB, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
                "p2": {
                    "hand": [VANILLA, FILLER],
                    "library": [SPARE, SPARE],
                    "mana": 10
                },
            }));

            set_param(s.card_mut(ID), "chance", 100);

            s.end_turn(); // P2 plays VANILLA into lane 2.
            s.play(VANILLA, json!({ "zone": 2 }));

            // VANILLA is now captive under Prison in P1 lane 1.
            s.end_turn(); // Back to P1's turn.

            let prison_id = s.card(ID).id.clone();
            // P1 casts Hit Job destroying Prison!
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": prison_id }] }));

            // Prison is destroyed, and the captive (VANILLA) resumes in lane 1 under P1!
            let resumed = s.unit(P1, 1).expect("captive resumed in lane 1");
            assert_eq!(resumed.def_id, VANILLA);
            assert_eq!(resumed.controller, P1);
            assert!(!jackioh_engine::zones::is_buried(s.state(), &resumed));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1284_radiant_face_has_lucky_1() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "field": [{ "def": ID, "radiant": true, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [SPARE, SPARE]
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 32 }));
            let prison = s.card(ID).clone();
            assert!(
                s.stats(&prison)
                    .keywords
                    .iter()
                    .any(|k| k == &Keyword::Lucky { n: 1 }),
                "radiant Prison has Lucky 1"
            );
        }
    }

    // R987, R1438: the capture rolls once, once more for each Lucky on Prison, printed or given, and once
    // more for each point of its controller's Luck; a capture draws nothing more, so at chance 100 the
    // rolls are the whole difference Prison makes to the cursor.
    mod luck {
        use super::*;

        const FENG_SHUI: &str = "meditative-040"; // (2) Field Spell: "Aura: You have Luck 1".

        /// The draws Prison's capture takes as p2 plays VANILLA at chance 100: the play's move of the
        /// rng cursor with Prison on p1's field, less the same play's without it (Gary's control,
        /// R32). Feng Shui is in `feng_shui`'s backrow, and Prison has Lucky `given` given to it.
        fn capture_rolls(radiant: bool, feng_shui: Option<PlayerId>, given: i32) -> i64 {
            let backrow = |player: PlayerId| {
                if feng_shui == Some(player) {
                    json!([FENG_SHUI])
                } else {
                    json!([])
                }
            };
            let play = |prison: bool| -> i64 {
                let field = if prison {
                    json!([{ "def": ID, "radiant": radiant, "lane": 1 }])
                } else {
                    json!([])
                };
                let mut s = crate::scenario(json!({
                    "seed": "prison-luck",
                    "p1": { "field": field, "backrow": backrow(P1), "hand": [FILLER], "library": [SPARE, SPARE] },
                    "p2": {
                        "backrow": backrow(P2),
                        "hand": [VANILLA, FILLER],
                        "library": [SPARE, SPARE],
                        "mana": 10
                    },
                }));
                assert_eq!(luck_of(s.state(), P1), i32::from(feng_shui == Some(P1)));
                if prison {
                    set_param(s.card_mut(ID), "chance", 100);
                    if given > 0 {
                        crate::give_lucky(&mut s, ID, given);
                    }
                }
                s.end_turn(); // P2's turn.
                let cursor = i64::from(s.state().rng_cursor);
                s.play(VANILLA, json!({ "zone": 2 }));
                if prison {
                    assert!(s.unit(P2, 2).is_none(), "captured at chance 100");
                }
                i64::from(s.state().rng_cursor) - cursor
            };
            play(true) - play(false)
        }

        #[test]
        fn r987_feng_shui_on_prison_s_side_rolls_the_capture_once_more_on_each_face() {
            crate::register_all();
            assert_eq!(capture_rolls(false, None, 0), 1);
            assert_eq!(capture_rolls(false, Some(P1), 0), 2);
            assert_eq!(capture_rolls(true, None, 0), 2);
            assert_eq!(capture_rolls(true, Some(P1), 0), 3);
        }

        #[test]
        fn r987_feng_shui_on_the_opponent_s_side_adds_no_roll() {
            crate::register_all();
            assert_eq!(capture_rolls(false, Some(P2), 0), 1);
            assert_eq!(capture_rolls(true, Some(P2), 0), 2);
        }

        #[test]
        fn r1438_lucky_given_to_prison_rolls_once_more() {
            crate::register_all();
            assert_eq!(capture_rolls(false, None, 1), 2);
            assert_eq!(capture_rolls(true, None, 1), 3);
        }

        #[test]
        fn r987_r767000_a_game_on_v0_3_7_folds_under_the_kept_script_without_its_controller_s_luck() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                assert_eq!(capture_rolls(false, Some(P1), 0), 1);
                assert_eq!(capture_rolls(true, Some(P1), 0), 2);
            });
        }
    }

    /// R1081000: a Flying Unit the opponent plays into the backrow is captured as one played into a unit
    /// zone is.
    #[test]
    fn r1081000_a_flying_unit_played_into_the_backrow_is_captured() {
        crate::register_all();
        let mut s = crate::scenario(json!({
            "p1": { "field": [{ "def": ID, "lane": 1 }], "hand": [FILLER], "library": [SPARE, SPARE] },
            "p2": { "hand": [VANILLA, FILLER], "library": [SPARE, SPARE], "mana": 10 },
        }));
        set_param(s.card_mut(ID), "chance", 100);
        s.end_turn();
        let vanilla = s.card(VANILLA).id.clone();
        crate::give_flying(&mut s, &vanilla);

        s.play(VANILLA, json!({ "row": "backrow", "zone": 2 }));

        let pile = jackioh_engine::zones::pile_at(s.state(), slot(P1, 1)).unwrap();
        assert_eq!(pile.len(), 2);
        assert_eq!(pile[1].id, vanilla);
        assert_eq!(pile[1].controller, P1);
        assert!(s.backrow(P2, 2).is_none());
    }

    /// R1081000: a Prison standing in the backrow captures beneath itself there, and the capture resumes
    /// standing there when Prison leaves.
    #[test]
    fn r1081000_a_prison_standing_in_the_backrow_captures_beneath_itself() {
        crate::register_all();
        let mut s = crate::scenario(json!({
            "p1": { "field": [{ "def": ID, "lane": 1 }], "hand": [FILLER], "library": [SPARE, SPARE] },
            "p2": { "hand": [VANILLA, FILLER], "library": [SPARE, SPARE], "mana": 10 },
        }));
        set_param(s.card_mut(ID), "chance", 100);
        let prison = s.card(ID).id.clone();
        crate::stand_in_backrow(&mut s, &prison, 3);
        s.end_turn();
        let vanilla = s.card(VANILLA).id.clone();

        s.play(VANILLA, json!({ "zone": 2 }));

        let at = ZoneSlot { player: P1, row: Row::Backrow, lane: 3 };
        let beneath = jackioh_engine::zones::beneath_at(s.state(), at);
        assert_eq!(beneath.iter().map(|card| card.id.as_str()).collect::<Vec<_>>(), [vanilla.as_str()]);
        assert_eq!(beneath[0].controller, P1);
        assert!(s.unit(P2, 2).is_none());
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, row: Row::Backrow, lane: 3, .. } if *instance_id == vanilla
        )));

        let mut gone = s.card(prison.as_str()).clone();
        remove_from_any_zone(s.state_mut(), &mut gone);
        assert_eq!(s.backrow(P1, 3).map(|card| card.id), Some(vanilla.clone()));
        assert!(stands_in_backrow(s.state(), s.card(vanilla.as_str())));
    }
}
