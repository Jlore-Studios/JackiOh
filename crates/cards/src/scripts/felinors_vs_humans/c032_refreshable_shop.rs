//! FvH #32 Refreshable Shop (SPEC §8.14 row 32): (2) Field Spell, Epic.
//!   Base:    "Durability 3\nActivate: Replace each card in your hand with a different random card. Make one
//!            of them Radiant."
//!   Radiant: "Durability 3\nActivate: Replace each card in your hand with a different random Radiant
//!            card."
//!
//! Engine: Activate, once a turn (R384). Durability 3 is the engine's keyword (R5236000): each use spends
//! 1 of the card's one counter; at 0 its Activate is refused (R5236001) and, at the settled state check
//! after the use resolves, it goes to its owner's graveyard by a plain move, no Death (R5236002); Refresh
//! restores none (R5236003). The use is `replace_hand_with_random` with `exclude_replaced` (R5238006),
//! FV2's reading of "a different one": each hand card goes to the graveyard by a move, no discard, and a
//! random card of a pool naming no set takes its place, never the card it replaces nor this card (R387,
//! R380). Base: then `set_radiant_random` over the hand, count 1, one uniform pick among the new cards
//! (R60); Radiant: every new card Radiant. An empty hand replaces nothing, the use and the Durability
//! spent all the same. Its 3 is a keyword count (R1748003); it declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-032";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;
/// "Make one of them Radiant."
const MADE_RADIANT: i32 = 1;

/// "Replace each card in your hand with a different random [Radiant] card", then on the base face one of
/// the new cards made Radiant.
fn restock(radiant: bool) -> Vec<Effect> {
    let mut args = json!({ "query": {}, "excludeReplaced": true });
    if radiant {
        args["radiant"] = json!(true);
    }
    let mut effects = vec![replace_hand_with_random(json_as(args))];
    if !radiant {
        effects.push(set_radiant_random(json_as(json!({ "zones": "hand", "count": MADE_RADIANT }))));
    }
    effects
}

fn face(radiant: bool) -> Script {
    let shop = ActivationDecl {
        id: "restock".to_string(),
        label: if radiant {
            "Replace each card in your hand with a different random Radiant card".to_string()
        } else {
            "Replace each card in your hand with a different random card".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| restock(radiant)),
    };
    Script {
        activations: vec![shop],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #32 Refreshable Shop — SPEC §8.14 row 32 (R384, R5236000–R5236003, R5238006, R387, R380, R60).
// The hand turns random, so every game is built with the turn's auto-end off (`weapon_board`).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{def_ids, ids, run_as, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "core-008"; // Mr. Vanilla
    /// Its printed Durability.
    const DURABILITY: i32 = 3;
    /// How many seeds a random replacement is drawn under.
    const SEEDS: usize = 8;
    /// The cards in the hand it replaces.
    const HAND: usize = 3;

    fn shop(seed: &str, radiant: bool, hand: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": hand, "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true }] }),
            json!({}),
        )
    }

    /// Three copies of one card, so a replacement by its own definition would show.
    fn vanillas() -> Value {
        json!([VANILLA, VANILLA, VANILLA])
    }

    /// p1's hand after one use on `seed`, with the cards it replaced.
    fn restocked(seed: &str, radiant: bool) -> (Scenario, Vec<String>) {
        let mut s = shop(seed, radiant, vanillas());
        let old = ids(&s.hand(P1));
        s.activate(ID, json!({}));
        assert_eq!(s.last_count("addedToHand"), HAND, "{seed}: one arrival per card replaced");
        (s, old)
    }

    mod base {
        use super::*;

        #[test]
        fn r5238006_each_hand_card_is_replaced_by_a_random_card_of_another_definition_one_of_them_radiant() {
            for n in 0..SEEDS {
                let seed = format!("fvh-032-base-{n}");
                let (mut s, old) = restocked(&seed, false);
                let hand = s.hand(P1);
                let defs = def_ids(&hand);
                assert!(!defs.iter().any(|id| id == VANILLA || id == ID), "{seed}: {defs:?}");
                assert_eq!(hand.iter().filter(|card| card.radiant).count(), 1, "{seed}");
                assert_eq!(s.last_count("discarded"), 0, "{seed}: a move, not a discard");
                for id in nonempty(old, "the replaced cards") {
                    s.expect_in_zone(id.as_str(), "graveyard");
                }
            }
        }

        #[test]
        fn r5236000_an_empty_hand_replaces_nothing_and_the_use_still_spends_its_durability() {
            let mut s = shop("fvh-032-empty", false, json!([]));
            s.activate(ID, json!({}));
            assert!(s.hand(P1).is_empty());
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY - 1));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn r5236002_r5236003_three_uses_spend_its_durability_3_and_it_goes_to_the_graveyard_by_a_plain_move() {
            let mut s = shop("fvh-032-walk", false, vanillas());
            let card = s.card(ID).clone();
            for left in (0..DURABILITY).rev() {
                s.activate(&card.id, json!({}));
                if left > 0 {
                    assert_eq!(durability_left(s.state(), s.card(&card.id)), Some(left));
                    // A Refresh gives the turn's use back, never the Durability.
                    run_as(&mut s, refresh_activates(), P1);
                    assert_eq!(durability_left(s.state(), s.card(&card.id)), Some(left));
                }
            }
            s.expect_in_zone(&card, "graveyard");
            assert_eq!(s.count("destroyed"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238006_each_hand_card_is_replaced_by_a_random_radiant_card_of_another_definition() {
            for n in 0..SEEDS {
                let seed = format!("fvh-032-radiant-{n}");
                let (mut s, old) = restocked(&seed, true);
                let hand = s.hand(P1);
                let defs = def_ids(&hand);
                assert!(!defs.iter().any(|id| id == VANILLA || id == ID), "{seed}: {defs:?}");
                assert!(hand.iter().all(|card| card.radiant), "{seed}");
                assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY - 1));
                for id in nonempty(old, "the replaced cards") {
                    s.expect_in_zone(id.as_str(), "graveyard");
                }
            }
        }
    }
}
