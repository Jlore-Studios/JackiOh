//! C+ #36 Conjure Bones (SPEC §8.7 row 36): (2) embiggen (4) Spell, Rare.
//!   Base:    "Shuffle {count} Bone Storms into your deck. Paid (4): {paidCount} instead."
//!   Radiant: the same, the Bone Storms Radiant.
//! The embiggen price is a play-time choice that lands on the instance (R81, `ctx.embiggened`);
//! `shuffleInto` puts each fresh token at a random position and R80's cap turns the rest away.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-036";

const BONE_STORM: &str = "classicplus-036-1";

fn conjure_bones(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let key = if ctx.embiggened { "paidCount" } else { "count" };
            vec![shuffle_into(json_as(json!({
                "defId": BONE_STORM,
                "count": param(&*ctx, key),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: conjure_bones(false),
        radiant: conjure_bones(true),
    }
}

// C+ #36 Conjure Bones — SPEC §8.7 row 36, BUILD M9 Classic+ row C+ 36: "Shuffles 7 Bone Storms
// (C+ #36.1) into your deck, each at a uniformly random position; the embiggen price (4) chosen with
// the play (R81) shuffles 17; a deck at `LIBRARY_CAP` (60) turns the rest away with `libraryOverflow`
// (R80); your library list shows them (R311) and `shuffledIn` positions are blank for both players
// (R97); count and paid count read through `param()` (steps 2 and 4); radiant the Bone Storms are
// Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BONES: &str = "classicplus-036";
    const BONE_STORM: &str = "classicplus-036-1";
    const FILLER: &str = "core-005"; // Stockpile: inert in a library.

    /// The harness's `scenario`, with the shipped cards registered first (the TS harness did it at import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn bones_in(s: &Scenario, player: PlayerId) -> usize {
        s.pile(player, "library").into_iter().filter(|card| card.def_id == BONE_STORM).count()
    }

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events()
            .iter()
            .map(|event| serde_json::to_value(event).expect("an event is JSON"))
            .filter(|event| event["type"] == kind)
            .collect()
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn cast(radiant: bool, embiggen: bool, library: Option<i32>) -> Scenario {
        let library: Vec<&str> = (0..library.unwrap_or(10)).map(|_| FILLER).collect();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": BONES, "radiant": radiant }, FILLER], "library": library, "mana": 4 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(BONES, if embiggen { json!({ "embiggen": true }) } else { json!({}) });
        s
    }

    mod base {
        use super::*;

        #[test]
        fn shuffles_7_base_bone_storms_into_your_deck() {
            let mut s = cast(false, false, None);
            assert_eq!(bones_in(&s, P1), 7);
            assert_eq!(s.pile(P1, "library").len(), 17);
            assert!(s.pile(P1, "library").iter().filter(|card| card.def_id == BONE_STORM).all(|card| !card.radiant));
            assert_eq!(events_of(&s, "shuffledIn").len(), 7);
            s.expect_mana(P1, 2);
        }

        #[test]
        fn each_goes_in_at_a_random_position_the_7_are_not_stacked_on_top_or_at_the_bottom() {
            let s = cast(false, false, Some(20));
            let library = s.pile(P1, "library");
            let at: Vec<usize> = library
                .iter()
                .enumerate()
                .filter(|(_, card)| card.def_id == BONE_STORM)
                .map(|(index, _)| index)
                .collect();
            assert_eq!(at.len(), 7);
            assert_ne!(at, vec![0, 1, 2, 3, 4, 5, 6]);
            assert_ne!(at, vec![20, 21, 22, 23, 24, 25, 26]);
        }

        #[test]
        fn r81_the_embiggen_price_4_is_chosen_with_the_play_and_shuffles_17() {
            let mut s = cast(false, true, None);
            assert_eq!(bones_in(&s, P1), 17);
            s.expect_mana(P1, 0);
        }

        #[test]
        fn r80_a_deck_at_library_cap_turns_the_rest_away_each_with_libraryoverflow() {
            let s = cast(false, false, Some(LIBRARY_CAP - 3));
            assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
            assert_eq!(bones_in(&s, P1), 3);
            let overflow = events_of(&s, "libraryOverflow");
            assert_eq!(overflow.len(), 4);
            assert!(overflow.iter().all(|event| event["defId"] == BONE_STORM && event["player"] == "p1"));
        }

        #[test]
        fn r311_your_library_list_shows_them_as_they_went_in_openly() {
            let s = cast(false, false, None);
            let own = view(&s, P1)["you"]["ownLibrary"].clone();
            let cards = own["cards"].as_array().cloned().unwrap_or_default();
            assert!(cards.contains(&json!({ "defId": BONE_STORM, "radiant": false, "count": 7 })));
            assert_eq!(own["unknown"], 0);
        }

        #[test]
        fn r97_shuffledin_carries_no_position_for_either_player() {
            let s = cast(false, false, None);
            for viewer in [P1, P2] {
                let seen = view(&s, viewer);
                let shuffled: Vec<Value> = seen["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] == "shuffledIn")
                    .collect();
                assert!(!shuffled.is_empty());
                let real: Vec<Value> = events_of(&s, "shuffledIn").iter().map(|event| event["position"].clone()).collect();
                for event in &shuffled {
                    assert!(!real.contains(&event["position"]));
                }
            }
            // The opponent sees a count, not the list.
            let theirs = view(&s, P2);
            assert!(theirs["opponent"].get("ownLibrary").is_none());
            assert_eq!(theirs["opponent"]["libraryCount"], 17);
        }

        #[test]
        fn r386_count_and_paid_count_read_through_param_steps_2_and_4() {
            let mut up = scenario(json!({ "p1": { "hand": [BONES, FILLER], "library": [FILLER], "mana": 4 }, "p2": { "hand": [FILLER] } }));
            step_param(up.card_mut(BONES), "count", 1);
            up.play(BONES, json!({}));
            assert_eq!(bones_in(&up, P1), 9);

            let mut paid = scenario(json!({ "p1": { "hand": [BONES, FILLER], "library": [FILLER], "mana": 4 }, "p2": { "hand": [FILLER] } }));
            step_param(paid.card_mut(BONES), "paidCount", -1);
            paid.play(BONES, json!({ "embiggen": true }));
            assert_eq!(bones_in(&paid, P1), 13);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn shuffles_7_radiant_bone_storms() {
            let s = cast(true, false, None);
            let storms: Vec<CardInstance> =
                s.pile(P1, "library").into_iter().filter(|card| card.def_id == BONE_STORM).collect();
            assert_eq!(storms.len(), 7);
            assert!(storms.iter().all(|card| card.radiant));
            let listed = view(&s, P1)["you"]["ownLibrary"]["cards"].as_array().cloned().unwrap_or_default();
            assert!(listed.contains(&json!({ "defId": BONE_STORM, "radiant": true, "count": 7 })));
        }

        #[test]
        fn paid_4_17_radiant_bone_storms() {
            let s = cast(true, true, None);
            let storms: Vec<CardInstance> =
                s.pile(P1, "library").into_iter().filter(|card| card.def_id == BONE_STORM).collect();
            assert_eq!(storms.len(), 17);
            assert!(storms.iter().all(|card| card.radiant));
        }
    }
}
