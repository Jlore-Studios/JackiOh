//! FvH #95 Platebreaker (SPEC §8.14 row 95): (2) Unit, Human, Common, 6/6 → 12/12. Built by S08 (#5258).
//!   Base:    "Aura: Your cards have Pierce."
//!   Radiant: "Cry: For the rest of the game, your cards have Pierce."
//!
//! "Your cards" is every card its controller controls on the field, in either row, and every card it has
//! resolving; heroes are no cards, and a card in a hand or a deck is held, not controlled in play. Pierce
//! is read off the source through the layers (`damage::pierces`, R346): a Unit, a resolving Spell, a
//! Field Spell or a springing Trap. Base: the aura `cards_have_aura(Keyword::Pierce)` while it acts
//! (R5237008, M+ #7 Life Mage's `spells_have_aura` shape). Radiant: no Aura; the Cry (R1) gives its
//! controller `give_cards_keyword` Pierce, a never-expiring lend to every card they control, those
//! arriving later too, which outlasts the Platebreaker (R5237008). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-095";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            aura: Some(cards_have_aura(Keyword::Pierce)),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![give_cards_keyword(GiveUnitsKeywordArgs {
                    keyword: Keyword::Pierce,
                })]
            })),
            ..Script::default()
        },
    }
}

// FvH #95 Platebreaker — SPEC §8.14 row 95.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ALLY: &str = "stand-in (1) 3/3";
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";
    const KILL: &str = "core-016"; // Hit Job: the removal
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn pierces(s: &Scenario, card: &str) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Pierce)
    }

    /// p1's ally (lane 1) and Field Spell (backrow lane 1), the Platebreaker in hand; p2's Unit (lane 1)
    /// and 5 Armor.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, ALLY, KILL, FILLER],
                "field": [ALLY],
                "backrow": [FIELD_SPELL],
            },
            "p2": { "hand": [FILLER], "field": [ALLY], "armor": 5 },
        }))
    }

    /// p1's ally strikes the armored enemy hero: the health and Armor it lost.
    fn strike_the_hero(s: &mut Scenario) -> (i32, i32) {
        let ally = s.unit_at(P1, 1).id;
        let before = hero_of(s.state(), P2);
        s.attack(ally.as_str(), "hero");
        let after = hero_of(s.state(), P2);
        (before.health - after.health, before.armor - after.armor)
    }

    fn kill_platebreaker(s: &mut Scenario) {
        let platebreaker = s.card(ID).id.clone();
        s.play(KILL, pick::targets(pick::id(&platebreaker)));
        s.expect_in_zone(platebreaker.as_str(), "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn your_cards_have_pierce_itself_and_a_field_spell_included_and_the_opponents_do_not() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let ally = s.unit_at(P1, 1).id;
            let spell = must(s.backrow(P1, 1), "p1's Field Spell").id;
            let theirs = s.unit_at(P2, 1).id;
            assert!(pierces(&s, ID));
            assert!(pierces(&s, &ally));
            assert!(pierces(&s, &spell));
            assert!(!pierces(&s, &theirs));
        }

        #[test]
        fn your_units_strike_through_armor() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let shown = s.stats(s.unit_at(P1, 1).id.as_str()).attack;
            assert_eq!(strike_the_hero(&mut s), (shown, 0));
        }

        #[test]
        fn the_aura_ends_when_it_leaves_the_field() {
            let mut s = board(false);
            s.play(ID, json!({}));
            kill_platebreaker(&mut s);
            let ally = s.unit_at(P1, 1).id;
            assert!(!pierces(&s, &ally));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_gives_your_cards_pierce_for_the_rest_of_the_game() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let ally = s.unit_at(P1, 1).id;
            let spell = must(s.backrow(P1, 1), "p1's Field Spell").id;
            let theirs = s.unit_at(P2, 1).id;
            assert!(pierces(&s, &ally));
            assert!(pierces(&s, &spell));
            assert!(pierces(&s, ID));
            assert!(!pierces(&s, &theirs));
        }

        #[test]
        fn the_lend_outlasts_it_and_reaches_a_unit_played_later() {
            let mut s = board(true);
            s.play(ID, json!({}));
            kill_platebreaker(&mut s);
            let ally = s.unit_at(P1, 1).id;
            assert!(pierces(&s, &ally));
            s.play(ALLY, json!({}));
            let later = must(
                active_units_of(s.state(), P1)
                    .into_iter()
                    .find(|unit| unit.id != ally)
                    .map(|unit| unit.id.clone()),
                "the ally played later",
            );
            assert!(pierces(&s, &later));
            let shown = s.stats(ally.as_str()).attack;
            assert_eq!(strike_the_hero(&mut s), (shown, 0));
        }

        #[test]
        fn one_that_enters_without_its_cry_lends_nothing() {
            let s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }, ALLY], "hand": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let ally = s.unit_at(P1, 2).id;
            assert!(!pierces(&s, &ally), "the Radiant face prints no Aura");
        }
    }
}
