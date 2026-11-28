//! SPEC §8.14 FvH #84 𓃠 ((2) Unit, Felinor, Common, 1/1 → 2/2), a card of the held Felinors vs Humans set
//! (R5235000). Its name is the Egyptian cat hieroglyph, U+130E0, so its slug is `cat_glyph` (FV1).
//!   Base:    "Reborn\nDeath: Gain {jade} Jade." (jade 1)
//!   Radiant: "Flashback, Reborn\nDeath: Gain {jade} Jade." (jade 1)
//!
//! Reborn and the Radiant face's Flashback are catalog data. The Death is `add_jade` of the declared `jade`
//! onto the Jade Counter of the player who controlled it as it died (R961, R962), so the body its Reborn
//! returns gains Jade again when it dies. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-084";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![add_jade(json_as(json!({ "amount": param(ctx, "jade") })))]
        })),
        ..Script::default()
    };
    // The same script: Flashback is the Radiant face's catalog keyword, its Jade the declared `jade`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #84 𓃠 — SPEC §8.14 row 84.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB, FILLER],
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 10,
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn glyph(s: &Scenario) -> CardInstance {
        must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the 𓃠 on p1's field",
        )
    }

    fn kill(s: &mut Scenario) {
        let me = glyph(s);
        s.play(HIT_JOB, pick::targets(pick::instance(&me)));
    }

    fn jade(s: &Scenario, player: PlayerId) -> Option<i32> {
        s.state().players[player].jade
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_gains_its_controller_1_jade_and_reborn_brings_it_back() {
            let mut s = board(false);
            kill(&mut s);
            assert_eq!(jade(&s, P1), Some(1));
            assert_eq!(jade(&s, PlayerId::P2), None, "the opponent gains none");
            // Reborn: the body it returns stands on the field (`glyph` finds it there).
            assert_eq!(glyph(&s).def_id, ID);
        }

        #[test]
        fn the_reborn_bodys_death_gains_jade_again() {
            let mut s = board(false);
            kill(&mut s);
            kill(&mut s);
            assert_eq!(jade(&s, P1), Some(2));
            assert!(active_units_of(s.state(), P1).iter().all(|unit| unit.def_id != ID), "it stays dead");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_death_gains_1_jade_twice_through_reborn() {
            let mut s = board(true);
            kill(&mut s);
            assert_eq!(jade(&s, P1), Some(1));
            kill(&mut s);
            assert_eq!(jade(&s, P1), Some(2));
        }

        #[test]
        fn flashback_lets_it_be_played_from_the_graveyard() {
            let s = scenario(json!({
                "p1": { "graveyard": [{ "def": ID, "radiant": true }], "hand": [FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let dead = must(s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == ID), "the 𓃠 in the graveyard");
            assert!(s.can_play(&dead), "the Radiant face's Flashback plays it from the graveyard");
            let plain = scenario(json!({
                "p1": { "graveyard": [ID], "hand": [FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let dead = must(plain.pile(P1, "graveyard").into_iter().find(|card| card.def_id == ID), "the 𓃠");
            assert!(!plain.can_play(&dead), "the base face has no Flashback");
        }
    }
}
