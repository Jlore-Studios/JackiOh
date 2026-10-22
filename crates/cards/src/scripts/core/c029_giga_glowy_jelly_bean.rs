//! #29 GIGA Glowy Jelly Bean (SPEC §8.2): "Every card in your hand becomes Radiant", radiant "Hand
//! and all your permanents", and on both faces "Float reduces cost" (#1563). The radiant cell restates
//! the whole scope, so it replaces the base clause and adds the permanents (§8 Conventions).
//!
//! Not `setRadiantRandom`: "every card" would burn rng draws on a foregone result and shift the
//! cursor for every later roll (§9.3). The hook READS `ctx.state` to name the cards and returns one
//! `setRadiant` per card, a no-op on a card already Radiant (§6.3 Make Radiant).
//!
//! "All your permanents" is §6.3's: units and backrow. `activeUnitsOf` gives only the top of each
//! pile, since cards under a Stack are not on the field (R13, §3.2); control, not ownership, decides
//! what is yours (R12). A permanent converts in place, the Cry not re-firing (§5.2, R22).
//!
//! Cost 7 is catalog data (6 until #1563). MAX_MANA is 4, so the card needs temporary mana (#6, #94, #95)
//! or Floats; §2.3 lets current mana exceed max.
//!
//! "Float reduces cost" (§6.2, R1563000) is the engine's `float_reduces_cost`, a hand trigger and a deck
//! trigger (R464): each Float of its owner's lowers its cost by that much, kept until a graveyard or
//! exile (R766). Until #1563 it had no such trigger (`script_v0_4_0f`, kept for the replays of v0.4.0f
//! and before, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-029";

/// Your hand, through the read-only `zoneCards`, which hands back a copy. The card itself sits in
/// the `resolving` zone while its script runs (§10.5), so it does not flag itself.
fn hand_of(ctx: &EffectContext<'_>) -> Vec<String> {
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// Your permanents: units (top of each pile only, R13) then backrow, in lane order.
fn permanents_of(ctx: &EffectContext<'_>) -> Vec<String> {
    let backrow: Vec<String> = slots_of(ctx.controller, Row::Backrow)
        .iter()
        .filter_map(|slot| card_at(ctx.state, slot).map(|card| card.id.clone()))
        .collect();
    let mut ids: Vec<String> = active_units_of(ctx.state, ctx.controller)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    ids.extend(backrow);
    ids
}

/// One `setRadiant` per card the hook named by id: the hook reads, never writes.
fn make_radiant(cards: &[String]) -> Vec<Effect> {
    cards
        .iter()
        .map(|id| set_radiant(json_as(json!({ "instanceId": id }))))
        .collect()
}

/// The two faces; `float` adds "Float reduces cost", which the card printed from #1563 on.
fn giga(float: bool) -> CardScripts {
    let (hand_triggers, deck_triggers) = if float {
        (
            vec![float_reduces_cost("giga-float-hand")],
            vec![float_reduces_cost("giga-float-deck")],
        )
    } else {
        (vec![], vec![])
    };
    let base = Script {
        cry: Some(hook(|ctx| make_radiant(&hand_of(ctx)))),
        hand_triggers: hand_triggers.clone(),
        deck_triggers: deck_triggers.clone(),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            let mut cards = hand_of(ctx);
            cards.extend(permanents_of(ctx));
            make_radiant(&cards)
        })),
        hand_triggers,
        deck_triggers,
        ..Script::default()
    };

    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    giga(true)
}

/// R767000: GIGA Glowy Jelly Bean through v0.4.0f, with no Float discount.
pub fn script_v0_4_0f() -> CardScripts {
    giga(false)
}

// #29 GIGA Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 29): "Uncastable at 4 mana, castable at 7
// after gains; whole hand radiant; radiant also permanents; each Float of its owner's lowers its cost in
// hand and deck (R1563000)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Your permanents as §6.3 means them: units (top of each pile only, R13) and backrow.
    fn permanents_of(state: &GameState) -> Vec<CardInstance> {
        let side = &state.players.p1;
        let mut cards: Vec<CardInstance> = side.units.iter().flatten().filter_map(|pile| pile.first().cloned()).collect();
        cards.extend(side.backrow.iter().flatten().cloned());
        cards
    }

    fn giga(mana: i32, radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": "core-029", "radiant": radiant }, "core-005", "core-016", "core-010"],
                "field": [{ "def": "core-013", "damage": 2 }, "core-025"],
                "backrow": ["core-006"],
                "library": ["core-043"],
                "mana": mana,
            },
            "p2": { "hand": ["core-005"], "field": ["core-025"], "library": ["core-010"] },
        }))
    }

    fn other_hand_ids(state: &GameState) -> Vec<String> {
        state
            .players
            .p1
            .hand
            .iter()
            .filter(|card| card.def_id != "core-029")
            .map(|card| card.id.clone())
            .collect()
    }

    mod n29_giga_glowy_jelly_bean_base {
        use super::*;

        #[test]
        fn costs_7_so_a_mana_refresh_alone_can_never_cast_it_max_mana_is_4() {
            crate::register_all();
            assert_eq!(MAX_MANA, 4);
            let mut s = giga(4, false);

            s.expect_refused_with(|s| s.play("core-029", json!({})), "7");
            // Nothing happened: no card played, no flag set.
            assert!(!s.state().players.p1.hand.iter().any(|card| card.radiant));
            assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
        }

        #[test]
        fn is_castable_at_7_once_mana_gains_take_current_above_max_s2_3() {
            crate::register_all();
            let mut s = giga(7, false);

            s.play("core-029", json!({}));

            s.expect_in_zone("core-029", "graveyard").expect_mana(P1, 0);
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
        }

        #[test]
        fn flags_every_card_in_your_hand_and_nothing_in_any_other_zone() {
            crate::register_all();
            let mut s = giga(7, false);
            let hand_ids = other_hand_ids(s.state());
            let permanent_ids: Vec<String> = permanents_of(s.state()).into_iter().map(|card| card.id).collect();

            s.play("core-029", json!({}));

            let mut held: Vec<String> = s.state().players.p1.hand.iter().map(|card| card.id.clone()).collect();
            held.sort();
            let mut wanted = hand_ids.clone();
            wanted.sort();
            assert_eq!(held, wanted);
            for id in &hand_ids {
                assert!(s.card(id).radiant);
            }
            // The base cell is the hand only: permanents, library and the opponent are untouched.
            for id in &permanent_ids {
                assert!(!s.card(id).radiant);
            }
            for card in &s.state().players.p1.library {
                assert!(!card.radiant);
            }
            for card in &s.state().players.p2.hand {
                assert!(!card.radiant);
            }
        }

        #[test]
        fn s10_5_never_flags_itself_the_card_is_in_the_resolving_zone_while_its_script_runs() {
            crate::register_all();
            let mut s = giga(7, false);
            let self_id = s
                .state()
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == "core-029")
                .map(|card| card.id.clone())
                .unwrap_or_default();

            s.play("core-029", json!({}));

            assert!(!s.card(&self_id).radiant);
        }
    }

    mod n29_giga_glowy_jelly_bean_radiant {
        use super::*;

        #[test]
        fn flags_your_hand_and_all_your_permanents_units_and_backrow_alike() {
            crate::register_all();
            let mut s = giga(7, true);
            let hand_ids = other_hand_ids(s.state());
            let permanent_ids: Vec<String> = permanents_of(s.state()).into_iter().map(|card| card.id).collect();
            assert_eq!(permanent_ids.len(), 3); // two units plus a Field Spell in the backrow

            s.play("core-029", json!({}));

            for id in hand_ids.iter().chain(permanent_ids.iter()) {
                assert!(s.card(id).radiant);
            }
            // "Your" is the controller (R12), so nothing of the opponent's converts.
            for card in &s.state().players.p2.hand {
                assert!(!card.radiant);
            }
            assert_eq!(s.unit(P2, 1).map(|card| card.radiant), Some(false));
            // The library is in neither cell.
            for card in &s.state().players.p1.library {
                assert!(!card.radiant);
            }
        }

        #[test]
        fn r22_a_permanent_swaps_its_base_stats_in_place_keeping_the_damage_it_has_taken() {
            crate::register_all();
            let mut s = giga(7, true);
            let unit_id = s.unit(P1, 1).map(|card| card.id.clone()).unwrap_or_default();
            s.expect_stats(&unit_id, json!({ "attack": 6, "maxHealth": 10, "health": 8 }));

            s.play("core-029", json!({}));

            // #13 Jlockeed Shredder-10 is 6/10 → 12/20; the 2 damage stays, so health is 18 and no Cry
            // re-fires, because setting a flag is not an entry to the field.
            s.expect_stats(&unit_id, json!({ "attack": 12, "maxHealth": 20, "health": 18 }));
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Summoned { .. }))
                    .count(),
                0
            );
        }

        #[test]
        fn r13_leaves_a_card_under_a_stack_pile_alone_it_is_not_on_the_field() {
            crate::register_all();
            let mut s = giga(7, true);
            assert!(s.state().players.p1.units[0].is_some());
            let Some(dormant_id) = s.state().players.p1.units[0]
                .as_ref()
                .map(|pile| pile.first().map(|card| card.id.clone()).unwrap_or_default())
            else {
                return;
            };
            let top = new_instance(
                s.state_mut(),
                "core-025",
                P1,
                Zone::Field {
                    player: P1,
                    row: Row::Units,
                    lane: 1,
                },
            );
            if let Some(pile) = s.state_mut().players.p1.units[0].as_mut() {
                pile.insert(0, top.clone());
            }

            s.play("core-029", json!({}));

            assert!(s.card(&top.id).radiant);
            assert!(!s.card(&dormant_id).radiant);
        }
    }

    mod n29_giga_glowy_jelly_bean_float {
        use super::*;

        fn price_of(s: &Scenario, id: &str) -> i32 {
            jackioh_engine::mana::effective_cost(s.state(), s.card(id), jackioh_engine::mana::CostOptions::default())
        }

        /// p1 holds one GIGA and has one in the deck, with `mana` left to Float.
        fn floating(mana: i32, radiant: bool) -> (Scenario, String, String) {
            crate::register_all();
            let s = scenario(json!({
                "p1": {
                    "hand": [{ "def": "core-029", "radiant": radiant }, "core-005"],
                    "library": [{ "def": "core-029", "radiant": radiant }, "core-005", "core-005"],
                    "mana": mana,
                },
                "p2": { "hand": ["core-005"], "library": ["core-005", "core-005"] },
            }));
            let hand = s.hand(P1).into_iter().find(|card| card.def_id == "core-029").map(|card| card.id).unwrap_or_default();
            let deck = s.pile(P1, "library").into_iter().find(|card| card.def_id == "core-029").map(|card| card.id).unwrap_or_default();
            (s, hand, deck)
        }

        #[test]
        fn r1563000_a_float_lowers_its_cost_in_hand_and_deck() {
            for radiant in [false, true] {
                let (mut s, hand, deck) = floating(3, radiant);
                s.end_turn();
                assert_eq!(s.card(hand.as_str()).cost_mod, -3, "radiant {radiant}");
                assert_eq!(s.card(deck.as_str()).cost_mod, -3, "radiant {radiant}");
                assert_eq!(price_of(&s, &hand), 4, "radiant {radiant}");
            }
        }

        #[test]
        fn r1563000_no_float_and_the_opponents_float_lower_nothing() {
            let (mut s, hand, _) = floating(0, false);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, 0);
            // p2 now ends a turn with mana left: their Float, not p1's.
            s.state_mut().players.p2.mana.current = 4;
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        }
    }
}
