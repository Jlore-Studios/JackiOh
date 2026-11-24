//! SPEC §8.13 IA #185 Deep Crawler ((3) Unit, Epic, 5/7 → 10/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Discover a card from your opponent's deck."
//!   Radiant: "Start of turn: Discover a card from your opponent's deck twice."
//!
//! Built by C8 (#4592). R4592000, R4592002, R4592005.

use jackioh_engine::effects::{
    DiscoverFromLibraryArgs, add_to_hand, discover_from_library,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-185";

const STEP_FIRST: &str = "deep-crawler";
const STEP_SECOND: &str = "deep-crawler-again";
const DISCOVER_COUNT: i32 = 3;

fn deep_crawler(radiant: bool) -> Script {
    let mut resume = IndexMap::new();

    resume.insert(
        STEP_FIRST,
        hook(move |_ctx| {
            let Some(Selection::Instance { instance_id }) = _ctx.targets.first() else {
                return vec![];
            };
            let mut effects = vec![add_to_hand(json_as(json!({ "copyOf": instance_id })))];
            if radiant {
                effects.push(discover_from_library(DiscoverFromLibraryArgs {
                    step: STEP_SECOND.into(),
                    count: Some(DISCOVER_COUNT),
                    player: Some(PlayerSpec::Enemy),
                    prompt: Some("Discover a card from your opponent's deck".into()),
                    ..Default::default()
                }));
            }
            effects
        }),
    );

    if radiant {
        resume.insert(
            STEP_SECOND,
            hook(|ctx| {
                let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "copyOf": instance_id })))]
            }),
        );
    }

    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![discover_from_library(DiscoverFromLibraryArgs {
                step: STEP_FIRST.into(),
                count: Some(DISCOVER_COUNT),
                player: Some(PlayerSpec::Enemy),
                prompt: Some("Discover a card from your opponent's deck".into()),
                ..Default::default()
            })]
        })),
        resume,
        secret_option_counts: true,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: deep_crawler(false),
        radiant: deep_crawler(true),
    }
}

// IA #185 Deep Crawler — SPEC §8.13 row 185.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TARGET1: &str = "core-001";
    const TARGET2: &str = "core-008";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "mana": 10,
            },
            "p2": {
                "library": [TARGET1, TARGET2],
                "mana": 10,
            },
        }))
    }

    fn first_option(s: &Scenario) -> String {
        let pending = must(s.state().pending.clone(), "pending prompt");
        match &pending.options[0].selection {
            Selection::Instance { instance_id } => instance_id.clone(),
            _ => panic!("expected instance selection"),
        }
    }

    mod base {
        use super::*;

        #[test]
        fn discovers_from_opponents_deck_at_start_of_turn() {
            let mut s = board(false);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 5, "health": 7 }));
            s.end_turn(); // p1 turn starts -> discover prompt opens
            assert!(s.state().pending.is_some());
            let pick = first_option(&s);
            s.answer(json!(pick));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.state().players[P2].library.len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_10_14_and_discovers_twice_at_start_of_turn() {
            let mut s = board(true);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 10, "health": 14 }));
            s.end_turn();
            assert!(s.state().pending.is_some());
            let pick1 = first_option(&s);
            s.answer(json!(pick1));
            // Second prompt
            assert!(s.state().pending.is_some());
            let pick2 = first_option(&s);
            s.answer(json!(pick2));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
