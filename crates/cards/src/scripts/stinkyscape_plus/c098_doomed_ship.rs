//! SPEC §8.12 S+ #98 Doomed Ship (2) Spell, Legendary, a card of the Stinkyscape+ set (R1420; R3558002,
//! R3567006).
//!   Base:    "Add an iKwzki and Shockwave447, Shadiris and xxFreshJack, or Tylannt and Siva 1 to the
//!            left and right most cards of your hand. When they meet both become Radiant."
//!   Radiant: "… When they meet both become Radiant and cost (0)."
//!
//! Engine: one of the three pairs, drawn from the game's Rng (SP6; R3567006), goes to the two ends of its
//! caster's hand through E8's `addMeetingPair` (R3558002): the first card to the left end, the second to
//! the right. The moment the two sit side by side — every card between them having left — both become
//! Radiant, and on the Radiant face cost (0) as well. A card a full hand burns never meets its partner.
//! "Kwzki" is S #110 iKwzki, the one beside S #111 Shockwave447 and S #109 xxFreshJack (SP5); Shadiris is
//! S+ #165, Tylannt S #6 and Siva 1 S #87.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-098";

/// R3567006: the three pairs, left card first, in the order the text names them.
const PAIRS: [(&str, &str); 3] = [
    ("stinkyscape-110", "stinkyscape-111"),
    (super::c165_shadiris::ID, "stinkyscape-109"),
    ("stinkyscape-006", "stinkyscape-087"),
];

/// "and cost (0)", the Radiant face's.
const MEET_COST: i32 = 0;

fn doomed_ship(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some((left, right)) = ctx.rng.pick(&PAIRS).copied() else {
                return vec![];
            };
            vec![add_meeting_pair(AddMeetingPairArgs {
                left: left.to_string(),
                right: right.to_string(),
                player: None,
                radiant: None,
                meet_cost: radiant.then_some(MEET_COST),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: doomed_ship(false),
        radiant: doomed_ship(true),
    }
}

// S+ #98 Doomed Ship — SPEC §8.12 row 98 (R3558002, R3567006).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MEET_COST, PAIRS};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    /// p1 casts the Ship (`radiant` its face) holding one other card: its hand after.
    fn cast(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The pair at the two ends of p1's hand, which must be one of the three, in order.
    fn ends(s: &Scenario) -> (CardInstance, CardInstance) {
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 3, "{hand:?}");
        let (left, right) = (hand[0].clone(), hand[2].clone());
        assert!(
            PAIRS.contains(&(left.def_id.as_str(), right.def_id.as_str())),
            "{} … {}",
            left.def_id,
            right.def_id
        );
        assert_eq!(hand[1].def_id, FILLER, "the held card stays between them");
        (left, right)
    }

    #[test]
    fn r3567006_one_pair_goes_to_the_two_ends_and_each_pair_comes_over_the_seeds() {
        let mut seen = Vec::new();
        for n in 0..SEED_SEARCH_BOUND {
            let s = cast(&format!("r3567006-ship-{n}"), false);
            let (left, right) = ends(&s);
            assert!(!left.radiant && !right.radiant, "they have not met");
            if !seen.contains(&left.def_id) {
                seen.push(left.def_id);
            }
            if seen.len() == PAIRS.len() {
                return;
            }
        }
        panic!("only {seen:?} in {SEED_SEARCH_BOUND} seeds");
    }

    #[test]
    fn r3558002_when_they_meet_both_become_radiant() {
        let mut s = cast("r3558002-ship-meet", false);
        let (left, right) = ends(&s);
        s.play(FILLER, json!({}));
        for id in [&left.id, &right.id] {
            assert!(s.card(id).radiant, "{id} met Radiant");
            assert_eq!(s.card(id).cost_override, None, "the base face keeps their price");
        }
    }

    #[test]
    fn r3558002_radiant_when_they_meet_both_become_radiant_and_cost_0() {
        let mut s = cast("r3558002-ship-meet-radiant", true);
        let (left, right) = ends(&s);
        assert!(!left.radiant && !right.radiant, "added on their own faces");
        s.play(FILLER, json!({}));
        for id in [&left.id, &right.id] {
            assert!(s.card(id).radiant, "{id} met Radiant");
            assert_eq!(effective_cost(s.state(), s.card(id), Default::default()), MEET_COST);
        }
    }
}
