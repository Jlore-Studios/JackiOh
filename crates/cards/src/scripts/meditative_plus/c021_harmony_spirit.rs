//! Meditative+ #21 Harmony Spirit (SPEC §8.9 row 21, R1185002, R23, R942, R1136000, R1300). (2) Unit,
//! CN, Rare, 3/7 → 6/14.
//!   Base:    "Taunt / Cry: Set every other Unit's Attack and Health to 3/3."
//!   Radiant: "Taunt / Cry: Set every other Unit's Attack and Health to 3/3. Fill your board with Rush
//!            Tokens in Chinese."
//!
//! THE SET (R1185002) is Hearthstone's: every other Unit of both sides (`set_stats_all`, the tops of
//! piles, carried and standing Units included) wears 3/3 as its §10.4 layer 1 (`statsOverride`), its
//! buffs, the stat half of its tuning and its damage gone, auras still applying on top; an Immutable
//! Unit is untouched (R23). The 3/3 is the frame, not a declared number: it is good for one side and
//! bad for the other, so no `better` direction fits it.
//! THE FILL (Radiant face) comes after the set: Rush Tokens at their printed 3/3 in every empty zone of
//! yours, a Locked one included (`fill_board`, R942, R1136000), each in Chinese (the `chinese` rider,
//! R1300), so CN once #1176 makes every card in Chinese CN (`query::tags_of`).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-021";

/// "3/3": the Attack and Health the Cry sets, the frame of the card (no declared number).
const HARMONY_STATS: i32 = 3;
/// The Rush Token (§7).
const RUSH_TOKEN: &str = "core-t-rush";

fn set_every_other_unit() -> Effect {
    set_stats_all(json_as(json!({
        "side": "any",
        "excludeSelf": true,
        "attack": HARMONY_STATS,
        "health": HARMONY_STATS,
    })))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![set_every_other_unit()])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    set_every_other_unit(),
                    fill_board(json_as(json!({ "defId": RUSH_TOKEN, "chinese": true }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// Meditative+ #21 Harmony Spirit — SPEC §8.9 row 21, BUILD M11 row M+ 21: "Cry: every other Unit of
// both sides is 3/3 undamaged, a buffed one losing its buffs, an aura still applying on top; an
// Immutable Unit is untouched; Harmony Spirit itself stays 3/7; radiant 6/14, then each empty zone of
// yours gets a Rush Token in Chinese, CN (once #1176 lands), at 3/3".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    /// Core #11 Tempo Timmy, a 3/3 (any body to set).
    const TIMMY: &str = "core-011";
    /// Core #8 Mr. Vanilla, a 4/4.
    const BIG: &str = "core-008";
    /// Core #19 Midrange Menace, whose Radiant face is an Immutable 18/18.
    const IMMUTABLE: &str = "core-019";
    /// C+ #46 Felinor Flagbearer: "Aura: Your other Felinors have +{aura}/+{aura}."
    const FLAGBEARER: &str = "classicplus-046";
    /// Core #12 Duplicating Felinors, a 3/4 Felinor.
    const FELINOR: &str = "core-012";

    fn spirit(seed: &str, radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": p1_field, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": p2_field, "library": [FILLER, FILLER] },
        }))
    }

    fn stats(s: &Scenario, card: &CardInstance) -> (i32, i32, i32) {
        let view = s.stats(card);
        (view.attack, view.max_health, view.health)
    }

    fn played(s: &Scenario) -> CardInstance {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|card| card.def_id == ID)
            .expect("Harmony Spirit on the field")
    }

    mod base {
        use super::*;

        #[test]
        fn r1185002_every_other_unit_of_both_sides_is_3_3_undamaged_and_unbuffed() {
            let mut s = spirit(
                "harmony",
                false,
                json!([{ "def": BIG, "lane": 1, "damage": 2 }]),
                json!([{ "def": TIMMY, "lane": 1 }, { "def": BIG, "lane": 2 }]),
            );
            let mine = s.unit(P1, 1).expect("p1's body");
            let small = s.unit(P2, 1).expect("p2's 3/3");
            let theirs = s.unit(P2, 2).expect("p2's body");
            s.card_mut(&small).buffs = AttackHealth { attack: 4, health: 4 };
            s.play(ID, json!({ "zone": 2 }));
            for card in [&mine, &small, &theirs] {
                assert_eq!(stats(&s, card), (3, 3, 3), "{}", card.def_id);
                assert_eq!(s.card(card).damage, 0);
                assert_eq!(s.card(card).buffs, AttackHealth { attack: 0, health: 0 });
            }
            assert_eq!(stats(&s, &played(&s)), (3, 7, 7), "Harmony Spirit itself stays 3/7");
        }

        #[test]
        fn r23_an_immutable_unit_is_untouched() {
            let mut s = spirit(
                "harmony-immutable",
                false,
                json!([]),
                json!([{ "def": IMMUTABLE, "radiant": true, "lane": 1 }]),
            );
            let statue = s.unit(P2, 1).expect("the Immutable Unit");
            let before = stats(&s, &statue);
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(stats(&s, &statue), before);
            assert_eq!(s.card(&statue).stats_override, None);
        }

        #[test]
        fn r1185002_an_aura_still_applies_on_top() {
            let mut s = spirit(
                "harmony-aura",
                false,
                json!([{ "def": FLAGBEARER, "lane": 1 }, { "def": FELINOR, "lane": 2 }]),
                json!([]),
            );
            let felinor = s.unit(P1, 2).expect("the Felinor");
            let aura = param_value(s.state(), s.unit(P1, 1).as_ref(), "aura", ParamValueOptions::default());
            s.play(ID, json!({ "zone": 3 }));
            assert_eq!(stats(&s, &felinor), (3 + aura, 3 + aura, 3 + aura));
            assert_eq!(stats(&s, &s.unit(P1, 1).expect("the Flagbearer")), (3, 3, 3));
        }

        #[test]
        fn the_base_face_fills_nothing() {
            let mut s = spirit("harmony-nofill", false, json!([]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!((2..=5).all(|lane| s.unit(P1, lane).is_none()));
        }

        #[test]
        fn declares_no_numbers() {
            assert!(crate::card_def(ID).params.is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1185002_sets_every_other_unit_then_fills_your_board_with_rush_tokens_in_chinese_at_3_3() {
            let mut s = spirit(
                "harmony-radiant",
                true,
                json!([{ "def": BIG, "lane": 1 }]),
                json!([{ "def": BIG, "lane": 1 }]),
            );
            s.play(ID, json!({ "zone": 2 }));
            let me = played(&s);
            assert_eq!(stats(&s, &me), (6, 14, 14), "the Radiant Spirit stays 6/14");
            assert_eq!(stats(&s, &s.unit(P1, 1).expect("p1's body")), (3, 3, 3));
            assert_eq!(stats(&s, &s.unit(P2, 1).expect("p2's body")), (3, 3, 3));
            for lane in 3..=5 {
                let token = s.unit(P1, lane).expect("a Rush Token");
                assert_eq!(token.def_id, RUSH_TOKEN);
                assert!(!token.radiant);
                assert_eq!(token.chinese, Some(true), "in Chinese");
                assert_eq!(stats(&s, &token), (3, 3, 3), "the tokens arrive after the set, at their printed 3/3");
                // CN: holds once #1176 lands (every card in Chinese is CN, `query::tags_of`).
                assert!(tags_of(s.state(), &token).contains(&Tag::Cn), "a Rush Token in Chinese is CN");
            }
            assert!((1..=5).all(|lane| s.unit(P2, lane).is_none_or(|card| card.def_id == BIG)), "the opponent's side is not filled");
        }

        #[test]
        fn r1136000_a_locked_zone_of_yours_takes_a_token_too() {
            let mut s = spirit("harmony-locked", true, json!([]), json!([]));
            s.state_mut().players.p1.locks.units[4] = true;
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.unit(P1, 5).expect("a Rush Token in the Locked zone").def_id, RUSH_TOKEN);
        }
    }
}
