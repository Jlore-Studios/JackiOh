//! Meditative+ #22 Mountaintop Garden (SPEC §8.9 row 22, BUILD M11 row M+ 22, R1181002, R1178001,
//! R1563004): (3) Field Spell, Epic.
//!   Base:    "Aura: Embiggen cards play their Embiggened version at their normal price. Any card that
//!            isn't Radiant may be played Radiant for ({premium}) more." (2)
//!   Radiant: the same. (1)
//!
//! Mountaintop's aura (R1181002): the engine's `embiggenGarden` static flag. While the Garden acts on
//! the field — face-up, in its controller's backrow — every embiggen card of both players (R1178001)
//! played from a hand or a graveyard plays its Embiggened version at its normal price (stamped at
//! §10.5 step 3), and its dearer choice costs the normal price plus the premium and plays it Radiant
//! and Embiggened; a Radiant card has its normal price only. Cost modifiers apply to both prices (R65),
//! two Gardens apply the least premium, and a cast (R545) or a card set face-down is not a play it
//! reaches. R1563004 (#1563): the dearer price reaches every card that is not Radiant, not only an
//! embiggen card — a card of one price (no X, R65) is offered its price plus the premium beside its
//! own, the play's `embiggen: true`, and that play is made Radiant (the engine's
//! `garden_offers_radiant`); until then a card of one price had no dearer price under a Garden. The
//! premium is the declared `premium`, read through `params::declared_or` as each price is read, so a
//! Nerf or a Buff moves it (R386). Tunes: premium 2 ↓ (Radiant 1).

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

fn garden(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            embiggen_garden: Some(declared_premium(radiant)),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: garden(false),
        radiant: garden(true),
    }
}

// Meditative+ #22 Mountaintop Garden — SPEC §8.9 row 22, BUILD M11 row M+ 22: "while it acts, Core 84
// Going Long played at (2) gains Armor as its Embiggened version does; its other choice costs (4) and
// plays it Radiant and Embiggened; a Radiant Going Long offers (2) only; the opponent's embiggen
// cards too; a cast is unchanged (R545); leaving the field ends it; premium reads through `param()`;
// radiant the premium is (1)".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::effects::cast_new;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// Core #84 Going Long: (2, Embiggened 4) Field Spell, "Your hero has Armor 2. Embiggened (4):
    /// Armor 4 instead." (Radiant 4 and 8).
    const GOING_LONG: &str = "core-084";
    /// Core #36 Magic Jammed: (1) Spell, "Destroy target backrow card; Lock its zone".
    const MAGIC_JAMMED: &str = "core-036";
    /// A (1) 4/4 with no text.
    const FILLER: &str = "core-008";

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 has 10 mana and fillers to draw, p2 a filler to hold; `p1` and `p2` are merged over each side.
    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let p1 = crate::merged(json!({ "library": library(), "mana": 10 }), p1);
        let p2 = crate::merged(json!({ "hand": [FILLER], "library": library() }), p2);
        crate::scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
    }

    /// p1's hand card of `def`.
    fn in_hand(s: &Scenario, def: &str) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == def)
            .map(|card| card.id)
            .expect("the card is in the hand")
    }

    /// The `embiggen` flags `legalActions` offers p1's plays of the card at.
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

    /// The last play's `cardPlayed`: what it paid.
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
            assert_eq!(hero_armor_of(s.state(), P1), 4, "the Embiggened version's Armor");
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
            assert_eq!(hero_armor_of(s.state(), P1), 8, "the Radiant Embiggened version's Armor");
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
            assert_eq!(hero_armor_of(s.state(), P1), 8, "Radiant and Embiggened at (2)");
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
            let mut s = game("m+022-none", json!({ "hand": [FILLER, MAGIC_JAMMED] }), json!({}));
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
            assert_eq!(hero_armor_of(s.state(), P1), 4);
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
            assert_eq!(hero_armor_of(s.state(), P1), 8);
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
}
