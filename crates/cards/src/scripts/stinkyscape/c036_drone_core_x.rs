//! S #36 Drone Core X (SPEC §8.10 row 36): (8) Spell, Drone Core, Mythic, a card of the Stinkyscape set.
//!   Base:    "Float reduces cost.\nSummon the Drone Battleship."
//!   Radiant: "Float reduces cost.\nSummon a Radiant Drone Battleship."
//! Engine: "Float reduces cost" as S #34 (R1563000): a hand trigger on its owner's Float (a deck trigger too through v0.5.9e, R2657002), a
//!   `costMod` kept until a graveyard or exile (R766). The Drone Battleship is its three layer tokens
//!   summoned as one pile (`summon_pile`, R1842000) into one unit zone, bottom first: the Meltdown Layer,
//!   then the Propagation Layer, then the Shield Layer on top, so the Shield Layer is fought first and acts
//!   alone (§3.2, R13); each layer is its own summon, no Cry (R1); with no free zone none of it is
//!   summoned. "The Drone Battleship" names the three layers by R279's parenthesis rule. Radiant: all three
//!   layers Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-036";

/// The Drone Battleship's layers (§8.10 T-meltdown-layer, T-propagation-layer, T-shield-layer).
const MELTDOWN: &str = "stinkyscape-t-meltdown-layer";
const PROPAGATION: &str = "stinkyscape-t-propagation-layer";
const SHIELD: &str = "stinkyscape-t-shield-layer";

/// One face. `deck` adds the Float deck trigger it had through v0.5.9e (R2657002).
fn face(radiant: bool, deck: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon_pile(json_as(json!({
                "defIds": [MELTDOWN, PROPAGATION, SHIELD],
                "radiant": radiant,
            })))]
        })),
        hand_triggers: vec![float_reduces_cost("drone-core-x-float-hand")],
        deck_triggers: if deck {
            vec![float_reduces_cost("drone-core-x-float-deck")]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

/// R767000: through v0.5.9e a Float lowered its cost in the deck too, before R2657002; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_9e() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

// S #36 Drone Core X — SPEC §8.10 row 36, BUILD M12 row S 36: "Float lowers its cost as S 34's; summons
// the Meltdown, Propagation and Shield Layers into one unit zone in that order, only the Shield Layer
// acting, the Propagation Layer resuming when it dies and the Meltdown Layer last (R1842000); with no free
// unit zone nothing; radiant all three Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-036";
    const MELTDOWN: &str = "stinkyscape-t-meltdown-layer";
    const PROPAGATION: &str = "stinkyscape-t-propagation-layer";
    const SHIELD: &str = "stinkyscape-t-shield-layer";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn price_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), CostOptions::default())
    }

    /// p1 holds one Drone Core X and has one in the deck, with `mana` left to Float.
    fn floating(mana: i32, radiant: bool) -> (Scenario, String, String) {
        let s = scenario(json!({
            "p1": {
                "hand": [{ "def": CORE, "radiant": radiant }, FILLER],
                "library": [{ "def": CORE, "radiant": radiant }, FILLER, FILLER],
                "mana": mana,
            },
            "p2": { "hand": [FILLER], "library": filler(2) },
        }));
        let hand = s.hand(P1).into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        let deck = s.pile(P1, "library").into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        (s, hand, deck)
    }

    fn setup(radiant: bool, hand: Value, field: Value) -> Scenario {
        let mut cards = vec![json!({ "def": CORE, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        cards.push(json!(FILLER));
        scenario(json!({
            "p1": { "hand": cards, "field": field, "library": filler(4), "mana": 40 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The definitions and faces of p1's unit zone `lane`, top first.
    fn pile(s: &Scenario, lane: i32) -> Vec<(String, bool)> {
        pile_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane })
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|card| (card.def_id, card.radiant))
            .collect()
    }

    fn layers(radiant: bool) -> Vec<(String, bool)> {
        [SHIELD, PROPAGATION, MELTDOWN]
            .iter()
            .map(|id| (id.to_string(), radiant))
            .collect()
    }

    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn hit(s: &mut Scenario, id: &str) {
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    #[test]
    fn r1563000_r2657002_a_float_of_2_makes_it_cost_two_less_in_hand_never_in_the_deck() {
        for radiant in [false, true] {
            let (mut s, hand, deck) = floating(2, radiant);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, -2, "radiant {radiant}");
            assert_eq!(s.card(deck.as_str()).cost_mod, 0, "radiant {radiant}");
            assert_eq!(price_of(&s, &hand), 6, "radiant {radiant}");
        }
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        let (mut s, hand, _) = floating(0, false);
        s.end_turn();
        s.state_mut().players.p2.mana.current = 4;
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        assert_eq!(price_of(&s, &hand), 8);
    }

    #[test]
    fn r1842000_the_three_layers_share_one_zone_shield_on_top() {
        let mut s = setup(false, json!([]), json!([{ "def": VANILLA, "lane": 1 }]));
        s.play(CORE, json!({}));
        assert_eq!(pile(&s, 2), layers(false), "one pile in the leftmost open zone, the Shield on top");
        assert_eq!(summoned(&s), vec![MELTDOWN, PROPAGATION, SHIELD], "bottom first");
        assert!(s.unit(P1, 3).is_none());
        assert_eq!(s.unit(P1, 2).expect("the pile's top").def_id, SHIELD);
    }

    #[test]
    fn r1842000_with_no_free_zone_nothing() {
        let mut s = setup(false, json!([]), json!(vec![VANILLA; 5]));
        s.play(CORE, json!({}));
        assert!(summoned(&s).is_empty());
        for lane in 1..=5 {
            assert_eq!(pile(&s, lane), vec![(VANILLA.to_string(), false)]);
        }
    }

    #[test]
    fn r1842000_r13_the_layers_resume_in_order() {
        let mut s = setup(false, json!([HIT_JOB, HIT_JOB, HIT_JOB]), json!([]));
        s.play(CORE, json!({}));
        let top = s.unit(P1, 1).expect("the Shield Layer").id;
        hit(&mut s, &top);
        assert_eq!(pile(&s, 1), layers(false)[1..].to_vec(), "the Propagation Layer resumes");
        let top = s.unit(P1, 1).expect("the Propagation Layer").id;
        hit(&mut s, &top);
        assert_eq!(pile(&s, 1), layers(false)[2..].to_vec(), "the Meltdown Layer last");
        let top = s.unit(P1, 1).expect("the Meltdown Layer").id;
        hit(&mut s, &top);
        assert!(pile(&s, 1).is_empty());
    }

    #[test]
    fn radiant_all_three_layers_radiant() {
        let mut s = setup(true, json!([]), json!([]));
        s.play(CORE, json!({}));
        assert_eq!(pile(&s, 1), layers(true));
        let top = s.unit(P1, 1).expect("the Shield Layer").id;
        let stats = s.stats(top.as_str());
        assert_eq!((stats.attack, stats.health), (24, 24));
    }
}
