//! SPEC §8.14 FvH #60.1 Tools of Creation ((2) Field Spell, Token, printed Legendary), the token FvH #60
//! Toolmaster Luna summons. Built by S10 (#5260).
//!   Base:    "After you play the rightmost card in your hand, draw {draw}." (draw 1)
//!   Radiant: "After you play the rightmost card in your hand, draw {draw}." (draw 3)
//!
//! A `cardPlayed` trigger on its controller's plays when `played_rightmost` holds (FV2 #60.1): the card's
//! place as it left the hand, `cardPlayed.fromHand` (R5237005, R3558000), the one card of a one-card hand
//! both ends, so a cast or a card played from no hand, which leaves no hand place, never counts (R70); then
//! `draw` of the declared `draw` (R386), the hand cap burning the rest (R4). `fromHand` is kept in any game
//! holding a Felinors vs Humans card (`fvh_kept`), so in every game holding this one. Tunes: draw 1 ↑
//! (Radiant 3).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-060-1";

/// "After you play the rightmost card in your hand" (R5237005).
fn you_played_the_rightmost(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller)
        && played_rightmost(event)
}

fn creation() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("tools-of-creation", &[GameEventType::CardPlayed], |ctx, _event| {
                vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))]
            })
            .with_when(you_played_the_rightmost),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ only in the declared number: catalog data.
    let base = creation();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #60.1 Tools of Creation — SPEC §8.14 row 60.1 (R5237005, R386).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    /// The draws the base face makes, and the Radiant face's.
    const DRAW: usize = 1;
    const RADIANT_DRAW: usize = 3;

    /// A textless Spell to play from the hand.
    const SPELL: &str = "stand-in (0) Spell";

    /// p1's Tools in its backrow, `hand` Spells in its hand; `active` takes the turn.
    fn tools_out(radiant: bool, hand: usize, active: &str) -> Scenario {
        let mut s = weapon_board(
            "fvh060-1",
            json!({
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": vec![SPELL; hand],
                "library": filler(8),
            }),
            json!({ "hand": vec![SPELL; 3], "library": filler(8) }),
        );
        if active == "p2" {
            s.end_turn();
        }
        s
    }

    fn drawn_by(s: &Scenario, player: PlayerId) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: drawer, .. } if *drawer == player))
            .count()
    }

    /// p1 plays the card at `index` of its hand.
    fn play_at(s: &mut Scenario, index: usize) {
        let card = s.hand(P1)[index].clone();
        s.play(card, json!({}));
    }

    mod base {
        use super::*;

        #[test]
        fn r5237005_playing_the_rightmost_card_draws_1() {
            let mut s = tools_out(false, 3, "p1");
            play_at(&mut s, 2);
            assert_eq!(drawn_by(&s, P1), DRAW);
            assert_eq!(s.hand(P1).len(), 2 + DRAW);
        }

        #[test]
        fn r5237005_playing_the_leftmost_or_a_middle_card_draws_nothing() {
            let mut s = tools_out(false, 3, "p1");
            play_at(&mut s, 0);
            assert_eq!(drawn_by(&s, P1), 0);
            play_at(&mut s, 0);
            // Two cards left: the left one is not the rightmost.
            assert_eq!(drawn_by(&s, P1), 0);
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn r5237005_the_one_card_of_a_one_card_hand_is_the_rightmost() {
            let mut s = tools_out(false, 1, "p1");
            play_at(&mut s, 0);
            assert_eq!(drawn_by(&s, P1), DRAW);
        }

        #[test]
        fn the_opponents_plays_draw_nothing() {
            let mut s = tools_out(false, 3, "p2");
            let theirs = s.hand(PlayerId::P2);
            let rightmost = must(theirs.last().cloned(), "p2's rightmost card");
            s.play(rightmost, json!({}));
            assert_eq!(drawn_by(&s, P1), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237005_playing_the_rightmost_card_draws_3() {
            let mut s = tools_out(true, 3, "p1");
            play_at(&mut s, 2);
            assert_eq!(drawn_by(&s, P1), RADIANT_DRAW);
            assert_eq!(s.hand(P1).len(), 2 + RADIANT_DRAW);
        }

        #[test]
        fn r5237005_playing_the_leftmost_card_draws_nothing() {
            let mut s = tools_out(true, 3, "p1");
            play_at(&mut s, 0);
            assert_eq!(drawn_by(&s, P1), 0);
        }
    }
}
