//! S #45 Drone Commander (SPEC §8.10 row 45): (4) Unit, Human, Epic, 8/12 → 16/24, a card of the held
//! Stinkyscape set (R1420).
//!   Base:    "Cry: Add {cards|random Drone Core card|random Drone Core cards} to your hand. Each costs
//!            ({discount}) less." — cards 6, discount 1
//!   Radiant: "Cry: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand.
//!            Each costs ({discount}) less." — cards 6, discount 1
//! Engine: Cry (R1; the list prints no trigger, the plan's fix ST8). Picks from S #41's Drone Core pool (the
//!   non-token cards tagged Drone Core, R380), one draw of the match rng per card (R60), each with a
//!   `costMod` of minus `discount` (R65), kept until a graveyard or exile (R766); a full hand burns the
//!   overflow (§2.4). `cards` and `discount` are declared Cry numbers (R386, R1748000). The pool names no
//!   set, so while the set is held it reaches no card (R1420). Radiant: added Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-045";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Drone Core"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
                "costMod": -param(&*ctx, "discount"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S #45 Drone Commander — SPEC §8.10 row 45, BUILD M12 row S 45: "played, adds 6 Drone Core cards (the set
// previewed), each (1) less, a full hand burning the rest; cards and discount read through `param()`;
// radiant 16/24 and Radiant ones".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const COMMANDER: &str = "stinkyscape-045";
    const FILLER: &str = "core-005";

    /// The Drone Cores in p1's hand, as (face, cost layer).
    fn cores(s: &Scenario) -> Vec<(bool, i32)> {
        s.hand(P1)
            .into_iter()
            .filter(|card| {
                (27..=36)
                    .map(|n| format!("stinkyscape-{n:03}"))
                    .any(|tier| tier == card.def_id)
            })
            .map(|card| (card.radiant, card.cost_mod))
            .collect()
    }

    fn setup(radiant: bool, hand: usize) -> Scenario {
        let mut cards = vec![json!({ "def": COMMANDER, "radiant": radiant })];
        cards.extend((0..hand).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": "drone-commander",
            "p1": { "hand": cards, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn burned(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Burned { .. }))
            .count()
    }

    #[test]
    fn r65_played_adds_six_each_costing_one_less() {
        let _p = preview_sets(&[SetName::Stinkyscape]);
        let mut s = setup(false, 1);
        s.play(COMMANDER, json!({}));
        assert_eq!(cores(&s), vec![(false, -1); 6]);
        assert_eq!(burned(&s), 0);
    }

    #[test]
    fn a_full_hand_burns_the_rest() {
        let _p = preview_sets(&[SetName::Stinkyscape]);
        let mut s = setup(false, 6);
        s.play(COMMANDER, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert_eq!(cores(&s).len(), 4);
        assert_eq!(burned(&s), 2);
    }

    #[test]
    fn cards_and_discount_read_through_param() {
        let _p = preview_sets(&[SetName::Stinkyscape]);
        let mut s = setup(false, 1);
        set_param(s.card_mut(COMMANDER), "cards", 2);
        set_param(s.card_mut(COMMANDER), "discount", 3);
        s.play(COMMANDER, json!({}));
        assert_eq!(cores(&s), vec![(false, -3); 2]);
    }

    #[test]
    fn r1420_with_the_set_held_nothing_is_added() {
        let mut s = setup(false, 1);
        s.play(COMMANDER, json!({}));
        assert!(cores(&s).is_empty());
    }

    #[test]
    fn radiant_16_24_adds_radiant_ones() {
        let _p = preview_sets(&[SetName::Stinkyscape]);
        let mut s = setup(true, 1);
        s.play(COMMANDER, json!({}));
        let commander = s.unit(P1, 1).expect("the Commander");
        let stats = s.stats(commander.id.as_str());
        assert_eq!((stats.attack, stats.health), (16, 24));
        assert_eq!(cores(&s), vec![(true, -1); 6]);
    }
}
