//! M+ #19 Grubby Gloves (SPEC §8.9 row 19, BUILD M11 row M+ 19): (1, Embiggened 2)
//! Spell, Common.
//!   Base:    "Steal the top {cards|card|cards} of your opponent's deck. Embiggened (2): Steal
//!            {cards|random card|random cards} from their hand instead." (2/3)
//!   Radiant: the same, each card taken costing (0).
//!
//! Played at its normal price, `take_from_library { from: enemy, pick: top }` (C+ #12.3): the
//! opponent's top cards in order, with no random number (R1179004). Played Embiggened,
//! `give_from_hand { from: enemy, cards: random }` (as M+ #10). Either way the cards become the
//! taker's (R12), hidden from the opponent but for the count (R97), the hand cap burning the
//! overflow (§2.4). The Radiant face's `TakenRiders.cost_override` is 0 on each card taken, either
//! version. The count is declared and read through `param` (R386). Tunes: cards 2 ↑ (Radiant 3).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-019";

/// The Radiant face's price for each card taken.
const TAKEN_COST: i32 = 0;

fn gloves(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "from": "enemy",
                "count": param(&*ctx, "cards"),
            });
            if radiant {
                args["costOverride"] = json!(TAKEN_COST);
            }
            if ctx.embiggened {
                args["cards"] = json!("random");
                vec![give_from_hand(json_as(args))]
            } else {
                args["pick"] = json!("top");
                vec![take_from_library(json_as(args))]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: gloves(false),
        radiant: gloves(true),
    }
}

// M+ #19 Grubby Gloves — SPEC §8.9 row 19, BUILD M11 row M+ 19: "at (1) the opponent's top 2 deck
// cards move to your hand as yours, hidden from them but their count (R97); with 1 left, 1;
// empty, none and no fatigue; Embiggened at (2) 2 random cards of their hand instead; a full hand
// burns the overflow into your graveyard; cards reads through `param()`; radiant 3 each way, each
// costing (0)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GLOVES: &str = "meditativeplus-019";
    const MENACE: &str = "core-019"; // (3) Unit
    const VANILLA: &str = "core-008"; // (1) Unit
    const LUNAR: &str = "core-035"; // (1) Spell
    const FILLER: &str = "core-005"; // (1) Spell

    use crate::scenario;

    use crate::js;

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    fn taken(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.owner == P1 && card.def_id != FILLER).collect()
    }

    #[test]
    fn r1179004_takes_the_top_2_of_the_opponents_deck_with_no_random_number() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE, LUNAR, VANILLA] },
        }));
        let cursor = s.state().rng_cursor;
        s.play(GLOVES, json!({}));
        assert_eq!(s.state().rng_cursor, cursor);
        // The top 2 in order: Menace, then Lunar.
        assert_eq!(hand_defs(&s, P1), [FILLER, MENACE, LUNAR]);
        let taken = taken(&s);
        assert_eq!(taken.len(), 2);
        for card in &taken {
            assert_eq!(card.owner, P1);
            assert_eq!(card.controller, P1);
        }
        assert_eq!(
            s.pile(P2, "library").into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
            vec![VANILLA]
        );
    }

    #[test]
    fn r1179004_with_1_left_takes_1() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE] },
        }));
        s.play(GLOVES, json!({}));
        assert_eq!(hand_defs(&s, P1), [FILLER, MENACE]);
        assert!(s.pile(P2, "library").is_empty());
    }

    #[test]
    fn r1179004_an_empty_deck_gives_nothing_and_no_fatigue() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [FILLER], "library": [] },
        }));
        let cursor = s.state().rng_cursor;
        s.play(GLOVES, json!({}));
        assert_eq!(s.state().rng_cursor, cursor);
        assert_eq!(hand_defs(&s, P1), [FILLER]);
        assert_eq!(s.state().players.p2.fatigue_count, 0);
        assert!(
            !s.events().iter().any(|event| event.event_type() == GameEventType::Fatigue),
            "no fatigue from an empty deck"
        );
    }

    #[test]
    fn r1179004_embiggened_takes_2_random_cards_of_their_hand() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [MENACE, LUNAR], "library": [VANILLA, VANILLA] },
        }));
        s.play(GLOVES, json!({ "embiggen": true }));
        s.expect_mana(P1, 2);
        // Both cards of their hand, in whatever order the shuffle took them.
        let mut hand = hand_defs(&s, P1);
        hand.sort_unstable();
        assert_eq!(hand, vec![FILLER, MENACE, LUNAR]);
        let taken = taken(&s);
        assert_eq!(taken.len(), 2);
        for card in &taken {
            assert_eq!(card.owner, P1);
        }
        assert_eq!(hand_defs(&s, P2), Vec::<String>::new());
    }

    #[test]
    fn s2_4_a_full_hand_burns_the_overflow() {
        crate::register_all();
        // The Gloves and ten more: after the Gloves are played the hand is at its cap, so the two
        // taken cards burn.
        let mut hand = vec![json!(GLOVES)];
        hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER], "library": [MENACE, LUNAR, VANILLA] },
        }));
        s.play(GLOVES, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<String> = s
            .pile(P1, "graveyard")
            .into_iter()
            .filter(|card| card.owner == P1 && card.def_id != GLOVES)
            .map(|card| card.def_id)
            .collect();
        assert_eq!(burned, vec![MENACE, LUNAR]);
    }

    #[test]
    fn r97_hidden_from_the_opponent_but_their_count() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE, LUNAR, VANILLA] },
        }));
        s.play(GLOVES, json!({}));
        // p2's view of p1's hand is a count, and nothing in it names the taken cards.
        let theirs = js(&s.view(P2));
        assert_eq!(theirs["opponent"]["hand"], json!({ "count": 3 }));
        let text = serde_json::to_string(&theirs).expect("serialisable");
        assert!(!text.contains(MENACE));
        assert!(!text.contains(LUNAR));
        // Their own deck is two cards smaller.
        assert_eq!(s.view(P2).you.library_count, 1);
    }

    #[test]
    fn r386_cards_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GLOVES, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE, LUNAR, VANILLA] },
        }));
        step_param(s.card_mut(GLOVES), "cards", 1);
        s.play(GLOVES, json!({}));
        assert_eq!(hand_defs(&s, P1), [FILLER, MENACE, LUNAR, VANILLA]);
    }

    #[test]
    fn radiant_takes_3_each_costing_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": GLOVES, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE, LUNAR, VANILLA] },
        }));
        s.play(GLOVES, json!({}));
        assert_eq!(hand_defs(&s, P1), [FILLER, MENACE, LUNAR, VANILLA]);
        for card in taken(&s) {
            assert_eq!(card.cost_override, Some(0));
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0);
        }
    }

    #[test]
    fn radiant_embiggened_takes_3_random_each_costing_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": GLOVES, "radiant": true }, FILLER] },
            "p2": { "hand": [MENACE, LUNAR, VANILLA], "library": [FILLER, FILLER] },
        }));
        s.play(GLOVES, json!({ "embiggen": true }));
        // All three cards of their hand, in whatever order the shuffle took them.
        let mut hand = hand_defs(&s, P1);
        hand.sort_unstable();
        assert_eq!(hand, vec![FILLER, VANILLA, MENACE, LUNAR]);
        let taken = taken(&s);
        assert_eq!(taken.len(), 3);
        for card in &taken {
            assert_eq!(card.cost_override, Some(0));
            assert_eq!(card.owner, P1);
        }
        assert_eq!(hand_defs(&s, P2), Vec::<String>::new());
    }
}
