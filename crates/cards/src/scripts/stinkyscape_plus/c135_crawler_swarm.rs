//! SPEC §8.12 S+ #135 Crawler Swarm (§7, R64, R1136000, R1420, R3535000, R3560000): (2) Spell, Mindustrial,
//! Common, a card of the Stinkyscape+ set.
//!   Base:    "Fill your board with Crawlers."
//!   Radiant: "Fill your board with Radiant Crawlers."
//!
//! Engine: its Cry is one `fillBoard` of the Crawler (S+ #120): a Crawler in every empty unit zone of its
//! caster's side, left to right (R64, R1136000), Radiant on the Radiant face. The first Crawler to arrive
//! each game adds a Toxopid to its player's hand, once whatever the number that arrive (R3560000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-135";

/// The Crawler (S+ #120).
const CRAWLER: &str = "stinkyscapeplus-120";

fn swarm(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![fill_board(json_as(json!({ "defId": CRAWLER, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: swarm(false),
        radiant: swarm(true),
    }
}

// S+ #135 Crawler Swarm — SPEC §8.12 row S+ 135 (R64, R3560000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const TOXOPID: &str = "stinkyscapeplus-121";
    const STAND_IN: &str = "stand-in (0) 1/1";

    fn swarming(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "s135",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The Crawlers on p1's field, by lane.
    fn crawlers(s: &Scenario) -> Vec<(i32, CardInstance)> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane).map(|unit| (lane, unit)))
            .filter(|(_, card)| card.def_id == CRAWLER)
            .collect()
    }

    fn toxopids(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == TOXOPID).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r64_fills_every_empty_unit_zone_with_crawlers() {
            let mut s = swarming(false, json!([{ "def": STAND_IN, "lane": 1 }, { "def": STAND_IN, "lane": 2 }]));
            s.play(ID, json!({}));
            let found = crawlers(&s);
            assert_eq!(found.iter().map(|(lane, _)| *lane).collect::<Vec<_>>(), vec![3, 4, 5]);
            assert!(found.iter().all(|(_, card)| !card.radiant));
            assert_eq!(s.unit_at(P1, 1).def_id, STAND_IN);
            assert_eq!(s.unit_at(P1, 2).def_id, STAND_IN);
        }

        #[test]
        fn r3560000_exactly_one_toxopid_comes_of_the_whole_swarm() {
            let mut s = swarming(false, json!([{ "def": STAND_IN, "lane": 1 }, { "def": STAND_IN, "lane": 2 }]));
            s.play(ID, json!({}));
            assert_eq!(crawlers(&s).len(), 3);
            assert_eq!(toxopids(&s), 1);
        }

        #[test]
        fn r64_an_empty_board_takes_five() {
            let mut s = swarming(false, json!([]));
            s.play(ID, json!({}));
            assert_eq!(crawlers(&s).len(), UNIT_ZONES as usize);
        }

        #[test]
        fn r64_a_full_board_takes_none_and_the_spell_still_resolves() {
            let mut s = swarming(false, json!(vec![STAND_IN; UNIT_ZONES as usize]));
            s.play(ID, json!({}));
            assert!(crawlers(&s).is_empty());
            assert_eq!(toxopids(&s), 0);
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r64_fills_every_empty_unit_zone_with_radiant_crawlers() {
            let mut s = swarming(true, json!([{ "def": STAND_IN, "lane": 1 }, { "def": STAND_IN, "lane": 2 }]));
            s.play(ID, json!({}));
            let found = crawlers(&s);
            assert_eq!(found.iter().map(|(lane, _)| *lane).collect::<Vec<_>>(), vec![3, 4, 5]);
            assert!(found.iter().all(|(_, card)| card.radiant));
        }
    }
}
