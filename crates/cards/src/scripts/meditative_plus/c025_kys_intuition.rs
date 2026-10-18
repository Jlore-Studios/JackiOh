//! M+ #25 KY's Intuition (SPEC §8.9 row 25; R380, R387, R1183004, R1183005): (1) Spell, KY, Rare.
//!
//! Base:    "Discover one of 3 blocks of card code. Add the Radiant card with that code to your hand."
//! Radiant: "Discover one of 3 blocks of card code. Add the Radiant card with that code to your hand.
//!          It costs ({setCost})."
//! Engine: a code Discover (`discover_from_catalog`'s `offer: code`, R1183005) over 3 different
//! non-token cards of the sets that ship (R380), never itself (R387), each offered as its code block
//! alone — its own script's lines of code, compiled in by `crates/cards/build.rs` (R1183004) — so the
//! chooser reads no id, name or face of it until the pick (§10.8, CLAUDE.md rule 7), and the other
//! player only that a prompt is open. The card behind the picked code goes to the hand Radiant (M #88's
//! shape), a full hand burning it (§2.4); the Radiant face adds it at a `costOverride` of setCost.
//! Tunes: setCost 0 ↓, on the Radiant face only (R824000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-025";

/// The one resume step: the Discover's answer comes back here (§10.6).
const PICKED: &str = "picked";

fn intuition(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "offer": "code",
                "prompt": "Discover a card by its code",
            })))]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let Some(picked) = discovered_code_card(ctx) else {
                    return vec![];
                };
                let mut added = json!({ "defId": picked, "radiant": true });
                if radiant {
                    added["costOverride"] = json!(param(ctx, "setCost"));
                }
                vec![add_to_hand(json_as(added))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: intuition(false),
        radiant: intuition(true),
    }
}

// M+ #25 KY's Intuition — SPEC §8.9 row 25, BUILD M11 row M+ 25: "offers 3 distinct non-token cards'
// code blocks, the chooser's view holding no id, name or face of them and the opponent's nothing (rule
// 7); the pick goes to the hand Radiant; a full hand burns it; the AI and a timeout answer it; radiant
// the card costs (0)". The AI's own answer is proved in `crates/ai/tests/ai/outside_data.rs`.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::effects::{CODE_OFFERS_KEY, CODE_OPTION_PREFIX};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    /// p1 holds KY's Intuition (Radiant if `radiant`) and `fillers` fillers; both sides keep cards in
    /// hand so no turn auto-ends.
    fn holding(seed: &str, radiant: bool, fillers: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(vec![json!(FILLER); fillers]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    /// `holding`, KY's Intuition cast: its Discover open.
    fn casting(seed: &str, radiant: bool, fillers: usize) -> Scenario {
        let mut s = holding(seed, radiant, fillers);
        s.play(ID, json!({}));
        s
    }

    /// The cards behind the open Discover's places, in order.
    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.as_ref().expect("the Discover is open");
        pending.resume.data[CODE_OFFERS_KEY]
            .as_array()
            .expect("the places' cards")
            .iter()
            .map(|id| id.as_str().expect("an id").to_string())
            .collect()
    }

    /// The cards p1 holds but the fillers.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod mp25_kys_intuition {
        use super::*;

        #[test]
        fn r1183005_offers_3_distinct_non_token_cards_each_as_its_code_block() {
            let s = casting("intuition", false, 1);
            let pending = s.state().pending.clone().expect("the Discover is open");
            assert_eq!(pending.kind, PromptKind::Discover);
            let behind = offered(&s);
            assert_eq!(behind.len(), 3);
            for (at, id) in behind.iter().enumerate() {
                let def = crate::card_def(id);
                assert!(!def.token, "{id} is no token");
                assert!(set_ships(def.set), "{id} is of a set that ships (R380)");
                assert_ne!(id, ID, "never itself (R387)");
                assert!(!behind[..at].contains(id), "three different cards");
                assert_eq!(
                    Some(pending.options[at].label.as_str()),
                    code_block_of(id),
                    "the place shows its card's code"
                );
            }
        }

        #[test]
        fn r1183005_the_choosers_view_names_no_card_and_the_opponents_holds_nothing() {
            let s = casting("intuition-view", false, 1);
            let behind = offered(&s);
            let mine = serde_json::to_value(view_for(s.state(), P1).pending).expect("serialises");
            let options = mine["options"].as_array().expect("the options").clone();
            assert_eq!(options.len(), 3);
            for option in &options {
                assert_eq!(option["code"], json!(true), "drawn as code");
                assert!(option.get("defId").is_none(), "no definition");
            }
            let text = mine.to_string();
            for id in &behind {
                let def = crate::card_def(id);
                assert!(!text.contains(id.as_str()), "no id of {id}");
                assert!(!text.contains(&format!("\"{}\"", def.name)), "no name of {id}");
                assert!(
                    def.base.text.is_empty() || !text.contains(&def.base.text),
                    "no face of {id}"
                );
            }
            let theirs = serde_json::to_value(view_for(s.state(), P2).pending).expect("serialises");
            assert_eq!(theirs, json!({ "forYou": false, "pendingFor": "p1" }));
        }

        #[test]
        fn r1183005_the_pick_goes_to_the_hand_radiant() {
            let mut s = casting("intuition-pick", false, 1);
            let behind = offered(&s);
            s.answer(json!("code-2"));
            let hand = added(&s);
            assert_eq!(hand.len(), 1);
            assert_eq!(hand[0].def_id, behind[2]);
            assert!(hand[0].radiant, "Radiant");
            assert_eq!(hand[0].cost_override, None, "at its own price");
        }

        #[test]
        fn a_full_hand_burns_it() {
            let mut s = casting("intuition-burn", false, HAND_CAP as usize);
            let behind = offered(&s);
            s.answer(json!("code-0"));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(
                s.state()
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .any(|card| card.def_id == behind[0]),
                "the pick burned"
            );
        }

        #[test]
        fn a_timeout_answers_it() {
            let mut s = casting("intuition-timeout", false, 1);
            assert!(s.state().pending.is_some());
            s.act(ActionBody::Timeout, P1, "p1's clock runs out");
            let picked: Vec<CardInstance> = s
                .state()
                .players
                .p1
                .hand
                .iter()
                .chain(s.state().players.p1.graveyard.iter())
                .filter(|card| card.radiant && card.def_id != ID)
                .cloned()
                .collect();
            assert_eq!(picked.len(), 1, "the timeout picked one, Radiant");
        }

        #[test]
        fn the_random_policy_answers_it_with_one_of_the_places() {
            let s = casting("intuition-policy", false, 1);
            let mut rng = Rng::new("intuition-policy", 0);
            let chosen = subsystems::choose_action(s.state(), P1, &mut rng).expect("an answer");
            let ActionBody::Answer { selection, .. } = chosen else {
                panic!("the policy answers the prompt: {chosen:?}");
            };
            assert!(matches!(
                selection.as_slice(),
                [Selection::Mode { option }] if option.starts_with(CODE_OPTION_PREFIX)
            ));
        }

        #[test]
        fn the_radiant_face_adds_it_at_0() {
            let mut s = casting("intuition-radiant", true, 1);
            let behind = offered(&s);
            s.answer(json!("code-1"));
            let hand = added(&s);
            assert_eq!(hand.len(), 1);
            assert_eq!(hand[0].def_id, behind[1]);
            assert!(hand[0].radiant);
            assert_eq!(hand[0].cost_override, Some(0));
        }

        #[test]
        fn r824000_its_set_cost_is_tuned_on_the_radiant_face_alone() {
            let def = crate::card_def(ID);
            let set_cost = def
                .params
                .as_ref()
                .and_then(|params| params.iter().find(|param| param.key == "setCost"))
                .expect("setCost is declared");
            assert_eq!(set_cost.tuned_on, Some(ParamTunedOn::Radiant));
            let mut s = holding("intuition-param", true, 1);
            step_param(s.card_mut(ID), "setCost", 1);
            s.play(ID, json!({}));
            s.answer(json!("code-0"));
            assert_eq!(added(&s)[0].cost_override, Some(1), "it reads through param()");
        }
    }
}
