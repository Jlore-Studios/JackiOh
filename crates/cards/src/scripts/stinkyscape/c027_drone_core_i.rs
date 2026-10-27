//! S #27 Drone Core I (SPEC §8.10 row 27): (0) Spell, Drone Core, Common, a card of the held Stinkyscape
//! set (R1420).
//!   Base:    "Discard a random card. Summon a Drone Interceptor."
//!   Radiant: "Discard a random card. Summon a Radiant Drone Interceptor."
//! Engine: a random discard of its caster's hand (R682), nothing with an empty hand, then a summon of the
//!   Drone Interceptor into the caster's leftmost open unit zone (R64), no Cry (R1); with no open zone
//!   nothing is summoned and the discard is still made. Radiant: the Interceptor is summoned Radiant.
//!   Drone Core is a family tag no rule reads (R1857000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-027";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                discard_random(json_as(json!({}))),
                summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant }))),
            ]
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

// S #27 Drone Core I — SPEC §8.10 row 27, BUILD M12 row S 27: "discards one random card of its caster's
// hand (none with an empty hand, the summon still made) and summons a Drone Interceptor into the leftmost
// open unit zone; a full unit row summons nothing; radiant the Interceptor is Radiant (4/4, two random
// combat keywords)".
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::drawn_combat_keywords;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-027";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, hand: Value, field: Value) -> Scenario {
        let mut cards = vec![json!({ "def": CORE, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": "drone-core-i",
            "p1": { "hand": cards, "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE, json!({}));
        s
    }

    fn discarded(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Discarded { .. }))
            .count()
    }

    #[test]
    fn discards_one_random_card_then_summons_an_interceptor() {
        let mut s = cast(false, json!([FILLER, FILLER, FILLER]), json!([]));
        assert_eq!(s.hand(P1).len(), 2, "one of the three is discarded");
        assert_eq!(discarded(&s), 1);
        assert_eq!(
            s.pile(P1, "graveyard")
                .iter()
                .filter(|card| card.def_id == FILLER)
                .count(),
            1
        );
        let drone = s.unit(P1, 1).expect("the Interceptor in lane 1");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(!drone.radiant);
        s.expect_events(json!(["discarded", "summoned"]));
    }

    #[test]
    fn with_an_empty_hand_it_still_summons() {
        let s = cast(false, json!([]), json!([]));
        assert_eq!(discarded(&s), 0);
        assert_eq!(s.unit(P1, 1).expect("the Interceptor").def_id, INTERCEPTOR);
    }

    #[test]
    fn a_full_unit_row_summons_nothing_but_still_discards() {
        let s = cast(false, json!([FILLER]), json!(vec![VANILLA; 5]));
        assert_eq!(discarded(&s), 1);
        assert!(s.hand(P1).is_empty());
        for lane in 1..=5 {
            assert_eq!(s.unit(P1, lane).expect("a Vanilla").def_id, VANILLA);
        }
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Summoned { .. })));
    }

    #[test]
    fn radiant_the_interceptor_is_radiant_4_4_with_two_keywords() {
        let s = cast(true, json!([FILLER]), json!([]));
        assert_eq!(discarded(&s), 1);
        let drone = s.unit(P1, 1).expect("the Interceptor");
        assert!(drone.radiant);
        let stats = s.stats(drone.id.as_str());
        assert_eq!((stats.attack, stats.health), (4, 4));
        assert_eq!(drawn_combat_keywords(s.card(drone.id.as_str()), None).len(), 2);
    }
}
