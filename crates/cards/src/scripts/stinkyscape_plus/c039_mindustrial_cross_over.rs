//! SPEC §8.12 S+ #39 Mindustrial Cross-over (§5.1, §6.1 Equip and Starship Piece, §6.3 Discover; R65, R387,
//! R1420, R1878003, R3535000, R3549000): (1) Spell, Rare, a card of the Stinkyscape+ set.
//!   Base:    "Discover a Mindustrial Unit. Give it Starship Piece."
//!   Radiant: "Discover a Radiant Mindustrial Unit. Give it Starship Piece. It costs (1) less."
//!
//! Engine: one Discover of three (§6.3) from the Stinkyscape+ Units tagged Mindustrial, tokens out, the pool
//! naming its set so that it reaches them while the set is held (R1420); it never offers this card (R387),
//! a Spell. The pick reaches its caster's hand (§2.4's cap burns it) given Starship Piece, kept onto the
//! field (R1878003), as `addRandomFromCatalog`'s `keywords` rider gives it: a pool of the one card picked,
//! since `addToHand` has no rider (R3549000). "Starpiece" is Starship Piece (SP5). Its death fuses a copy
//! into its owner's Dormant Starship (R1852000). The Radiant face adds the pick Radiant and costing
//! `DISCOUNT` less, never below (0) (R65). R4285001: through v0.6.7k it gave Equip too, so the card might be
//! Equipped into one of its owner's Units, its Starship Piece staying only with a piece (R1852001)
//! (`script_v0_6_7k`, kept, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-039";

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";
/// "it costs (1) less" on the Radiant face (R1748003: printed, declared nowhere).
const DISCOUNT: i32 = 1;

/// R3549000: "a random Mindustrial Unit", the held set's.
fn mindustrial_units() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "type": CardType::Unit, "tags": [Tag::Mindustrial] }))
}

/// R4285001: what the pick is given: Starship Piece, and through v0.6.7k Equip too (`with_equip`).
fn granted(with_equip: bool) -> Vec<Keyword> {
    if with_equip {
        vec![Keyword::StarshipPiece, Keyword::Equip]
    } else {
        vec![Keyword::StarshipPiece]
    }
}

fn cross_over(radiant: bool, with_equip: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "query": mindustrial_units(),
                "prompt": if radiant {
                    "Discover a Radiant Mindustrial Unit"
                } else {
                    "Discover a Mindustrial Unit"
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
                let mut args = AddRandomFromCatalogArgs {
                    query: Some(json_as(json!({ "defId": [picked] }))),
                    count: Some(1),
                    keywords: Some(granted(with_equip)),
                    ..AddRandomFromCatalogArgs::default()
                };
                if radiant {
                    args.radiant = Some(true);
                    args.cost_mod = Some(-DISCOUNT);
                }
                vec![add_random_from_catalog(args)]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: cross_over(false, false),
        radiant: cross_over(true, false),
    }
}

/// R767000, R4285001: through v0.6.7k the pick was given Equip too (issue #4285); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_7k() -> CardScripts {
    CardScripts {
        base: cross_over(false, true),
        radiant: cross_over(true, true),
    }
}

// S+ #39 Mindustrial Cross-over — SPEC §8.12 row S+ 39 (R3549000, R4285001; R1878003, R387, R1420).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    fn crossing(radiant: bool, seed: &str, fillers: usize) -> Scenario {
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
        fn r3549000_the_discover_offers_three_mindustrial_units_of_the_held_set() {
            let mut s = crossing(false, "s39-offer", 0);
            s.play(ID, json!({}));
            let options = offered(&s);
            assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
            assert_eq!(options.iter().collect::<IndexSet<_>>().len(), options.len(), "without replacement");
            for id in &options {
                let def = crate::card_def(id);
                assert_eq!(def.set, SetName::StinkyscapePlus, "{id}");
                assert_eq!(def.type_, CardType::Unit, "{id}");
                assert!(def.tags.contains(&Tag::Mindustrial), "{id}");
                assert!(!def.is_token(), "{id}");
            }
        }

        #[test]
        fn r3549000_over_many_seeds_the_pool_is_the_held_sets_mindustrial_units_never_this_one() {
            // The scenario registers the catalog, which the pool reads.
            let _registered = crossing(false, "s39-pool", 0);
            let mut expected: Vec<String> =
                crate::query::pool(ID, &mindustrial_units()).iter().map(|def| def.id.clone()).collect();
            expected.sort();
            assert!(expected.len() > DISCOVER_OPTIONS as usize);
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..150 {
                let mut s = crossing(false, &format!("s39-pool-{i}"), 0);
                s.play(ID, json!({}));
                seen.extend(offered(&s));
            }
            let mut found: Vec<String> = seen.into_iter().collect();
            found.sort();
            assert_eq!(found, expected);
            assert!(!found.iter().any(|id| id == ID));
        }

        #[test]
        fn r3549000_r4285001_the_pick_reaches_the_hand_with_starship_piece_and_no_equip() {
            let mut s = crossing(false, "s39-pick", 0);
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
        fn r3549000_it_is_the_picked_card_that_arrives_whichever_option_is_taken() {
            for n in 0..DISCOVER_OPTIONS as usize {
                let mut s = crossing(false, "s39-which", 0);
                let picked = pick(&mut s, n);
                assert_eq!(added(&s).expect("the pick").def_id, picked);
                assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == picked).count(), 1);
            }
        }

        /// The (0) pick played into lane 1: the keywords it wears there.
        fn worn_on_the_field() -> Vec<Keyword> {
            let seed = seed_where("s39-keeps", |seed| {
                let mut s = crossing(false, seed, 0);
                s.play(ID, json!({}));
                crate::card_def(&offered(&s)[0]).cost == CardCost::Fixed(0)
            });
            let mut s = crossing(false, &seed, 0);
            pick(&mut s, 0);
            let card = added(&s).expect("a (0) Unit");
            s.play(card.id.as_str(), json!({ "zone": 1 }));
            let unit = s.unit(P1, 1).expect("it stands on the field").id;
            assert_eq!(unit, card.id);
            s.stats(unit.as_str()).keywords
        }

        #[test]
        fn r1878003_r4285001_the_card_keeps_starship_piece_on_the_field_and_has_no_equip() {
            let keywords = worn_on_the_field();
            assert!(has_keyword(&keywords, KeywordKind::StarshipPiece));
            assert!(!has_keyword(&keywords, KeywordKind::Equip));
        }

        /// R767000, R4285001: through v0.6.7k the pick was given Equip too, and wore both on the field.
        #[test]
        fn r1878003_r767000_the_script_kept_for_v0_6_7k_gives_starship_piece_and_equip() {
            crate::under_kept(ID, script_v0_6_7k(), || {
                for radiant in [false, true] {
                    let mut s = crossing(radiant, "s39-pick", 0);
                    pick(&mut s, 1);
                    let card = added(&s).expect("the pick reaches the hand");
                    assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece, Keyword::Equip], "radiant {radiant}");
                }
                let keywords = worn_on_the_field();
                assert!(has_keyword(&keywords, KeywordKind::StarshipPiece));
                assert!(has_keyword(&keywords, KeywordKind::Equip));
            });
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_pick() {
            let mut s = crossing(false, "s39-full", 10);
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
            let mut s = crossing(false, "s39-hidden", 0);
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
            let mut s = crossing(false, "s39-policy", 0);
            s.play(ID, json!({}));
            let state = s.state().clone();
            let mut rng = Rng::new("s39-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "s39-policy-answer".to_string()));
            assert!(next.error.is_none(), "{:?}", next.error);
            assert!(next.state.pending.is_none());
            assert!(next.state.players.p1.hand.iter().any(|card| card.granted_keywords.contains(&Keyword::StarshipPiece)));
        }

        #[test]
        fn r79_a_timeout_answers_the_discover() {
            let mut s = crossing(false, "s39-timeout", 0);
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
        fn r3549000_r4285001_the_pick_is_radiant_with_starship_piece_and_costs_1_less() {
            let mut s = crossing(true, "s39-radiant", 0);
            let picked = pick(&mut s, 0);
            let card = added(&s).expect("the pick reaches the hand");
            assert_eq!(card.def_id, picked);
            assert!(card.radiant);
            assert_eq!(card.cost_mod, -DISCOUNT);
            assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece]);
        }

        #[test]
        fn r3549000_the_radiant_discover_offers_the_same_pool() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = crossing(true, &format!("s39-rpool-{i}"), 0);
                s.play(ID, json!({}));
                for id in offered(&s) {
                    let def = crate::card_def(&id);
                    assert!(def.type_ == CardType::Unit && def.tags.contains(&Tag::Mindustrial), "{id}");
                    seen.insert(id);
                }
            }
            assert!(seen.len() > 20, "{}", seen.len());
        }

        #[test]
        fn s2_3_the_discount_floors_at_0() {
            let seed = seed_where("s39-floor", |seed| {
                let mut s = crossing(true, seed, 0);
                s.play(ID, json!({}));
                crate::card_def(&offered(&s)[0]).cost == CardCost::Fixed(0)
            });
            let mut s = crossing(true, &seed, 0);
            pick(&mut s, 0);
            let card = added(&s).expect("a (0) Unit");
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0, "{}", card.def_id);
        }

        #[test]
        fn r97_r177_the_opponent_sees_neither_the_radiant_card_nor_its_price_change() {
            let mut s = crossing(true, "s39-rhidden", 0);
            pick(&mut s, 0);
            let card = added(&s).expect("the pick").id;
            let theirs = s.view(P2);
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&format!("\"{card}\"")));
            // The picked card's own events read "hidden".
            let mut hidden = 0;
            for event in &theirs.events {
                if let GameEvent::AddedToHand { instance_id, .. } | GameEvent::CostChanged { instance_id, .. } = event {
                    assert_ne!(*instance_id, card, "the picked card's id is never sent");
                    if instance_id == "hidden" {
                        hidden += 1;
                    }
                }
            }
            assert!(hidden > 0, "the pick's own events are sent hidden");
        }

        #[test]
        fn r79_a_timeout_answers_the_radiant_discover_too() {
            let mut s = crossing(true, "s39-rtimeout", 0);
            s.play(ID, json!({}));
            s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
            assert!(s.state().pending.is_none());
            let card = s.hand(P1).into_iter().find(|card| card.granted_keywords.contains(&Keyword::StarshipPiece));
            assert!(card.is_some_and(|card| card.radiant && card.cost_mod == -DISCOUNT));
        }
    }
}
