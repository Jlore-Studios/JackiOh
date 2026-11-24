//! IA #32 Glitch Specter (SPEC §8.13 row 32), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Evasive while you have a face-down card."
//!   Radiant: "Lifesteal. Evasive while you have a face-down card."
//!
//! Lifesteal on the Radiant face is catalog data. The conditional Evasive reads the board as the
//! layers do: a face-down card of its controller's — one set by its Cloaked counts, a Revealed
//! one does not (R4586001) — lends it Evasive, and none takes it away.
//!
//! Engine: `conditional_keywords` over `face_down_count` (R4586001). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-032";

fn specter() -> Script {
    Script {
        conditional_keywords: Some(read_hook(|args| {
            if face_down_count(args.state, args.self_.controller) > 0 {
                vec![Keyword::Evasive]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: specter(),
        // The same condition: the Radiant face's Lifesteal is catalog data.
        radiant: specter(),
    }
}

// IA #32 Glitch Specter — SPEC §8.13 row 32 (R4586001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn evasive(s: &Scenario) -> bool {
        let specter = s.unit(P1, 1).expect("the Specter");
        unit_has(s.state(), &specter, KeywordKind::Evasive)
    }

    mod base {
        use super::*;

        #[test]
        fn r4586001_without_a_face_down_card_it_is_no_evasive() {
            let s = played(false, "specter-bare");
            assert!(!evasive(&s));
        }

        #[test]
        fn r4586001_with_a_face_down_card_it_is_evasive() {
            let mut s = played(false, "specter-hidden");
            crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            assert!(evasive(&s));
        }

        #[test]
        fn r4586001_a_revealed_card_is_no_face_down_card() {
            let mut s = played(false, "specter-revealed");
            let set = crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            assert!(evasive(&s));
            s.activate(&set, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
            assert!(!evasive(&s), "revealed, it is face-up");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586001_the_condition_holds_on_the_radiant_face_too() {
            let mut s = played(true, "specter-radiant");
            assert!(!evasive(&s));
            crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            assert!(evasive(&s));
        }
    }
}
