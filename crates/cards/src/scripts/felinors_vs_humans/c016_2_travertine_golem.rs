//! SPEC §8.14 FvH #16.2 Travertine Golem ((3) Unit, Token, printed Legendary; 0/5 → 0/5), a token of the
//! Felinors vs Humans set (R5235000) that FvH #16.1 Travertine Stack summons.
//!   Base:    "Has Attack and Armor equal to your Float last turn, at most {cap}." (15)
//!   Radiant: "Has Attack and Armor equal to three times your Float last turn, at most {cap}."
//!
//! A self-aura read live (§10.4 layer 5, IA #80 Hive Ganger's shape): +x Attack and Armor x (§6.1 Armor
//! X, which a Unit keeps and which takes x off each hit), x = min(`cap`, `float_last_turn`) of its
//! controller (R5237002: the mana left unspent at the cleanup of their last ended turn, 0 before any;
//! R1563000's Float), the Radiant face three times it before the cap (FV2 #16.2). With x at 0 it is the
//! printed 0/5 and carries no Armor.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-016-2";

/// The Radiant face's "three times": a printed word, a term of the formula (R1748003).
const THRICE: i32 = 3;

/// The face whose Float counts `times` over.
fn golem(times: i32) -> Script {
    Script {
        aura: Some(aura_hook(move |ctx| {
            let cap = param(&ctx, "cap");
            let x = float_last_turn(ctx.state, ctx.self_.controller)
                .saturating_mul(times)
                .min(cap)
                .max(0);
            if x == 0 {
                return Vec::new();
            }
            let me = ctx.self_.id.clone();
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| unit.id == me),
                mod_: StatMod {
                    attack: Some(x),
                    keywords: Some(vec![Keyword::Armor { n: x }]),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: golem(1),
        radiant: golem(THRICE),
    }
}

// FvH #16.2 Travertine Golem — SPEC §8.14 row 16.2 (R5237002, R1563000, R386).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, weapon_board};

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    const ATTACKER: &str = "stand-in (1) 4/4";

    /// p1's Golem (`radiant` its face) on the field with `mana` to leave unspent; p2's 4/4 across.
    fn golem_board(seed: &str, radiant: bool, mana: i32) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": [FILLER], "mana": mana }),
            json!({ "field": [ATTACKER], "hand": [FILLER] }),
        )
    }

    /// The Golem's Attack and Armor now, its Health always the printed 5.
    fn attack_and_armor(s: &mut Scenario) -> (i32, i32) {
        s.expect_stats(ID, json!({ "health": 5, "maxHealth": 5 }));
        let view = s.stats(ID);
        (view.attack, view.armor)
    }

    mod base {
        use super::*;

        #[test]
        fn r5237002_before_any_turn_has_ended_it_is_a_0_5_without_armor() {
            let mut s = golem_board("fvh016-2-none", false, 3);
            assert_eq!(attack_and_armor(&mut s), (0, 0));
        }

        #[test]
        fn r5237002_attack_and_armor_are_your_float_last_turn_read_live() {
            let mut s = golem_board("fvh016-2-float", false, 3);
            s.end_turn();
            assert_eq!(attack_and_armor(&mut s), (3, 3));
            // It holds through the player's own next turn, until that turn's end writes the new one.
            next_turn_of(&mut s, P1);
            assert_eq!(attack_and_armor(&mut s), (3, 3));
            s.end_turn();
            assert_eq!(attack_and_armor(&mut s), (MAX_MANA, MAX_MANA));
        }

        #[test]
        fn its_armor_takes_x_off_each_hit() {
            let mut s = golem_board("fvh016-2-armor", false, 3);
            s.end_turn();
            s.attack(ATTACKER, ID);
            // 4 less Armor 3.
            s.expect_stats(ID, json!({ "health": 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237002_three_times_your_float_last_turn() {
            let mut s = golem_board("fvh016-2-thrice", true, 3);
            s.end_turn();
            assert_eq!(attack_and_armor(&mut s), (9, 9));
        }

        #[test]
        fn the_cap_applies_after_the_three_times() {
            let mut s = golem_board("fvh016-2-cap", true, 6);
            s.end_turn();
            assert_eq!(attack_and_armor(&mut s), (15, 15));
        }

        #[test]
        fn r386_the_cap_moves_with_a_nerf() {
            let mut s = golem_board("fvh016-2-nerf", true, 6);
            assert_eq!(crate::degrade_number(&mut s, ID, "cap"), 14);
            s.end_turn();
            assert_eq!(attack_and_armor(&mut s), (14, 14));
        }
    }
}
