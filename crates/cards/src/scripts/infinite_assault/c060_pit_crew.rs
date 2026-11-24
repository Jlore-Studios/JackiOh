//! SPEC §8.13 IA #60 Pit Crew ((1) Unit, Rare, 2/3 → 4/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Your next Unit this turn costs ({discount}) less." (discount 2)
//!   Radiant: "Cry: Your next Unit this turn costs ({discount}) less." (discount 3)
//!
//! Core #35 Lunar Eclipse's `costDiscount` narrowed to Units (`onlyType: "Unit"`): the next Unit its
//! controller plays this turn costs the declared `discount` less, spent on that play
//! (`oncePerTurn`, expiry `thisTurn`, §2.2); a Spell neither uses nor spends it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-060";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![add_player_modifier(json_as(json!({
                "player": "self",
                "mod": {
                    "kind": "costDiscount",
                    "amount": param(&*ctx, "discount"),
                    // "Your next Unit": Units only, and only the next one.
                    "onlyType": "Unit",
                    "oncePerTurn": true,
                    "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                },
            })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 3 is its declared `discount`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #60 Pit Crew — SPEC §8.13 row 60.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TWO: &str = "stand-in (2) 2/2";
    const SPELL: &str = "stand-in (1) Spell";

    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }))
    }

    fn hand(radiant: bool) -> Value {
        json!([{ "def": ID, "radiant": radiant }, TWO, TWO, SPELL])
    }

    mod base {
        use super::*;

        #[test]
        fn the_next_unit_costs_2_less_and_the_one_after_full_price() {
            let mut s = board(hand(false));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_mana(P1, 9);

            s.play(TWO, json!({ "zone": 2 }));
            // The (2) Unit paid (0).
            s.expect_mana(P1, 9);

            s.play(TWO, json!({ "zone": 3 }));
            s.expect_mana(P1, 7);
        }

        #[test]
        fn a_spell_does_not_spend_it() {
            let mut s = board(hand(false));
            s.play(ID, json!({ "zone": 1 }));
            s.play(SPELL, json!({}));
            s.expect_mana(P1, 8);

            s.play(TWO, json!({ "zone": 2 }));
            s.expect_mana(P1, 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_next_unit_costs_3_less_and_the_one_after_full_price() {
            let mut s = board(hand(true));
            s.play(ID, json!({ "zone": 1 }));
            s.play(TWO, json!({ "zone": 2 }));
            s.expect_mana(P1, 9);

            s.play(TWO, json!({ "zone": 3 }));
            s.expect_mana(P1, 7);
        }

        #[test]
        fn a_spell_does_not_spend_it() {
            let mut s = board(hand(true));
            s.play(ID, json!({ "zone": 1 }));
            s.play(SPELL, json!({}));
            s.expect_mana(P1, 8);

            s.play(TWO, json!({ "zone": 2 }));
            s.expect_mana(P1, 8);
        }
    }
}
