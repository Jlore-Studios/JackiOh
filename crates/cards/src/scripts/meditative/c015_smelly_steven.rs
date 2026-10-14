//! M #15 Smelly Steven (SPEC §8.8 row 15, R70, R455, R458, R386): (2) Unit, Human, Common, 5/5 → 10/10.
//!
//! Base:    "Cry: Your opponent's Spells cost ({surcharge}) more during their next turn."
//! Radiant: the same, surcharge 2.
//! Engine: a price rule on the opponent (`add_cost_rule`, R455) for Spells only, lasting
//! "theirNextTurn": T-AI-7 Alignment Tax's shape narrowed to the Spell type. It is inert for the rest of
//! this turn, holds through their next turn and is gone at its cleanup (R458: "next turn" skips the
//! current one). It is a price for a play, so a Field Spell, a Trap and a Unit keep theirs, and a cast
//! pays nothing (R70).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-015";

fn smelly_steven() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![add_cost_rule(json_as(json!({
                "player": "enemy",
                "rule": { "types": ["Spell"], "amount": param(&*ctx, "surcharge") },
                "lasts": "theirNextTurn",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: smelly_steven(),
        radiant: smelly_steven(),
    }
}

// M #15 Smelly Steven — SPEC §8.8 row 15, BUILD M10 row M 15: "A price rule on the opponent for Spells only,
// lasting their next turn (R455): inert for the rest of your turn, live through theirs and gone at its
// cleanup (R458); a Field Spell, a Trap and a Unit keep their price; your own Spells keep theirs; a cast pays
// nothing (R70); a Spell the surcharge lifts past their mana drops out of `legalActions`; surcharge reads
// through param() (R386); the Radiant face adds 2".
//
// Fixtures, as T-AI-7 Alignment Tax's: p2's hand holds one card of each price and type, #8 Mr. Vanilla (a
// (1) Unit), #5 Stockpile (a (1) Spell), #17 Flood (a (4) Spell), #6 Mana Well (a (3) Field Spell) and #17
// Counterspell (classic, a (2) Trap); #21 Hinder is the Spell cast on draw.
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const STOCKPILE: &str = "core-005";
    const FLOOD: &str = "core-017";
    const MANA_WELL: &str = "core-006";
    const COUNTERSPELL: &str = "classic-017";
    const HINDER: &str = "core-021";

    /// p1 holds Steven (Radiant or not) and a Stockpile of its own; both decks are deep enough to cross turns.
    fn board(radiant: bool) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                "mana": 8,
            },
            "p2": {
                "hand": [VANILLA, STOCKPILE, FLOOD, MANA_WELL, COUNTERSPELL],
                "library": [VANILLA, VANILLA, VANILLA, VANILLA],
            },
        }))
    }

    /// What `player` reads as the price of their own hand card of `def_id` (the client prints this).
    fn price_of(s: &Scenario, player: PlayerId, def_id: &str) -> i64 {
        let hand = js(&s.view(player))["you"]["hand"].clone();
        let card = s.hand(player).into_iter().find(|card| card.def_id == def_id);
        let shown = card.and_then(|card| {
            hand.as_array()
                .and_then(|cards| cards.iter().find(|shown| shown["instanceId"] == card.id.as_str()).cloned())
        });
        match shown {
            Some(shown) => shown["cost"].as_i64().unwrap_or_default(),
            None => panic!("{player:?} holds no {def_id}"),
        }
    }

    fn playable(s: &Scenario, player: PlayerId, def_id: &str) -> bool {
        let ids: Vec<String> =
            s.hand(player).into_iter().filter(|card| card.def_id == def_id).map(|card| card.id).collect();
        legal_actions(s.state(), player).iter().any(|action| {
            let action = js(action);
            action["type"] == "play" && action["instanceId"].as_str().is_some_and(|id| ids.iter().any(|own| own == id))
        })
    }

    fn has_cost_rule(s: &Scenario) -> bool {
        s.state().players.p2.mods.iter().any(|modifier| js(modifier)["kind"] == "costRule")
    }

    #[test]
    fn is_a_2_cost_5_5_human_declaring_surcharge() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Common);
        assert_eq!(js(&def.tags), json!(["Human"]));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(5), Some(5), Some(10), Some(10)]
        );
        assert_eq!(
            js(&def.params),
            json!([{ "key": "surcharge", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = super::script();
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r458_inert_for_the_rest_of_your_turn() {
            let mut s = board(false);
            s.play(ID, json!({}));
            assert!(has_cost_rule(&s));
            assert_eq!(price_of(&s, P2, STOCKPILE), 1);
            assert_eq!(price_of(&s, P2, FLOOD), 4);
        }

        #[test]
        fn their_spells_cost_1_more_during_their_next_turn() {
            let mut s = board(false);
            s.play(ID, json!({})).end_turn();
            assert_eq!(s.state().active, P2);
            assert_eq!(price_of(&s, P2, STOCKPILE), 2);
            assert_eq!(price_of(&s, P2, FLOOD), 5);
            s.play(STOCKPILE, json!({}));
            // 4 mana, the taxed Stockpile paid 2.
            s.expect_mana(P2, 2);
        }

        #[test]
        fn field_spells_traps_and_units_keep_their_price() {
            let mut s = board(false);
            s.play(ID, json!({})).end_turn();
            assert_eq!(price_of(&s, P2, MANA_WELL), 3);
            assert_eq!(price_of(&s, P2, COUNTERSPELL), 2);
            assert_eq!(price_of(&s, P2, VANILLA), 1);
        }

        #[test]
        fn gone_at_their_cleanup() {
            let mut s = board(false);
            s.play(ID, json!({})).end_turn().end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(price_of(&s, P2, STOCKPILE), 1);
            assert!(!has_cost_rule(&s));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert_eq!(price_of(&s, P2, STOCKPILE), 1);
        }

        #[test]
        fn your_own_spells_keep_their_price() {
            let mut s = board(false);
            s.play(ID, json!({}));
            assert_eq!(price_of(&s, P1, STOCKPILE), 1);
            s.end_turn().end_turn();
            assert_eq!(price_of(&s, P1, STOCKPILE), 1);
        }

        #[test]
        fn r70_a_cast_spell_pays_nothing() {
            // Hinder is on top of p2's deck: it draws it at its turn start and casts it on the draw, with the
            // surcharge live and nothing paid.
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, STOCKPILE], "library": [VANILLA, VANILLA, VANILLA], "mana": 8 },
                "p2": { "hand": [VANILLA, STOCKPILE], "library": [HINDER, VANILLA, VANILLA] },
            }));
            s.play(ID, json!({})).end_turn();
            assert_eq!(s.state().active, P2);
            let cast = s.events().iter().any(|event| {
                matches!(event, GameEvent::CardPlayed { player, def_id, cost_paid: 0, .. } if *player == P2 && def_id == HINDER)
            });
            assert!(cast, "{:?}", s.events());
            let mana = &s.state().players.p2.mana;
            assert_eq!(mana.current, mana.max);
        }

        #[test]
        fn a_spell_lifted_past_their_mana_drops_from_legal_actions() {
            let mut s = board(false);
            s.end_turn();
            assert!(playable(&s, P2, FLOOD));
            s.end_turn();
            s.play(ID, json!({})).end_turn();
            assert_eq!(s.state().players.p2.mana.current, 4);
            assert!(!playable(&s, P2, FLOOD));
            s.expect_refused(|s| s.play(FLOOD, json!({})));
            assert!(playable(&s, P2, STOCKPILE));
        }

        #[test]
        fn r386_a_buff_makes_it_2() {
            let mut s = board(false);
            assert_eq!(crate::upgrade_number(&mut s, ID, "surcharge"), 2);
            s.play(ID, json!({})).end_turn();
            assert_eq!(price_of(&s, P2, STOCKPILE), 3);
            assert_eq!(price_of(&s, P2, FLOOD), 6);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_10_10_and_their_spells_cost_2_more() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.expect_stats(ID, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
            assert_eq!(price_of(&s, P2, STOCKPILE), 1);
            s.end_turn();
            assert_eq!(price_of(&s, P2, STOCKPILE), 3);
            assert_eq!(price_of(&s, P2, FLOOD), 6);
            assert_eq!(price_of(&s, P2, VANILLA), 1);
            assert!(!playable(&s, P2, FLOOD));
            s.end_turn();
            assert_eq!(price_of(&s, P2, STOCKPILE), 1);
        }
    }
}
