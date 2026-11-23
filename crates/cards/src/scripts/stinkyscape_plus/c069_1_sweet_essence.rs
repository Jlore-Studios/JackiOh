//! SPEC §8.12 S+ #69.1 Sweet Essence (0) Spell, Jelly Bean, Token, a token of the Stinkyscape+ set
//! (R1420; R3558001), made by #69 Exotic Jellybean.
//!   Base:    "If this is the left most card in your hand, the card to the right of this is Radiant."
//!   Radiant: "If this is the left most card in your hand, the card to the right of this is Radiant and
//!            costs (1) less. The card to the left of this costs (1) less."
//!
//! Engine: E8's hand aura (`hand_aura_hook`, R3558001), read on the card in its owner's hand: while it is
//! the leftmost card (`is_leftmost_in_hand`, index 0), the card to its right is Radiant, and on the
//! Radiant face costs 1 less too; the Radiant face also takes 1 off the card to its left, which a
//! leftmost Essence has none of. A lent Radiance and price last while the two sit side by side and
//! are taken back at the next settled check once they do not, as a field aura's are; a lent card
//! played stays Radiant (R3558001). Its own play does nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-069-1";

/// "costs (1) less", to either side.
const DISCOUNT: i32 = -1;

fn essence() -> Script {
    Script {
        hand_aura: Some(hand_aura_hook(|args| {
            let mut lends = Vec::new();
            if args.radiant {
                lends.push(HandAura {
                    to: HandSide::Left,
                    radiant: false,
                    cost: DISCOUNT,
                });
            }
            if is_leftmost_in_hand(args.state, &args.self_.id) {
                lends.push(HandAura {
                    to: HandSide::Right,
                    radiant: true,
                    cost: if args.radiant { DISCOUNT } else { 0 },
                });
            }
            lends
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: essence(),
        radiant: essence(),
    }
}

// S+ #69.1 Sweet Essence — SPEC §8.12 row 69.1 (R3558001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    /// A (2) neighbour with no text.
    const NEIGHBOUR: &str = "stand-in (2) 1/1 R2/2";

    /// p1's hand: the Essence (`radiant` its face) first or second, beside a (2) neighbour.
    fn holding(seed: &str, essence_first: bool, radiant: bool) -> Scenario {
        crate::register_all();
        let essence = json!({ "def": ID, "radiant": radiant });
        let hand = if essence_first {
            json!([essence, NEIGHBOUR])
        } else {
            json!([NEIGHBOUR, essence])
        };
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": ["core-005"], "library": filler(4) },
        }))
    }

    fn neighbour(s: &Scenario) -> CardInstance {
        s.hand(P1).into_iter().find(|card| card.def_id != ID).expect("the neighbour")
    }

    fn price(s: &Scenario) -> i32 {
        effective_cost(s.state(), &neighbour(s), Default::default())
    }

    #[test]
    fn r3558001_leftmost_its_right_neighbour_is_radiant_at_its_price() {
        let s = holding("r3558001-essence-base", true, false);
        assert!(neighbour(&s).radiant);
        assert_eq!(price(&s), 2);
    }

    #[test]
    fn r3558001_not_leftmost_it_lends_nothing() {
        let s = holding("r3558001-essence-base-right", false, false);
        assert!(!neighbour(&s).radiant);
        assert_eq!(price(&s), 2);
    }

    #[test]
    fn r3558001_played_away_the_neighbour_loses_its_radiance() {
        let mut s = holding("r3558001-essence-played", true, false);
        let lent = neighbour(&s).id;
        s.play(ID, json!({}));
        assert!(!s.card(&lent).radiant);
    }

    #[test]
    fn r3558001_radiant_leftmost_the_right_neighbour_is_radiant_and_1_cheaper() {
        let s = holding("r3558001-essence-radiant", true, true);
        assert!(neighbour(&s).radiant);
        assert_eq!(price(&s), 1);
    }

    #[test]
    fn r3558001_radiant_the_card_to_its_left_is_1_cheaper() {
        let s = holding("r3558001-essence-radiant-left", false, true);
        assert!(!neighbour(&s).radiant, "only a leftmost Essence lends Radiance");
        assert_eq!(price(&s), 1);
    }
}
