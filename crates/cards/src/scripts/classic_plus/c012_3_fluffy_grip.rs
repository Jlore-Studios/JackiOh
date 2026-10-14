//! C+ #12.3 Fluffy Grip (SPEC §8.7 row 12.3): (1) Spell, Pancake, Token (printed Legendary).
//!   Base:    "Steal a random Unit from your opponent's deck and put it in your hand. It costs (0)."
//!   Radiant: "… It costs (0) and becomes Radiant."
//! E2/E16 (`takeFromLibrary`): a random Unit card of their library becomes yours, in your hand, with
//! `costOverride` 0; a full hand burns it into your graveyard, now its owner's; none, nothing (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-3";

pub fn script() -> CardScripts {
    CardScripts {
        // "It costs (0)": the declared number `setCost` (R386), on both faces.
        base: Script {
            cry: Some(hook(|ctx| {
                vec![take_from_library(json_as(json!({
                    "from": "enemy",
                    "pick": "random",
                    "filter": { "type": "Unit" },
                    "costOverride": param(&*ctx, "setCost"),
                })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| {
                vec![take_from_library(json_as(json!({
                    "from": "enemy",
                    "pick": "random",
                    "filter": { "type": "Unit" },
                    "costOverride": param(&*ctx, "setCost"),
                    "radiant": true,
                })))]
            })),
            ..Script::default()
        },
    }
}

// C+ #12.3 Fluffy Grip — SPEC §8.7 row 12.3, BUILD M9 Classic+ row C+ 12.3: "A random Unit card of the
// opponent's deck (R60) moves to your hand and becomes yours (R12), `costOverride` 0; a deck with no
// Unit, or an empty deck, gives nothing and draws nothing (R129); a full hand burns it into your
// graveyard, now its owner's (R12); `stolen` names the card only to you, the opponent reading the
// sentinel and a deck one card smaller (R97); a unit-token card follows R11; radiant it also becomes
// Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const GRIP: &str = "classicplus-012-3";
    const MENACE: &str = "core-019"; // (3) Unit
    const ROCK: &str = "core-066"; // (4) Unit
    const LUNAR: &str = "core-035"; // a Spell
    const FILLER: &str = "core-005"; // a Spell
    const RUSH_TOKEN: &str = "core-t-rush"; // a unit-token card (R11)

    use crate::merged;

    use crate::matches_object;

    fn grip(radiant: bool, p2_library: Value, p1: Value, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "p1": merged(json!({ "hand": [{ "def": GRIP, "radiant": radiant }, FILLER] }), p1),
            "p2": { "hand": [FILLER], "library": p2_library },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        let mut s = scenario(options);
        s.play(GRIP, json!({}));
        s
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn r386_a_degrade_makes_the_taken_unit_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GRIP, FILLER] },
            "p2": { "hand": [FILLER], "library": [MENACE] },
        }));
        assert!(!crate::can_upgrade_number(&s, GRIP, "setCost"));
        assert_eq!(crate::degrade_number(&mut s, GRIP, "setCost"), 1);
        s.play(GRIP, json!({}));
        let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE);
        assert_eq!(taken.and_then(|card| card.cost_override), Some(1));
    }

    mod base {
        use super::*;

        #[test]
        fn r12_a_unit_card_of_the_opponent_s_deck_moves_to_your_hand_becomes_yours_and_costs_0() {
            crate::register_all();
            let s = grip(false, json!([LUNAR, MENACE, FILLER]), json!({}), None);
            let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE);
            assert!(taken.is_some());
            assert_eq!(taken.as_ref().map(|card| card.owner), Some(P1));
            assert_eq!(taken.as_ref().and_then(|card| card.cost_override), Some(0));
            let card = taken.unwrap_or_else(|| s.card(MENACE).clone());
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0);
            assert_eq!(def_ids(&s.pile(P2, "library")), vec![LUNAR, FILLER]);
        }

        #[test]
        fn r60_the_unit_is_a_random_one_of_the_deck_s_units_never_a_spell() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..16 {
                let s = grip(false, json!([LUNAR, MENACE, FILLER, ROCK, LUNAR]), json!({}), Some(&format!("grip-{i}")));
                let taken: Vec<CardInstance> =
                    s.hand(P1).into_iter().filter(|card| card.owner == P1 && card.def_id != FILLER).collect();
                assert_eq!(taken.len(), 1);
                seen.insert(taken.first().map(|card| card.def_id.clone()).unwrap_or_default());
            }
            let mut seen: Vec<String> = seen.into_iter().collect();
            seen.sort();
            let mut want = vec![MENACE.to_string(), ROCK.to_string()];
            want.sort();
            assert_eq!(seen, want);
        }

        #[test]
        fn r129_a_deck_with_no_unit_or_an_empty_one_gives_nothing_and_draws_nothing_from_the_rng() {
            crate::register_all();
            let mut spells = scenario(json!({
                "p1": { "hand": [GRIP, FILLER] },
                "p2": { "hand": [FILLER], "library": [LUNAR, FILLER] },
            }));
            let cursor = spells.state().rng_cursor;
            spells.play(GRIP, json!({}));
            assert_eq!(spells.state().rng_cursor, cursor);
            assert_eq!(spells.pile(P2, "library").len(), 2);
            assert_eq!(def_ids(&spells.hand(P1)), vec![FILLER]);

            let empty = grip(false, json!([]), json!({}), None);
            assert_eq!(def_ids(&empty.hand(P1)), vec![FILLER]);
        }

        #[test]
        fn r12_a_full_hand_burns_it_into_your_graveyard_now_its_owner_s() {
            crate::register_all();
            // Grip and ten more: after Grip is played the hand is at its cap.
            let mut hand = vec![json!(GRIP)];
            hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
            let s = grip(false, json!([MENACE]), json!({ "hand": hand }), None);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let burned = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == MENACE);
            assert_eq!(burned.map(|card| card.owner), Some(P1));
            assert!(!s.pile(P2, "graveyard").iter().any(|card| card.def_id == MENACE));
        }

        #[test]
        fn r11_r218_a_unit_token_card_leaves_a_library_only_by_a_draw_so_the_steal_passes_over_it() {
            crate::register_all();
            let mut only = scenario(json!({
                "p1": { "hand": [GRIP, FILLER] },
                "p2": { "hand": [FILLER], "library": [RUSH_TOKEN] },
            }));
            let cursor = only.state().rng_cursor;
            only.play(GRIP, json!({}));
            assert_eq!(def_ids(&only.pile(P2, "library")), vec![RUSH_TOKEN]);
            assert_eq!(def_ids(&only.hand(P1)), vec![FILLER]);
            assert_eq!(only.state().rng_cursor, cursor);

            for i in 0..6 {
                let s = grip(
                    false,
                    json!([RUSH_TOKEN, MENACE, RUSH_TOKEN]),
                    json!({}),
                    Some(&format!("grip-token-{i}")),
                );
                assert_eq!(def_ids(&s.hand(P1)), vec![FILLER, MENACE]);
                assert_eq!(def_ids(&s.pile(P2, "library")), vec![RUSH_TOKEN, RUSH_TOKEN]);
            }
        }

        #[test]
        fn r97_stolen_names_the_card_to_you_only_the_opponent_sees_the_sentinel_and_a_smaller_deck() {
            crate::register_all();
            let s = grip(false, json!([MENACE, FILLER]), json!({}), None);
            let mine = s.view(P1).events.into_iter().find(|event| event.event_type() == GameEventType::Stolen);
            assert!(matches_object(
                &serde_json::to_value(&mine).unwrap(),
                &json!({ "type": "stolen", "defId": MENACE, "from": "p2", "to": "p1" })
            ));
            let theirs = s.view(P2).events.into_iter().find(|event| event.event_type() == GameEventType::Stolen);
            assert!(theirs.is_some());
            if let Some(GameEvent::Stolen { def_id, .. }) = &theirs {
                assert_ne!(def_id, MENACE);
            }
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(MENACE));
            assert_eq!(s.view(P2).you.library_count, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_stolen_unit_also_becomes_radiant() {
            crate::register_all();
            let s = grip(true, json!([MENACE]), json!({}), None);
            let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE);
            assert_eq!(taken.as_ref().map(|card| card.radiant), Some(true));
            assert_eq!(taken.as_ref().and_then(|card| card.cost_override), Some(0));
            assert_eq!(taken.as_ref().map(|card| card.owner), Some(P1));
        }
    }
}
