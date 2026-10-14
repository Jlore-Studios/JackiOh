//! M #100 Greaser (SPEC §8.8 row 100): a 2-cost 7/7 → 21/21 Unit with no tags and no text on either
//! face. A card with no text has no hooks, so both Scripts are empty, as Core #8 Mr. Vanilla's are.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-100";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = Script::default();
    CardScripts { base, radiant }
}

// M #100 Greaser — SPEC §8.8 row 100, BUILD M10 row M 100: "A 2-cost 7/7 with no tags and no text: it
// enters with no keywords and its play is the play alone; radiant a 21/21 with no text, three times
// the base face's stats (R275)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const FILLER: &str = "core-016";

    /// p1 plays Greaser (Radiant on `radiant`), with a filler behind it so no turn auto-ends.
    fn played(seed: &str, radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 2 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    mod m100_greaser {
        use super::*;

        #[test]
        fn base_is_a_2_cost_7_7_with_no_text_and_no_cry() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert!(def.tags.is_empty());
            assert_eq!((def.base.attack, def.base.health), (Some(7), Some(7)));
            assert!(def.base.keywords.is_empty());
            assert_eq!(def.base.text, "");
            assert!(script().base.cry.is_none());

            let mut s = played("greaser-base", false);
            s.expect_stats(ID, json!({ "attack": 7, "health": 7, "maxHealth": 7 }));
            assert!(s.stats(ID).keywords.is_empty());
            // No text: the play emits the play itself and nothing else.
            let types: Vec<&str> = s
                .last_events()
                .iter()
                .map(|event| event.event_type().as_str())
                .collect();
            assert_eq!(
                types,
                vec![
                    "manaChanged",
                    "cardAnnounced",
                    "cardPlayed",
                    "summoned",
                    "cardResolved"
                ]
            );
        }

        #[test]
        fn r275_radiant_is_21_21_with_no_text() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(21), Some(21)));
            assert!(def.radiant.keywords.is_empty());
            assert_eq!(def.radiant.text, "");
            assert!(script().radiant.cry.is_none());

            let mut s = played("greaser-radiant", true);
            s.expect_stats(ID, json!({ "attack": 21, "health": 21, "maxHealth": 21 }));
            assert!(s.stats(ID).keywords.is_empty());
        }
    }
}
