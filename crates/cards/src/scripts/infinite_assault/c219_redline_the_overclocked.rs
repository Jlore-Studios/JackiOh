//! IA #219 Redline, the Overclocked (SPEC §8.13 row 219), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Charge. Start of turn: Gain {mana} mana this turn. Deal {damage} damage to your
//!            hero." (2, 2)
//!   Radiant: "Charge, Windfury. Start of turn: Gain {mana} mana this turn. Deal {damage} damage
//!            to your hero." (2, 2)
//!
//! Charge (and the Radiant face's Windfury) are catalog data. The Start of turn gains temporary
//! mana, then hits its controller's own hero (R62: only its controller's turn fires it).
//!
//! Engine: `gain_mana`, `damage` (R62). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-219";

fn redline() -> Script {
    Script {
        start_of_turn: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                damage(json_as(json!({
                    "to": { "of": "selfHero" },
                    "amount": param(&*ctx, "damage"),
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's numbers are its own declared `mana` and `damage`.
    let base = redline();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #219 Redline, the Overclocked — SPEC §8.13 row 219.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn standing(radiant: bool, active: &str) -> Scenario {
        scenario(json!({
            "seed": "redline",
            "active": active,
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": [STOCKPILE], "library": crate::filler(4) },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn start_of_turn_gains_mana_and_hits_its_own_hero() {
            let mut s = standing(false, "p1");
            let (mana, hero) = (s.state().players[P1].mana.current, s.state().players[P1].hero.health);
            s.start_turn();
            assert_eq!(s.state().players[P1].mana.current, mana + 2, "2 mana this turn");
            assert_eq!(s.state().players[P1].hero.health, hero - 2, "2 damage to its own hero");
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let mut s = standing(false, "p2");
            let (mana, hero) = (s.state().players[P1].mana.current, s.state().players[P1].hero.health);
            s.start_turn();
            assert_eq!(s.state().players[P1].mana.current, mana, "no mana");
            assert_eq!(s.state().players[P1].hero.health, hero, "no hit");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn start_of_turn_gains_mana_and_hits_its_own_hero() {
            let mut s = standing(true, "p1");
            let (mana, hero) = (s.state().players[P1].mana.current, s.state().players[P1].hero.health);
            s.start_turn();
            assert_eq!(s.state().players[P1].mana.current, mana + 2, "2 mana this turn");
            assert_eq!(s.state().players[P1].hero.health, hero - 2, "2 damage to its own hero");
        }

        #[test]
        fn the_radiant_face_attacks_twice() {
            let s = standing(true, "p1");
            let view = unit_view(s.state(), &s.unit_at(P1, 1));
            assert!(view.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Windfury));
        }
    }
}
