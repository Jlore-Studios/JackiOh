//! SPEC §8.14 FvH #109 Keymaster Kalabaster ((3) Unit, Human, Legendary, 6/8 → 12/16), a card of the
//! Felinors vs Humans set (R5235000).
//!   Base:    "Whenever your opponent draws a card, add a copy of it to your hand. It costs ({setCost})." (1)
//!   Radiant: "Whenever your opponent draws a card, add a copy of it to your hand. It costs ({setCost})." (0)
//!
//! A trigger on each `drawn` of its controller's opponent (R5261000): a card cast on draw is drawn and
//! copied; a card the hand cap burned (§2.4) left nothing to copy, nor did a draw from an empty deck,
//! which draws no card (R3). `add_to_hand` with `copyOf` the drawn card: a fresh card of its definition
//! and face with R57's riders, its price set at `setCost` by `costOverride` (R65). The copy shows its
//! holder what was drawn, by the card's own design; the opponent's view tells them only that a card was
//! added (R466). Both faces run this script: the Radiant (0) is its declared `setCost`. Built by S11
//! (#5261).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-109";

/// R5261000: the card an opponent's draw drew, while there is one to copy: a card in a graveyard that
/// did not cast on draw is one the hand cap burned.
fn copyable_draw(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::Drawn { player, instance_id, .. } = event else {
        return None;
    };
    if *player != opponent_of(ctx.controller) {
        return None;
    }
    let card = find_instance(ctx.state, instance_id)?;
    let burned =
        card.zone.z() == ZoneName::Graveyard && !jackioh_engine::draw::casts_on_draw(ctx.state, card);
    if burned {
        return None;
    }
    Some(instance_id.clone())
}

fn copies_draws() -> TriggerDef {
    TriggerDef::new("keymaster-kalabaster", &[GameEventType::Drawn], |ctx, event| {
        let Some(drawn) = copyable_draw(ctx, event) else {
            return vec![];
        };
        vec![add_to_hand(json_as(json!({ "copyOf": drawn, "costOverride": param(&*ctx, "setCost") })))]
    })
    .with_when(|ctx, event| copyable_draw(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![copies_draws()],
        ..Script::default()
    };
    // The same script: the Radiant face's (0) is its declared `setCost`.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #109 Keymaster Kalabaster — SPEC §8.14 row 109 (R5261000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself
    /// What the opponent draws.
    const THEIR_DRAW: &str = "stand-in (3) 9/9";
    /// What p1 draws.
    const OUR_DRAW: &str = "stand-in (2) 5/5";

    fn board(radiant: bool, their_hand: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": [OUR_DRAW, OUR_DRAW, OUR_DRAW],
            },
            "p2": { "hand": their_hand, "library": [THEIR_DRAW, THEIR_DRAW, THEIR_DRAW] },
        }))
    }

    fn copies_in_hand(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    /// The opponent's start of turn draws one card: the copy in p1's hand, at `cost`.
    fn copies_their_draw_at(radiant: bool, cost: i32) {
        let mut s = board(radiant, json!([FILLER]));
        s.end_turn();
        assert_eq!(copies_in_hand(&s, P2, THEIR_DRAW).len(), 1, "they drew it");
        let copies = copies_in_hand(&s, P1, THEIR_DRAW);
        assert_eq!(copies.len(), 1, "one copy for the one draw");
        for copy in nonempty(copies, "the copy") {
            assert_eq!(effective_cost(s.state(), &copy, Default::default()), cost);
            assert_eq!(copy.owner, P1);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r5261000_when_your_opponent_draws_a_card_a_copy_costing_1_is_added_to_your_hand() {
            copies_their_draw_at(false, 1);
        }

        #[test]
        fn r5261000_your_own_draws_are_not_copied() {
            let mut s = board(false, json!([FILLER]));
            s.end_turn();
            s.end_turn();
            assert_eq!(copies_in_hand(&s, P1, OUR_DRAW).len(), 1, "p1's own draw, and no copy of it");
            assert_eq!(copies_in_hand(&s, P2, OUR_DRAW).len(), 0);
        }

        #[test]
        fn r5261000_a_card_the_hand_cap_burns_leaves_nothing_to_copy() {
            let full: Vec<Value> = (0..HAND_CAP).map(|_| json!(FILLER)).collect();
            let mut s = board(false, json!(full));
            s.end_turn();
            assert_eq!(s.count(GameEventType::Burned), 1, "their draw burned");
            assert!(copies_in_hand(&s, P1, THEIR_DRAW).is_empty());
        }

        #[test]
        fn r5261000_a_draw_from_an_empty_deck_leaves_nothing_to_copy() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": ID, "lane": 1 }], "hand": [FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": [] },
            }));
            let hand = s.hand(P1).len();
            s.end_turn();
            assert_eq!(s.count(GameEventType::Fatigue), 1, "their draw fatigued");
            assert_eq!(s.hand(P1).len(), hand, "nothing added");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5261000_when_your_opponent_draws_a_card_a_copy_costing_0_is_added_to_your_hand() {
            copies_their_draw_at(true, 0);
        }
    }
}
