//! Meditative+ #22 Mountaintop Garden (SPEC §8.9 row 22, BUILD M11 row M+ 22, R1181002, R1178001,
//! R1563004, R3140001): (3) Field Spell, Epic.
//!   Base:    "Cry: Add a random Embiggen card to your hand.\nAura: Embiggen cards play their Embiggened
//!            version at their normal price. Any card that isn't Radiant may be played Radiant for
//!            ({premium}) more." (2)
//!   Radiant: "Cry: Add a random Embiggen card to your hand. It costs (0).\nAura: …" the same. (1)
//!
//! Cry (since #3140; `script_v0_5_11o` has none): one `add_random_from_catalog` over the embiggen pool
//! (R1178001: the non-token cards of the sets that ship that print two prices), never this card (R387),
//! on the base face the card as printed, on the Radiant face with a `costOverride` of (0) (R3140001).
//! Embiggen cards play Embiggened version at normal price via embiggenGarden (R1181002, R1178001, §10.5 step 3).
//! Cost modifiers apply to both prices (R65, R545). Non-Radiant cards may be played Radiant for premium (R1563004, #1563).
//! Declared premium read via declared_or (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-022";

/// The catalog's declared premium, read off the face that is up (R386).
fn declared_premium(radiant: bool) -> i32 {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "premium"))
    {
        Some(param) if radiant => param.radiant,
        Some(param) => param.base,
        None => panic!("{ID} declares premium (catalog params)"),
    }
}

/// "Add a random Embiggen card": one card.
const CARDS: i32 = 1;

/// "It costs (0).": the price the Radiant face's card is given, before every modifier (R3140001).
const RADIANT_CARD_COST: i32 = 0;

/// One face. `cry` is the Cry the Garden has had since #3140, which `script_v0_5_11o` lacks.
fn garden(radiant: bool, cry: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            embiggen_garden: Some(declared_premium(radiant)),
            ..StaticFlags::default()
        }),
        cry: cry.then(|| {
            hook(move |_ctx| {
                vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                    query: Some(excluding_def_id(None, &crate::query::embiggen_cards(), Some(ID))),
                    count: Some(CARDS),
                    cost_override: radiant.then_some(RADIANT_CARD_COST),
                    ..AddRandomFromCatalogArgs::default()
                })]
            })
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: garden(false, true),
        radiant: garden(true, true),
    }
}

/// R767000: through v0.5.11r the Garden had no Cry; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: garden(false, false),
        radiant: garden(true, false),
    }
}

// Meditative+ #22 Mountaintop Garden — SPEC §8.9 row 22, BUILD M11 row M+ 22 (R545).
#[cfg(test)]
mod tests {
    use super::{ID, script_v0_5_11o};
    use jackioh_engine::CardDefs;
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::effects::cast_new;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// Core #84 Going Long: (2, Embiggened 4) Field Spell, "Your hero has Armor 2. Embiggened (4):
    const GOING_LONG: &str = "core-084";
    /// Core #36 Magic Jammed: (1) Spell, "Destroy target backrow card; Lock its zone".
    const MAGIC_JAMMED: &str = "core-036";
    const FILLER: &str = "core-008";

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER])
    }

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let p1 = crate::merged(json!({ "library": library(), "mana": 10 }), p1);
        let p2 = crate::merged(json!({ "hand": [FILLER], "library": library() }), p2);
        crate::scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
    }

    fn in_hand(s: &Scenario, def: &str) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == def)
            .map(|card| card.id)
            .expect("the card is in the hand")
    }

    // Differs from Scenario::offered: one card's face-up embiggen flags, not the seat's whole legal list.
    fn offered(s: &Scenario, id: &str) -> Vec<Option<bool>> {
        let mut flags: Vec<Option<bool>> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    embiggen,
                    face_down: None,
                    ..
                } if instance_id == id => Some(embiggen),
                _ => None,
            })
            .collect();
        flags.sort();
        flags.dedup();
        flags
    }

    fn paid(s: &Scenario) -> i32 {
        s.last_events()
            .iter()
            .find_map(|event| match event {
                GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
                _ => None,
            })
            .expect("the play is announced")
    }

    fn garden_at(lane: i32, radiant: bool) -> Value {
        json!({ "def": ID, "lane": lane, "radiant": radiant })
    }

    mod base {
        use super::*;

        #[test]
        fn r1181002_going_long_at_2_plays_its_embiggened_version() {
            let mut s = game(
                "m+022-normal",
                json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, false)] }),
                json!({}),
            );
            let long = in_hand(&s, GOING_LONG);
            s.play(long.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 2);
            assert_eq!(s.card(long.as_str()).embiggened, Some(true));
            assert!(!s.card(long.as_str()).radiant);
            assert_eq!(hero_armor_of(s.state(), P1), 5, "the Embiggened version's Armor");
        }

        #[test]
        fn r1181002_its_dearer_price_costs_4_and_plays_it_radiant_and_embiggened() {
            let mut s = game(
                "m+022-dearer",
                json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, false)] }),
                json!({}),
            );
            let long = in_hand(&s, GOING_LONG);
            assert_eq!(offered(&s, &long), vec![Some(false), Some(true)]);
            s.play(long.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 4);
            assert!(s.card(long.as_str()).radiant);
            assert_eq!(s.card(long.as_str()).embiggened, Some(true));
            assert_eq!(hero_armor_of(s.state(), P1), 10, "the Radiant Embiggened version's Armor");
        }

        #[test]
        fn r1181002_a_radiant_going_long_offers_2_only() {
            let mut s = game(
                "m+022-radiant-card",
                json!({
                    "hand": [{ "def": GOING_LONG, "radiant": true }, FILLER],
                    "backrow": [garden_at(1, false)],
                }),
                json!({}),
            );
            let long = in_hand(&s, GOING_LONG);
            assert_eq!(offered(&s, &long), vec![Some(false)]);
            s.expect_refused_with(
                |s| s.play(long.as_str(), json!({ "zone": 2, "embiggen": true })),
                "has only its normal price now",
            );
            s.play(long.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 2);
            assert_eq!(hero_armor_of(s.state(), P1), 10, "Radiant and Embiggened at (2)");
        }

        #[test]
        fn r1563004_any_card_that_isnt_radiant_may_be_played_radiant_for_2_more() {
            let mut s = game(
                "m+022-any",
                json!({ "hand": [FILLER, MAGIC_JAMMED], "backrow": [garden_at(1, false)] }),
                json!({}),
            );
            let vanilla = in_hand(&s, FILLER);
            assert_eq!(offered(&s, &vanilla), vec![Some(false), Some(true)]);
            s.play(vanilla.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 3, "its (1) and the premium (2)");
            let played = s.card(vanilla.as_str()).clone();
            assert!(played.radiant, "played Radiant");
            assert_eq!(played.embiggened, None, "a card of one price has no Embiggened version");
            s.expect_stats(&played, json!({ "attack": 12, "maxHealth": 12 }));
        }

        #[test]
        fn r1563004_at_its_own_price_a_card_of_one_price_plays_as_it_is() {
            let mut s = game(
                "m+022-any-normal",
                json!({ "hand": [FILLER, MAGIC_JAMMED], "backrow": [garden_at(1, false)] }),
                json!({}),
            );
            let vanilla = in_hand(&s, FILLER);
            s.play(vanilla.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 1);
            assert!(!s.card(vanilla.as_str()).radiant);
            assert_eq!(s.card(vanilla.as_str()).embiggened, None);
        }

        #[test]
        fn r1563004_a_radiant_card_of_one_price_has_no_dearer_price() {
            let mut s = game(
                "m+022-any-radiant",
                json!({
                    "hand": [{ "def": FILLER, "radiant": true }, MAGIC_JAMMED],
                    "backrow": [garden_at(1, false)],
                }),
                json!({}),
            );
            let vanilla = in_hand(&s, FILLER);
            assert_eq!(offered(&s, &vanilla), vec![None]);
            s.expect_refused(|s| s.play(vanilla.as_str(), json!({ "zone": 2, "embiggen": true })));
        }

        #[test]
        fn r1563004_with_no_garden_a_card_of_one_price_has_one_price() {
            let s = game("m+022-none", json!({ "hand": [FILLER, MAGIC_JAMMED] }), json!({}));
            let vanilla = in_hand(&s, FILLER);
            assert_eq!(offered(&s, &vanilla), vec![None]);
            assert_eq!(jackioh_engine::play_choices::embiggen_play_cost(s.state(), s.card(vanilla.as_str())), None);
        }

        #[test]
        fn r1181002_the_opponents_garden_reaches_your_cards_too() {
            let mut s = game(
                "m+022-theirs",
                json!({ "hand": [GOING_LONG, FILLER] }),
                json!({ "backrow": [garden_at(1, false)] }),
            );
            let long = in_hand(&s, GOING_LONG);
            s.play(long.as_str(), json!({ "zone": 1 }));
            assert_eq!(paid(&s), 2);
            assert_eq!(hero_armor_of(s.state(), P1), 5);
        }

        #[test]
        fn r1181002_a_cast_is_unchanged_r545() {
            let mut s = game("m+022-cast", json!({ "hand": [FILLER], "backrow": [garden_at(1, false)] }), json!({}));
            let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions {
                            controller: Some(P1),
                            ..HookOptions::default()
                        },
                    );
                    apply_effects(&[cast_new(json_as(json!({ "def": GOING_LONG })))], &mut ctx);
                }
                settle(&mut sink, SettleOptions::default());
            }
            s.state_mut().rng_cursor = rng.cursor();
            let cast = s.backrow(P1, 2).expect("the cast Going Long took the next zone");
            assert_eq!(cast.def_id, GOING_LONG);
            assert_ne!(cast.embiggened, Some(true), "a cast resolves at its base");
            assert!(!cast.radiant);
            assert_eq!(hero_armor_of(s.state(), P1), 2);
        }

        #[test]
        fn r1181002_destroyed_it_ends_the_aura() {
            let mut s = game(
                "m+022-gone",
                json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, false)] }),
                json!({ "hand": [MAGIC_JAMMED, FILLER], "mana": 10 }),
            );
            let garden = s.backrow(P1, 1).expect("the Garden").id;
            s.end_turn();
            s.play(
                MAGIC_JAMMED,
                json!({ "targets": [{ "pick": "instance", "instanceId": garden }] }),
            );
            s.expect_in_zone(garden.as_str(), "graveyard");
            s.end_turn();
            let long = in_hand(&s, GOING_LONG);
            s.play(long.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 2);
            assert_ne!(s.card(long.as_str()).embiggened, Some(true));
            assert_eq!(hero_armor_of(s.state(), P1), 2, "the normal version again");
        }

        #[test]
        fn r386_a_degrade_raises_the_premium() {
            let mut s = game(
                "m+022-tuned",
                json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, false)] }),
                json!({}),
            );
            let garden = s.backrow(P1, 1).expect("the Garden").id;
            assert_eq!(crate::degrade_number(&mut s, &garden, "premium"), 3);
            let long = in_hand(&s, GOING_LONG);
            s.play(long.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_premium_is_1_so_the_dearer_price_is_3() {
            let mut s = game(
                "m+022-radiant",
                json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, true)] }),
                json!({}),
            );
            let long = in_hand(&s, GOING_LONG);
            s.play(long.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 3);
            assert!(s.card(long.as_str()).radiant);
            assert_eq!(hero_armor_of(s.state(), P1), 10);
        }

        #[test]
        fn r1563004_a_card_of_one_price_is_played_radiant_for_1_more() {
            let mut s = game(
                "m+022-radiant-any",
                json!({ "hand": [FILLER, MAGIC_JAMMED], "backrow": [garden_at(1, true)] }),
                json!({}),
            );
            let vanilla = in_hand(&s, FILLER);
            s.play(vanilla.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 2);
            assert!(s.card(vanilla.as_str()).radiant);
        }
    }

    /// Runs its closure when dropped, on a panic too.
    struct Finally<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for Finally<F> {
        fn drop(&mut self) {
            if let Some(f) = self.0.take() {
                f();
            }
        }
    }

    /// The shipped catalog with every embiggen card but `keep` left out, so the Cry's random pick is `keep`.
    fn only_embiggen(keep: &str) -> CardDefs {
        crate::CATALOG
            .iter()
            .filter(|(id, def)| !matches!(def.cost, CardCost::Embiggen { .. }) || id.as_str() == keep)
            .map(|(id, def)| (id.clone(), def.clone()))
            .collect()
    }

    /// Registers `defs`; the guard puts the shipped catalog back.
    fn with_catalog(defs: CardDefs) -> Finally<impl FnOnce()> {
        jackioh_engine::testkit::register_catalog_as(defs, crate::CATALOG_VERSION);
        Finally(Some(|| {
            jackioh_engine::testkit::register_catalog_as(crate::CATALOG.clone(), crate::CATALOG_VERSION);
        }))
    }

    /// p1 holds the Garden (Radiant on `radiant`) to play, with `others` fillers beside it.
    fn holding_garden(seed: &str, radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), others));
        game(seed, json!({ "hand": hand }), json!({}))
    }

    /// The cards the Cry added to p1's hand: whatever is not a filler.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod cry {
        use super::*;

        #[test]
        fn r3140001_it_adds_one_random_embiggen_card_never_itself_on_either_face() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for radiant in [false, true] {
                for n in 0..30 {
                    let mut s = holding_garden(&format!("m+022-cry-{radiant}-{n}"), radiant, 2);
                    s.play(ID, json!({ "zone": 1 }));
                    s.expect_in_zone(ID, "field");
                    let cards = added(&s);
                    assert_eq!(cards.len(), 1, "{radiant} {n}");
                    let def = crate::card_def(&cards[0].def_id);
                    assert!(matches!(def.cost, CardCost::Embiggen { .. }), "{} is an embiggen card", def.id);
                    assert!(!def.token && def.id != ID);
                    assert!(!cards[0].radiant, "the card is not made Radiant");
                    seen.insert(def.id.clone());
                }
            }
            assert!(seen.len() > 5, "the pool is wide: {seen:?}");
        }

        #[test]
        fn r3140001_the_base_card_costs_what_it_prints_and_plays_embiggened_at_that_price_under_the_garden() {
            let _restore = with_catalog(only_embiggen(GOING_LONG));
            let mut s = holding_garden("m+022-cry-base", false, 2);
            s.play(ID, json!({ "zone": 1 }));
            let long = in_hand(&s, GOING_LONG);
            assert_eq!(s.card(long.as_str()).cost_override, None);
            s.play(long.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 2);
            assert_eq!(s.card(long.as_str()).embiggened, Some(true));
            assert_eq!(hero_armor_of(s.state(), P1), 5, "the Embiggened version's Armor");
        }

        #[test]
        fn r3140001_the_radiant_card_costs_0_and_the_garden_plays_it_embiggened_for_that() {
            let _restore = with_catalog(only_embiggen(GOING_LONG));
            let mut s = holding_garden("m+022-cry-radiant", true, 2);
            s.play(ID, json!({ "zone": 1 }));
            let long = in_hand(&s, GOING_LONG);
            assert_eq!(s.card(long.as_str()).cost_override, Some(0));
            assert!(!s.card(long.as_str()).radiant);
            assert_eq!(crate::card_def(GOING_LONG).cost, CardCost::Embiggen { base: 2, embiggen: 4 });
            s.state_mut().players.p1.mana.current = 0;
            s.play(long.as_str(), json!({ "zone": 2 }));
            assert_eq!(paid(&s), 0);
            assert_eq!(s.card(long.as_str()).embiggened, Some(true));
            assert_eq!(hero_armor_of(s.state(), P1), 5, "the Embiggened version's Armor");
        }

        #[test]
        fn r3140001_a_costs_0_override_holds_for_the_gardens_dearer_choice_too() {
            let _restore = with_catalog(only_embiggen(GOING_LONG));
            let mut s = holding_garden("m+022-cry-dearer", true, 2);
            s.play(ID, json!({ "zone": 1 }));
            let long = in_hand(&s, GOING_LONG);
            assert_eq!(offered(&s, &long), vec![Some(false), Some(true)]);
            s.state_mut().players.p1.mana.current = 0;
            s.play(long.as_str(), json!({ "zone": 2, "embiggen": true }));
            assert_eq!(paid(&s), 0);
            assert!(s.card(long.as_str()).radiant);
            assert_eq!(s.card(long.as_str()).embiggened, Some(true));
        }

        #[test]
        fn r3140001_a_full_hand_burns_the_card() {
            for radiant in [false, true] {
                let mut s = holding_garden("m+022-cry-full", radiant, HAND_CAP as usize);
                s.play(ID, json!({ "zone": 1 }));
                assert!(added(&s).is_empty(), "radiant {radiant}");
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { owner, .. } if *owner == P1)));
            }
        }

        #[test]
        fn r3140001_a_garden_that_is_not_played_adds_nothing() {
            let s = game("m+022-no-cry", json!({ "hand": [FILLER], "backrow": [garden_at(1, false)] }), json!({}));
            assert!(added(&s).is_empty());
        }

        /// R767000: through v0.5.11r the Garden had no Cry, so playing it added nothing to the hand.
        #[test]
        fn r767000_a_game_on_v0_5_11o_adds_no_card_when_it_is_played() {
            for radiant in [false, true] {
                crate::under_kept(ID, script_v0_5_11o(), || {
                    let mut s = holding_garden("m+022-kept", radiant, 2);
                    s.play(ID, json!({ "zone": 1 }));
                    s.expect_in_zone(ID, "field");
                    assert!(added(&s).is_empty(), "radiant {radiant}");
                });
            }
        }

        /// R767000: the script kept for v0.5.11o still has the Aura: Going Long plays Embiggened at (2),
        /// or Radiant and Embiggened at its dearer price.
        #[test]
        fn r767000_r1181002_a_game_on_v0_5_11o_keeps_the_aura() {
            for (radiant, dearer) in [(false, 4), (true, 3)] {
                crate::under_kept(ID, script_v0_5_11o(), || {
                    let setup = || {
                        game(
                            "m+022-kept-aura",
                            json!({ "hand": [GOING_LONG, FILLER], "backrow": [garden_at(1, radiant)] }),
                            json!({}),
                        )
                    };
                    let mut s = setup();
                    let long = in_hand(&s, GOING_LONG);
                    assert_eq!(offered(&s, &long), vec![Some(false), Some(true)], "radiant {radiant}");
                    s.play(long.as_str(), json!({ "zone": 2 }));
                    assert_eq!(paid(&s), 2, "radiant {radiant}");
                    assert_eq!(s.card(long.as_str()).embiggened, Some(true), "radiant {radiant}");

                    let mut s = setup();
                    let long = in_hand(&s, GOING_LONG);
                    s.play(long.as_str(), json!({ "zone": 2, "embiggen": true }));
                    assert_eq!(paid(&s), dearer, "radiant {radiant}");
                    assert!(s.card(long.as_str()).radiant, "radiant {radiant}");
                    assert_eq!(s.card(long.as_str()).embiggened, Some(true), "radiant {radiant}");
                });
            }
        }
    }
}
