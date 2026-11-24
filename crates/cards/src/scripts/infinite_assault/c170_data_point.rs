//! SPEC §8.13 IA #170 Data Point ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Discover a card from your opponent's deck."
//!   Radiant: "Discover two cards from your opponent's deck."
//!
//! Built by C8 (#4592). R4592000, R4592002, R4592005.

use jackioh_engine::effects::{
    DiscoverFromLibraryArgs, add_to_hand, discover_from_library,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-170";

const STEP_FIRST: &str = "data-point";
const STEP_SECOND: &str = "data-point-again";
const DISCOVER_COUNT: i32 = 3;

fn data_point(radiant: bool) -> Script {
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
        cry: Some(hook(|_ctx| {
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
        base: data_point(false),
        radiant: data_point(true),
    }
}

// IA #170 Data Point — SPEC §8.13 row 170.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const TARGET1: &str = "core-016";
    const TARGET2: &str = "core-019";

    fn board(radiant: bool, opp_library: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": [FILLER], "library": opp_library },
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
        fn discover_adds_a_copy_and_original_stays() {
            let mut s = board(false, json!([TARGET1, TARGET2]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some());
            let pick = first_option(&s);
            s.answer(json!(pick));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.state().players[P2].library.len(), 2);
        }

        #[test]
        fn with_empty_deck_discover_fizzles() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn discovers_two_cards_in_succession() {
            let mut s = board(true, json!([TARGET1, TARGET2]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some());
            let pick1 = first_option(&s);
            s.answer(json!(pick1));
            // Second prompt opens
            assert!(s.state().pending.is_some());
            let pick2 = first_option(&s);
            s.answer(json!(pick2));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.state().players[P2].library.len(), 2);
        }
    }
}
