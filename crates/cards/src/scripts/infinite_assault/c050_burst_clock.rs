//! SPEC §8.13 IA #50 Burst Clock ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Gain {mana} mana this turn. Lose ({loss}) mana next turn." (mana 2, loss 1)
//!   Radiant: "Gain {mana} mana this turn. Lose ({loss}) mana next turn." (mana 3, loss 1)
//!
//! The gain is this turn's mana (`gain_mana`, §2.3, §6.3), which may pass the max; the loss is a
//! next-turn rider (`next_turn_mana`, as M #18 Expedition12's), read at the next refresh, never
//! below 0.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-050";

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
    // The same script: the Radiant face's 3 is its declared `mana`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #50 Burst Clock — SPEC §8.13 row 50.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "library": crate::filler(4) },
        }))
    }

    /// p1's next turn: the opponent's turn passes, and p1's refresh has run.
    fn next_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
    }

    mod base {
        use super::*;

        #[test]
        fn gains_2_now_and_the_next_refresh_gives_1_less() {
            let mut s = board(false);
            s.play(ID, json!({}));
            // 4 paid 1, then the gain: 3 + 2.
            s.expect_mana(P1, 5);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -1);

            next_turn(&mut s);
            s.expect_mana(P1, 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn gains_3_now_and_the_next_refresh_gives_1_less() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.expect_mana(P1, 6);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -1);

            next_turn(&mut s);
            s.expect_mana(P1, 3);
        }
    }
}
