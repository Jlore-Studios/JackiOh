//! M #97.8 Prison (SPEC §8.8 row 97.8, R1284). (4) Unit, Token (printed Legendary), 0/16 → 0/32.
//!   Base:    "Tribute 2\nAfter your opponent plays a Unit: {chance}% chance to stack it under this; you control it."
//!   Radiant: "Tribute 2, Lucky 1\nAfter your opponent plays a Unit: {chance}% chance to stack it under this. You control it."
//!   Engine:  "NEW: a capture (CAPTURE): a trigger on the resolution of a Unit its controller's opponent
//!            played or cast (R70), in the window #41 Sheepish uses (after the Unit resolves, its Cry
//!            included, §10.5 step 7, R427), while Prison acts: one `rng.chance`, the Radiant face's
//!            Lucky 1 rolling again. On a success the Unit, if still on the field on that stay (R174),
//!            moves from the top of its zone to directly beneath Prison in Prison's pile without leaving
//!            the field (no R78 reset, its damage kept), dormant (R13), its controller becoming Prison's
//!            (control only, its owner unchanged, R12; an entry, R171), with a public `controlChanged`
//!            (how: steal); a card it uncovered on the opponent's side resumes there; several captures
//!            lie newest first beneath Prison, and when Prison leaves the field the newest resumes under
//!            Prison's controller (MD-F16). Tunes: chance 50 ↑; Tribute 2 ↓ (never below 1); Lucky 1 ↑,
//!            on the Radiant face only."

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::steal::{CaptureArgs, capture};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-8";

/// §8.8: "Tribute 2".
const TRIBUTE_COST: i32 = 2;

/// Declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// The face's Lucky X keyword.
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(own) = ctx.self_.as_ref() else {
        return 0;
    };
    numbered_keywords_on(ctx.sink.state, own)
        .into_iter()
        .find(|keyword| keyword.key == NumberedKey::Lucky)
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

fn prison_trigger() -> TriggerDef {
    TriggerDef::new(
        "prison-capture",
        &[GameEventType::CardResolved],
        |ctx, event| {
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
            let lucky = lucky_of(ctx);
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

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        triggers: vec![prison_trigger()],
        ..Script::default()
    };
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
}
