//! SPEC §8.14 FvH #113 Ball Knower ((1) Unit, Human, Epic, 1/1 → 2/2), a card of the Felinors vs Humans
//! set (R5235000).
//!   Base:    "Cry: Discover a Unit, one of them from the past. If you pick that one, this gains
//!            +{buff}/+{buff}." (3)
//!   Radiant: "Cry: Discover a Radiant Unit, one of them from the past. If you pick that one, this gains
//!            +{buff}/+{buff}." (6)
//!
//! Cry (R1): `discover_from_catalog` of the non-token Units of the sets that ship with `past_options` 1
//! (R5239008): one of the three options is a past Unit version (R3538002), the offer keyed by place so the
//! other seat is not told which, each shown as its card and never labelled. The resume reads the pick with
//! `discovered_card` and `add_to_hand`s it, Radiant on the Radiant face (a past id adds as R3538004's do);
//! the pick always reaches the hand (the designer's note for the AI), and a past pick also `buff`s this
//! +`buff`/+`buff` (R5261004). With no past Unit in the catalog the three options are current ones. Built
//! by S11 (#5261).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-113";

/// The resume step the Discover's answer re-enters (§10.6).
const PICKED: &str = "ball-knower-pick";

/// "One of them from the past" (R5239008).
const PAST_OPTIONS: i32 = 1;

fn knower(radiant: bool) -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        PICKED,
        hook(move |ctx| {
            let Some(picked) = discovered_card(ctx) else {
                return vec![];
            };
            // R5261004: the pick is added either way; a past one also grows this.
            let from_the_past = split_past_id(&picked).is_some();
            let mut effects = vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))];
            if from_the_past {
                let gain = param(&*ctx, "buff");
                let grown = json!({ "target": { "of": "self" }, "attack": gain, "health": gain });
                effects.push(buff(json_as(grown)));
            }
            effects
        }),
    );
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "query": { "type": "Unit" },
                "prompt": "Discover a Unit, one of them from the past",
                "pastOptions": PAST_OPTIONS,
            })))]
        })),
        resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: knower(false),
        radiant: knower(true),
    }
}

// FvH #113 Ball Knower — SPEC §8.14 row 113 (R5261004, R5239008).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const KEEPER: &str = "stand-in (0) 1/1"; // a play always open, so no turn ends by itself

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, KEEPER], "library": filler(4) },
            "p2": { "hand": [KEEPER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The cards behind the open offer's places, in order (R5239008's resume data).
    fn offer_cards(s: &Scenario) -> Vec<String> {
        let pending = must(s.state().pending.clone(), "an open Discover");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, P1);
        let behind = must(pending.resume.data.get(DISCOVER_OFFERS_KEY).cloned(), "the offer's cards");
        must(behind.as_array().cloned(), "a list")
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect()
    }

    /// The place of the offer whose card is (or is not) a past version.
    fn place_of(s: &Scenario, past: bool) -> (String, String) {
        let cards = offer_cards(s);
        let at = must(
            cards.iter().position(|id| split_past_id(id).is_some() == past),
            if past { "a past option" } else { "a current option" },
        );
        (format!("{OFFER_OPTION_PREFIX}{at}"), cards[at].clone())
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != KEEPER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r5239008_it_offers_three_units_exactly_one_from_the_past_keyed_by_place() {
            let s = played(false, "ball-knower-offer");
            let cards = offer_cards(&s);
            assert_eq!(cards.len(), DISCOVER_OPTIONS as usize);
            assert_eq!(cards.iter().filter(|id| split_past_id(id).is_some()).count(), PAST_OPTIONS as usize);
            for id in nonempty(cards, "the offered cards") {
                assert_eq!(def_of(Some(s.state()), &id).type_, CardType::Unit, "{id}");
            }
            for mode in nonempty(s.prompt_modes(), "the options") {
                assert!(mode.starts_with(OFFER_OPTION_PREFIX), "{mode} names its place, never its card");
            }
        }

        #[test]
        fn r5261004_picking_the_past_one_adds_it_and_this_gains_3_3() {
            let mut s = played(false, "ball-knower-past");
            let (place, card) = place_of(&s, true);
            s.answer(pick::mode(&place));
            let got = added(&s);
            assert_eq!(crate::def_ids(&got), vec![card]);
            assert!(!got[0].radiant);
            let knower = s.unit_at(P1, 1);
            s.expect_stats(&knower, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r5261004_picking_a_current_one_adds_it_and_this_gains_nothing() {
            let mut s = played(false, "ball-knower-now");
            let (place, card) = place_of(&s, false);
            s.answer(pick::mode(&place));
            assert_eq!(crate::def_ids(&added(&s)), vec![card]);
            let knower = s.unit_at(P1, 1);
            s.expect_stats(&knower, json!({ "attack": 1, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5261004_picking_the_past_one_adds_it_radiant_and_this_gains_6_6() {
            let mut s = played(true, "ball-knower-radiant-past");
            let (place, card) = place_of(&s, true);
            s.answer(pick::mode(&place));
            let got = added(&s);
            assert_eq!(crate::def_ids(&got), vec![card]);
            assert!(got[0].radiant, "the pick is Radiant");
            let knower = s.unit_at(P1, 1);
            s.expect_stats(&knower, json!({ "attack": 8, "health": 8 }));
        }

        #[test]
        fn r5261004_picking_a_current_one_adds_it_radiant_and_this_gains_nothing() {
            let mut s = played(true, "ball-knower-radiant-now");
            let (place, card) = place_of(&s, false);
            s.answer(pick::mode(&place));
            let got = added(&s);
            assert_eq!(crate::def_ids(&got), vec![card]);
            assert!(got[0].radiant);
            let knower = s.unit_at(P1, 1);
            s.expect_stats(&knower, json!({ "attack": 2, "health": 2 }));
        }
    }
}
