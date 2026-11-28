//! FvH #74 Yile, Acceleration Supremist (SPEC §8.14 row 74): (2) Unit, Human, Catalyst, Acclaimed, Mythic,
//! 2/6 → 2/6 (the designer's data, FV2). Built by S08 (#5258).
//!   Base:    "Cry: Add a Post Human Aesthetist to your hand.\nDeath: Shuffle a Yile, Acceleration
//!            Supremist Prime into your deck."
//!   Radiant: "Cry: Add a Radiant Post Human Aesthetist to your hand.\nDeath: Shuffle a Radiant Yile,
//!            Acceleration Supremist Prime into your deck."
//!
//! M #49 YileGPT's Catalyst shape. The Cry is an `add_to_hand` of FvH #79, Radiant on the Radiant face; a
//! full hand burns it (R4). The Death is a `shuffle_into` of one FvH #74.1 at a uniformly random place in
//! the deck of the player who controlled it as it died, Radiant on the Radiant face, turned away at the
//! deck cap (R80); its owner's own-library list shows it (R311). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-074";

/// FvH #79 Post Human Aesthetist, the Cry's card.
const AESTHETIST: &str = "felinorsvshumans-079";
/// FvH #74.1 Yile, Acceleration Supremist Prime, the Death's card.
const PRIME: &str = "felinorsvshumans-074-1";

fn yile(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_to_hand(json_as(json!({ "defId": AESTHETIST, "radiant": radiant })))]
        })),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "player": "self",
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: yile(false),
        radiant: yile(true),
    }
}

// FvH #74 Yile, Acceleration Supremist — SPEC §8.14 row 74.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const KILL: &str = "core-016"; // Hit Job: the removal
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn yile_game(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, KILL, FILLER], "library": library },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn of_def(cards: Vec<CardInstance>, def_id: &str) -> Vec<CardInstance> {
        cards.into_iter().filter(|card| card.def_id == def_id).collect()
    }

    /// Plays Yile, then destroys it with the removal.
    fn play_and_kill(s: &mut Scenario) {
        s.play(ID, json!({}));
        let yile = s.card(ID).id.clone();
        s.play(KILL, pick::targets(pick::id(&yile)));
        s.expect_in_zone(yile.as_str(), "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_adds_a_post_human_aesthetist_to_your_hand() {
            let mut s = yile_game(false, json!([FILLER]));
            s.play(ID, json!({}));
            let added = nonempty(of_def(s.hand(P1), AESTHETIST), "the Aesthetist in hand");
            assert_eq!(added.len(), 1);
            assert!(!added[0].radiant);
        }

        #[test]
        fn its_death_shuffles_a_prime_into_your_deck() {
            let mut s = yile_game(false, json!([FILLER]));
            play_and_kill(&mut s);
            let primes = nonempty(of_def(s.pile(P1, "library"), PRIME), "the Prime in the deck");
            assert_eq!(primes.len(), 1);
            assert!(!primes[0].radiant);
            assert_eq!(s.pile(P1, "library").len(), 2);
        }

        #[test]
        fn a_full_deck_turns_the_prime_away() {
            let full: Vec<&str> = vec![FILLER; LIBRARY_CAP as usize];
            let mut s = yile_game(false, json!(full));
            play_and_kill(&mut s);
            assert!(of_def(s.pile(P1, "library"), PRIME).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_adds_a_radiant_post_human_aesthetist() {
            let mut s = yile_game(true, json!([FILLER]));
            s.play(ID, json!({}));
            let added = nonempty(of_def(s.hand(P1), AESTHETIST), "the Aesthetist in hand");
            assert!(added[0].radiant);
        }

        #[test]
        fn its_death_shuffles_a_radiant_prime_into_your_deck() {
            let mut s = yile_game(true, json!([FILLER]));
            play_and_kill(&mut s);
            let primes = nonempty(of_def(s.pile(P1, "library"), PRIME), "the Prime in the deck");
            assert_eq!(primes.len(), 1);
            assert!(primes[0].radiant);
        }
    }
}
