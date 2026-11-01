//! M #32 Spiritually 中国 (SPEC §8.8 row 32): (4) Spell, CN, Epic ((3) through v0.5.2).
//!
//! Base:    "Get ready to learn Chinese! Transform every card in your hand into the same random CN
//!           card, which costs (1). Summon {units} random CN Units." ((0) through v0.5.9e, kept as
//!           `script_v0_5_9e`, R767000)
//! Radiant: "… the same random Radiant CN card … Summon {units} random Radiant CN Units." Printed in
//!           English since issue #2179 (in Chinese before, MD-B13).
//! Engine:
//! - **All three, in order** since #2179: the learning, then the hand, then the summon. Through
//!   v0.5.2 one uniform pick of the three by the match rng as the Spell resolved (MD-B9, R920),
//!   Call to Chaos's single-effect shape, which `script_v0_5_2` keeps (R767000).
//! - **Get ready to learn Chinese** (ME-CN) sets `chinese` on every card in both hands and both
//!   libraries, silently (R440); on the Radiant face only the opponent's. As the designer asked,
//!   the printed text never says what it does.
//! - **Transform the hand** draws one definition from `query({ tags: [CN] })`, never this card
//!   (R387), then replaces every card in the caster's hand in its place (R31, R671; an Immutable
//!   hand card too, R35), each costing `CN_CARD_COST` and Radiant on the Radiant face (MD-B10, R921). Each is
//!   generated into a hand for R673's own Glitch roll.
//! - **Summon** `units` random CN Units (repeats allowed, R60), each into the leftmost empty,
//!   unlocked unit zone (R64); a full row takes fewer, drawing nothing for the rest (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-032";

/// The three effects, in the order the text prints them; v0.5.2's match rng picked among them as
/// the Spell resolved (MD-B9, R920).
const EFFECTS: i32 = 3;

/// "which costs (1)": what the new hand cards cost since #2657, (0) before.
const CN_CARD_COST: i32 = 1;
/// R767000: what they cost through v0.5.9e.
const CN_CARD_COST_V0_5_9E: i32 = 0;

/// One of the three: 0 the learning, 1 the hand, 2 the summon; the hand's cards cost `cost`.
fn effect(ctx: &mut EffectContext<'_>, radiant: bool, which: i32, cost: i32) -> Vec<Effect> {
    match which {
        // Get ready to learn Chinese: the flag on every card of both hands and both libraries (the
        // Radiant face: the opponent's only), silently (R440).
        0 => vec![translate(json_as(json!({
            "scope": {
                "side": if radiant { "enemy" } else { "any" },
                "zones": ["hand", "library"],
            },
        })))],
        // Transform the hand into the same random CN card, each costing `cost`.
        1 => {
            let mut args = json!({
                "scope": { "side": "self", "zones": ["hand"] },
                "same": true,
                "query": { "tags": ["CN"] },
                "costOverride": cost,
            });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![transform_random(json_as(args))]
        }
        // Summon `units` random CN Units, Radiant on the Radiant face.
        _ => (0..param(&*ctx, "units"))
            .map(|_| {
                let mut args = json!({ "query": { "type": "Unit", "tags": ["CN"] } });
                if radiant {
                    args["radiant"] = json!(true);
                }
                summon_random(json_as(args))
            })
            .collect(),
    }
}

fn spiritually(radiant: bool, cost: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..EFFECTS)
                .flat_map(|which| effect(ctx, radiant, which, cost))
                .collect()
        })),
        ..Script::default()
    }
}

/// v0.5.2's face: one random effect of the three (MD-B9, R920).
fn spiritually_v0_5_2(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let which = ctx.rng.int(EFFECTS);
            effect(ctx, radiant, which, CN_CARD_COST_V0_5_9E)
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face learns only the opponent's cards and makes Radiant cards and Units; the
    // numbers are the same on both faces.
    let base = spiritually(false, CN_CARD_COST);
    let radiant = spiritually(true, CN_CARD_COST);
    CardScripts { base, radiant }
}

/// R767000: through v0.5.9e the new hand cards cost (0); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9e() -> CardScripts {
    CardScripts {
        base: spiritually(false, CN_CARD_COST_V0_5_9E),
        radiant: spiritually(true, CN_CARD_COST_V0_5_9E),
    }
}

/// R767000: through v0.5.2 it did one of the three at random; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts {
        base: spiritually_v0_5_2(false),
        radiant: spiritually_v0_5_2(true),
    }
}

// M #32 Spiritually 中国 — SPEC §8.8 row 32, BUILD M10 row M 32: "All three in order as it resolves:
// (1) every card of both hands and both decks gains the `chinese` flag, silently in hands and decks
// (R440); (2) one random non-token CN definition (never this card, R387) replaces every card in your
// hand in place, Immutable ones too, each costing (1) since #2657, each a generated card for R673's roll (MD-B10);
// (3) 5 random CN Units summoned per R64, a full row taking fewer and drawing nothing for the rest
// (R129); units reads through `param()`; radiant the learning reaches only the opponent's hand and
// deck, and the new hand cards and the Units are Radiant".

#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use crate::filler;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SPIRITUALLY: &str = "meditative-032";
    const FILLER: &str = "core-005";
    /// Shipped non-token CN cards the pools may draw (R387 keeps this card out): SPEC §8's CN rows of
    /// every set that ships, the Meditative set's among them since it ships (R380, R1420).
    const CN_IDS: [&str; 26] = [
        "core-080",
        "core-090",
        "classicplus-004",
        "classicplus-023",
        "classicplus-076",
        "meditative-033",
        "meditative-034",
        "meditative-035",
        "meditative-036",
        "meditative-037",
        "meditative-038",
        "meditative-039",
        "meditative-040",
        "meditative-041",
        "meditative-042",
        "meditative-043",
        "meditative-044",
        "meditative-045",
        "meditative-047",
        "meditative-049",
        "meditative-050",
        "meditative-055",
        "meditative-078",
        "meditative-087",
        "meditative-101",
        "meditative-102",
    ];

    /// p1 holds Spiritually (base unless `radiant_face`) with two more hand cards and cards in
    /// both decks; both sides keep units off the field so summons land.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": SPIRITUALLY, "radiant": radiant_face }, FILLER, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The roll the match rng would make for a scenario built but not yet played: the kept script's
    /// first draw, as c036 predicts its coins.
    fn roll(s: &Scenario) -> i32 {
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        rng.int(EFFECTS)
    }

    /// The units p1 has, lane by lane.
    fn units_of(s: &Scenario) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(P1, lane)).collect()
    }

    mod m32_spiritually_zhongguo {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn all_three_in_order_the_learning_the_hand_and_five_cn_units() {
                let mut s = casting("spiritually-all", false);
                s.play(SPIRITUALLY, json!({}));
                // The learning: the opponent's hand and both decks are Chinese.
                for card in s.hand(P2) {
                    assert_eq!(card.chinese, Some(true));
                }
                for player in [P1, P2] {
                    let library = &s.state().players[player].library;
                    assert!(!library.is_empty());
                    for card in library {
                        assert_eq!(card.chinese, Some(true), "{}", card.id);
                    }
                }
                // The hand: one CN definition, each costing (1).
                let defs = def_ids(&s.hand(P1));
                assert_eq!(defs.len(), 2);
                assert!(defs.iter().all(|def| *def == defs[0]));
                assert!(CN_IDS.contains(&defs[0].as_str()));
                // The summon: 5 CN Units take the empty row.
                let units = units_of(&s);
                assert_eq!(units.len(), 5);
                for unit in &units {
                    let def = crate::card_def(&unit.def_id);
                    assert_eq!(def.type_, CardType::Unit);
                    assert!(def.tags.contains(&Tag::Cn));
                    assert!(!unit.radiant);
                }
                // In the text's order: the translation, then the transform, then the summons.
                let order: Vec<GameEventType> = s
                    .events()
                    .iter()
                    .map(GameEvent::event_type)
                    .filter(|kind| {
                        matches!(kind, GameEventType::Translated | GameEventType::Transformed | GameEventType::Summoned)
                    })
                    .collect();
                let first_transform = order.iter().position(|kind| *kind == GameEventType::Transformed);
                let first_summon = order.iter().position(|kind| *kind == GameEventType::Summoned);
                assert!(first_transform < first_summon, "{order:?}");
            }

            #[test]
            fn r921_one_cn_definition_replaces_the_hand_in_place_at_1() {
                let mut s = casting("spiritually-hand", false);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.play(SPIRITUALLY, json!({}));
                let defs = def_ids(&s.hand(P1));
                // The played Spell left; the other two were replaced in place by one definition.
                assert_eq!(defs.len(), 2);
                assert!(defs.iter().all(|def| *def == defs[0]));
                assert!(CN_IDS.contains(&defs[0].as_str()));
                for card in s.hand(P1) {
                    assert!(!before.contains(&card.id));
                    assert_eq!(card.cost_override, Some(1));
                    assert!(!card.radiant);
                }
            }

            #[test]
            fn r129_a_full_row_takes_fewer_units() {
                crate::register_all();
                let mut full = scenario(json!({
                    "seed": "spiritually-full",
                    "p1": {
                        "hand": [{ "def": SPIRITUALLY }, FILLER],
                        "field": [
                            { "def": "core-008", "lane": 1 },
                            { "def": "core-008", "lane": 2 },
                            { "def": "core-008", "lane": 3 },
                        ],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                full.play(SPIRITUALLY, json!({}));
                assert_eq!(units_of(&full).len(), 5, "three stood and two arrive");
            }

            #[test]
            fn units_reads_through_param() {
                let mut s = casting("spiritually-param", false);
                set_param(s.card_mut(SPIRITUALLY), "units", 2);
                s.play(SPIRITUALLY, json!({}));
                assert_eq!(units_of(&s).len(), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r920_learning_reaches_only_the_opponent_and_the_hand_cards_and_units_are_radiant() {
                let mut s = casting("spiritually-radiant", true);
                s.play(SPIRITUALLY, json!({}));
                assert!(!s.hand(P2).is_empty());
                for card in s.hand(P2) {
                    assert_eq!(card.chinese, Some(true));
                }
                for card in &s.state().players[P1].library {
                    assert_eq!(card.chinese, None);
                }
                assert_eq!(def_ids(&s.hand(P1)).len(), 2);
                for card in s.hand(P1) {
                    assert!(card.radiant);
                    assert_eq!(card.cost_override, Some(1));
                }
                let units = units_of(&s);
                assert_eq!(units.len(), 5);
                for unit in &units {
                    assert!(unit.radiant);
                }
            }

            #[test]
            fn its_radiant_face_is_printed_in_english() {
                let def = crate::card_def(SPIRITUALLY);
                assert!(def.radiant.text.starts_with("Get ready to learn Chinese!"));
                assert!(def.radiant.text.contains("Radiant CN"));
            }
        }

        /// R767000: the script kept for the replays of v0.5.9e and before, whose hand cards cost (0).
        mod v0_5_9e {
            use super::*;

            #[test]
            fn r767000_a_game_on_v0_5_9e_makes_the_hand_cost_0() {
                crate::under_kept(SPIRITUALLY, script_v0_5_9e(), || {
                    let mut s = casting("spiritually-kept-9e", false);
                    s.play(SPIRITUALLY, json!({}));
                    assert!(!s.hand(P1).is_empty());
                    for card in s.hand(P1) {
                        assert_eq!(card.cost_override, Some(0));
                    }
                });
            }
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, which did one of the three
        /// at random (MD-B9, R920).
        mod v0_5_2 {
            use super::*;

            #[test]
            fn r767000_r920_a_game_on_v0_5_2_does_one_effect_by_one_roll() {
                crate::under_kept(SPIRITUALLY, script_v0_5_2(), || {
                    let mut saw = [false; 3];
                    for n in 0..32 {
                        let seed = format!("spiritually-rolls-{n}");
                        let mut s = casting(&seed, false);
                        let effect = roll(&s);
                        saw[effect as usize] = true;
                        s.play(SPIRITUALLY, json!({}));
                        let transformed = s.events().iter().any(|event| event.event_type() == GameEventType::Transformed);
                        let summoned = !units_of(&s).is_empty();
                        let learned = s.hand(P2).iter().all(|card| card.chinese == Some(true));
                        assert_eq!((learned, transformed, summoned), (effect == 0, effect == 1, effect == 2), "{seed}");
                    }
                    assert!(saw.iter().all(|seen| *seen), "all three effects occur over 32 seeds");
                });
            }
        }
    }
}
