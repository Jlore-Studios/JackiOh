//! C+ #39 Book Worm (SPEC §8.7 row 39): (1) Unit, Common, 1/4 → 2/8.
//!   Base:    "Start of Turn: Place a Plague Counter on this. Death: Add a random Book to your hand for
//!            each Plague Counter on this."
//!   Radiant: the same, the Books Radiant.
//!   Engine:  "It starts with no Plague Counters (balance patch 1: no N counter, no start-of-turn
//!            increment — the tokens on itself are the count). At each start of its controller's turn
//!            one Plague Counter is placed on it; its Death reads those tokens last-known (R78, R89) and
//!            adds that many random Books. The pool is the non-token Books of every set (R380, R60);
//!            a full hand burns what doesn't fit (§2.4). Tunes: none."
//!
//! The Death's count is the snapshot's own counters (`plagueOn(ctx.self)`), so a bounced worm keeps
//! nothing (R78) and a worm that died with none adds none.

use jackioh_engine::effects::{add_random_from_catalog, place_plague};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-039";

/// The printed "a Plague Counter": one per start of turn.
const PLAGUE_PER_TURN: i32 = 1;

/// The tokens on itself as it died (R89), or none when it has no self.
fn tokens_on_self(self_: Option<&CardInstance>) -> i32 {
    match self_ {
        None => 0,
        Some(card) => plague_on(card),
    }
}

fn book_worm(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![place_plague(json_as(json!({ "target": { "of": "self" }, "amount": PLAGUE_PER_TURN })))]
        })),
        death: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "tags": ["Book"] }, "count": tokens_on_self(ctx.self_.as_ref()) });
            if radiant {
                args["radiant"] = json!(radiant);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        // R280: the tokens stacked on itself are the Books its Death would add — the public count.
        preview: Some(condition_hook(|c| {
            vec![PreviewValue {
                label: "Plague Counter".to_string(),
                value: tokens_on_self(Some(c.self_)),
                display: None,
                ids: None,
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: book_worm(false),
        radiant: book_worm(true),
    }
}

// C+ #39 Book Worm — SPEC §8.7 row 39, BUILD M9 Classic+ row C+ 39: "It starts with no Plague Counters
// (balance patch 1: no N counter — the tokens stacked on itself are the count); at each start of its
// controller's turn one Plague Counter is placed on it; Death adds one random non-token Book of any set
// (R380, repeats allowed) per token, reading the tokens last-known (R78, R89), a full hand burning the
// rest; bounced and played again it starts at none; the public token count is the preview both players
// see; no tunable; radiant the Books are Radiant".
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WORM: &str = "classicplus-039";
    const MENACE: &str = "core-019"; // 9/9: the worm dies attacking it.
    const FLOOD: &str = "core-017"; // (4) Spell: bounce all Units.
    const FILLER: &str = "core-005";

    /// The harness's `scenario`, with the shipped cards registered first (the TS harness did it at import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn on_field(radiant: bool, hand: &[&str]) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": WORM, "radiant": radiant }], "library": [FILLER, FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": [MENACE], "library": [FILLER, FILLER, FILLER, FILLER] },
        }))
    }

    /// Two `endTurn`s: the opponent's turn and back to the start of ours.
    fn next_own_turn(s: &mut Scenario) -> &mut Scenario {
        s.end_turn().end_turn()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.id).collect()
    }

    fn books_in(s: &Scenario, before: &[String]) -> Vec<String> {
        s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).map(|card| card.def_id).collect()
    }

    /// The Plague Counters stacked on the Worm (§10.1, public in every view).
    fn tokens_of(s: &Scenario, card: &str) -> i32 {
        s.card(card).counters.plague.unwrap_or(0)
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    /// TS `stepParam(s.card(ref), key, delta)` on the live card.
    fn step(s: &mut Scenario, card: &str, key: &str, delta: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the game");
        step_param(live, key, delta);
    }

    mod base {
        use super::*;

        #[test]
        fn it_starts_with_no_plague_counters_dying_at_once_adds_no_book() {
            let mut s = on_field(false, &[FILLER]);
            assert_eq!(tokens_of(&s, WORM), 0);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            s.expect_in_zone(WORM, "graveyard");
            assert_eq!(books_in(&s, &before).len(), 0);
        }

        #[test]
        fn r380_the_books_are_random_non_token_book_cards_of_any_set_never_the_worm() {
            let mut sets: BTreeSet<String> = BTreeSet::new();
            for seed in ["bw-1", "bw-2", "bw-3", "bw-4", "bw-5", "bw-6", "bw-7", "bw-8"] {
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [FILLER], "field": [WORM], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                next_own_turn(&mut s); // One token: one Book.
                assert_eq!(tokens_of(&s, WORM), 1);
                let before = hand_ids(&s);
                s.attack(WORM, MENACE);
                let added = books_in(&s, &before);
                assert_eq!(added.len(), 1);
                for id in &added {
                    let def = def_of(Some(s.state()), id);
                    assert!(def.tags.contains(&Tag::Book));
                    assert!(!def.token);
                    assert_ne!(id, WORM);
                    sets.insert(def.set.to_string());
                }
            }
            assert!(sets.len() > 1);
        }

        #[test]
        fn one_token_lands_at_each_start_of_its_controllers_turn_never_the_opponents() {
            let mut s = on_field(false, &[FILLER, FILLER]);
            let worm = s.card(WORM).id.clone();
            s.end_turn(); // p2's start of turn: no token.
            assert_eq!(tokens_of(&s, &worm), 0);
            s.end_turn(); // p1's: the first token.
            assert_eq!(tokens_of(&s, &worm), 1);
            next_own_turn(&mut s); // The second.
            assert_eq!(tokens_of(&s, &worm), 2);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            assert_eq!(books_in(&s, &before).len(), 2);
        }

        #[test]
        fn r78_r89_death_reads_the_tokens_last_known_the_count_it_had_as_it_died() {
            let mut s = on_field(false, &[FILLER, FILLER]);
            next_own_turn(&mut s);
            next_own_turn(&mut s);
            assert_eq!(tokens_of(&s, WORM), 2);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            assert_eq!(books_in(&s, &before).len(), 2);
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_books_that_do_not_fit() {
            let hand: Vec<&str> = (0..HAND_CAP - 1).map(|_| FILLER).collect();
            let mut s = on_field(false, &hand);
            next_own_turn(&mut s); // One token; the turn's draw fills the hand to the cap.
            assert_eq!(tokens_of(&s, WORM), 1);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            s.attack(WORM, MENACE);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let burned = s
                .events()
                .iter()
                .map(|event| serde_json::to_value(event).expect("an event is JSON"))
                .filter(|event| event["type"] == "burned")
                .count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r78_bounced_and_played_again_it_starts_at_no_tokens() {
            let mut s = on_field(false, &[FLOOD, FILLER, FILLER]);
            next_own_turn(&mut s); // One token.
            assert_eq!(tokens_of(&s, WORM), 1);
            s.play(FLOOD, json!({}));
            s.expect_in_zone(WORM, "hand");
            // With no mana left the turn may end by itself (§2.5); come round to p1's next turn either way.
            let turn = s.state().turn;
            while s.state().active != P1 || s.state().turn == turn {
                s.end_turn();
            }
            s.play(WORM, json!({}));
            assert_eq!(tokens_of(&s, WORM), 0);
        }

        #[test]
        fn r280_the_token_count_is_public_on_the_field_to_both_players_in_hand_it_reads_no_tokens() {
            let mut s = scenario(json!({
                "p1": { "hand": [WORM, FILLER], "field": [WORM], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": [FILLER, FILLER] },
            }));
            next_own_turn(&mut s);
            assert_eq!(view(&s, P1)["you"]["units"][0]["counters"]["plague"], 1);
            assert_eq!(view(&s, P2)["opponent"]["units"][0]["counters"]["plague"], 1);
            let in_hand = view(&s, P1)["you"]["hand"].clone();
            let Some(cards) = in_hand.as_array() else {
                panic!("own hand in full");
            };
            let worm_in_hand = cards.iter().find(|card| card["defId"] == WORM);
            assert!(worm_in_hand.is_none_or(|card| card.get("counters").is_none()));
            // The text names what the counters are.
            assert!(def_of(Some(s.state()), WORM).base.text.contains("Plague Counter"));
            // What the counters say is what the Death then adds.
            let before = hand_ids(&s);
            let attacker = s.unit(P1, 1).map(|card| card.id).unwrap_or_default();
            s.attack(&attacker, MENACE);
            assert_eq!(books_in(&s, &before).len(), 1);
        }

        #[test]
        fn r97_the_books_it_adds_are_hidden_from_the_opponent() {
            let mut s = on_field(false, &[FILLER]);
            next_own_turn(&mut s);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            let added: Vec<String> =
                s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).map(|card| card.def_id).collect();
            assert_eq!(added.len(), 1);
            let shown: Vec<Value> = view(&s, P2)["events"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|event| event["type"] == "addedToHand")
                .collect();
            assert!(!shown.is_empty());
            for event in &shown {
                let def_id = event["defId"].as_str().unwrap_or_default().to_string();
                assert!(!added.contains(&def_id));
            }
        }

        #[test]
        fn r386_there_is_no_tunable_growth_an_upgrade_still_stacks_one_token_a_turn() {
            let mut s = on_field(false, &[FILLER, FILLER]);
            step(&mut s, WORM, "growth", 1);
            next_own_turn(&mut s);
            assert_eq!(tokens_of(&s, WORM), 1);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            assert_eq!(books_in(&s, &before).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_books_it_adds_are_radiant_one_per_token() {
            let mut s = on_field(true, &[FILLER, FILLER]);
            next_own_turn(&mut s);
            next_own_turn(&mut s);
            let before = hand_ids(&s);
            s.attack(WORM, MENACE);
            let added: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
            assert_eq!(added.len(), 2);
            assert!(added
                .iter()
                .all(|card| card.radiant && def_of(Some(s.state()), &card.def_id).tags.contains(&Tag::Book)));
        }

        #[test]
        fn r280_its_token_count_is_public_on_the_radiant_face_too() {
            let mut s = on_field(true, &[FILLER, FILLER]);
            next_own_turn(&mut s);
            assert_eq!(view(&s, P1)["you"]["units"][0]["counters"]["plague"], 1);
        }
    }
}
