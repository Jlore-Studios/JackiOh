//! SPEC §8.12 S+ #134 Basket of Crawlers (§2.4, §6.3 Add to hand; R386, R1420, R3535000): (2) Spell,
//! Mindustrial, Common, a card of the Stinkyscape+ set.
//!   Base:    "Add 3 Crawlers to your hand."
//!   Radiant: "Add 3 Radiant Crawlers to your hand."
//!
//! Engine: its Cry adds `crawlers` fresh Crawlers (S+ #120) to its caster's hand, one `addToHand` for each,
//! the number declared per face (R386, 3 on both). The Radiant face adds them Radiant. A full hand burns
//! the overflow (§2.4). No prompt opens and nothing is drawn from the rng.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-134";

/// The Crawler (S+ #120).
const CRAWLER: &str = "stinkyscapeplus-120";

fn basket(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(ctx, "crawlers"))
                .map(|_| add_to_hand(json_as(json!({ "defId": CRAWLER, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: basket(false),
        radiant: basket(true),
    }
}

// S+ #134 Basket of Crawlers — SPEC §8.12 row S+ 134 (R386, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// "Add 3 Crawlers", declared as `crawlers`.
    const CRAWLERS: usize = 3;

    fn basketing(radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": "s134",
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn crawlers(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == CRAWLER).collect()
    }

    #[test]
    fn declared_crawlers_is_3() {
        let s = basketing(false, 0);
        for radiant in [false, true] {
            let def = crate::card_def(ID);
            let param = def.params.as_ref().and_then(|params| params.first()).expect("a declared number");
            assert_eq!(param.key, "crawlers");
            assert_eq!(if radiant { param.radiant } else { param.base }, CRAWLERS as i32);
        }
        assert_eq!(param_value(s.state(), Some(s.card(ID)), "crawlers", ParamValueOptions::default()), CRAWLERS as i32);
    }

    mod base {
        use super::*;

        #[test]
        fn adds_three_crawlers() {
            let mut s = basketing(false, 0);
            s.play(ID, json!({}));
            let held = crawlers(&s);
            assert_eq!(held.len(), CRAWLERS);
            assert!(held.iter().all(|card| !card.radiant));
            assert_eq!(s.hand(P1).len(), CRAWLERS);
            assert!(s.state().pending.is_none(), "no prompt opens");
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_overflow() {
            let mut s = basketing(false, HAND_CAP as usize - 1);
            s.play(ID, json!({}));
            // The cast leaves 9 cards in the hand, so one Crawler fits and two burn.
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(crawlers(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == CRAWLER))
                .count();
            assert_eq!(burned, CRAWLERS - 1);
        }

        #[test]
        fn r386_an_upgrade_adds_more_crawlers() {
            let mut s = basketing(false, 0);
            let moved = crate::upgrade_number(&mut s, ID, "crawlers");
            assert_eq!(moved, CRAWLERS as i32 + 1);
            s.play(ID, json!({}));
            assert_eq!(crawlers(&s).len(), CRAWLERS + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_three_radiant_crawlers() {
            let mut s = basketing(true, 0);
            s.play(ID, json!({}));
            let held = crawlers(&s);
            assert_eq!(held.len(), CRAWLERS);
            assert!(held.iter().all(|card| card.radiant));
        }
    }
}
