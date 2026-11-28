//! SPEC §8.14 FvH #48 Toxic Timmy ((2) Unit, Human, Rare, 3/4 → 6/8), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Poisonous\nCry: Discover a Felinors vs Humans card."
//!   Radiant: "Rush, Poisonous\nCry: Discover a Radiant Felinors vs Humans card."
//!
//! Engine: Poisonous (and the Radiant face's Rush) are catalog keywords. The Cry (R1) is one Discover of
//! three (§6.3) from a pool naming the set, so it reaches the held set's non-token cards (R1420, R5235000),
//! never this card (R387); the pick reaches its controller's hand (a full one burns it, R317), Radiant on
//! the Radiant face, as S+ #39 Mindustrial Cross-over's does (R3549000). Declares no numbers. Built by S01
//! (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-048";

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

fn toxic_timmy(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "query": { "set": SetName::FelinorsVsHumans },
                "prompt": if radiant {
                    "Discover a Radiant Felinors vs Humans card"
                } else {
                    "Discover a Felinors vs Humans card"
                },
            })))]
        })),
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: toxic_timmy(false),
        radiant: toxic_timmy(true),
    }
}

// FvH #48 Toxic Timmy — SPEC §8.14 row 48 (R1420, R387, R3549000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays Toxic Timmy (`radiant` its face) into lane 1.
    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The definitions the open Discover offers p1, in option order.
    fn discover_options(s: &Scenario) -> Vec<String> {
        let pending = must(s.state().pending.as_ref(), "the Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        s.prompt_modes()
    }

    /// R1420, R387: every option is a non-token card of the held set, none of them this one.
    fn offers_the_set(radiant: bool) {
        let s = played(radiant, "fvh-48-offer");
        let options = nonempty(discover_options(&s), "the Discover's options");
        assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
        for id in &options {
            let def = crate::card_def(id);
            assert_eq!(def.set, SetName::FelinorsVsHumans, "{id}");
            assert!(!def.is_token(), "{id}");
            assert_ne!(id, ID, "never this card");
        }
    }

    /// The pick reaches p1's hand, Radiant on the Radiant face.
    fn pick_reaches_hand(radiant: bool) {
        let mut s = played(radiant, "fvh-48-pick");
        let options = discover_options(&s);
        let picked = must(options.first().cloned(), "an option");
        s.answer(json!(picked));
        assert!(s.state().pending.is_none(), "one Discover, answered");
        let added = must(
            s.last_events().iter().find_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            }),
            "the pick reaches the hand",
        );
        assert_eq!(added.def_id, picked);
        assert_eq!(added.zone.z(), ZoneName::Hand);
        assert_eq!(added.radiant, radiant);
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1420_r387_it_discovers_among_the_sets_cards_never_itself() {
            offers_the_set(false);
        }

        #[test]
        fn the_pick_reaches_the_hand() {
            pick_reaches_hand(false);
        }

        #[test]
        fn it_is_poisonous() {
            let mut s = played(false, "fvh-48-poison");
            let first = must(discover_options(&s).into_iter().next(), "an option");
            s.answer(json!(first));
            let timmy = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&timmy).keywords, KeywordKind::Poisonous));
            assert!(!has_keyword(&s.stats(&timmy).keywords, KeywordKind::Rush));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1420_r387_it_discovers_among_the_sets_cards_never_itself() {
            offers_the_set(true);
        }

        #[test]
        fn r3549000_the_pick_reaches_the_hand_radiant() {
            pick_reaches_hand(true);
        }

        #[test]
        fn it_is_poisonous_with_rush() {
            let mut s = played(true, "fvh-48-rush");
            let first = must(discover_options(&s).into_iter().next(), "an option");
            s.answer(json!(first));
            let timmy = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&timmy).keywords, KeywordKind::Poisonous));
            assert!(has_keyword(&s.stats(&timmy).keywords, KeywordKind::Rush));
        }
    }
}
