//! SPEC §8.11 Training Grounds 1 Spikey Steven: (1) Unit, Human, Common, 5/2 → 15/6, a card of the
//! held Training Grounds set (R1420, R2194000). Vanilla Unit (as Core #8 Mr. Vanilla, C #86 Genn; R275, `docs/radiant-audit.md`).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-001";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face is the base face on tripled stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #1 Spikey Steven — SPEC §8.11 row 1, BUILD M13 row TG 1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4, an enemy Unit to swing at.

    fn played(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": super::ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }));
        s.play(super::ID, json!({}));
        s
    }

    #[test]
    fn base_enters_as_a_5_2_human_with_no_text_and_no_keyword() {
        let mut s = played(false);
        s.expect_stats(super::ID, json!({ "attack": 5, "health": 2, "maxHealth": 2 }));
        assert!(s.stats(super::ID).keywords.is_empty());
        let def = crate::card_def(super::ID);
        assert_eq!(def.base.text, "");
        assert!(def.tags.contains(&Tag::Human));
    }

    #[test]
    fn base_is_summoning_sick_on_the_turn_it_is_played() {
        let mut s = played(false);
        let body = s.unit_at(PlayerId::P2, 1);
        s.expect_refused(|s| s.attack(super::ID, "hero"));
        s.expect_refused(|s| s.attack(super::ID, &body));
    }

    #[test]
    fn radiant_enters_as_a_15_6_human_with_no_text() {
        let mut s = played(true);
        s.expect_stats(super::ID, json!({ "attack": 15, "health": 6, "maxHealth": 6 }));
        assert!(s.stats(super::ID).keywords.is_empty());
        assert_eq!(crate::card_def(super::ID).radiant.text, "");
    }
}
