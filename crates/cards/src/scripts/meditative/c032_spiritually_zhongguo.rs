//! M #32 Spiritually 中国 (SPEC §8.8 row 32): (2) Spell, CN, Epic.
//!
//! Base:    "One random effect: Get ready to learn Chinese; transform every card in your hand into
//!           the same random CN card, which cost (0); summon {units} random CN Units."
//! Radiant: printed in Chinese (no English form exists).
//! Engine:
//! - **The pick:** one uniform pick of the three effects by the match rng as the Spell resolves
//!   (MD-B9, R920), whatever each would do then — Call to Chaos's single-effect shape.
//! - **Get ready to learn Chinese** (ME-CN) sets `chinese` on every card in both hands and both
//!   libraries, silently (R440); on the Radiant face only the opponent's. As the designer asked,
//!   the printed text never says what it does.
//! - **Transform the hand** draws one definition from `query({ tags: [CN] })`, never this card
//!   (R387), then replaces every card in the caster's hand in its place (R31, R671; an Immutable
//!   hand card too, R35), each costing (0) and Radiant on the Radiant face (MD-B10, R921). Each is
//!   generated into a hand for R673's own Glitch roll.
//! - **Summon** `units` random CN Units (repeats allowed, R60), each into the leftmost empty,
//!   unlocked unit zone (R64); a full row takes fewer, drawing nothing for the rest (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-032";

/// The three effects the match rng picks among as the Spell resolves (MD-B9, R920).
const EFFECTS: i32 = 3;

fn spiritually(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            match ctx.rng.int(EFFECTS) {
                // Get ready to learn Chinese: the flag on every card of both hands and both
                // libraries (the Radiant face: the opponent's only), silently (R440).
                0 => vec![translate(json_as(json!({
                    "scope": {
                        "side": if radiant { "enemy" } else { "any" },
                        "zones": ["hand", "library"],
                    },
                })))],
                // Transform the hand into the same random CN card, each costing (0).
                1 => {
                    let mut args = json!({
                        "scope": { "side": "self", "zones": ["hand"] },
                        "same": true,
                        "query": { "tags": ["CN"] },
                        "costOverride": 0,
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
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face learns only the opponent's cards and makes Radiant cards and Units; the
    // numbers are the same on both faces.
    let base = spiritually(false);
    let radiant = spiritually(true);
    CardScripts { base, radiant }
}

// M #32 Spiritually 中国 — SPEC §8.8 row 32, BUILD M10 row M 32: "One of three by one uniform roll
// as it resolves (MD-B9), whatever the board: (1) every card of both hands and both decks gains the
// `chinese` flag, silently in hands and decks (R440); (2) one random non-token CN definition (never
// this card, R387) replaces every card in your hand in place, Immutable ones too, each costing (0),
// each a generated card for R673's roll (MD-B10); (3) 5 random CN Units summoned per R64, a full row
// taking fewer and drawing nothing for the rest (R129); units reads through `param()`; radiant its
// face is printed in Chinese (MD-B13), the learning reaches only the opponent's hand and deck, and
// the new hand cards and the Units are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SPIRITUALLY: &str = "meditative-032";
    const FILLER: &str = "core-005";
    /// Shipped non-token CN cards the pools may draw (R387 keeps this card out).
    const CN_IDS: [&str; 5] = [
        "core-080",
        "core-090",
        "classicplus-004",
        "classicplus-023",
        "classicplus-076",
    ];

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

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

    /// The roll the match rng would make for a scenario built but not yet played: the cry's first
    /// draw, as c036 predicts its coins.
    fn roll(s: &Scenario) -> i32 {
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        rng.int(EFFECTS)
    }

    /// Find a seed whose roll is `wanted` (at most 64 tries), with the scenario built.
    fn with_roll(wanted: i32, radiant_face: bool) -> (String, Scenario) {
        for n in 0..64 {
            let seed = format!("spiritually-{wanted}-{n}");
            let s = casting(&seed, radiant_face);
            if roll(&s) == wanted {
                return (seed, s);
            }
        }
        panic!("no roll of {wanted} over 64 seeds");
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().map(|card| card.def_id.clone()).collect()
    }

    mod m32_spiritually_zhongguo {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r920_each_effect_occurs_by_one_roll() {
                let mut saw = [false; 3];
                for n in 0..32 {
                    let seed = format!("spiritually-rolls-{n}");
                    let mut s = casting(&seed, false);
                    let effect = roll(&s);
                    saw[effect as usize] = true;
                    s.play(SPIRITUALLY, json!({}));
                    match effect {
                        // The learning: every card of both hands and both decks is Chinese.
                        0 => {
                            for player in [P1, P2] {
                                for card in s.hand(player) {
                                    assert_eq!(card.chinese, Some(true), "{seed}");
                                }
                            }
                        }
                        // The transform: one CN definition for the whole hand, each costing (0).
                        1 => {
                            let defs = hand_defs(&s, P1);
                            assert_eq!(defs.len(), 2, "{seed}");
                            assert!(defs.iter().all(|def| *def == defs[0]), "{seed}");
                            assert!(CN_IDS.contains(&defs[0].as_str()), "{seed}");
                            for card in s.hand(P1) {
                                assert_eq!(card.cost_override, Some(0), "{seed}");
                                assert!(!card.radiant, "{seed}");
                            }
                        }
                        // The summon: 5 CN Units take the empty row.
                        _ => {
                            for lane in 1..=5 {
                                let unit = s.unit(P1, lane).expect("a summoned unit");
                                let def = crate::card_def(&unit.def_id);
                                assert_eq!(def.type_, CardType::Unit, "{seed}");
                                assert!(def.tags.contains(&Tag::Cn), "{seed}");
                                assert!(!unit.radiant, "{seed}");
                            }
                        }
                    }
                }
                assert!(saw.iter().all(|seen| *seen), "all three effects occur over 32 seeds");
            }

            #[test]
            fn r920_learning_translates_both_hands_and_decks() {
                let (seed, mut s) = with_roll(0, false);
                s.play(SPIRITUALLY, json!({}));
                for player in [P1, P2] {
                    assert!(!s.hand(player).is_empty(), "{seed}");
                    for card in s.hand(player) {
                        assert_eq!(card.chinese, Some(true), "{seed}");
                    }
                    let library = &s.state().players[player].library;
                    assert!(!library.is_empty(), "{seed}");
                    for card in library {
                        assert_eq!(card.chinese, Some(true), "{seed} {}", card.id);
                    }
                }
            }

            #[test]
            fn r921_one_cn_definition_replaces_the_hand_in_place_at_0() {
                let (seed, mut s) = with_roll(1, false);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.play(SPIRITUALLY, json!({}));
                let defs = hand_defs(&s, P1);
                // The played Spell left; the other two were replaced in place by one definition.
                assert_eq!(defs.len(), 2, "{seed}");
                assert!(defs.iter().all(|def| *def == defs[0]), "{seed}");
                assert!(CN_IDS.contains(&defs[0].as_str()), "{seed}");
                for card in s.hand(P1) {
                    assert!(!before.contains(&card.id), "{seed}");
                    assert_eq!(card.cost_override, Some(0), "{seed}");
                }
            }

            #[test]
            fn summons_cn_units_and_a_full_row_takes_fewer() {
                let (seed, mut s) = with_roll(2, false);
                s.play(SPIRITUALLY, json!({}));
                let units: Vec<CardInstance> =
                    (1..=5).filter_map(|lane| s.unit(P1, lane)).collect();
                assert_eq!(units.len(), 5, "{seed}");
                for unit in &units {
                    let def = crate::card_def(&unit.def_id);
                    assert!(def.tags.contains(&Tag::Cn), "{seed}");
                }

                // Three zones taken: only two Units arrive.
                let mut full = None;
                for n in 0..64 {
                    crate::register_all();
                    let candidate = scenario(json!({
                        "seed": format!("{seed}-full-{n}"),
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
                    if roll(&candidate) == 2 {
                        full = Some(candidate);
                        break;
                    }
                }
                let mut full = full.expect("a summon roll over 64 seeds");
                full.play(SPIRITUALLY, json!({}));
                let units: Vec<CardInstance> =
                    (1..=5).filter_map(|lane| full.unit(P1, lane)).collect();
                assert_eq!(units.len(), 5, "three stood and two arrive");
            }

            #[test]
            fn units_reads_through_param() {
                let (seed, mut s) = with_roll(2, false);
                set_param(s.card_mut(SPIRITUALLY), "units", 2);
                s.play(SPIRITUALLY, json!({}));
                let units: Vec<CardInstance> =
                    (1..=5).filter_map(|lane| s.unit(P1, lane)).collect();
                assert_eq!(units.len(), 2, "{seed}");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r920_learning_reaches_only_the_opponent() {
                let (seed, mut s) = with_roll(0, true);
                s.play(SPIRITUALLY, json!({}));
                for card in s.hand(P1) {
                    assert_eq!(card.chinese, None, "{seed}");
                }
                assert!(!s.hand(P2).is_empty(), "{seed}");
                for card in s.hand(P2) {
                    assert_eq!(card.chinese, Some(true), "{seed}");
                }
            }

            #[test]
            fn r921_hand_cards_and_units_are_radiant() {
                let (seed, mut s) = with_roll(1, true);
                s.play(SPIRITUALLY, json!({}));
                assert_eq!(hand_defs(&s, P1).len(), 2, "{seed}");
                for card in s.hand(P1) {
                    assert!(card.radiant, "{seed}");
                    assert_eq!(card.cost_override, Some(0), "{seed}");
                }

                let (seed, mut s) = with_roll(2, true);
                s.play(SPIRITUALLY, json!({}));
                for lane in 1..=5 {
                    let unit = s.unit(P1, lane).expect("a summoned unit");
                    assert!(unit.radiant, "{seed}");
                }
            }
        }
    }
}
