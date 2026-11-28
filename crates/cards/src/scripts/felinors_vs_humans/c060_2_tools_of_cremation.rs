//! SPEC §8.14 FvH #60.2 Tools of Cremation ((2) Field Spell, Token, printed Legendary), the token FvH #60
//! Toolmaster Luna summons. Built by S10 (#5260).
//!   Base:    "After you play the leftmost card in your hand, give {cards|random card|random cards} in your
//!            graveyard Flashback." (cards 1)
//!   Radiant: the same text. (cards 3)
//!
//! FvH #60.1's `cardPlayed` trigger with `played_leftmost` (FV2 #60.2, R5237005): a play of its
//! controller's whose card left the hand from its left end, the one card of a one-card hand included; a
//! cast or a play from no hand never counts (R70). Then the declared `cards` (R386) different cards of its
//! controller's graveyard that lack Flashback, drawn at random as the trigger resolves (R60), each
//! `gain_keyword_for_good` Flashback, kept in every zone so its graveyard play is offered (R4154202,
//! Classic #37 Last Hurrah's shape). Fewer such cards, all of them with no draw; none, nothing (R129).
//! Tunes: cards 1 ↑ (Radiant 3).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-060-2";

/// "After you play the leftmost card in your hand" (R5237005).
fn you_played_the_leftmost(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller)
        && played_leftmost(event)
}

/// R4154202, R60, R129: `cards` different graveyard cards of the controller's that lack Flashback, drawn
/// as the effect applies; all of them, with no draw, when they are no more than that.
fn flashback_picks(c: &mut EffectContext<'_>) -> Vec<String> {
    let count = param(&*c, "cards").max(0) as usize;
    let candidates: Vec<CardInstance> = zone_cards(&*c.state, c.controller, OffFieldZone::Graveyard)
        .into_iter()
        .filter(|card| card.flashback_spent != Some(true) && !has_flashback(&*c.state, card))
        .collect();
    if candidates.len() <= count {
        return candidates.into_iter().map(|card| card.id).collect();
    }
    c.sink.rng.shuffle(&candidates).into_iter().take(count).map(|card| card.id).collect()
}

fn cremation() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("tools-of-cremation", &[GameEventType::CardPlayed], |_ctx, _event| {
                vec![for_each_card(ForEachCardArgs {
                    cards: Arc::new(flashback_picks),
                    each: Arc::new(|instance_id: &str| {
                        gain_keyword_for_good(json_as(json!({
                            "keyword": { "kind": "Flashback" },
                            "instanceId": instance_id,
                        })))
                    }),
                })]
            })
            .with_when(you_played_the_leftmost),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ only in the declared number: catalog data.
    let base = cremation();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #60.2 Tools of Cremation — SPEC §8.14 row 60.2 (R5237005, R4154202, R60, R129, R386).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    /// The cards the base face gives Flashback, and the Radiant face's.
    const CARDS: usize = 1;
    const RADIANT_CARDS: usize = 3;

    /// A Unit to play from the hand: it stays on the field, so the graveyard holds only what the test
    /// put there.
    const UNIT: &str = "stand-in (0) 1/1";
    /// A textless card lying in the graveyard.
    const DEAD: &str = "stand-in (1) Spell";

    /// p1's Tools in its backrow, `hand` Units in its hand and `graveyard` cards in its graveyard.
    fn tools_out(radiant: bool, hand: usize, graveyard: usize) -> Scenario {
        weapon_board(
            "fvh060-2",
            json!({
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": vec![UNIT; hand],
                "graveyard": vec![DEAD; graveyard],
                "library": filler(4),
                "mana": 10,
            }),
            json!({}),
        )
    }

    /// How many of p1's graveyard cards have Flashback now.
    fn with_flashback(s: &Scenario) -> usize {
        s.pile(P1, "graveyard").iter().filter(|card| has_flashback(s.state(), card)).count()
    }

    fn play_at(s: &mut Scenario, index: usize) {
        let card = s.hand(P1)[index].clone();
        s.play(card, json!({}));
    }

    mod base {
        use super::*;

        #[test]
        fn r5237005_playing_the_leftmost_card_gives_1_random_graveyard_card_flashback() {
            let mut s = tools_out(false, 3, 4);
            play_at(&mut s, 0);
            assert_eq!(with_flashback(&s), CARDS);
        }

        #[test]
        fn r5237005_playing_the_rightmost_card_of_several_gives_nothing() {
            let mut s = tools_out(false, 3, 4);
            play_at(&mut s, 2);
            assert_eq!(with_flashback(&s), 0);
        }

        #[test]
        fn r5237005_the_one_card_of_a_one_card_hand_is_the_leftmost() {
            let mut s = tools_out(false, 1, 2);
            play_at(&mut s, 0);
            assert_eq!(with_flashback(&s), CARDS);
        }

        #[test]
        fn r4154202_the_flashback_offers_the_card_from_the_graveyard() {
            let mut s = tools_out(false, 2, 1);
            play_at(&mut s, 0);
            let dead = must(s.pile(P1, "graveyard").first().cloned(), "the graveyard card");
            assert!(has_flashback(s.state(), &dead));
            assert!(s.can_play(&dead));
        }

        #[test]
        fn r129_an_empty_graveyard_gives_nothing() {
            let mut s = tools_out(false, 2, 0);
            play_at(&mut s, 0);
            assert_eq!(with_flashback(&s), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r60_playing_the_leftmost_card_gives_3_different_random_graveyard_cards_flashback() {
            let mut s = tools_out(true, 3, 5);
            play_at(&mut s, 0);
            assert_eq!(with_flashback(&s), RADIANT_CARDS);
        }

        #[test]
        fn r129_with_fewer_cards_lacking_flashback_every_one_gains_it() {
            let mut s = tools_out(true, 3, 2);
            play_at(&mut s, 0);
            for card in nonempty(s.pile(P1, "graveyard"), "p1's graveyard cards") {
                assert!(has_flashback(s.state(), &card));
            }
        }

        #[test]
        fn r5237005_playing_a_middle_card_gives_nothing() {
            let mut s = tools_out(true, 3, 5);
            play_at(&mut s, 1);
            assert_eq!(with_flashback(&s), 0);
        }
    }
}
