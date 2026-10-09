//! M #77 Bulk Booster (SPEC §8.8 row 77): (3) Spell, Common.
//!
//! Base:    "Fill your hand with random Common cards."
//! Radiant: "Fill your hand with random Radiant Common cards."
//! Engine:
//! - **How many:** this player's hand cap (`hand_cap_of`: `HAND_CAP`, or the hand size Meditative #79
//!   set, R1143) less the cards in hand as it resolves; Bulk Booster itself is resolving, not in hand.
//!   "Fill" is C+ #42's reward wording, which reads the same. A full hand adds nothing and draws
//!   nothing from the rng (R129).
//! - **The pool:** `add_random_from_catalog` over `{ rarity: Common }`: non-token Commons of every set
//!   that ships (R380, R382, R1420), never this card (R387), repeats allowed (R60), each on its base
//!   face at its printed cost; the Radiant face's are Radiant (§5.2).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-077";

fn bulk_booster(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let room = hand_cap_of(ctx.state, ctx.controller) - zone_count(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room <= 0 {
                return vec![];
            }
            vec![add_random_from_catalog(json_as(json!({
                "query": { "rarity": "Common" },
                "count": room,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bulk_booster(false),
        radiant: bulk_booster(true),
    }
}

// M #77 Bulk Booster — SPEC §8.8 row 77, BUILD M10 row M 77: "Adds random non-token Common cards of
// any set, never itself, until your hand holds your hand cap (10, or 12 after M 79), counted as it
// resolves; with a full hand, nothing and no random number (R129); radiant the cards are Radiant".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-016";

    /// p1 holds Bulk Booster (Radiant on `radiant`) and `others` fillers.
    fn holding(seed: &str, radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER], "mana": 3 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The cards the play added: p1's hand past the fillers it kept.
    fn added(s: &Scenario, kept: usize) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(kept).collect()
    }

    mod m77_bulk_booster {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn fills_to_ten_with_non_token_commons_never_itself() {
                // The Meditative set previewed, so its Commons, this card among them, are in the pool.
                let _preview = preview_sets(&[SetName::Meditative]);
                for n in 0..6 {
                    let mut s = holding(&format!("bulk-booster-{n}"), false, 3);
                    s.play(ID, json!({}));
                    assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                    let added = added(&s, 3);
                    assert_eq!(added.len(), 7);
                    for card in &added {
                        let def = crate::card_def(&card.def_id);
                        assert_eq!(def.rarity, Rarity::Common, "{}", card.def_id);
                        assert!(!def.token, "{}", card.def_id);
                        assert_ne!(card.def_id, ID);
                        assert!(!card.radiant);
                    }
                }
            }

            #[test]
            fn r1420_unpreviewed_it_draws_only_the_shipped_sets() {
                let mut s = holding("bulk-booster-shipped", false, 0);
                s.play(ID, json!({}));
                let added = added(&s, 0);
                assert_eq!(added.len() as i32, HAND_CAP);
                assert!(added.iter().all(|card| crate::card_def(&card.def_id).set != SetName::Meditative));
            }

            #[test]
            fn r1143_fills_to_a_set_cap() {
                let mut s = holding("bulk-booster-cap", false, 3);
                s.state_mut().players[P1].hand_cap = Some(12);
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len(), 12);
                assert_eq!(added(&s, 3).len(), 9);
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Burned));
            }

            #[test]
            fn r129_full_hand_adds_nothing() {
                // Ten cards behind it: the hand is full as it resolves.
                let mut s = holding("bulk-booster-full", false, 10);
                let cursor = s.state().rng_cursor;
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                assert!(added(&s, 10).is_empty());
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Burned));
                assert_eq!(s.state().rng_cursor, cursor, "no random number");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_cards_radiant() {
                let mut s = holding("bulk-booster-radiant", true, 4);
                s.play(ID, json!({}));
                let added = added(&s, 4);
                assert_eq!(added.len(), 6);
                for card in &added {
                    assert!(card.radiant, "{}", card.def_id);
                    assert_eq!(crate::card_def(&card.def_id).rarity, Rarity::Common);
                }
            }
        }
    }
}
