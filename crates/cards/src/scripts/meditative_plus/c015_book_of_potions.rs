//! M+ #15 Book of Potions (SPEC §8.9 row 15, BUILD M11 row M+ 15): (1) Spell, Book, Epic.
//!   Base and Radiant: "Add {cards|random Radiant Potion card|random Radiant Potion cards} to your
//!   hand." (1/2)
//!
//! As M+ #14 Potion Pouch, Radiant on both faces: `add_random_from_catalog { query: { tags:
//! [Potion] }, count, radiant: true }`. The count is declared and read through `param` (R386).
//! Tunes: cards 1 ↑ (Radiant 2).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-015";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Potion"] },
                "count": param(&*ctx, "cards"),
                "radiant": true,
            })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared count, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// M+ #15 Book of Potions — SPEC §8.9 row 15, BUILD M11 row M+ 15: "adds 1 random Radiant Potion
// card (M+ 11 to 13, previewed); cards reads through `param()`; radiant 2".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const BOOK: &str = "meditativeplus-015";
    /// The Potion pool under a preview of the set (R1179003): M+ #11 to #13, nothing else.
    const POTIONS: [&str; 3] = ["meditativeplus-011", "meditativeplus-012", "meditativeplus-013"];
    const FILLER: &str = "core-005"; // (1) Spell

    use crate::scenario;

    /// The def ids the Book added to p1's hand, beside the Filler held throughout.
    fn added(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).map(|card| card.def_id).collect()
    }

    fn booked(seed: &str, radiant: bool) -> Scenario {
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(BOOK, json!({}));
        s
    }

    #[test]
    fn r1179003_adds_1_random_radiant_potion_card() {
        crate::register_all();
        let s = booked("m+015-one", false);
        let added = added(&s);
        assert_eq!(added.len(), 1);
        assert!(POTIONS.contains(&added[0].as_str()), "{} is no Potion card", added[0]);
        let card = s.hand(P1).into_iter().find(|card| card.def_id != FILLER).expect("the added potion");
        assert!(card.radiant, "the added potion is Radiant");
    }

    #[test]
    fn r386_cards_reads_through_param() {
        crate::register_all();
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        let mut s = scenario(json!({
            "seed": "m+015-param",
            "p1": { "hand": [BOOK, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        step_param(s.card_mut(BOOK), "cards", 1);
        s.play(BOOK, json!({}));
        assert_eq!(added(&s).len(), 2);
    }

    #[test]
    fn radiant_adds_2() {
        crate::register_all();
        let s = booked("m+015-radiant", true);
        let added = added(&s);
        assert_eq!(added.len(), 2);
        for def in &added {
            assert!(POTIONS.contains(&def.as_str()), "{def} is no Potion card");
        }
        for card in s.hand(P1) {
            if card.def_id != FILLER {
                assert!(card.radiant, "the added potion is Radiant");
            }
        }
    }
}
