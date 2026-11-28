//! SPEC §8.14 FvH #16 Travertine ((1, Embiggened 4) Spell, Legendary), a card of the Felinors vs Humans set
//! (R5235000).
//!   Base:    "Add a Travertine Stack with Temporary 2 to your hand.\nEmbiggened (+3): Add {stacks} instead,
//!            with Temporary 1 to {stacks}." (6)
//!   Radiant: the same with Radiant Travertine Stacks.
//!
//! Embiggen (the play's `embiggen`, read as `ctx.embiggened`; the label R1175000, the price R5109000).
//! `add_to_hand` of FvH #16.1 with `temporary_count` (R5238000): one Stack with Temporary 2; Embiggened,
//! the declared `stacks`, the k-th with Temporary k, so one is discarded, and pays out, at each of the
//! caster's next ends of turn (R2179001) (FV2 #16, R5256002). Radiant: Radiant Stacks. A full hand burns the rest,
//! and a burned Stack is no discard (R317). The Temporary counts and the (+3) are printed (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-016";

/// FvH #16.1 Travertine Stack, the token it adds.
const STACK: &str = "felinorsvshumans-016-1";
/// "with Temporary 2": the normal play's one Stack, a printed number (R1748003).
const NORMAL_TEMPORARY: i32 = 2;

/// One Stack on `radiant`, Temporary `count`.
fn add_stack(radiant: bool, count: i32) -> Effect {
    add_to_hand(json_as(json!({ "defId": STACK, "radiant": radiant, "temporaryCount": count })))
}

/// One face; `radiant` is the face its Stacks arrive on.
fn travertine(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            if ctx.embiggened {
                let stacks = param(&*ctx, "stacks");
                (1..=stacks).map(|count| add_stack(radiant, count)).collect()
            } else {
                vec![add_stack(radiant, NORMAL_TEMPORARY)]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: travertine(false),
        radiant: travertine(true),
    }
}

// FvH #16 Travertine — SPEC §8.14 row 16 (R5238000, R2179001, R1748000).
#[cfg(test)]
mod tests {
    use super::{ID, STACK};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, weapon_board};

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    /// FvH #16.2 Travertine Golem, what a Stack summons as it pays out.
    const GOLEM: &str = "felinorsvshumans-016-2";

    /// p1 holds Travertine (`radiant` its face) and a FILLER.
    fn holding(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER] }),
            json!({ "hand": [FILLER] }),
        )
    }

    /// The Stacks in p1's hand, in hand order.
    fn stacks(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == STACK).collect()
    }

    /// The Stacks' Temporary counts, each on `radiant`.
    fn counts(s: &Scenario, radiant: bool) -> Vec<i32> {
        let held = stacks(s);
        for card in nonempty(held.clone(), "the Stacks added") {
            assert_eq!(card.radiant, radiant);
        }
        held.iter().map(|card| card.temporary.unwrap_or(0)).collect()
    }

    fn golems(s: &Scenario) -> usize {
        units_on_field_of(s.state(), P1).iter().filter(|unit| unit.def_id == GOLEM).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r5256002_r5238000_adds_one_stack_with_temporary_2() {
            let mut s = holding("fvh016-normal", false);
            s.play(ID, json!({}));
            assert_eq!(counts(&s, false), vec![2]);
        }

        #[test]
        fn r5256002_r5238000_embiggened_adds_6_stacks_with_temporary_1_to_6() {
            let mut s = holding("fvh016-embiggened", false);
            s.play(ID, json!({ "embiggen": true }));
            s.expect_mana(P1, 0);
            assert_eq!(counts(&s, false), vec![1, 2, 3, 4, 5, 6]);
        }

        #[test]
        fn r5256002_r2179001_one_stack_is_discarded_and_pays_out_at_each_of_your_next_ends_of_turn() {
            let mut s = holding("fvh016-payout", false);
            s.play(ID, json!({ "embiggen": true }));
            s.end_turn();
            assert_eq!((stacks(&s).len(), golems(&s)), (5, 1));
            assert_eq!(s.state().players[P1].market_credits, Some(15));
            next_turn_of(&mut s, P1);
            s.end_turn();
            assert_eq!((stacks(&s).len(), golems(&s)), (4, 2));
            assert_eq!(s.state().players[P1].market_credits, Some(30));
        }

        #[test]
        fn r1748000_the_embiggened_count_moves_with_a_buff() {
            let mut s = holding("fvh016-buff", false);
            assert_eq!(crate::upgrade_number(&mut s, ID, "stacks"), 7);
            s.play(ID, json!({ "embiggen": true }));
            assert_eq!(counts(&s, false), vec![1, 2, 3, 4, 5, 6, 7]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238000_adds_one_radiant_stack_with_temporary_2() {
            let mut s = holding("fvh016-radiant", true);
            s.play(ID, json!({}));
            assert_eq!(counts(&s, true), vec![2]);
        }

        #[test]
        fn r5238000_embiggened_adds_6_radiant_stacks_with_temporary_1_to_6() {
            let mut s = holding("fvh016-radiant-embiggened", true);
            s.play(ID, json!({ "embiggen": true }));
            assert_eq!(counts(&s, true), vec![1, 2, 3, 4, 5, 6]);
        }
    }
}
