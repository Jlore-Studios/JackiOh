//! SPEC §8.10 Stinkyscape 165 The Trade Union, a card of the Stinkyscape set.
//!   (1) Field Spell, Rare.
//!   Base:    "Tradable / Aura: Cards in your hand have Tradable."
//!   Radiant: "Tradable / Aura: Cards in your hand have Tradable. You may Trade up to {trades|card|cards}
//!            each turn."
//! Engine: Tradable on both faces is catalog data the engine reads in the hand (R1838000). The aura
//!   lends Tradable to every card in its controller's hand while it acts on their field
//!   (`tradable_hand_aura`, R1864002), read through the layers as Cloaked is. The Radiant face gives
//!   its controller `trades` Trades a turn while it acts (`trades_per_turn`, R1864004), a pure read
//!   through `param()` (R824000: the number is the Radiant face's alone), the most of several holding,
//!   never a sum.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-165";

fn union(radiant: bool) -> Script {
    Script {
        aura: Some(tradable_hand_aura()),
        trades_per_turn: radiant.then(|| -> TradesPerTurnHook { Arc::new(|args| param(&args, "trades")) }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: union(false),
        radiant: union(true),
    }
}

// S #165 The Trade Union — SPEC §8.10 row 165, BUILD M12 row S 165 (R824000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile, a Spell that prints no Tradable.
    const PRESTIGE: &str = "meditative-053"; // Prints Tradable.

    fn unionised(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER, FILLER, FILLER, PRESTIGE],
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn union_of(s: &Scenario) -> CardInstance {
        s.backrow(P1, 1).expect("the Union in p1's backrow")
    }

    fn held(s: &Scenario, seat: PlayerId, def: &str) -> Vec<String> {
        s.hand(seat)
            .into_iter()
            .filter(|card| card.def_id == def)
            .map(|card| card.id)
            .collect()
    }

    fn tradable_now(s: &Scenario, id: &str) -> bool {
        is_tradable(s.state(), s.card(id))
    }

    fn trade_of(id: &str) -> ActionBody {
        ActionBody::Trade {
            instance_id: id.to_string(),
        }
    }

    fn trade_refusal(s: &Scenario, id: &str) -> Option<String> {
        reduce(s.state(), &Action::new(trade_of(id), P1, "t")).error
    }

    fn by_hand(s: &mut Scenario, effects: Vec<Effect>) {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    #[test]
    fn r1864002_cards_in_your_hand_have_tradable_while_it_acts() {
        for radiant in [false, true] {
            let mut s = unionised("union-lent", radiant);
            let stockpile = held(&s, P1, FILLER)[0].clone();
            assert!(tradable_now(&s, &stockpile), "radiant {radiant}: the aura lends it Tradable");
            let theirs = held(&s, P2, FILLER)[0].clone();
            assert!(!tradable_now(&s, &theirs), "radiant {radiant}: only its controller's hand");
            assert!(legal_actions(s.state(), P1).contains(&trade_of(&stockpile)));
            s.trade(&stockpile);
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
        }
    }

    #[test]
    fn r1864002_off_the_field_only_printed_tradable_cards_trade() {
        let mut s = unionised("union-gone", false);
        let union = union_of(&s);
        by_hand(
            &mut s,
            vec![destroy(json_as(json!({ "target": { "of": "instance", "instanceId": union.id } })))],
        );
        assert!(
            s.state().players.p1.graveyard.iter().any(|card| card.id == union.id),
            "the Union left the field"
        );
        let stockpile = held(&s, P1, FILLER)[0].clone();
        assert!(!tradable_now(&s, &stockpile), "nothing lends it Tradable now");
        let name = def_of(Some(s.state()), FILLER).name.clone();
        assert_eq!(trade_refusal(&s, &stockpile), Some(format!("{name} is not Tradable")));
        let prestige = held(&s, P1, PRESTIGE)[0].clone();
        assert_eq!(trade_refusal(&s, &prestige), None, "a printed Tradable card still trades");
    }

    #[test]
    fn r1864004_the_base_face_allows_one_trade() {
        let mut s = unionised("union-one", false);
        assert_eq!(trades_per_turn_of(s.state(), P1), TRADES_PER_TURN);
        let cards = held(&s, P1, FILLER);
        s.trade(&cards[0]);
        assert_eq!(
            trade_refusal(&s, &cards[1]).as_deref(),
            Some("you have made this turn's 1 Trade")
        );
    }

    #[test]
    fn r1864004_the_radiant_face_allows_two_and_refuses_a_third() {
        let mut s = unionised("union-two", true);
        let trades = param_value(s.state(), Some(&union_of(&s)), "trades", ParamValueOptions::default());
        assert_eq!(trades, 2, "the printed number");
        assert_eq!(trades_per_turn_of(s.state(), P1), trades);
        let cards = held(&s, P1, FILLER);
        s.trade(&cards[0]);
        s.trade(&cards[1]);
        assert_eq!(trades_left(s.state(), P1), 0);
        assert_eq!(
            trade_refusal(&s, &cards[2]).as_deref(),
            Some(format!("you have made this turn's {trades} Trades").as_str())
        );
        assert_eq!(trades_per_turn_of(s.state(), P2), TRADES_PER_TURN, "the opponent keeps theirs");
    }

    #[test]
    fn r1864004_leaving_the_field_the_limit_falls_back() {
        let mut s = unionised("union-leaves", true);
        let union = union_of(&s);
        let stockpile = held(&s, P1, FILLER)[0].clone();
        s.trade(&stockpile);
        assert_eq!(trades_left(s.state(), P1), 1, "one of the two left");
        by_hand(
            &mut s,
            vec![destroy(json_as(json!({ "target": { "of": "instance", "instanceId": union.id } })))],
        );
        assert_eq!(trades_per_turn_of(s.state(), P1), TRADES_PER_TURN);
        let prestige = held(&s, P1, PRESTIGE)[0].clone();
        assert_eq!(
            trade_refusal(&s, &prestige).as_deref(),
            Some("you have made this turn's 1 Trade"),
            "the Trade already made still counts"
        );
    }

    #[test]
    fn r1864004_two_unions_still_make_two() {
        let mut s = scenario(json!({
            "seed": "union-twice",
            "p1": {
                "hand": [FILLER, FILLER, FILLER],
                "field": [
                    { "def": ID, "lane": 1, "radiant": true },
                    { "def": ID, "lane": 2, "radiant": true },
                ],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        assert_eq!(trades_per_turn_of(s.state(), P1), 2, "the most holds, never a sum");
        let cards = held(&s, P1, FILLER);
        s.trade(&cards[0]);
        s.trade(&cards[1]);
        assert_eq!(
            trade_refusal(&s, &cards[2]).as_deref(),
            Some("you have made this turn's 2 Trades")
        );
    }

    #[test]
    fn r1864004_trades_reads_param_on_the_radiant_face_only() {
        for radiant in [false, true] {
            let mut s = unionised("union-param", radiant);
            let union = union_of(&s).id;
            set_param(s.card_mut(&union), "trades", 3);
            let expected = if radiant { 3 } else { TRADES_PER_TURN };
            assert_eq!(trades_per_turn_of(s.state(), P1), expected, "radiant {radiant}");
            assert_eq!(trades_per_turn_of(s.state(), P2), TRADES_PER_TURN);
        }
    }
}
