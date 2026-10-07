//! C+ #77 Anti-Softlock (SPEC §8.7 row 77, E20, E21, E38, R81, R346, R386, R440). (2) Spell, Epic.
//!   Draw {draw}. Every card on the field, in hands and in decks gains Stack and Pierce. Unlock every
//!   zone. Radiant: Draw {draw}. Choose all cards or only yours: each of them gains Stack and Pierce.
//!   Unlock every zone.
//!
//! In the order written, so the drawn card gains the keywords too. The grants are E38's: on the field
//! at once, in a hand or a deck carried onto the field as the card enters, silent on a card someone may
//! not read (R440). This card, resolving, is in none of those zones, and a card dormant under a Stack is
//! not on the field. Stack on a backrow card lets it be played onto an occupied backrow zone (E21); on a
//! Spell, Pierce is R346's and Stack does nothing. Unlock clears every Locked zone of both players and
//! leaves a reserved one reserved. The Radiant's choice is a mode declared at play (R81).

use jackioh_engine::effects::{chosen_options, draw, grant_keyword_cards, unlock_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-077";

const ALL: &str = "All cards";
const YOURS: &str = "Only yours";

fn anti_softlock(side: impl Fn(&EffectContext<'_>) -> &'static str + Send + Sync + 'static) -> Hook {
    hook(move |ctx| {
        let scope = json!({ "side": side(&*ctx), "zones": ["field", "hand", "library"] });
        vec![
            draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            grant_keyword_cards(json_as(json!({ "scope": scope, "keyword": { "kind": "Stack" } }))),
            grant_keyword_cards(json_as(json!({ "scope": scope, "keyword": { "kind": "Pierce" } }))),
            unlock_all(Default::default()),
        ]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(anti_softlock(|_ctx| "any")),
            ..Script::default()
        },
        radiant: Script {
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: vec![ALL.to_string(), YOURS.to_string()],
            }],
            cry: Some(anti_softlock(|ctx| {
                if chosen_options(ctx).first().map(String::as_str) == Some(YOURS) {
                    "self"
                } else {
                    "any"
                }
            })),
            ..Script::default()
        },
    }
}

// C+ #77 Anti-Softlock — SPEC §8.7 row 77, E20, E21, E38, R81, R97, R346, R386, R440, BUILD M9 row C+ 77.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const SOFTLOCK: &str = "classicplus-077";
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const ARMORED: &str = "core-025"; // 7/7, Armor 7
    const ECLIPSE: &str = "core-035"; // (1) Spell: deal 3 damage to a target
    const MANA_WELL: &str = "core-006"; // a Field Spell
    const CONJURE_KY: &str = "core-057"; // (2) Spell: add 3 random KY cards to your hand
    const FILLER: &str = "core-005";
    const ALL: &str = "All cards";
    const YOURS: &str = "Only yours";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// TS's live `s.card(ref)`, written through: the card under that id in the state.
    fn card_mut<'a>(s: &'a mut Scenario, card: &str) -> &'a mut CardInstance {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state")
    }

    fn zone(player: &str, row: &str, lane: i32) -> ZoneRef {
        json_as(json!({ "player": player, "row": row, "lane": lane }))
    }

    fn softlock(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": spread(
                json!({ "hand": [{ "def": SOFTLOCK, "radiant": radiant }, VANILLA], "library": [TIMMY, FILLER, FILLER] }),
                &p1,
            ),
            "p2": spread(json!({ "hand": [VANILLA], "field": [TIMMY], "library": [FILLER] }), &p2),
        }))
    }

    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| js(keyword)["kind"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The keywords a card was granted, wherever it is (E38).
    fn granted(s: &Scenario, card: &str) -> Vec<String> {
        kinds(&s.card(card).granted_keywords)
    }

    fn stack_and_pierce() -> Vec<String> {
        vec!["Stack".to_string(), "Pierce".to_string()]
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    mod c_n77_anti_softlock {
        use super::*;

        #[test]
        fn draws_its_declared_draw_the_radiant_face_declares_the_choice() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, SOFTLOCK);
            let params: Option<Vec<Value>> = def.params.as_ref().map(|params| {
                params
                    .iter()
                    .map(|entry| {
                        let entry = js(entry);
                        json!([entry["key"], entry["base"], entry["radiant"]])
                    })
                    .collect()
            });
            assert_eq!(params, Some(vec![json!(["draw", 1, 2])]));
            let scripts = super::super::script();
            assert!(scripts.base.modes.is_empty());
            assert_eq!(js(&scripts.radiant.modes), json!([{ "kind": "mode", "options": [ALL, YOURS] }]));
        }

        mod base {
            use super::*;

            #[test]
            fn e38_draws_1_then_every_card_on_the_field_in_both_hands_and_in_both_decks_gains_stack_and_pierce_the_drawn_card_too()
             {
                let mut s = softlock(false, json!({ "field": [ARMORED], "backrow": [MANA_WELL] }), json!({}));
                s.play(SOFTLOCK, json!({}));
                let hand: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(hand, vec![VANILLA, TIMMY]);
                let mut everywhere: Vec<Option<CardInstance>> = Vec::new();
                everywhere.extend(s.hand(P1).into_iter().map(Some));
                everywhere.extend(s.pile(P1, "library").into_iter().map(Some));
                everywhere.extend(s.hand(P2).into_iter().map(Some));
                everywhere.extend(s.pile(P2, "library").into_iter().map(Some));
                everywhere.push(s.unit(P1, 1));
                everywhere.push(s.backrow(P1, 1));
                everywhere.push(s.unit(P2, 1));
                for card in everywhere {
                    let card = card.expect("a card in every place the test reads");
                    assert_eq!(granted(&s, &card.id), stack_and_pierce());
                }
                let theirs = s.unit(P2, 1).expect("p2's unit").id;
                let shown = kinds(&s.stats(theirs.as_str()).keywords);
                assert!(shown.iter().any(|kind| kind == "Stack") && shown.iter().any(|kind| kind == "Pierce"));
            }

            #[test]
            fn this_card_resolving_gains_nothing_a_card_dormant_under_a_stack_is_not_on_the_field() {
                let mut s = softlock(
                    false,
                    json!({}),
                    json!({ "hand": [VANILLA], "field": [VANILLA, { "def": "core-092", "stack": true }], "library": [FILLER] }),
                );
                // The pile reads top first: the Fiender on top, the Vanilla dormant beneath it.
                let buried = s
                    .state()
                    .players
                    .p2
                    .units
                    .first()
                    .and_then(|pile| pile.as_ref())
                    .and_then(|pile| pile.get(1))
                    .cloned()
                    .expect("a card beneath the top of p2's first pile");
                s.play(SOFTLOCK, json!({}));
                assert!(granted(&s, SOFTLOCK).is_empty());
                assert!(granted(&s, &buried.id).is_empty());
            }

            #[test]
            fn e38_a_hand_unit_carries_them_onto_the_field_and_is_played_onto_an_occupied_unit_zone_stack() {
                let mut s = softlock(false, json!({ "field": [ARMORED] }), json!({}));
                s.play(SOFTLOCK, json!({}));
                s.play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
                let pile: Option<Vec<String>> = s
                    .state()
                    .players
                    .p1
                    .units
                    .first()
                    .and_then(|pile| pile.as_ref())
                    .map(|pile| pile.iter().map(|card| card.def_id.clone()).collect());
                assert_eq!(pile, Some(vec![VANILLA.to_string(), ARMORED.to_string()]));
                let top = s.unit(P1, 1).expect("a unit in lane 1").id;
                let shown = kinds(&s.stats(top.as_str()).keywords);
                assert!(shown.iter().any(|kind| kind == "Stack") && shown.iter().any(|kind| kind == "Pierce"));
            }

            #[test]
            fn e21_a_backrow_card_with_stack_is_played_onto_an_occupied_backrow_zone_a_pile_whose_top_acts() {
                let mut s = softlock(
                    false,
                    json!({ "hand": [{ "def": SOFTLOCK }, MANA_WELL], "backrow": [MANA_WELL], "mana": 5 }),
                    json!({}),
                );
                let under = s.backrow(P1, 1).expect("a Mana Well in lane 1");
                s.play(SOFTLOCK, json!({}));
                let top = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == MANA_WELL)
                    .expect("a Mana Well in hand");
                s.play(&top, json!({ "zone": 1 }));
                assert_eq!(s.backrow(P1, 1).map(|card| card.id), Some(top.id.clone()));
                assert_eq!(ids(&beneath_at(s.state(), zone("p1", "backrow", 1))), vec![under.id]);
            }

            #[test]
            fn r13_e21_in_a_backrow_pile_only_the_top_acts_two_mana_wells_piled_give_1_mana_at_the_start_of_turn_not_2() {
                let mut s = softlock(
                    false,
                    json!({ "hand": [{ "def": SOFTLOCK }, MANA_WELL], "backrow": [MANA_WELL], "mana": 5 }),
                    json!({}),
                );
                s.play(SOFTLOCK, json!({}));
                s.play(MANA_WELL, json!({ "zone": 1 }));
                assert_eq!(beneath_at(s.state(), zone("p1", "backrow", 1)).len(), 1);
                // Spending the last mana ends the turn by itself; then the opponent's turn passes back.
                if s.state().active == P1 {
                    s.end_turn();
                }
                s.end_turn();
                assert_eq!(s.state().active, P1);
                let mana = &s.state().players.p1.mana;
                assert_eq!(mana.current, mana.max + 1);
            }

            #[test]
            fn r346_a_spells_pierce_skips_armor_and_its_stack_does_nothing() {
                let mut s = softlock(
                    false,
                    json!({ "hand": [{ "def": SOFTLOCK }, ECLIPSE] }),
                    json!({ "field": [ARMORED] }),
                );
                s.play(SOFTLOCK, json!({}));
                let target = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                s.expect_stats(ARMORED, json!({ "health": 4 }));
                s.expect_in_zone(ECLIPSE, "graveyard");
            }

            #[test]
            fn a_card_created_later_lacks_both() {
                let mut s = softlock(
                    false,
                    json!({ "hand": [{ "def": SOFTLOCK }, CONJURE_KY], "library": [FILLER] }),
                    json!({}),
                );
                s.play(SOFTLOCK, json!({}));
                let before: IndexSet<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.play(CONJURE_KY, json!({}));
                let made: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
                assert_eq!(made.len(), 3);
                for card in &made {
                    assert!(granted(&s, &card.id).is_empty());
                }
            }

            #[test]
            fn e20_unlocks_every_locked_zone_of_both_players_unlocked_a_reserved_zone_stays_reserved() {
                let mut s = softlock(false, json!({}), json!({}));
                lock_zone(s.state_mut(), zone("p1", "units", 2));
                lock_zone(s.state_mut(), zone("p2", "backrow", 3));
                s.state_mut().reserved.push(zone("p2", "units", 4));
                s.play(SOFTLOCK, json!({}));
                assert!(!is_locked(s.state(), zone("p1", "units", 2)));
                assert!(!is_locked(s.state(), zone("p2", "backrow", 3)));
                assert_eq!(s.events().iter().filter(|event| event.event_type() == GameEventType::Unlocked).count(), 2);
                assert_eq!(js(&s.state().reserved), json!([{ "player": "p2", "row": "units", "lane": 4 }]));
            }

            #[test]
            fn r97_r440_the_grants_in_the_opponents_hand_and_deck_name_no_card() {
                let mut s = softlock(false, json!({}), json!({}));
                s.play(SOFTLOCK, json!({}));
                let mut hidden: Vec<String> = ids(&s.hand(P2));
                hidden.extend(ids(&s.pile(P2, "library")));
                hidden.extend(ids(&s.pile(P1, "library")));
                let grants: Vec<Value> = js(&s.view(P1))["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] == "keywordGranted")
                    .collect();
                // The public grants are reported (p2's unit's two); the hidden ones name no card.
                let theirs = s.unit(P2, 1).map(|unit| unit.id);
                assert!(
                    grants
                        .iter()
                        .any(|event| theirs.as_deref().is_some_and(|id| event["instanceId"] == id))
                );
                for event in &grants {
                    let id = event["instanceId"].as_str().unwrap_or_default();
                    assert!(!hidden.iter().any(|card| card == id));
                }
                let first = s.hand(P2).first().map(|card| card.id.clone()).unwrap_or_else(|| "-".to_string());
                assert!(!serde_json::to_string(&s.view(P1)).expect("serialises").contains(&first));
            }

            #[test]
            fn r386_the_draw_reads_through_param_an_upgrade_draws_2() {
                let mut s = softlock(false, json!({}), json!({}));
                step_param(card_mut(&mut s, SOFTLOCK), "draw", 1);
                s.play(SOFTLOCK, json!({}));
                assert_eq!(s.hand(P1).len(), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_2_all_cards_both_sides_gain_stack_and_pierce() {
                let mut s = softlock(true, json!({}), json!({}));
                s.play(SOFTLOCK, json!({ "modes": [ALL] }));
                assert_eq!(s.hand(P1).len(), 3);
                let unit = s.unit(P2, 1).expect("p2's unit").id;
                assert_eq!(granted(&s, &unit), stack_and_pierce());
                let held = s.hand(P2).first().expect("p2's hand card").id.clone();
                assert_eq!(granted(&s, &held), stack_and_pierce());
            }

            #[test]
            fn r81_only_yours_your_cards_gain_them_the_opponents_do_not_and_every_zone_is_still_unlocked() {
                let mut s = softlock(true, json!({}), json!({}));
                lock_zone(s.state_mut(), zone("p2", "units", 5));
                s.play(SOFTLOCK, json!({ "modes": [YOURS] }));
                let mut yours = s.hand(P1);
                yours.extend(s.pile(P1, "library"));
                for card in &yours {
                    assert_eq!(granted(&s, &card.id), stack_and_pierce());
                }
                let mut theirs = s.hand(P2);
                theirs.extend(s.pile(P2, "library"));
                theirs.push(s.unit(P2, 1).expect("p2's unit"));
                for card in &theirs {
                    assert!(granted(&s, &card.id).is_empty());
                }
                assert!(!is_locked(s.state(), zone("p2", "units", 5)));
            }
        }
    }
}
