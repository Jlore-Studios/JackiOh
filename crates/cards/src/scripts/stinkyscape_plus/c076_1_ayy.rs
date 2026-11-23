//! SPEC §8.12 S+ #76.1 Ayy, a token of the Stinkyscape+ set (R1420, R3535000). (1) Unit, Token,
//! Common, summoned by #76 Ayys LMAO and #85 Body Snatch.
//!   Base:    "2/2, Cloaked"
//!   Radiant: "4/4, Cloaked, Poisonous"
//!
//! Every word is a catalog keyword and every number catalog data (§6.1), so there is no script to run.
//! Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-076-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #76.1 Ayy — SPEC §8.12 row 76.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    fn stands(radiant: bool) -> (Scenario, CardInstance) {
        let s = scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }] },
            "p2": {},
        }));
        let card = s.unit(P1, 1).expect("the Ayy");
        (s, card)
    }

    #[test]
    fn base_is_a_2_2_cloaked_unit() {
        crate::register_all();
        let (s, card) = stands(false);
        let view = unit_view(s.state(), &card);
        assert_eq!((view.attack, view.health), (2, 2));
        assert!(unit_has(s.state(), &card, KeywordKind::Cloaked));
        assert!(!unit_has(s.state(), &card, KeywordKind::Poisonous));
    }

    #[test]
    fn radiant_is_a_4_4_cloaked_poisonous_unit() {
        crate::register_all();
        let (s, card) = stands(true);
        let view = unit_view(s.state(), &card);
        assert_eq!((view.attack, view.health), (4, 4));
        assert!(unit_has(s.state(), &card, KeywordKind::Cloaked));
        assert!(unit_has(s.state(), &card, KeywordKind::Poisonous));
    }
}
