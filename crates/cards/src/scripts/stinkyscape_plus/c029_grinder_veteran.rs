//! SPEC §8.12 S+ #29 Grinder Veteran, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    5/5, Deft; "Activate: Discard a Felinor to add {summons|Warhero Token|Warhero Tokens}
//!            to your hand."
//!   Radiant: 10/10, Deft; "… add {summons|Radiant Warhero Token|Radiant Warhero Tokens} …"
//!
//! Engine: Activate (§6.2, R384), once a turn: its controller picks a Felinor-tagged card of their
//! own hand (R3553001) as the activation resolves — a `chooseFromHand` prompt the `resume` step
//! answers (C #81's shape) — discards it, then adds the declared `summons` Warhero Tokens
//! (Radiant ones on the Radiant face) as fresh cards (`add_to_hand` with the token's `defId`,
//! C+ #60's shape), minting them, which starts the worth (R3548002). With no Felinor in hand the
//! ability is not offered. Deft is catalog data.

use std::sync::Arc;

use jackioh_engine::effects::{add_to_hand, choose_from_hand, discard};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-029";

/// The tokens the Activate adds (S+ #T-warhero-token Warhero Token).
const WARHERO: &str = "stinkyscapeplus-t-warhero-token";
/// The resume step answering the Felinor pick.
const DISCARD: &str = "discard";

/// R3553001: "a Felinor" is a Felinor-tagged card.
fn felinor(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    tags_of(ctx.state, card).contains(&Tag::Felinor)
}

fn veteran(radiant: bool) -> Script {
    let label = if radiant {
        "Discard a Felinor: add 2 Radiant Warhero Tokens to your hand"
    } else {
        "Discard a Felinor: add 2 Warhero Tokens to your hand"
    };
    Script {
        activations: vec![ActivationDecl {
            id: "grind".to_string(),
            label: label.to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: Some(condition_hook(|c| {
                zone_cards(c.state, c.controller, OffFieldZone::Hand)
                    .iter()
                    .any(|card| tags_of(c.state, card).contains(&Tag::Felinor))
            })),
            has: None,
            run: hook(move |_ctx| {
                vec![choose_from_hand(ChooseFromHandArgs {
                    step: DISCARD.to_string(),
                    count: Some(1),
                    prompt: Some(label.to_string()),
                    where_: Some(Arc::new(felinor) as HandFilter),
                    ..Default::default()
                })]
            }),
        }],
        resume: IndexMap::from([(
            DISCARD,
            hook(move |ctx| {
                let mut effects =
                    vec![discard(json_as(json!({ "target": { "of": "chosen", "index": 0 } })))];
                effects.extend((0..param(&*ctx, "summons")).map(|_| {
                    add_to_hand(json_as(json!({ "defId": WARHERO, "radiant": radiant })))
                }));
                effects
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: veteran(false),
        radiant: veteran(true),
    }
}

// S+ #29 Grinder Veteran — SPEC §8.12 row S+ 29 (R3553001; R384, R3548002).
// Activate discarding a chosen Felinor for two Warhero Tokens.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const FELINOR_A: &str = "core-086"; // "Miss" Mrow: a Felinor
    const FELINOR_B: &str = "meditative-030"; // Fickle E-Kitten: a Felinor
    const FILLER: &str = "core-005"; // Stockpile: not a Felinor

    fn veteran_board(radiant: bool, seed: &str, hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": hand, "library": [FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER] },
        }))
    }

    fn two_felinors_and_filler() -> Value {
        json!([FELINOR_A, FELINOR_B, FILLER])
    }

    /// The instance ids the open hand prompt offers.
    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("a hand prompt is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Hand, P1));
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn offered_def_ids(s: &Scenario) -> Vec<String> {
        offered(s)
            .iter()
            .map(|id| s.card(id.as_str()).def_id.clone())
            .collect()
    }

    /// How many Activate actions p1's veteran is offered.
    fn activates(s: &Scenario) -> usize {
        let id = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|body| {
                matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == id)
            })
            .count()
    }

    fn activate(s: &mut Scenario) -> &mut Scenario {
        s.activate(ID, json!({ "ability": "grind" }))
    }

    fn warhero_tokens(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == WARHERO)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3553001_the_discard_is_the_chosen_felinor() {
            let mut s = veteran_board(false, "s29-chosen", two_felinors_and_filler());
            let second = s
                .hand(P1)
                .iter()
                .find(|card| card.def_id == FELINOR_B)
                .expect("the second Felinor")
                .id
                .clone();
            assert_eq!(activates(&s), 1);
            activate(&mut s);
            assert_eq!(offered_def_ids(&s), vec![FELINOR_A.to_string(), FELINOR_B.to_string()]);

            s.answer(json!(second));

            assert!(s.state().pending.is_none());
            assert!(
                s.pile(P1, "graveyard").iter().any(|card| card.id == second),
                "the chosen Felinor reached the graveyard"
            );
            assert!(s.hand(P1).iter().any(|card| card.def_id == FELINOR_A), "the other stays");
            assert!(s.hand(P1).iter().any(|card| card.def_id == FILLER), "never an option");
            let tokens = warhero_tokens(&s);
            assert_eq!(tokens.len(), 2);
            assert!(tokens.iter().all(|card| !card.radiant));
        }

        #[test]
        fn the_prompt_offers_only_felinors() {
            let mut s = veteran_board(false, "s29-offers", two_felinors_and_filler());
            activate(&mut s);
            assert_eq!(offered(&s).len(), 2);
            for def_id in offered_def_ids(&s) {
                assert!(
                    crate::card_def(&def_id).tags.contains(&Tag::Felinor),
                    "{def_id} is no Felinor"
                );
            }
        }

        #[test]
        fn no_felinor_no_activate() {
            let mut s = veteran_board(false, "s29-none", json!([FILLER]));
            assert_eq!(activates(&s), 0);
            s.expect_refused(activate);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn base_adds_two_warhero_tokens_and_starts_the_worth() {
            let mut s = veteran_board(false, "s29-worth", two_felinors_and_filler());
            assert_eq!(s.state().warhero_worth, None);
            activate(&mut s);
            s.answer(json!(offered(&s)[0]));
            assert_eq!(warhero_tokens(&s).len(), 2);
            assert_eq!(s.state().warhero_worth, Some(0), "minting starts the worth (R3548002)");
            s.expect_events(json!(["activated", "discarded", "addedToHand", "addedToHand"]));
        }

        #[test]
        fn the_random_policy_answers_with_a_felinor() {
            let mut s = veteran_board(false, "s29-policy", two_felinors_and_filler());
            activate(&mut s);
            let state = s.state().clone();
            let mut rng = Rng::new("s29-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "s29-policy-answer".to_string()));
            assert!(next.error.is_none(), "{:?}", next.error);
            assert!(next.state.pending.is_none());
            let discarded: Vec<&CardInstance> = next.state.players.p1
                .graveyard
                .iter()
                .filter(|card| {
                    card.def_id == FELINOR_A || card.def_id == FELINOR_B
                })
                .collect();
            assert_eq!(discarded.len(), 1, "one Felinor discarded");
            assert_eq!(
                next.state.players.p1
                    .hand
                    .iter()
                    .filter(|card| card.def_id == WARHERO)
                    .count(),
                2
            );
        }

        #[test]
        fn a_timeout_answers_with_a_felinor() {
            let mut s = veteran_board(false, "s29-timeout", two_felinors_and_filler());
            activate(&mut s);
            s.act(ActionBody::Timeout, P1, "the controller's clock runs out");
            assert!(s.state().pending.is_none());
            assert_eq!(
                s.pile(P1, "graveyard")
                    .iter()
                    .filter(|card| card.def_id == FELINOR_A || card.def_id == FELINOR_B)
                    .count(),
                1
            );
            assert_eq!(warhero_tokens(&s).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_adds_two_radiant_tokens() {
            let mut s = veteran_board(true, "s29-radiant", two_felinors_and_filler());
            activate(&mut s);
            s.answer(json!(offered(&s)[0]));
            let tokens = warhero_tokens(&s);
            assert_eq!(tokens.len(), 2);
            assert!(tokens.iter().all(|card| card.radiant), "Radiant Warhero Tokens");
            assert_eq!(s.state().warhero_worth, Some(0));
        }
    }
}
