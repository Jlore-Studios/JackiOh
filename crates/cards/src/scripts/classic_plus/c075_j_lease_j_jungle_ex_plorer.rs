//! C+ #75 J-lease J-Jungle EX-plorer (SPEC §8.7 row 75). (2) Unit, Legendary, 5/5 → 10/10.
//!   Base:    "Cry: Shuffle {packs|J-lease J-Jungle EX-plorer Pack|J-lease J-Jungle EX-plorer Packs} into
//!            your deck."
//!   Radiant: "Cry: Shuffle {packs|Radiant J-lease J-Jungle EX-plorer Pack|…} into your deck."
//!   Engine:  "The Pack is C+ #75.1, shuffled in openly at a random position (§6.3), so its owner's list
//!            shows it (R311); R80's cap turns it away. The Radiant Pack's five cards cost (0), so the
//!            effect grows with the stats (R275). Tunes: packs 1 ↑."
//!
//! `shuffle_into` is §6.3's Shuffle into a library: a fresh Pack at a random position (one rng draw each),
//! shown to the deck's owner (R311) and refused at the library cap (R80). A copy of this Unit is summoned
//! with no Cry (R1), so only a played one shuffles.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-075";

/// §7, §8.7: the token the Cry shuffles in (TS `cardDef("classicplus-075-1").id`, read off the catalog).
fn pack_id() -> String {
    crate::card_def("classicplus-075-1").id
}

fn explorer(pack: String, radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "defId": pack.as_str(), "count": param(&*ctx, "packs") });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![shuffle_into(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let pack = pack_id();
    CardScripts {
        base: explorer(pack.clone(), false),
        radiant: explorer(pack, true),
    }
}

// C+ #75 J-lease J-Jungle EX-plorer — SPEC §8.7 row 75, BUILD M9 Classic+ row C+ 75: "Cry shuffles a
// J-lease J-Jungle EX-plorer Pack (C+ #75.1) into your deck at a random position (R80's cap), shown in
// your library list; no Cry from a copy (R1); the count reads through `param()`; radiant the Pack is
// Radiant".
//
// "No Cry from a copy" is driven through the engine's own Summon a copy (§6.3, R57), since no card in
// the preview pool can copy a non-Human 2-Cost Unit.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon_copy;
    use jackioh_engine::testkit::*;

    const EXPLORER: &str = "classicplus-075";
    const PACK: &str = "classicplus-075-1";
    const FILLER: &str = "core-005";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS's live `s.card(ref)`, written through: the card under that id in the state.
    fn card_mut<'a>(s: &'a mut Scenario, card: &str) -> &'a mut CardInstance {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state")
    }

    fn packs(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == PACK).collect()
    }

    fn explorer(radiant: bool, library: usize) -> Scenario {
        crate::register_all();
        let mut card = json!({ "def": EXPLORER });
        if radiant {
            card["radiant"] = json!(true);
        }
        let library: Vec<Value> = (0..library).map(|_| json!(FILLER)).collect();
        scenario(json!({
            "seed": "explorer",
            "p1": { "hand": [card, FILLER], "library": library },
            "p2": { "hand": [FILLER] },
        }))
    }

    mod c_n75_j_lease_j_jungle_ex_plorer {
        use super::*;

        #[test]
        fn is_a_2_5_5_legendary_unit_10_10_radiant_that_names_its_pack() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, EXPLORER);
            assert_eq!(
                [
                    js(&def.cost),
                    js(&def.base.attack),
                    js(&def.base.health),
                    js(&def.radiant.attack),
                    js(&def.radiant.health)
                ],
                [json!(2), json!(5), json!(5), json!(10), json!(10)]
            );
            assert!(def.refs.as_ref().is_some_and(|refs| refs.iter().any(|id| id == PACK)));
            let scripts = super::super::script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn s6_3_its_cry_shuffles_one_pack_into_your_deck_at_a_random_position() {
                let mut s = explorer(false, 4);
                s.play(EXPLORER, json!({}));
                assert_eq!(packs(&s).len(), 1);
                assert_eq!(packs(&s).first().map(|card| card.radiant), Some(false));
                assert_eq!(s.pile(P1, "library").len(), 5);
                s.expect_events(json!(["cardPlayed", "shuffledIn"]));
            }

            #[test]
            fn r311_the_pack_is_shown_in_your_own_library_list_never_its_place_the_opponent_sees_a_count() {
                let mut s = explorer(false, 4);
                s.play(EXPLORER, json!({}));
                let own = js(&s.view(P1))["you"]["ownLibrary"].clone();
                assert!(
                    own["cards"]
                        .as_array()
                        .is_some_and(|cards| cards.iter().any(|entry| entry["defId"] == PACK))
                );
                let theirs = s.view(P2);
                assert_eq!(js(&theirs)["opponent"]["libraryCount"], json!(5));
                assert!(!serde_json::to_string(&theirs).expect("serialises").contains(PACK));
            }

            #[test]
            fn s10_7_the_position_is_the_match_rngs_some_seed_puts_it_on_top_another_below() {
                crate::register_all();
                let mut positions: IndexSet<i64> = IndexSet::new();
                for i in 0..20 {
                    let seed = format!("explorer-{i}");
                    let mut s = scenario(json!({
                        "seed": seed,
                        "p1": { "hand": [EXPLORER, FILLER], "library": [FILLER, FILLER, FILLER] },
                        "p2": { "hand": [FILLER] },
                    }));
                    s.play(EXPLORER, json!({}));
                    let at = s.pile(P1, "library").iter().position(|card| card.def_id == PACK);
                    positions.insert(at.map(|index| index as i64).unwrap_or(-1));
                }
                assert!(positions.len() > 1);
            }

            #[test]
            fn r1_a_copy_summoned_of_it_fires_no_cry_no_pack() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [EXPLORER], "library": [FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                let original = s.unit(P1, 1).expect("the Explorer on the field");
                let mut events: Vec<GameEvent> = Vec::new();
                {
                    let state = s.state_mut();
                    let mut rng = Rng::new(&state.seed, state.rng_cursor);
                    {
                        let mut sink = EngineSink::new(state, &mut events, &mut rng);
                        {
                            let mut ctx = make_context(
                                &mut sink,
                                None,
                                HookOptions {
                                    controller: Some(P1),
                                    ..Default::default()
                                },
                            );
                            apply_effects(
                                &[summon_copy(json_as(json!({
                                    "of": { "of": "instance", "instanceId": original.id },
                                    "lane": 2,
                                })))],
                                &mut ctx,
                            );
                        }
                        settle(&mut sink, SettleOptions::default());
                    }
                    state.rng_cursor = rng.cursor();
                }
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(EXPLORER.to_string()));
                assert!(packs(&s).is_empty());
                assert!(!events.iter().any(|event| event.event_type() == GameEventType::ShuffledIn));
            }

            #[test]
            fn r80_a_full_library_turns_the_pack_away_no_pack_is_made() {
                let mut s = explorer(false, LIBRARY_CAP as usize);
                s.play(EXPLORER, json!({}));
                assert!(packs(&s).is_empty());
                assert!(s.last_events().iter().any(|event| {
                    let event = js(event);
                    event["type"] == "libraryOverflow" && event["outcome"] == "notCreated"
                }));
            }

            #[test]
            fn r386_an_upgrade_of_its_count_shuffles_2_packs_a_degrade_never_takes_it_below_1() {
                let mut up = explorer(false, 4);
                step_param(card_mut(&mut up, EXPLORER), "packs", 1);
                up.play(EXPLORER, json!({}));
                assert_eq!(packs(&up).len(), 2);

                let mut down = explorer(false, 4);
                step_param(card_mut(&mut down, EXPLORER), "packs", -1);
                down.play(EXPLORER, json!({}));
                assert_eq!(packs(&down).len(), 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_the_radiant_face_shuffles_a_radiant_pack_and_is_a_10_10() {
                let mut s = explorer(true, 4);
                s.play(EXPLORER, json!({}));
                assert_eq!(packs(&s).len(), 1);
                assert_eq!(packs(&s).first().map(|card| card.radiant), Some(true));
                s.expect_stats(EXPLORER, json!({ "attack": 10, "maxHealth": 10 }));
            }

            #[test]
            fn r311_the_radiant_pack_is_listed_radiant_in_your_library_list() {
                let mut s = explorer(true, 4);
                s.play(EXPLORER, json!({}));
                let own = js(&s.view(P1))["you"]["ownLibrary"].clone();
                assert!(own["cards"].as_array().is_some_and(|cards| {
                    cards.iter().any(|entry| entry["defId"] == PACK && entry["radiant"] == true)
                }));
            }
        }
    }
}
