//! C #34 Ancient Acquisition (SPEC §8.6 row 34, §6.3 Add to hand; R4, R70, R97, R317, R684).
//! Spell, cost 1, Rare.
//!   Base:    "Return {cards|random card|random cards} from your graveyard to hand." (2)
//!   Radiant: "Return {cards|random card|random cards} from your graveyard or exile to hand." (4)
//!   Engine:  "That many random cards from the pile or piles, drawn uniformly through the match rng
//!            (balance patch 1: no pick prompt; R684); fewer cards than asked ends it; the hand cap
//!            applies (R4). C #47 Recurring Felinor casts it. Tunes: cards 2 ↑."
//!
//! Each return moves through §2.4's pipeline (`addRandomFromGraveyard`): a full hand burns it into
//! your graveyard (R4, R317). A card in your hand is yours to read alone again (R97). The Spell
//! itself is resolving (§10.5), in no pile, so it is never one of its own returns.
//!
//! Cast by C #47 Recurring Felinor, the returns are still its caster's (R70).

use jackioh_engine::prelude::*;
use jackioh_engine::effects::add_random_from_graveyard;

pub const ID: &str = "classic-034";

fn acquire(exile: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "count": param(ctx, "cards") });
            if exile {
                args["exile"] = json!(true);
            }
            vec![add_random_from_graveyard(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = acquire(false);

    let radiant = acquire(true);

    CardScripts { base, radiant }
}

// C #34 Ancient Acquisition — SPEC §8.6 row 34, BUILD M9 Classic row C 34: "Return 2 random cards
// from your graveyard to hand, with no prompt (R684; fewer if fewer; an empty graveyard: nothing);
// the resolving Spell is never one of its own returns; a full hand burns the overflow (R317); the
// returned cards follow R97 in the opponent's view once in your hand; cast by C #47 the returns are
// still its caster's; radiant: up to 4 from your graveyard or your exile; its tuned number (cards)
// reads through `param()` (R386)".
//
// The C #47 case casts this card from C #47 Recurring Felinor's Cry (B5 E12).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const ACQUIRE: &str = "classic-034";
    const RECURRING: &str = "classic-047"; // C #47 Recurring Felinor, (2) Unit: "Cry: Cast Ancient Acquisition."
    const FILLER: &str = "core-005"; // (1) Spell, a spare card (§2.5).
    // Graveyard and exile cards, one definition each so a def id names one card.
    const MENACE: &str = "core-019"; // (3) Unit
    const SEVEN: &str = "core-025"; // (4) Unit
    const FELINORS: &str = "core-012"; // (2) Unit
    const VANILLA: &str = "core-008"; // (1) Unit
    const REPLENISH: &str = "core-010"; // (0) Spell
    const ECLIPSE: &str = "core-035"; // (1) Spell
    const MANA_WELL: &str = "core-006"; // (3) Field Spell

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// `opts[key]`, or `fallback` where the TS default (`??`) applies.
    fn or(opts: &Value, key: &str, fallback: Value) -> Value {
        if opts[key].is_null() { fallback } else { opts[key].clone() }
    }

    /// `piles`: `graveyard`, `exile`, `hand`, as the TS helper's (`{}` for none).
    fn acquire(radiant_face: bool, piles: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ACQUIRE, "radiant": radiant_face })];
        hand.extend(or(&piles, "hand", json!([FILLER])).as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": {
                "hand": hand,
                "graveyard": or(&piles, "graveyard", json!([MENACE, SEVEN, FELINORS, VANILLA, REPLENISH])),
                "exile": or(&piles, "exile", json!([])),
            },
            "p2": { "hand": [FILLER] },
        }))
    }

    mod c_34_ancient_acquisition {
        use super::*;

        #[test]
        fn declares_its_one_number_cards_r386_2_radiant_4() {
            crate::register_all();
            assert_eq!(
                js(&registered_catalog()[ID])["params"],
                json!([{ "key": "cards", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 1 }]),
            );
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn r684_returns_2_random_cards_with_no_prompt_from_the_graveyard() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(PlayerId::P1).len(), 3);
                let grave: Vec<String> =
                    s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(grave.len(), 4);
                assert!(grave.iter().any(|def_id| def_id == ACQUIRE));
                // The two in hand came from the graveyard's five.
                let in_hand: Vec<String> = s
                    .hand(PlayerId::P1)
                    .iter()
                    .filter(|card| card.def_id != FILLER)
                    .map(|card| card.def_id.clone())
                    .collect();
                assert_eq!(in_hand.len(), 2);
                for def_id in &in_hand {
                    assert!([MENACE, SEVEN, FELINORS, VANILLA, REPLENISH].contains(&def_id.as_str()));
                }
            }

            #[test]
            fn r684_the_random_returns_come_from_the_match_rng_the_same_game_returns_the_same_cards() {
                crate::register_all();
                let mut first = acquire(false, json!({}));
                first.play(ACQUIRE, json!({}));
                let mut second = acquire(false, json!({}));
                second.play(ACQUIRE, json!({}));
                let ids = |s: &Scenario| -> Vec<Value> {
                    s.events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "addedToHand" && event["player"] == "p1")
                        .map(|event| event["instanceId"].clone())
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r684_each_return_is_its_own_addedtohand_event_naming_the_card_to_you() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                s.play(ACQUIRE, json!({}));
                assert_eq!(
                    s.events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "addedToHand" && event["player"] == "p1")
                        .count(),
                    2,
                );
            }

            #[test]
            fn fewer_if_fewer_a_graveyard_of_one_card_returns_it() {
                crate::register_all();
                let mut s = acquire(false, json!({ "graveyard": [SEVEN] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(SEVEN, "hand");
            }

            #[test]
            fn an_empty_graveyard_asks_nothing_and_the_spell_lands_there() {
                crate::register_all();
                let mut s = acquire(false, json!({ "graveyard": [] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                let grave: Vec<String> =
                    s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(grave, vec![ACQUIRE]);
            }

            #[test]
            fn the_resolving_spell_is_never_one_of_its_own_returns() {
                crate::register_all();
                let mut s = acquire(false, json!({ "graveyard": [VANILLA] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(VANILLA, "hand");
                s.expect_in_zone(ACQUIRE, "graveyard");
            }

            #[test]
            fn r317_a_full_hand_burns_the_overflow_back_into_your_graveyard_both_players_reading_which() {
                crate::register_all();
                let fillers: Vec<&str> = (0..9).map(|_| FILLER).collect();
                let mut s = acquire(false, json!({ "hand": fillers }));
                s.play(ACQUIRE, json!({}));
                assert_eq!(s.hand(PlayerId::P1).len(), 10);
                let burned: Vec<Value> = s.events().iter().map(js).filter(|event| event["type"] == "burned").collect();
                assert_eq!(burned.len(), 1);
                let burned_id = burned
                    .first()
                    .filter(|event| event["type"] == "burned")
                    .and_then(|event| event["instanceId"].as_str())
                    .unwrap_or("")
                    .to_string();
                s.expect_in_zone(&burned_id, "graveyard");
                let theirs: Vec<Value> =
                    s.view(PlayerId::P2).events.iter().map(js).filter(|event| event["type"] == "burned").collect();
                assert_eq!(theirs, burned);
            }

            #[test]
            fn r97_once_in_your_hand_the_returned_cards_are_named_in_no_event_or_pile_of_the_opponents_view() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                s.play(ACQUIRE, json!({}));
                let returned: Vec<(String, String)> = s
                    .hand(PlayerId::P1)
                    .iter()
                    .filter(|card| card.def_id != FILLER)
                    .map(|card| (card.id.clone(), card.def_id.clone()))
                    .collect();
                assert_eq!(returned.len(), 2);
                let theirs = serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serialises");
                for (id, def_id) in &returned {
                    assert!(!theirs.contains(&format!("\"{id}\"")));
                    assert!(!theirs.contains(def_id.as_str()));
                }
                let mine = serde_json::to_string(&s.view(PlayerId::P1)).expect("a view serialises");
                assert!(mine.contains(&format!("\"{}\"", returned[0].0)));
            }

            #[test]
            fn only_your_own_graveyard_the_opponents_graveyard_and_your_exile_are_untouched() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ACQUIRE, FILLER], "graveyard": [MENACE], "exile": [VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [SEVEN, FELINORS] },
                }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                // The one own-graveyard card is the only thing that could come: it did.
                s.expect_in_zone(MENACE, "hand");
                assert_eq!(s.pile(PlayerId::P2, "graveyard").len(), 2);
                let exile: Vec<String> =
                    s.pile(PlayerId::P1, "exile").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(exile, vec![VANILLA]);
            }

            #[test]
            fn r177_r684_no_prompt_opens_so_the_opponent_reads_only_the_public_events() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert!(s.view(PlayerId::P2).pending.is_none());
            }

            #[test]
            fn r386_an_upgrade_of_cards_lets_it_return_3() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                step_param(s.card_mut(ACQUIRE), "cards", 1);
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(PlayerId::P1).len(), 4);
                assert_eq!(s.pile(PlayerId::P1, "graveyard").len(), 3);
            }

            #[test]
            fn r386_a_degrade_of_cards_lets_it_return_1() {
                crate::register_all();
                let mut s = acquire(false, json!({}));
                step_param(s.card_mut(ACQUIRE), "cards", -1);
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(PlayerId::P1).len(), 2);
                assert_eq!(s.pile(PlayerId::P1, "graveyard").len(), 5);
            }

            #[test]
            fn r70_cast_by_c_47_recurring_felinor_the_returns_are_still_its_casters() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RECURRING, FILLER], "graveyard": [MENACE, VANILLA] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(RECURRING, json!({}));
                assert!(s.state().pending.is_none());
                let mut got: Vec<String> = s.hand(PlayerId::P1).iter().map(|card| card.def_id.clone()).collect();
                got.sort();
                let mut want = vec![FILLER, MENACE, VANILLA];
                want.sort();
                assert_eq!(got, want);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r684_returns_up_to_4_at_random_from_your_graveyard_and_your_exile() {
                crate::register_all();
                let mut s = acquire(
                    true,
                    json!({ "graveyard": [MENACE, SEVEN, FELINORS], "exile": [VANILLA, ECLIPSE, MANA_WELL] }),
                );
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(PlayerId::P1).len(), 5);
                // Six pooled cards, four taken: two stay, plus the spent Spell in the graveyard.
                assert_eq!(s.pile(PlayerId::P1, "graveyard").len() + s.pile(PlayerId::P1, "exile").len(), 3);
            }

            #[test]
            fn returns_cards_from_both_piles_to_your_hand() {
                crate::register_all();
                let mut s = acquire(true, json!({ "graveyard": [MENACE, SEVEN], "exile": [VANILLA, MANA_WELL] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                // All four pooled cards come: both piles empty but for the spent Spell.
                assert_eq!(s.hand(PlayerId::P1).len(), 5);
                let grave: Vec<String> =
                    s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(grave, vec![ACQUIRE]);
                assert!(s.pile(PlayerId::P1, "exile").is_empty());
            }

            #[test]
            fn an_exile_alone_returns_from_exile() {
                crate::register_all();
                let mut s = acquire(true, json!({ "graveyard": [], "exile": [ECLIPSE] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(ECLIPSE, "hand");
                assert!(s.pile(PlayerId::P1, "exile").is_empty());
            }

            #[test]
            fn only_your_own_piles_the_opponents_piles_are_untouched() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": ACQUIRE, "radiant": true }, FILLER], "graveyard": [MENACE], "exile": [VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [SEVEN], "exile": [FELINORS] },
                }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                // Both own cards come (2 of the 4 asked take all there is).
                s.expect_in_zone(MENACE, "hand");
                s.expect_in_zone(VANILLA, "hand");
                assert_eq!(s.pile(PlayerId::P2, "graveyard").len(), 1);
                assert_eq!(s.pile(PlayerId::P2, "exile").len(), 1);
            }

            #[test]
            fn both_piles_empty_asks_nothing() {
                crate::register_all();
                let mut s = acquire(true, json!({ "graveyard": [], "exile": [] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(ACQUIRE, "graveyard");
            }

            #[test]
            fn r97_a_card_returned_from_exile_is_hidden_from_the_opponent_once_in_your_hand() {
                crate::register_all();
                let mut s = acquire(true, json!({ "graveyard": [], "exile": [MANA_WELL] }));
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                let well = s.card(MANA_WELL).id.clone();
                s.expect_in_zone(&well, "hand");
                let theirs = serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serialises");
                assert!(!theirs.contains(&format!("\"{well}\"")));
                assert!(!theirs.contains(MANA_WELL));
            }

            #[test]
            fn r386_a_degrade_of_cards_lets_it_return_3() {
                crate::register_all();
                let mut s = acquire(true, json!({ "graveyard": [MENACE, SEVEN, FELINORS], "exile": [VANILLA, ECLIPSE] }));
                step_param(s.card_mut(ACQUIRE), "cards", -1);
                s.play(ACQUIRE, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(PlayerId::P1).len(), 4);
            }
        }
    }
}
