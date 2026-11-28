//! FvH #38 Short Term Loan (SPEC §8.14 row 38): (0) Spell, Epic, of the Felinors vs Humans set.
//!   Base:    "Gain {mana} mana. Overload ({loss})." (mana 3, loss 4)
//!   Radiant: the same, loss 1.
//!
//! `gain_mana` of the declared `mana`, this turn's mana, which may pass the cap (§2.3), then
//! `next_turn_mana` of minus the declared `loss`: the printed Overload, paid as the Spell resolves
//! (R4751000, R1848004) and floored at 0 by the next refresh. IA #50 Burst Clock's shape. Both faces run
//! one script: the Radiant face's 1 is its declared `loss`, read through `param` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-038";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's Overload (1) is its declared `loss`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #38 Short Term Loan — SPEC §8.14 row 38 (R4751000, R1848004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    /// Stockpile: a Spell each hand keeps.
    const FILLER: &str = "core-005";
    /// A free Spell, so p1's next turn has a play at (0) mana and does not end by itself.
    const FREE: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "short-term-loan",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FREE, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4751000_gains_3_now_past_the_cap_and_overloads_4_so_the_next_refresh_gives_0() {
            let mut s = board(false);
            // MAX_MANA 4, the Loan free: 4 + 3.
            s.play(ID, json!({}));
            s.expect_mana(P1, MAX_MANA + 3);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -4);
            next_turn_of(&mut s, P1);
            s.expect_mana(P1, 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4751000_gains_3_now_and_overloads_only_1() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.expect_mana(P1, MAX_MANA + 3);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -1);
            next_turn_of(&mut s, P1);
            s.expect_mana(P1, MAX_MANA - 1);
        }
    }
}
