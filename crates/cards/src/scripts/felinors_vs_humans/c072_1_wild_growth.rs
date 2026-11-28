//! FvH #72.1 Wild Growth (SPEC §8.14 row 72.1; §2.3, R3538006, R5148000, R5255001): (1) Spell,
//! Other-Origins, Token (printed Epic), a token FvH #72 Otherworldly Naturalism adds.
//!   Base:    "Gain {crystals|mana crystal|mana crystals}." (crystals 1)
//!   Radiant: "Gain {crystals|mana crystal|mana crystals}." (crystals 2)
//!
//! Engine: `changeMaxMana` of the declared `crystals` (R3538006's verb, which S+ #95 JC Backed lowers
//! with): the crystals stand past the mana cap for the rest of the game, empty, its caster's current
//! mana untouched (R5255001), every refresh from then on filling up to the higher maximum; never past
//! `MANA_CEILING` (R5148000). The faces differ only in the declared number. Built by S05 (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-072-1";

fn wild_growth() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![change_max_mana(ChangeMaxManaArgs {
                amount: param(&*ctx, "crystals"),
                player: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = wild_growth();
    // The same script: the Radiant face's 2 is its declared `crystals`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #72.1 Wild Growth — SPEC §8.14 row 72.1 (R3538006, R5255001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER, FILLER], "library": filler(4) },
        }))
    }

    /// p1's (max, current) mana.
    fn mana(s: &Scenario) -> (i32, i32) {
        let side = &s.state().players[P1];
        (side.mana.max, side.mana.current)
    }

    /// Wild Growth played at a full cap: the gain past it, and the mana left as the play left it.
    fn grown(radiant: bool, crystals: i32) {
        let mut s = board(radiant);
        let (max, current) = mana(&s);
        assert_eq!(max, MAX_MANA, "the scenario starts at the cap");
        let cost = effective_cost(s.state(), s.card(ID), Default::default());

        s.play(ID, json!({}));

        assert_eq!(mana(&s), (MAX_MANA + crystals, current - cost), "empty crystals past the cap");
    }

    /// The crystals last: two turns on, the refresh fills to the cap and past it.
    fn they_last(radiant: bool, crystals: i32) {
        let mut s = board(radiant);
        s.play(ID, json!({}));
        s.end_turn();
        s.end_turn();
        assert_eq!(mana(&s), (MAX_MANA + crystals, MAX_MANA + crystals));
    }

    mod base {
        use super::*;

        #[test]
        fn r5255001_gains_an_empty_mana_crystal_past_the_cap() {
            grown(false, 1);
        }

        #[test]
        fn r5255001_the_crystal_lasts_the_game() {
            they_last(false, 1);
        }

        #[test]
        fn r386_an_upgrade_makes_it_gain_two() {
            let mut s = board(false);
            step_param(s.card_mut(ID), "crystals", 1);
            s.play(ID, json!({}));
            assert_eq!(mana(&s).0, MAX_MANA + 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5255001_gains_two_empty_mana_crystals_past_the_cap() {
            grown(true, 2);
        }

        #[test]
        fn r5255001_the_crystals_last_the_game() {
            they_last(true, 2);
        }
    }
}
