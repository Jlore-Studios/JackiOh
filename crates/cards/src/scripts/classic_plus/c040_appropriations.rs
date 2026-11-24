//! C+ #40 Appropriations (SPEC §8.7 row 40, E38, E39, R58, R60, R80, R81, R177, R311, R348, R380, R581,
//! R3135002, R3563000, R3996201, R4154105, R4666000). (X) Spell, Epic. Choose one, with the play (R81); X is
//! at least 1 (R348); no number below 1; ⌊ ⌋ rounds down:
//!   Military:   your Units on the field and in your hand get +(2X−2) Attack and Rush.
//!   Education:  shuffle ⌊5X/3⌋−2 random Radiant Books into your deck; they have Cast on draw and aim at
//!               enemies when they harm and at your side when they help.
//!   Culture:    each card on your field, in your hand and in your deck has a 6X% chance to become Radiant.
//!   Healthcare: your Units on the field and in your hand get +(2X−2) Health and Armor X.
//!   Radiant:    +(4X−3) Attack; ⌊17X/5⌋−2 Books; 16X%; +(5X−1) Health and Armor 2X.
//! Issue #4666 took Military and Healthcare off the deck and gave them and Culture back the numbers #3996
//! and #4154 took, Education unchanged (R4666000); `script_v0_6_12b` keeps the script before it, Military
//! and Healthcare on the deck too at +(⌊5X/3⌋−2) [+(⌊17X/5⌋−4)] and +(⌊5X/3⌋−2) [+⌊17X/5⌋], Culture
//! ⌊17X/4⌋% [12X%]. Issue #4154 had cut each about 15% again, the Armor kept (R4154105; `script_v0_6_6h`
//! keeps the numbers before it, +(2X−3) [+(4X−5)], 2X−3 [4X−3], 5X% [14X%] and +(2X−3) [+4X]), and issue
//! #3996 about 15% at X = 4 (R3996201; `script_v0_6_5b` keeps the numbers before it, +(2X−2) [+(4X−3)],
//! 2X−2 [4X−1], 6X% [16X%] and +(2X−2) [+(5X−1)]).
//!
//! Military and Healthcare are E38's buffs and grants over your field (tops of piles) and hand, a hand
//! card's carried onto the field as it enters, silent where a card is hidden (R440). Education's Books are
//! non-token Books of every set (R380), unique until the pool runs out (R3563000), each Radiant and
//! enchanted (E39), R80's cap turning the rest away. Culture is `radiantChance`'s roll per card, #42
//! Selective Breeding's reading (R177, R581): every card is rolled, a Radiant one too, so neither the draws
//! nor the cues count the hidden Radiant cards; a library card made Radiant is listed as it went in (R311).
//! Tunes: none beyond X.

use jackioh_engine::effects::{
    buff_cards, chosen_options, grant_keyword_cards, radiant_chance, shuffle_random_from_catalog,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-040";

const MODES: &[&str] = &["Military", "Education", "Culture", "Healthcare"];

/// R4666000: the zones Military and Healthcare reach, your field and your hand.
const REACH: &[&str] = &["field", "hand"];
/// R767000: their reach through v0.6.12b, your deck too.
const REACH_V0_6_12B: &[&str] = &["field", "hand", "library"];

/// One number of a face: `per_x` times X over `over`, rounded down, less `less` (`at`).
type PerX = (i32, i32, i32);

/// One face's numbers: attack, Books, percent, health, Armor.
#[derive(Clone, Copy)]
struct Face {
    attack: PerX,
    books: PerX,
    percent: PerX,
    health: PerX,
    armor: PerX,
}

/// R4666000: 2X−2 [4X−3], ⌊5X/3⌋−2 [⌊17X/5⌋−2], 6X% [16X%], 2X−2 [5X−1], X [2X]: v0.6.5b's Attack,
/// percent and Health (R3996201) with v0.6.12b's Books and the Armor of every version.
const BASE: Face = Face { attack: (2, 1, 2), books: (5, 3, 2), percent: (6, 1, 0), health: (2, 1, 2), armor: (1, 1, 0) };
const RADIANT: Face = Face { attack: (4, 1, 3), books: (17, 5, 2), percent: (16, 1, 0), health: (5, 1, 1), armor: (2, 1, 0) };
/// R767000: the numbers through v0.6.12b, before #4666 (R4154105): ⌊5X/3⌋−2 [⌊17X/5⌋−4], ⌊5X/3⌋−2
/// [⌊17X/5⌋−2], ⌊17X/4⌋% [12X%], ⌊5X/3⌋−2 [⌊17X/5⌋], X [2X].
const BASE_V0_6_12B: Face = Face { attack: (5, 3, 2), books: (5, 3, 2), percent: (17, 4, 0), health: (5, 3, 2), armor: (1, 1, 0) };
const RADIANT_V0_6_12B: Face = Face { attack: (17, 5, 4), books: (17, 5, 2), percent: (12, 1, 0), health: (17, 5, 0), armor: (2, 1, 0) };
/// R767000: the numbers through v0.6.6h, before #4154's nerf (R3996201).
const BASE_V0_6_6H: Face = Face { attack: (2, 1, 3), books: (2, 1, 3), percent: (5, 1, 0), health: (2, 1, 3), armor: (1, 1, 0) };
const RADIANT_V0_6_6H: Face = Face { attack: (4, 1, 5), books: (4, 1, 3), percent: (14, 1, 0), health: (4, 1, 0), armor: (2, 1, 0) };
/// R767000: the numbers through v0.6.5b, before #3996's nerf (R3996201).
const BASE_V0_6_5B: Face = Face { attack: (2, 1, 2), books: (2, 1, 2), percent: (6, 1, 0), health: (2, 1, 2), armor: (1, 1, 0) };
const RADIANT_V0_6_5B: Face = Face { attack: (4, 1, 3), books: (4, 1, 1), percent: (16, 1, 0), health: (5, 1, 1), armor: (2, 1, 0) };
/// R767000: the numbers through v0.5.11r, before #3135's nerf.
const BASE_V0_5_11O: Face = Face { attack: (2, 1, 1), books: (2, 1, 1), percent: (7, 1, 0), health: (2, 1, 1), armor: (1, 1, 0) };
const RADIANT_V0_5_11O: Face = Face { attack: (4, 1, 2), books: (4, 1, 0), percent: (17, 1, 0), health: (5, 1, 0), armor: (2, 1, 0) };
/// R767000: the numbers through v0.4.0f, before #1704's nerf.
const BASE_V0_4_0F: Face = Face { attack: (2, 1, 0), books: (2, 1, 0), percent: (8, 1, 0), health: (2, 1, 0), armor: (1, 1, 0) };
const RADIANT_V0_4_0F: Face = Face { attack: (4, 1, 0), books: (5, 1, 0), percent: (20, 1, 0), health: (6, 1, 0), armor: (2, 1, 0) };
/// R767000: the numbers through patch v0.3.7b, before #1149's nerf.
const BASE_V0_3_7B: Face = Face { attack: (2, 1, 0), books: (2, 1, 0), percent: (10, 1, 0), health: (2, 1, 0), armor: (1, 1, 0) };
const RADIANT_V0_3_7B: Face = Face { attack: (5, 1, 0), books: (5, 1, 0), percent: (25, 1, 0), health: (7, 1, 0), armor: (2, 1, 0) };
const ALL: i32 = 100;

/// A face's number at `x`: "2X−3" is `(2, 1, 3)` and "⌊5X/3⌋−2" `(5, 3, 2)`, the division rounding down
/// (R4154105). No number is below 1 (R3135002): at X = 1 the base face's "2X−2" reads 1, not 0. The
/// floor has stood since #3135, so it reads v0.6.5b's and v0.6.6h's faces as they ran (their "2X−2" and
/// "2X−3" at X = 1 read 1 then too); no older kept face comes near it, so it moves none of their numbers.
fn at((per_x, over, less): PerX, x: i32) -> i32 {
    (per_x * x / over - less).max(1)
}

/// Your Units in the `reach` zones (an E38 card scope).
fn units(reach: &[&str]) -> Value {
    json!({ "side": "self", "zones": reach, "rows": ["units"], "types": ["Unit"] })
}

/// The card's script for one face's numbers, Military and Healthcare reaching the `reach` zones.
fn appropriations(face: Face, reach: &'static [&'static str]) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: MODES.iter().map(|mode| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let x = ctx.x;
            match chosen_options(ctx).first().map(String::as_str) {
                Some("Military") => vec![
                    buff_cards(json_as(json!({ "scope": units(reach), "attack": at(face.attack, x) }))),
                    grant_keyword_cards(json_as(json!({ "scope": units(reach), "keyword": { "kind": "Rush" } }))),
                ],
                Some("Education") => vec![shuffle_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Book"] },
                    "count": at(face.books, x),
                    "radiant": true,
                    "enchantments": [{ "kind": "castOnDraw" }, { "kind": "targetEnemies" }],
                })))],
                Some("Culture") => vec![radiant_chance(json_as(json!({
                    "zone": ["field", "hand", "library"],
                    "chance": f64::min(1.0, f64::from(at(face.percent, x)) / f64::from(ALL)),
                })))],
                Some("Healthcare") => vec![
                    buff_cards(json_as(json!({ "scope": units(reach), "health": at(face.health, x) }))),
                    grant_keyword_cards(json_as(json!({
                        "scope": units(reach),
                        "keyword": { "kind": "Armor", "n": at(face.armor, x) },
                    }))),
                ],
                _ => vec![],
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: appropriations(BASE, REACH),
        radiant: appropriations(RADIANT, REACH),
    }
}

/// R767000: the scripts v0.6.12b and the versions before it back to the one after v0.6.6h's ran,
/// Military and Healthcare on your deck too at +(⌊5X/3⌋−2) Attack [+(⌊17X/5⌋−4)] and +(⌊5X/3⌋−2) Health
/// [+⌊17X/5⌋], Culture ⌊17X/4⌋% [12X%], each at least 1 (R3135002); `KEPT_SCRIPTS` folds those versions'
/// replays under it (R4666000).
pub fn script_v0_6_12b() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_6_12B, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_6_12B, REACH_V0_6_12B),
    }
}

/// R767000: the scripts v0.6.6h and the versions before it back to the one after v0.6.5b's ran, +(2X−3)
/// Attack [+(4X−5)], 2X−3 Books [4X−3], 5X% [14X%] and +(2X−3) Health [+4X], each at least 1 (R3135002);
/// `KEPT_SCRIPTS` folds those versions' replays under it (R4154105).
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_6_6H, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_6_6H, REACH_V0_6_12B),
    }
}

/// R767000: the scripts v0.6.5b and the versions before it back to the one after v0.5.11r's ran, +(2X−2)
/// Attack [+(4X−3)], 2X−2 Books [4X−1], 6X% [16X%] and +(2X−2) Health [+(5X−1)], each at least 1
/// (R3135002); `KEPT_SCRIPTS` folds those versions' replays under it (R3996201).
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_6_5B, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_6_5B, REACH_V0_6_12B),
    }
}

/// R767000: the scripts v0.5.11r and the versions before it back to the one after v0.4.0f's ran, +(2X−1)
/// Attack [+(4X−2)], 2X−1 Books [4X], 7X% [17X%] and +(2X−1) Health [+5X]; `KEPT_SCRIPTS` folds those
/// versions' replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_5_11O, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_5_11O, REACH_V0_6_12B),
    }
}

/// R767000: the scripts v0.4.0f and the versions before it back to `script_v0_3_7b`'s ran, +2X
/// Attack [+4X], 2X Books [5X], 8X% [20X%] and +2X Health [+6X]; `KEPT_SCRIPTS` folds those versions'
/// replays under them.
pub fn script_v0_4_0f() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_4_0F, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_4_0F, REACH_V0_6_12B),
    }
}

/// R767000: the scripts v0.3.7b and every version before it ran, Culture at 10X% [25X%], the Radiant's
/// Military at +5X Attack and its Healthcare at +7X Health; `KEPT_SCRIPTS` folds those versions'
/// replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    CardScripts {
        base: appropriations(BASE_V0_3_7B, REACH_V0_6_12B),
        radiant: appropriations(RADIANT_V0_3_7B, REACH_V0_6_12B),
    }
}

// C+ #40 Appropriations — SPEC §8.7 row 40, E38, E39, R60, R80, R81, R97, R177, R311, R348, R380,
// R440, R581, R3135002, R3996201, R4154105, R4666000, BUILD M9 row C+ 40.
#[cfg(test)]
mod tests {
    use crate::events_json;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const APPROPRIATIONS: &str = "classicplus-040";
    const TIMMY: &str = "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MENACE: &str = "core-019"; // Midrange Menace: Taunt
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7 R14/14 Armor 7, Reborn";
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: a Spell
    const FILLER: &str = "core-005";
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack

    use crate::scenario;

    use crate::merged;

    fn appropriations(radiant: bool, p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": APPROPRIATIONS, "radiant": radiant }, VANILLA, ECLIPSE],
                    "field": [TIMMY],
                    "library": [MENACE, FILLER],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [VANILLA], "field": [TIMMY], "library": [FILLER] }), p2),
        }))
    }

    fn mine(s: &Scenario, def_id: &str, zone: &str) -> CardInstance {
        match s.pile(P1, zone).into_iter().find(|held| held.def_id == def_id) {
            Some(card) => card,
            None => panic!("{def_id} in p1's {zone}"),
        }
    }

    fn kind_of(keyword: &Keyword) -> String {
        js(keyword)["kind"].as_str().unwrap_or_default().to_string()
    }

    fn keywords_of(s: &Scenario, card: &CardInstance) -> Vec<String> {
        s.stats(card).keywords.iter().map(kind_of).collect()
    }

    fn unit(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane).expect("a unit in that lane")
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        js(&s.view(seat))
    }

    fn view_events(s: &Scenario, seat: PlayerId) -> Vec<Value> {
        view(s, seat)["events"].as_array().cloned().unwrap_or_default()
    }

    fn listed(s: &Scenario) -> Vec<Value> {
        view(s, P1)["you"]["ownLibrary"]["cards"].as_array().cloned().unwrap_or_default()
    }

    fn listed_count(entries: &[Value]) -> i64 {
        entries.iter().map(|entry| entry["count"].as_i64().unwrap_or(0)).sum()
    }

    #[test]
    fn r81_declares_its_four_modes_with_the_play_on_both_faces() {
        let def = crate::card_def(APPROPRIATIONS);
        assert_eq!(def.id, APPROPRIATIONS);
        assert_eq!(js(&def.cost), json!("X"));
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                js(&face.modes),
                json!([{ "kind": "mode", "options": ["Military", "Education", "Culture", "Healthcare"] }])
            );
        }
    }

    #[test]
    fn r348_x_is_at_least_1() {
        let mut s = appropriations(false, json!({}), json!({}));
        s.expect_refused_with(|s| s.play(APPROPRIATIONS, json!({ "x": 0, "modes": ["Military"] })), "X");
    }

    #[test]
    fn r767000_a_game_on_v0_3_7_folds_under_the_kept_script_radiant_5x_attack_and_7x_health() {
        crate::register_all();
        let shipped = crate::shipped_catalog("v0.3.0c").expect("v0.3.0c is carried");
        jackioh_engine::with_shipped(Some(shipped), || {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 10);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 10 }));
        });
    }

    #[test]
    fn r767000_the_kept_v0_4_0f_script_gives_2x_attack_5x_radiant_books_and_6x_radiant_health() {
        crate::under_kept(APPROPRIATIONS, super::script_v0_4_0f(), || {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 4);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 8);

            let mut s = appropriations(true, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 10);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 9 }));
        });
    }

    #[test]
    fn r767000_r3996201_the_kept_v0_6_5b_script_gives_2x_less_2_attack_4x_less_1_radiant_books_and_5x_less_1_radiant_health() {
        crate::under_kept(APPROPRIATIONS, super::script_v0_6_5b(), || {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 6);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 13);

            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 6);

            let mut s = appropriations(true, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 15);

            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Healthcare"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.health, 6);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Healthcare"] }));
            let vanilla = mine(&s, VANILLA, "hand");
            assert_eq!(vanilla.buffs.health, 19);
            assert_eq!(vanilla.granted_keywords, vec![Keyword::Armor { n: 8 }]);
        });
        let (base, radiant) = (super::BASE_V0_6_5B, super::RADIANT_V0_6_5B);
        assert_eq!((super::at(base.percent, 4), super::at(radiant.percent, 4)), (24, 64));
        // The floor reads its 2X−2 at X = 1 as 1, as v0.6.5b did.
        assert_eq!(super::at(base.attack, 1), 1);
    }

    #[test]
    fn r767000_r3135002_the_kept_v0_5_11o_script_gives_2x_less_1_attack_4x_radiant_books_7x_and_17x_percent() {
        crate::under_kept(APPROPRIATIONS, super::script_v0_5_11o(), || {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 3);

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 6);

            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 3);

            let mut s = appropriations(true, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 8);

            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 6 }));

            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 8 }));
        });
    }

    mod military {
        use super::*;

        #[test]
        fn e38_r4666000_your_units_on_the_field_and_in_your_hand_get_2x_less_2_attack_and_rush_your_deck_spells_and_the_opponents_units_do_not() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Military"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 7, "health": 3 }));
            assert!(keywords_of(&s, &timmy).contains(&"Rush".to_string()));
            let held = mine(&s, VANILLA, "hand");
            assert_eq!(held.buffs.attack, 4);
            assert!(held.granted_keywords.iter().map(kind_of).any(|kind| kind == "Rush"));
            let deck = mine(&s, MENACE, "library");
            assert_eq!(deck.buffs.attack, 0);
            assert!(deck.granted_keywords.is_empty());
            assert_eq!(mine(&s, ECLIPSE, "hand").buffs.attack, 0);
            assert_eq!(mine(&s, FILLER, "library").buffs.attack, 0);
            let theirs = unit(&s, P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 3 }));
        }

        #[test]
        fn r4453000_military_passes_your_immune_field_unit_by_and_reaches_your_immune_hand_unit() {
            // §6.1: Immune to Spells guards a Unit on the field only, so a copy in a hand is reached.
            let mut s = appropriations(false, json!({}), json!({}));
            let timmy = unit(&s, P1, 1).id;
            let held = mine(&s, VANILLA, "hand").id;
            for id in [&timmy, &held] {
                s.card_mut(id.as_str()).granted_keywords.push(Keyword::ImmuneToSpells);
            }
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Military"] }));
            s.expect_stats(timmy.as_str(), json!({ "attack": 3, "health": 3 }));
            assert_eq!(s.card(timmy.as_str()).buffs.attack, 0);
            assert!(!s.card(timmy.as_str()).granted_keywords.iter().map(kind_of).any(|kind| kind == "Rush"));
            let held = s.card(held.as_str()).clone();
            assert_eq!(held.buffs.attack, 4);
            assert!(held.granted_keywords.iter().map(kind_of).any(|kind| kind == "Rush"));
        }

        #[test]
        fn r13_a_unit_dormant_under_a_stack_is_not_on_the_field_only_the_piles_top_is_buffed() {
            let mut s = appropriations(false, json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), json!({}));
            let buried = s.state().players.p1.units[0].as_ref().expect("a pile in lane 1")[1].clone();
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            assert_eq!(buried.def_id, VANILLA);
            assert_eq!(s.card(&buried).buffs.attack, 0);
            assert!(s.card(&buried).granted_keywords.is_empty());
            assert_eq!(s.card(FIENDER).buffs.attack, 1);
        }

        /// R1081000: a Unit standing in the backrow is one of "your Units on the field", buffed once.
        #[test]
        fn r1081000_a_unit_standing_in_the_backrow_is_one_of_your_units_on_the_field() {
            let mut s = appropriations(false, json!({}), json!({}));
            let timmy = unit(&s, P1, 1).id;
            crate::stand_in_backrow(&mut s, &timmy, 2);
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Military"] }));
            assert_eq!(s.card(timmy.as_str()).buffs.attack, 4);
            s.expect_stats(timmy.as_str(), json!({ "attack": 7, "health": 3 }));
            assert!(keywords_of(&s, s.card(timmy.as_str())).contains(&"Rush".to_string()));
        }

        #[test]
        fn e38_r3135002_a_hand_unit_carries_the_buff_and_rush_onto_the_field_as_it_enters() {
            let mut s = appropriations(false, json!({}), json!({}));
            // 2X−2 is 0 at X = 1: the floor of 1 (R3135002).
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            s.play(VANILLA, json!({ "zone": 2 }));
            let vanilla = unit(&s, P1, 2);
            s.expect_stats(&vanilla, json!({ "attack": 5, "health": 4 }));
            assert!(keywords_of(&s, &vanilla).contains(&"Rush".to_string()));
        }

        #[test]
        fn r97_r440_hand_changes_are_hidden_from_the_opponent_the_deck_is_listed_to_its_owner_as_it_was_r311() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            let hidden = [mine(&s, VANILLA, "hand").id];
            let named = |events: &[Value]| -> Vec<String> {
                events
                    .iter()
                    .filter(|event| event["type"] == "buffed" || event["type"] == "keywordGranted")
                    .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                    .collect()
            };
            let theirs = named(&view_events(&s, P2));
            for id in &hidden {
                assert!(!theirs.contains(id));
            }
            assert!(listed(&s).contains(&json!({ "defId": MENACE, "radiant": false, "count": 1 })));
        }

        /// R4666000: 4X−3 at X = 2 is 5 (⌊17X/5⌋−4's 2 through v0.6.12b).
        #[test]
        fn r4666000_radiant_4x_less_3_attack_and_rush() {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 8 }));
            assert!(keywords_of(&s, &timmy).contains(&"Rush".to_string()));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 5);
            assert_eq!(mine(&s, MENACE, "library").buffs.attack, 0);
        }

        /// R767000, R4154105: through v0.6.12b the Radiant face's ⌊17X/5⌋−4 at X = 2 was 2, and it reached
        /// the deck too, Rush and all.
        #[test]
        fn r767000_r4154105_the_kept_v0_6_12b_script_gives_radiant_17x_over_5_less_4_attack_and_rush_to_the_deck_too() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_12b(), || {
                let mut s = appropriations(true, json!({}), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
                let timmy = unit(&s, P1, 1);
                s.expect_stats(&timmy, json!({ "attack": 5 }));
                assert!(keywords_of(&s, &timmy).contains(&"Rush".to_string()));
                for card in [mine(&s, VANILLA, "hand"), mine(&s, MENACE, "library")] {
                    assert_eq!(card.buffs.attack, 2);
                    assert!(card.granted_keywords.iter().map(kind_of).any(|kind| kind == "Rush"));
                }
            });
        }

        /// R767000, R3996201: through v0.6.6h the Radiant face's 4X−5 at X = 2 was 3.
        #[test]
        fn r767000_r3996201_the_kept_v0_6_6h_script_gives_radiant_4x_less_5_attack() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_6h(), || {
                let mut s = appropriations(true, json!({}), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
                s.expect_stats(unit(&s, P1, 1), json!({ "attack": 6 }));
                assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 3);
            });
        }
    }

    mod education {
        use super::*;

        #[test]
        fn e39_r380_r3996201_shuffles_2x_less_3_random_non_token_books_into_your_deck_radiant_with_cast_on_draw_and_target_enemies_riding_them() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Education"] }));
            let library = s.pile(P1, "library");
            assert_eq!(library.len(), 3);
            for card in &library {
                assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Book));
                assert!(!crate::card_def(&card.def_id).token);
                assert!(card.radiant);
                assert!(has_enchantment(card, EnchantmentKind::CastOnDraw));
                assert!(has_enchantment(card, EnchantmentKind::TargetEnemies));
            }
        }

        #[test]
        fn r656_every_books_target_declaration_says_which_it_aims_help_for_heal_and_buff_harm_by_default() {
            let scripts = crate::scripts_of();
            let aimed = |id: &str| -> Vec<Option<TargetAim>> {
                scripts[id]
                    .base
                    .targets
                    .iter()
                    .map(|decl| if decl.kind == PromptKind::Target { decl.aim } else { None })
                    .collect()
            };
            assert_eq!(aimed("classic-003"), vec![Some(TargetAim::Help)]); // Book of Heal
            assert_eq!(aimed("classicplus-057"), vec![Some(TargetAim::Help)]); // Book of Stats
            assert_eq!(aimed("classicplus-071"), vec![Some(TargetAim::Help)]); // Book of Buff
            assert_eq!(aimed("classicplus-010"), vec![Some(TargetAim::Help)]); // New Wraps
            // R3869003: the harmful Books declare the harm an unset aim already read as.
            assert_eq!(aimed("classic-016"), vec![Some(TargetAim::Harm)]); // Book of Flame
            assert_eq!(aimed("classic-012"), vec![Some(TargetAim::Harm)]); // Book of Blood
            assert_eq!(aimed("classic-055"), vec![Some(TargetAim::Harm)]); // Book of Wildfire
            assert_eq!(aimed("classicplus-072"), vec![Some(TargetAim::Harm)]); // Book of Nerf
            assert_eq!(aimed("classic-029"), vec![None]); // Book of Vital Kill, harm by default
        }

        #[test]
        fn r656_an_education_book_that_helps_aims_friends_a_drawn_book_of_heal_offers_only_your_side() {
            let mut s = scenario(json!({
                "seed": "appropriations-aim",
                "p1": { "hand": [VANILLA], "field": [TIMMY], "library": ["classic-003"] },
                "p2": { "hand": [VANILLA], "field": [TIMMY], "library": [FILLER] },
            }));
            let Some(book) = s.pile(P1, "library").into_iter().find(|card| card.def_id == "classic-003") else {
                panic!("Book of Heal in p1's library");
            };
            find_instance_mut(s.state_mut(), &book.id).expect("the Book").enchantments =
                Some(json_as(json!([{ "kind": "castOnDraw" }, { "kind": "targetEnemies" }])));
            let timmy = unit(&s, P1, 1);
            find_instance_mut(s.state_mut(), &timmy.id).expect("Timmy").damage = 1;
            s.end_turn().end_turn();
            // The draw casts the Book, which asks its caster for a target aimed at friends.
            let pending = s.state().pending.clone();
            assert_eq!(pending.as_ref().map(|prompt| prompt.kind), Some(PromptKind::Target));
            let selections: Vec<Selection> = pending
                .as_ref()
                .map(|prompt| prompt.options.iter().map(|option| option.selection.clone()).collect())
                .unwrap_or_default();
            assert!(!selections.is_empty());
            for selection in &selections {
                match selection {
                    Selection::Hero { player } => assert_eq!(*player, P1),
                    Selection::Instance { instance_id } => assert_eq!(*instance_id, timmy.id),
                    other => panic!("unexpected pick {other:?}"),
                }
            }
            let key = pending.and_then(|prompt| prompt.options.first().map(|option| option.key.clone())).unwrap_or_default();
            s.answer(json!(key));
            assert!(events_json(&s).iter().any(|event| event["type"] == "healed" && event["targetId"] == timmy.id.as_str()));
        }

        #[test]
        fn r311_the_shuffle_in_is_open_to_its_owner_the_deck_list_shows_the_radiant_books_the_opponent_sees_a_count() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Education"] }));
            let entries = listed(&s);
            assert_eq!(listed_count(&entries), 3);
            for entry in &entries {
                assert_eq!(entry["radiant"], true);
            }
            let theirs_view = view(&s, P2);
            assert_eq!(theirs_view["opponent"]["libraryCount"], 3);
            assert!(theirs_view["opponent"].get("ownLibrary").is_none());
            // R97: which Books went in is the owner's to know, not the opponent's.
            let theirs = serde_json::to_string(&theirs_view).expect("a view is JSON");
            for card in s.pile(P1, "library") {
                assert!(!theirs.contains(&card.def_id));
            }
        }

        #[test]
        fn e39_r58_a_book_it_made_is_cast_as_it_is_drawn_its_enchantment_riding_it_from_the_deck() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({ "library": [FILLER, FILLER] }));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Education"] }));
            let books: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            s.end_turn().end_turn();
            // A Book that asks for a target asks its caster; any answer will do here.
            let mut n = 0;
            while n < 6 {
                let Some(pending) = s.state().pending.clone() else {
                    break;
                };
                let key = pending.options.first().map(|option| option.key.clone()).unwrap_or_default();
                s.answer(json!(key));
                n += 1;
            }
            let mut cast: Vec<String> = events_json(&s)
                .iter()
                .filter(|event| event["type"] == "cardPlayed")
                .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                .filter(|id| books.contains(id))
                .collect();
            // The turn's draw casts one; a Book that draws casts the next as it comes (R58's chain).
            let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            let mut drawn: Vec<String> = books.iter().filter(|id| !library.contains(id)).cloned().collect();
            assert!(!drawn.is_empty());
            cast.sort();
            drawn.sort();
            assert_eq!(cast, drawn);
            assert!(!s.hand(P1).iter().any(|card| books.contains(&card.id)));
        }

        #[test]
        fn r80_a_full_deck_turns_the_rest_away() {
            let library: Vec<&str> = (0..LIBRARY_CAP - 1).map(|_| FILLER).collect();
            let mut s = appropriations(false, json!({ "library": library }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
            // Three Books, one room: two turned away.
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "libraryOverflow").count(), 2);
        }

        /// R4154105: ⌊17X/5⌋−2 at X = 2 is 4 (4X−3's 5 through v0.6.6h).
        #[test]
        fn r4154105_radiant_17x_over_5_less_2_books() {
            let mut s = appropriations(true, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 4);
        }
    }

    mod culture {
        use super::*;

        #[test]
        fn r60_r3996201_each_card_on_your_field_in_your_hand_and_in_your_deck_rolls_its_own_5x_over_seeds_some_turn_and_some_do_not() {
            let mut turned = 0;
            let mut stayed = 0;
            for n in 0..10 {
                let mut s = scenario(json!({
                    "seed": format!("culture-{n}"),
                    "p1": { "hand": [APPROPRIATIONS, VANILLA, ECLIPSE], "field": [TIMMY], "library": [MENACE, FILLER] },
                    "p2": { "hand": [VANILLA], "field": [TIMMY] },
                }));
                s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Culture"] }));
                let mut cards = s.hand(P1);
                cards.extend(s.pile(P1, "library"));
                cards.push(unit(&s, P1, 1));
                for card in &cards {
                    if card.radiant {
                        turned += 1;
                    } else {
                        stayed += 1;
                    }
                }
                assert_eq!(s.unit(P2, 1).map(|card| card.radiant), Some(false));
                assert_eq!(s.hand(P2).first().map(|card| card.radiant), Some(false));
            }
            assert!(turned > 0);
            assert!(stayed > 0);
        }

        /// Culture at `x` on seed `n`: how many of p1's five cards (field 1, hand 2, deck 2) turned
        /// Radiant, and how many stayed.
        fn culture(radiant: bool, x: i32, n: u32) -> (usize, usize) {
            let mut s = scenario(json!({
                "seed": format!("culture-step-{n}"),
                "p1": {
                    "hand": [{ "def": APPROPRIATIONS, "radiant": radiant }, VANILLA, ECLIPSE],
                    "field": [TIMMY],
                    "library": [MENACE, FILLER],
                    "mana": x,
                },
                "p2": { "hand": [VANILLA], "field": [TIMMY] },
            }));
            s.play(APPROPRIATIONS, json!({ "x": x, "modes": ["Culture"] }));
            let mut cards = s.hand(P1);
            cards.extend(s.pile(P1, "library"));
            cards.push(unit(&s, P1, 1));
            let turned = cards.iter().filter(|card| card.radiant).count();
            (turned, cards.len() - turned)
        }

        #[test]
        fn r4666000_base_6x_at_x_16_is_96_so_some_card_stays_over_seeds_and_at_x_17_every_card_turns() {
            assert!((0..200).any(|n| culture(false, 16, n).1 > 0));
            assert_eq!(culture(false, 17, 0), (5, 0));
        }

        #[test]
        fn r4666000_radiant_16x_at_x_6_is_96_so_some_card_stays_over_seeds_and_at_x_7_every_card_turns() {
            assert!((0..200).any(|n| culture(true, 6, n).1 > 0));
            assert_eq!(culture(true, 7, 0), (5, 0));
        }

        #[test]
        fn r767000_r4154105_the_kept_v0_6_12b_script_rolls_17x_over_4_and_12x_so_some_card_stays_at_x_23_and_x_8() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_12b(), || {
                assert!((0..200).any(|n| culture(false, 23, n).1 > 0));
                assert_eq!(culture(false, 24, 0), (5, 0));
                assert!((0..200).any(|n| culture(true, 8, n).1 > 0));
                assert_eq!(culture(true, 9, 0), (5, 0));
            });
        }

        #[test]
        fn r767000_r3996201_the_kept_v0_6_6h_script_rolls_14x_so_every_card_turns_at_x_8_where_12x_leaves_some() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_6h(), || {
                assert_eq!(culture(true, 8, 0), (5, 0));
            });
        }

        #[test]
        fn r767000_r3996201_the_kept_v0_6_5b_script_rolls_16x_so_every_card_turns_at_x_7_where_14x_leaves_some() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_5b(), || {
                assert_eq!(culture(true, 7, 0), (5, 0));
            });
        }

        #[test]
        fn r767000_r3135002_the_kept_v0_5_11o_script_rolls_7x_so_every_card_turns_at_x_15_where_5x_leaves_some() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_5_11o(), || {
                assert_eq!(culture(false, 15, 0), (5, 0));
            });
            // Today's 6X% is 90% there: some card stays over the seeds.
            assert!((0..80).any(|n| culture(false, 15, n).1 > 0));
        }

        #[test]
        fn r581_every_card_is_rolled_a_radiant_one_too_so_the_draws_never_count_the_hidden_radiant_cards() {
            let rolls = |hand_radiant: bool| -> u32 {
                let mut s = appropriations(
                    false,
                    json!({
                        "hand": [APPROPRIATIONS, { "def": VANILLA, "radiant": hand_radiant }, ECLIPSE],
                        "field": [TIMMY],
                        "library": [MENACE, FILLER],
                    }),
                    json!({}),
                );
                let cursor = s.state().rng_cursor;
                s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Culture"] }));
                s.state().rng_cursor - cursor
            };
            // Field 1 + hand 2 + deck 2: five rolls whether or not the hand's Vanilla is Radiant already.
            assert_eq!(rolls(false), 5);
            assert_eq!(rolls(true), 5);
        }

        #[test]
        fn r4453000_culture_never_rolls_your_immune_field_unit() {
            // §6.1: 108% turns every card it rolls, so the immune Unit stays as it is only because the
            // Spell passes it by; the hand and the deck are no field and still turn.
            let mut s = appropriations(true, json!({ "mana": 9 }), json!({}));
            let timmy = unit(&s, P1, 1).id;
            s.card_mut(timmy.as_str()).granted_keywords.push(Keyword::ImmuneToSpells);
            s.play(APPROPRIATIONS, json!({ "x": 9, "modes": ["Culture"] }));
            assert!(!s.card(timmy.as_str()).radiant);
            s.expect_stats(timmy.as_str(), json!({ "attack": 3, "health": 3 }));
            let mut cards = s.hand(P1);
            cards.extend(s.pile(P1, "library"));
            assert_eq!(cards.len(), 4);
            for card in &cards {
                assert!(card.radiant, "{}", card.def_id);
            }
        }

        #[test]
        fn r311_r177_radiant_at_x_9_is_108_every_card_turns_the_decks_listed_as_they_went_in_the_hands_hidden_from_the_opponent() {
            let mut s = appropriations(true, json!({ "mana": 9 }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 9, "modes": ["Culture"] }));
            let mut cards = s.hand(P1);
            cards.extend(s.pile(P1, "library"));
            for card in &cards {
                assert!(card.radiant);
            }
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 6, "health": 6 }));
            let entries = listed(&s);
            assert_eq!(listed_count(&entries), 2);
            for entry in &entries {
                assert_eq!(entry["radiant"], false);
            }
            let hand_ids: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
            let cues: Vec<Value> = view_events(&s, P2).into_iter().filter(|event| event["type"] == "radiantSet").collect();
            // Field 1 + hand 2 + deck 2: every card is cued, the hand's and the deck's under the sentinel.
            assert_eq!(cues.len(), 5);
            for event in &cues {
                let id = event["instanceId"].as_str().unwrap_or_default().to_string();
                assert!(!hand_ids.contains(&id));
            }
        }

        #[test]
        fn r581_r177_a_hand_card_already_radiant_is_cued_like_the_rest_so_the_opponents_cues_never_count_it() {
            let cues = |hand_radiant: bool| -> usize {
                let mut s = appropriations(
                    true,
                    json!({
                        "hand": [{ "def": APPROPRIATIONS, "radiant": true }, { "def": VANILLA, "radiant": hand_radiant }, ECLIPSE],
                        "field": [TIMMY],
                        "library": [MENACE, FILLER],
                    }),
                    json!({}),
                );
                s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Culture"] }));
                view_events(&s, P2).into_iter().filter(|event| event["type"] == "radiantSet").count()
            };
            assert_eq!(cues(true), cues(false));
        }
    }

    mod healthcare {
        use super::*;

        #[test]
        fn e38_r4666000_your_units_on_the_field_and_in_your_hand_get_2x_less_2_health_and_armor_x_which_stacks_with_printed_armor_your_deck_does_not() {
            let mut s = appropriations(false, json!({ "field": [TIMMY, ARMORED] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 3, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 3, "health": 7 }));
            assert_eq!(s.stats(&timmy).armor, 3);
            assert_eq!(s.stats(unit(&s, P1, 2)).armor, 10);
            let vanilla = mine(&s, VANILLA, "hand");
            assert_eq!(vanilla.buffs.health, 4);
            assert_eq!(vanilla.granted_keywords, vec![Keyword::Armor { n: 3 }]);
            let deck = mine(&s, MENACE, "library");
            assert_eq!(deck.buffs.health, 0);
            assert!(deck.granted_keywords.is_empty());
            let theirs = unit(&s, P2, 1);
            s.expect_stats(&theirs, json!({ "health": 3 }));
        }

        /// R4666000: a hand Unit carries its Health and Armor onto the field as it enters.
        #[test]
        fn e38_r4666000_a_hand_unit_carries_the_health_and_armor_onto_the_field_as_it_enters() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Healthcare"] }));
            s.play(VANILLA, json!({ "zone": 2 }));
            let vanilla = unit(&s, P1, 2);
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 6 }));
            assert_eq!(s.stats(&vanilla).armor, 2);
        }

        /// R4666000: 5X−1 at X = 2 is 9 (⌊17X/5⌋'s 6 through v0.6.12b); the Armor 2X is kept.
        #[test]
        fn r4666000_radiant_5x_less_1_health_and_armor_2x() {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 12 }));
            assert_eq!(s.stats(&timmy).armor, 4);
            assert_eq!(mine(&s, MENACE, "library").buffs.health, 0);
        }

        /// R767000, R4154105: through v0.6.12b the Radiant face's ⌊17X/5⌋ at X = 2 was 6, and it reached
        /// the deck too, Armor and all.
        #[test]
        fn r767000_r4154105_the_kept_v0_6_12b_script_gives_radiant_17x_over_5_health_and_armor_2x_to_the_deck_too() {
            crate::under_kept(APPROPRIATIONS, super::super::script_v0_6_12b(), || {
                let mut s = appropriations(true, json!({}), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Healthcare"] }));
                let timmy = unit(&s, P1, 1);
                s.expect_stats(&timmy, json!({ "health": 9 }));
                assert_eq!(s.stats(&timmy).armor, 4);
                let deck = mine(&s, MENACE, "library");
                assert_eq!(deck.buffs.health, 6);
                assert_eq!(deck.granted_keywords, vec![Keyword::Armor { n: 4 }]);
            });
        }
    }

    /// R3135002, R4666000: each number at X = 1, 4 and 6 on both faces, played end to end.
    mod the_numbers {
        use super::*;
        use crate::scripts::classic_plus::c040_appropriations as card;

        /// (radiant, X, Attack, Books, Health, Armor) by the formulas: 2X−2 [4X−3], ⌊5X/3⌋−2 [⌊17X/5⌋−2],
        /// 2X−2 [5X−1] and X [2X], each at least 1 (R4666000).
        const ROWS: [(bool, i32, i32, usize, i32, i32); 6] = [
            (false, 1, 1, 1, 1, 1),
            (false, 4, 6, 4, 6, 4),
            (false, 6, 10, 8, 10, 6),
            (true, 1, 1, 1, 4, 2),
            (true, 4, 13, 11, 19, 8),
            (true, 6, 21, 18, 29, 12),
        ];
        /// R767000, R4154105: the same through v0.6.12b: ⌊5X/3⌋−2 [⌊17X/5⌋−4], ⌊5X/3⌋−2 [⌊17X/5⌋−2],
        /// ⌊5X/3⌋−2 [⌊17X/5⌋] and X [2X].
        const ROWS_V0_6_12B: [(bool, i32, i32, usize, i32, i32); 6] = [
            (false, 1, 1, 1, 1, 1),
            (false, 4, 4, 4, 4, 4),
            (false, 6, 8, 8, 8, 6),
            (true, 1, 1, 1, 3, 2),
            (true, 4, 9, 11, 13, 8),
            (true, 6, 16, 18, 20, 12),
        ];
        /// R767000, R3996201: the same through v0.6.6h: 2X−3 [4X−5], 2X−3 [4X−3], 2X−3 [4X] and X [2X].
        const ROWS_V0_6_6H: [(bool, i32, i32, usize, i32, i32); 6] = [
            (false, 1, 1, 1, 1, 1),
            (false, 4, 5, 5, 5, 4),
            (false, 6, 9, 9, 9, 6),
            (true, 1, 1, 1, 4, 2),
            (true, 4, 11, 13, 16, 8),
            (true, 6, 19, 21, 24, 12),
        ];

        #[test]
        fn r4666000_military_education_and_healthcare_at_x_1_4_and_6_on_both_faces_none_on_the_deck() {
            played_rows(ROWS, false);
        }

        #[test]
        fn r767000_r4154105_the_kept_v0_6_12b_script_at_x_1_4_and_6_on_both_faces_the_deck_too() {
            crate::under_kept(APPROPRIATIONS, card::script_v0_6_12b(), || played_rows(ROWS_V0_6_12B, true));
        }

        #[test]
        fn r767000_r3996201_the_kept_v0_6_6h_script_at_x_1_4_and_6_on_both_faces() {
            crate::under_kept(APPROPRIATIONS, card::script_v0_6_6h(), || played_rows(ROWS_V0_6_6H, true));
        }

        /// Each row's Military, Education and Healthcare, played end to end; `deck` is whether Military
        /// and Healthcare reach the deck's Unit.
        fn played_rows(rows: [(bool, i32, i32, usize, i32, i32); 6], deck: bool) {
            let on_deck = |n: i32| if deck { n } else { 0 };
            for (radiant, x, attack, books, health, armor) in rows {
                let mut s = appropriations(radiant, json!({ "mana": 10 }), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": x, "modes": ["Military"] }));
                assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, attack, "Military radiant {radiant} X {x}");
                assert_eq!(mine(&s, MENACE, "library").buffs.attack, on_deck(attack));

                let mut s = appropriations(radiant, json!({ "library": [], "mana": 10 }), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": x, "modes": ["Education"] }));
                assert_eq!(s.pile(P1, "library").len(), books, "Education radiant {radiant} X {x}");

                let mut s = appropriations(radiant, json!({ "mana": 10 }), json!({}));
                s.play(APPROPRIATIONS, json!({ "x": x, "modes": ["Healthcare"] }));
                let vanilla = mine(&s, VANILLA, "hand");
                assert_eq!(vanilla.buffs.health, health, "Healthcare radiant {radiant} X {x}");
                assert_eq!(vanilla.granted_keywords, vec![Keyword::Armor { n: armor }]);
                assert_eq!(mine(&s, MENACE, "library").buffs.health, on_deck(health));
            }
        }

        #[test]
        fn r4666000_culture_is_6x_percent_and_radiant_16x_percent_at_x_1_4_and_6() {
            for (radiant, x, percent) in [
                (false, 1, 6),
                (false, 4, 24),
                (false, 6, 36),
                (true, 1, 16),
                (true, 4, 64),
                (true, 6, 96),
            ] {
                let face = if radiant { card::RADIANT } else { card::BASE };
                assert_eq!(card::at(face.percent, x), percent, "radiant {radiant} X {x}");
            }
            // Through v0.6.12b: ⌊17X/4⌋% and 12X% (R4154105); through v0.6.6h: 5X% and 14X%.
            let (base, radiant) = (card::BASE_V0_6_12B, card::RADIANT_V0_6_12B);
            assert_eq!((card::at(base.percent, 4), card::at(radiant.percent, 4)), (17, 48));
            let (base, radiant) = (card::BASE_V0_6_6H, card::RADIANT_V0_6_6H);
            assert_eq!((card::at(base.percent, 4), card::at(radiant.percent, 4)), (20, 56));
        }

        /// R4666000: Military's, Culture's and Healthcare's numbers are v0.6.5b's (R3996201) at every X,
        /// Education's v0.6.12b's, and the Armor every version's.
        #[test]
        fn r4666000_the_numbers_given_back_are_v0_6_5bs_and_education_keeps_v0_6_12bs() {
            let faces = [
                (card::BASE, card::BASE_V0_6_5B, card::BASE_V0_6_12B),
                (card::RADIANT, card::RADIANT_V0_6_5B, card::RADIANT_V0_6_12B),
            ];
            for (now, given_back, before) in faces {
                for x in 1..=30 {
                    for (a, b) in [
                        (now.attack, given_back.attack),
                        (now.percent, given_back.percent),
                        (now.health, given_back.health),
                        (now.armor, given_back.armor),
                    ] {
                        assert_eq!(card::at(a, x), card::at(b, x), "X {x}");
                    }
                    assert_eq!(card::at(now.books, x), card::at(before.books, x), "X {x}");
                    assert_eq!(card::at(now.armor, x), card::at(before.armor, x), "X {x}");
                }
            }
        }

        /// R4154105: no number of either face through v0.6.12b is above what it was through v0.6.6h at any
        /// X, the Armor the same.
        #[test]
        fn r4154105_no_v0_6_12b_number_is_above_its_v0_6_6h_value_at_any_x() {
            let pairs = [
                (card::BASE_V0_6_12B, card::BASE_V0_6_6H),
                (card::RADIANT_V0_6_12B, card::RADIANT_V0_6_6H),
            ];
            for (now, then) in pairs {
                for x in 1..=30 {
                    for (a, b) in [
                        (now.attack, then.attack),
                        (now.books, then.books),
                        (now.percent, then.percent),
                        (now.health, then.health),
                    ] {
                        assert!(card::at(a, x) <= card::at(b, x), "X {x}");
                    }
                    assert_eq!(card::at(now.armor, x), card::at(then.armor, x), "X {x}");
                }
            }
        }

        #[test]
        fn r3135002_no_number_of_either_face_is_below_1_at_any_x() {
            for face in [
                card::BASE,
                card::RADIANT,
                card::BASE_V0_6_12B,
                card::RADIANT_V0_6_12B,
                card::BASE_V0_6_6H,
                card::RADIANT_V0_6_6H,
                card::BASE_V0_6_5B,
                card::RADIANT_V0_6_5B,
            ] {
                for x in 1..=20 {
                    for number in [face.attack, face.books, face.percent, face.health, face.armor] {
                        assert!(card::at(number, x) >= 1, "X {x}");
                    }
                }
            }
        }
    }
}
