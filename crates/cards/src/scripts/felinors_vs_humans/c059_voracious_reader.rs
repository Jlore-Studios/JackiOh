//! SPEC §8.14 FvH #59 Voracious Reader ((1) Unit, Human, Rare, 1/3 → 2/6), a card of the held Felinors vs
//! Humans set. Built by S10 (#5260).
//!   Base:    "End of turn: Draw until you have {cards|card|cards} in hand." (cards 3)
//!   Radiant: "Divine Shield / End of turn: Draw until you have {cards|card|cards} in hand." (cards 4)
//!
//! End of turn of its controller (R62): `draw_while` the hand holds fewer than the declared `cards`
//! (R386), asked before each draw (Classic #46 Divine Favor's loop); a draw that adds nothing to the hand
//! ends it, so an empty deck costs one fatigue draw at most and a full hand burns nothing. A hand already
//! at the mark draws nothing. The Radiant face's Divine Shield is a catalog keyword. Its voice lines are
//! the presentation part's. Tunes: cards 3 ↑ (Radiant 4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-059";

/// "Draw until you have {cards} in hand": whether the hand is still short of the mark, asked before each
/// draw.
fn short_of_the_mark(now: &mut EffectContext<'_>) -> bool {
    zone_count(now.state, now.controller, OffFieldZone::Hand) < param(&*now, "cards")
}

fn reader() -> Script {
    Script {
        end_of_turn: Some(hook(|_ctx| {
            vec![draw_while(DrawWhileArgs {
                more: Arc::new(short_of_the_mark),
                player: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ in the declared number and the Radiant face's Divine Shield: catalog data.
    let base = reader();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #59 Voracious Reader — SPEC §8.14 row 59 (R62, R386).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    /// The hand the base face fills to, and the Radiant face's.
    const CARDS: usize = 3;
    const RADIANT_CARDS: usize = 4;

    /// A textless card to hold in a hand.
    const SPARE: &str = "stand-in (9) Spell";

    fn spares(count: usize) -> Vec<&'static str> {
        vec![SPARE; count]
    }

    /// p1's Reader on the field, `hand` in p1's hand and `library` cards in its deck; p2 to answer.
    fn reading(radiant: bool, hand: usize, library: Value) -> Scenario {
        weapon_board(
            "fvh059",
            json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": spares(hand), "library": library }),
            json!({ "hand": [SPARE] }),
        )
    }

    fn fatigue_of(s: &Scenario, player: PlayerId) -> usize {
        s.events_of("fatigue")
            .iter()
            .filter(|event| matches!(event, GameEvent::Fatigue { player: hit, .. } if *hit == player))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_the_end_of_your_turn_it_draws_until_you_hold_3_cards() {
            let mut s = reading(false, 1, filler(6));
            s.end_turn();
            assert_eq!(s.hand(P1).len(), CARDS);
            assert_eq!(s.pile(P1, "library").len(), 6 - (CARDS - 1));
        }

        #[test]
        fn a_hand_already_at_the_mark_draws_nothing() {
            let mut s = reading(false, CARDS + 1, filler(6));
            s.end_turn();
            assert_eq!(s.hand(P1).len(), CARDS + 1);
            assert_eq!(s.pile(P1, "library").len(), 6);
        }

        #[test]
        fn an_empty_deck_costs_one_fatigue_draw_at_most() {
            let mut s = reading(false, 0, json!([]));
            s.end_turn();
            assert_eq!(fatigue_of(&s, P1), 1);
            assert!(s.hand(P1).is_empty());
        }

        #[test]
        fn r62_the_opponents_end_of_turn_draws_nothing() {
            // p1 holds one card as p2's turn ends: only p1's own turn draw follows.
            let mut s = scenario(json!({
                "seed": "fvh059-their-end",
                "active": "p2",
                "p1": { "field": [ID], "hand": [SPARE], "library": filler(6) },
                "p2": { "hand": [SPARE], "library": filler(4) },
            }));
            for player in PLAYER_IDS {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.pile(P1, "library").len(), 5);
        }

        #[test]
        fn the_base_face_has_no_divine_shield() {
            let s = reading(false, 1, filler(6));
            assert!(!unit_has(s.state(), &s.unit_at(P1, 1), KeywordKind::DivineShield));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_at_the_end_of_your_turn_it_draws_until_you_hold_4_cards() {
            let mut s = reading(true, 1, filler(6));
            s.end_turn();
            assert_eq!(s.hand(P1).len(), RADIANT_CARDS);
            assert_eq!(s.pile(P1, "library").len(), 6 - (RADIANT_CARDS - 1));
        }

        #[test]
        fn its_divine_shield_reaches_play() {
            let s = reading(true, 1, filler(6));
            assert!(unit_has(s.state(), &s.unit_at(P1, 1), KeywordKind::DivineShield));
        }
    }
}
