//! SPEC §8.12 S+ #137 The Perfect Pair (§2.4, §6.3 Add to hand; R60, R1420, R3535000, R3565004): (1) Spell,
//! Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "Add an Automa-δ and its paired Automa-Ω card to your hand.
//!            The pairs are Scepter and Reign, Antumbra and Eclipse, Quad and Oct, Sei and Omura, Vela and
//!            Corvus."
//!   Radiant: "Add a Radiant Automa-δ and its paired Radiant Automa-Ω card to your hand."
//!
//! Engine: one of the five pairs, picked from the match rng (R60, R3565004), then the Automa-δ card and the
//! Automa-Ω card of the pair, in the text's order, each its own `addToHand`: both keep their printed
//! Tribute (the cards make no Tribute promise), and the Radiant face adds both Radiant. No prompt opens
//! and nothing is announced. A full hand burns what does not fit (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-137";

/// The pairs, Automa-δ and Automa-Ω: Scepter and Reign (S+ #113, #114), Antumbra and Eclipse (#118, #119),
/// Quad and Oct (#122, #123), Sei and Omura (#124, #125), Vela and Corvus (#126, #127).
const PAIRS: [(&str, &str); 5] = [
    ("stinkyscapeplus-113", "stinkyscapeplus-114"),
    ("stinkyscapeplus-118", "stinkyscapeplus-119"),
    ("stinkyscapeplus-122", "stinkyscapeplus-123"),
    ("stinkyscapeplus-124", "stinkyscapeplus-125"),
    ("stinkyscapeplus-126", "stinkyscapeplus-127"),
];

fn pair(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(&(delta, omega)) = ctx.rng.pick(&PAIRS) else {
                return vec![];
            };
            [delta, omega]
                .into_iter()
                .map(|card| add_to_hand(json_as(json!({ "defId": card, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pair(false),
        radiant: pair(true),
    }
}

// S+ #137 The Perfect Pair — SPEC §8.12 row S+ 137 (R3565004; R60, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn pairing(radiant: bool, seed: &str, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The cards the cast added to p1's hand, in order.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r3565004_every_pair_is_an_automa_delta_and_its_automa_omega() {
        crate::register_all();
        for (delta, omega) in PAIRS {
            assert!(crate::card_def(delta).tags.contains(&Tag::AutomaDelta), "{delta}");
            assert!(crate::card_def(omega).tags.contains(&Tag::AutomaOmega), "{omega}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3565004_adds_one_pair_delta_first() {
            let mut seen: IndexSet<(String, String)> = IndexSet::new();
            for i in 0..40 {
                let mut s = pairing(false, &format!("s137-{i}"), 0);
                s.play(ID, json!({}));
                assert!(s.state().pending.is_none(), "no prompt opens");
                let cards = added(&s);
                assert_eq!(cards.len(), 2, "two cards");
                let (delta, omega) = (&cards[0], &cards[1]);
                assert!(
                    PAIRS.contains(&(delta.def_id.as_str(), omega.def_id.as_str())),
                    "{} and {} are no pair",
                    delta.def_id,
                    omega.def_id
                );
                for card in &cards {
                    assert_eq!(card.zone.z(), ZoneName::Hand);
                    assert!(!card.radiant);
                    assert_eq!(card.cost_mod, 0, "the printed price");
                    assert!(card.cost_override.is_none());
                }
                seen.insert((delta.def_id.clone(), omega.def_id.clone()));
            }
            assert!(seen.len() >= 3, "the pairs vary: {seen:?}");
        }

        #[test]
        fn r3565004_both_keep_their_printed_tribute() {
            use jackioh_engine::play_choices::tribute_cost_of;
            let mut s = pairing(false, "s137-tribute", 0);
            s.play(ID, json!({}));
            let cards = added(&s);
            assert!(tribute_cost_of(s.state(), &cards[0]) > 0);
            assert!(tribute_cost_of(s.state(), &cards[1]) > 0);
            assert!(tribute_cost_of(s.state(), &cards[1]) > tribute_cost_of(s.state(), &cards[0]));
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let mut s = pairing(false, "s137-full", HAND_CAP as usize - 1);
            s.play(ID, json!({}));
            // The cast leaves 9 cards in the hand: the Automa-δ card fits and the Automa-Ω card burns.
            assert_eq!(added(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if PAIRS.iter().any(|pair| pair.1 == def_id)))
                .count();
            assert_eq!(burned, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565004_both_cards_are_radiant() {
            for i in 0..10 {
                let mut s = pairing(true, &format!("s137-r{i}"), 0);
                s.play(ID, json!({}));
                let cards = added(&s);
                assert_eq!(cards.len(), 2, "two cards");
                assert!(PAIRS.contains(&(cards[0].def_id.as_str(), cards[1].def_id.as_str())));
                assert!(cards.iter().all(|card| card.radiant));
            }
        }
    }
}
