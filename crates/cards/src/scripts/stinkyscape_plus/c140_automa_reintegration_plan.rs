//! SPEC §8.12 S+ #140 Automa Reintegration Plan (§5.1, §6.1 Starship Piece, §6.3 Discover; R65, R382, R387,
//! R1420, R1878003, R3535000, R3549000, R3565001, R3565005): (1) Spell, Mindustrial, Common, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Discover an Automa Unit, it gains Starship Piece."
//!   Radiant: "Discover a Radiant Automa Unit, it gains Starship Piece."
//!
//! Engine: one Discover of three (§6.3) from the Stinkyscape+ Units carrying any of the six Automa tags
//! (R3565001), tokens out (§5.1, R382), the pool naming its set so that it reaches them while the set is held
//! (R1420); it never offers this card (R387), a Spell. The pick reaches its caster's hand (§2.4's cap burns
//! it) given Starship Piece, kept onto the field (R1878003), as `addRandomFromCatalog`'s `keywords` rider
//! gives it: a pool of the one card picked, since `addToHand` has no rider (R3549000, R3565005). It is Cross-
//! over's Discover without Equip or a discount: a piece played as a Unit fuses into a Starship (R1852000). The
//! Radiant face adds the pick Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-140";

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

/// R3565001: "Automa" is any of the six Automa tags.
const AUTOMA: [Tag; 6] = [
    Tag::AutomaAlpha,
    Tag::AutomaBeta,
    Tag::AutomaGamma,
    Tag::AutomaDelta,
    Tag::AutomaOmega,
    Tag::AutomaZeta,
];

/// R3565005: "an Automa Unit", the held set's.
fn automa_units() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "type": CardType::Unit, "anyTags": AUTOMA }))
}

fn reintegration(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "query": automa_units(),
                "prompt": if radiant {
                    "Discover a Radiant Automa Unit"
                } else {
                    "Discover an Automa Unit"
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
                vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                    query: Some(json_as(json!({ "defId": [picked] }))),
                    count: Some(1),
                    radiant: radiant.then_some(true),
                    keywords: Some(vec![Keyword::StarshipPiece]),
                    ..AddRandomFromCatalogArgs::default()
                })]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reintegration(false),
        radiant: reintegration(true),
    }
}

// S+ #140 Automa Reintegration Plan — SPEC §8.12 row S+ 140 (R3565005; R3549000, R1878003, R387, R1420).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    /// Every non-token Automa Unit of the held set: Dagger to Corvus (S+ #110 to #127), Toxopid among them.
    fn expected_pool() -> Vec<String> {
        let mut ids: Vec<String> = (110..=127).map(|n| format!("stinkyscapeplus-{n}")).collect();
        ids.sort();
        ids
    }

    fn reintegrating(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The definitions the open Discover offers, in option order.
    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.as_ref().expect("the Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        let offered = s.prompt_modes();
        assert_eq!(offered.len(), pending.options.len(), "a card option each");
        offered
    }

    /// The card the last step added to p1's hand, if one was.
    fn added(s: &Scenario) -> Option<CardInstance> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
            _ => None,
        })
    }

    /// Plays the Spell and answers the Discover with its `n`th option.
    fn pick(s: &mut Scenario, n: usize) -> String {
        s.play(ID, json!({}));
        let options = offered(s);
        s.answer(json!(options[n]));
        options[n].clone()
    }

    mod base {
        use super::*;

        #[test]
        fn r3565005_the_discover_offers_three_automa_units_of_the_held_set() {
            let mut s = reintegrating(false, "s140-offer", 0);
            s.play(ID, json!({}));
            let options = offered(&s);
            assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
            assert_eq!(options.iter().collect::<IndexSet<_>>().len(), options.len(), "without replacement");
            for id in &options {
                let def = crate::card_def(id);
                assert_eq!(def.set, SetName::StinkyscapePlus, "{id}");
                assert_eq!(def.type_, CardType::Unit, "{id}");
                assert!(def.tags.iter().any(|tag| AUTOMA.contains(tag)), "{id}");
                assert!(!def.is_token(), "{id}");
            }
        }

        #[test]
        fn r3565005_over_many_seeds_the_pool_is_the_held_sets_automa_units_never_this_one() {
            // The scenario registers the catalog, which the pool reads.
            let _registered = reintegrating(false, "s140-pool", 0);
            let mut pool: Vec<String> =
                crate::query::pool(ID, &automa_units()).iter().map(|def| def.id.clone()).collect();
            pool.sort();
            assert_eq!(pool, expected_pool());
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..150 {
                let mut s = reintegrating(false, &format!("s140-pool-{i}"), 0);
                s.play(ID, json!({}));
                seen.extend(offered(&s));
            }
            let mut found: Vec<String> = seen.into_iter().collect();
            found.sort();
            assert_eq!(found, expected_pool());
            assert!(!found.iter().any(|id| id == ID));
        }

        #[test]
        fn r3565005_the_pick_reaches_the_hand_with_starship_piece() {
            let mut s = reintegrating(false, "s140-pick", 0);
            let picked = pick(&mut s, 1);
            assert!(s.state().pending.is_none(), "one Discover, answered");
            let card = added(&s).expect("the pick reaches the hand");
            assert_eq!(card.def_id, picked);
            assert_eq!(card.zone.z(), ZoneName::Hand);
            assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece]);
            assert!(!card.radiant);
            assert_eq!(card.cost_mod, 0, "the printed price");
            assert!(card.cost_override.is_none());
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3565005_it_is_the_picked_card_that_arrives_whichever_option_is_taken() {
            for n in 0..DISCOVER_OPTIONS as usize {
                let mut s = reintegrating(false, "s140-which", 0);
                let picked = pick(&mut s, n);
                assert_eq!(added(&s).expect("the pick").def_id, picked);
                assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == picked).count(), 1);
            }
        }

        #[test]
        fn r1878003_the_card_keeps_starship_piece_on_the_field_and_wears_it() {
            let seed = seed_where("s140-keeps", |seed| {
                let mut s = reintegrating(false, seed, 0);
                s.play(ID, json!({}));
                crate::card_def(&offered(&s)[0]).cost == CardCost::Fixed(0)
            });
            let mut s = reintegrating(false, &seed, 0);
            pick(&mut s, 0);
            let card = added(&s).expect("a (0) Unit");
            s.play(card.id.as_str(), json!({ "zone": 1 }));
            let unit = s.unit(P1, 1).expect("it stands on the field").id;
            assert_eq!(unit, card.id);
            let keywords = s.stats(unit.as_str()).keywords;
            assert!(has_keyword(&keywords, KeywordKind::StarshipPiece));
            assert!(!has_keyword(&keywords, KeywordKind::Equip), "no Equip, which Cross-over gives and this does not");
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_pick() {
            let mut s = reintegrating(false, "s140-full", 10);
            let picked = pick(&mut s, 0);
            assert!(added(&s).is_none());
            let burned: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned, vec![picked]);
        }

        #[test]
        fn r97_the_opponent_sees_neither_the_offer_nor_the_pick() {
            let mut s = reintegrating(false, "s140-hidden", 0);
            s.play(ID, json!({}));
            let options = offered(&s);
            let text = serde_json::to_string(&s.view(P2)).unwrap();
            for id in &options {
                assert!(!text.contains(&format!("\"{id}\"")), "{id}");
            }
            s.answer(json!(options[0]));
            let card = added(&s).expect("the pick").id;
            let theirs = s.view(P2);
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&format!("\"{card}\"")));
            let events: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: P1, .. }))
                .collect();
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events[0],
                GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
            ));
        }

        #[test]
        fn r44_the_random_policy_answers_the_discover() {
            let mut s = reintegrating(false, "s140-policy", 0);
            s.play(ID, json!({}));
            let state = s.state().clone();
            let mut rng = Rng::new("s140-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "s140-policy-answer".to_string()));
            assert!(next.error.is_none(), "{:?}", next.error);
            assert!(next.state.pending.is_none());
            assert!(
                next.state
                    .players
                    .p1
                    .hand
                    .iter()
                    .any(|card| card.granted_keywords.contains(&Keyword::StarshipPiece))
            );
        }

        #[test]
        fn r79_a_timeout_answers_the_discover() {
            let mut s = reintegrating(false, "s140-timeout", 0);
            s.play(ID, json!({}));
            let asked = s.state().pending.as_ref().map(|pending| pending.id.clone());
            s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
            assert_ne!(s.state().pending.as_ref().map(|pending| pending.id.clone()), asked);
            assert!(s.state().pending.is_none());
            assert!(s.hand(P1).iter().any(|card| card.granted_keywords.contains(&Keyword::StarshipPiece)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565005_the_pick_is_radiant_with_starship_piece_and_no_discount() {
            let mut s = reintegrating(true, "s140-radiant", 0);
            let picked = pick(&mut s, 0);
            let card = added(&s).expect("the pick reaches the hand");
            assert_eq!(card.def_id, picked);
            assert!(card.radiant);
            assert_eq!(card.cost_mod, 0, "no discount");
            assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece]);
        }

        #[test]
        fn r3565005_the_radiant_discover_offers_the_same_pool() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = reintegrating(true, &format!("s140-rpool-{i}"), 0);
                s.play(ID, json!({}));
                for id in offered(&s) {
                    let def = crate::card_def(&id);
                    assert!(def.type_ == CardType::Unit && def.tags.iter().any(|tag| AUTOMA.contains(tag)), "{id}");
                    seen.insert(id);
                }
            }
            assert!(seen.len() > 10, "{}", seen.len());
        }

        #[test]
        fn r97_the_opponent_sees_neither_the_radiant_card() {
            let mut s = reintegrating(true, "s140-rhidden", 0);
            pick(&mut s, 0);
            let card = added(&s).expect("the pick").id;
            let theirs = s.view(P2);
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&format!("\"{card}\"")));
            for event in &theirs.events {
                if let GameEvent::AddedToHand { player: P1, instance_id, .. } = event {
                    assert_eq!(instance_id, "hidden");
                }
            }
        }

        #[test]
        fn r79_a_timeout_answers_the_radiant_discover_too() {
            let mut s = reintegrating(true, "s140-rtimeout", 0);
            s.play(ID, json!({}));
            s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
            assert!(s.state().pending.is_none());
            let card = s.hand(P1).into_iter().find(|card| card.granted_keywords.contains(&Keyword::StarshipPiece));
            assert!(card.is_some_and(|card| card.radiant));
        }
    }
}
