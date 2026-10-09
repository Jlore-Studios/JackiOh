//! C #30 Recycle (SPEC §8.6 row 30). (1) Spell, Rare.
//!   Base:    "Shuffle your graveyard into your deck. Draw {draw}." — draw 1
//!   Radiant: "Shuffle your graveyard into your deck. They cost ({discount}) less. Draw {draw}." — 1, 1
//!   Engine:  "Every card in your graveyard (this Spell is resolving, not in it) at random positions;
//!            R80's cap: cards that don't fit stay in the graveyard, each reported by `libraryOverflow`
//!            (R316). Radiant: `costMod` −1 on each shuffled card. Then the draw."
//!
//! The graveyard is read once, as the Spell begins to resolve, so the Spell itself (in the resolving
//! zone, §10.5) is not among the cards. Each is shuffled in at its own random position, keeping its
//! `costMod` (R78), openly to its owner (R311) at a slot neither player reads (R97).
//! Radiant: "They" are the cards that went in, so a card the cap left behind gets no discount, and a
//! change made inside a library is read by nobody (R177). Then the draw (§2.4's pipeline).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-030";

fn recycle(discounts: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // The graveyard as the Spell begins to resolve, read once for both halves.
            let shuffled: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Graveyard)
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let mut effects = Vec::new();
            {
                let shuffled = shuffled.clone();
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |_c| shuffled.clone()),
                    each: Arc::new(|instance_id| shuffle_card_into(json_as(json!({ "instanceId": instance_id })))),
                }));
            }
            if discounts {
                // A card's declared numbers do not move while its Spell resolves, so the discount is
                // read here, where the context is in hand.
                let discount = param(&*ctx, "discount");
                let shuffled = shuffled.clone();
                effects.push(for_each_card(ForEachCardArgs {
                    // The cards that went in: the graveyard's, now in the library.
                    cards: Arc::new(move |c| {
                        zone_cards(&*c.state, c.controller, OffFieldZone::Library)
                            .iter()
                            .filter(|card| shuffled.contains(&card.id))
                            .map(|card| card.id.clone())
                            .collect()
                    }),
                    each: Arc::new(move |instance_id| {
                        set_cost_mod(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": -discount,
                        })))
                    }),
                }));
            }
            effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = recycle(false);

    let radiant = recycle(true);

    CardScripts { base, radiant }
}

// C #30 Recycle — SPEC §8.6 row 30, BUILD M9 Classic row C 30: shuffles every graveyard card into your
// deck at random positions; R80's cap: cards that don't fit stay in the graveyard; positions are blank
// in both views and your deck list names the cards (R311); then draw 1; radiant: each shuffled card costs
// (1) less (`costMod`, R78). Its name is a rules word, C #64's "Recycler" no reference (R381); numbers: R386.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RECYCLE: &str = "classic-030";
    const FILLER: &str = "core-005";
    const X: &str = "core-008";
    const Y: &str = "core-011";
    const A: &str = "core-019";
    const B: &str = "core-001";
    const C: &str = "core-020";

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    fn defs(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    fn library_and_hand(s: &Scenario) -> Vec<String> {
        let mut all = ids(&s.pile(P1, "library"));
        all.extend(ids(&s.hand(P1)));
        all.sort();
        all
    }

    fn nearly_full_library() -> Vec<&'static str> {
        (0..LIBRARY_CAP - 1).map(|_| X).collect()
    }

    fn view_events_of(s: &Scenario, viewer: PlayerId, kind: GameEventType) -> Vec<GameEvent> {
        s.view(viewer)
            .events
            .into_iter()
            .filter(|event| event.event_type() == kind)
            .collect()
    }

    mod c_n30_recycle {
        use super::*;

        #[test]
        fn has_a_script_per_face() {
            crate::register_all();
            assert_eq!(crate::card_def(RECYCLE).id, RECYCLE);
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        #[test]
        fn r381_its_name_is_a_rules_word_c_n64_malzahar_s_recycler_names_no_card_least_of_all_this_one() {
            crate::register_all();
            let refs = crate::CATALOG
                .get("classic-064")
                .and_then(|card| card.refs.clone())
                .unwrap_or_default();
            assert!(!refs.contains(&RECYCLE.to_string()));
            let named: Vec<String> = crate::CATALOG
                .values()
                .filter(|card| card.name == "Recycle")
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(named, vec![RECYCLE]);
        }

        mod base {
            use super::*;

            #[test]
            fn shuffles_every_graveyard_card_into_the_deck_then_draws_1_the_spell_itself_lands_in_the_graveyard_after() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RECYCLE], "library": [X, Y], "graveyard": [A, B, C] },
                    "p2": { "hand": [FILLER] },
                }));
                let mut before = ids(&s.pile(P1, "library"));
                before.extend(ids(&s.pile(P1, "graveyard")));
                before.sort();

                s.play(RECYCLE, json!({}));

                assert_eq!(defs(&s.pile(P1, "graveyard")), vec![RECYCLE]);
                assert_eq!(s.pile(P1, "library").len(), 4);
                assert_eq!(s.hand(P1).len(), 1);
                assert_eq!(library_and_hand(&s), before);
                assert_eq!(
                    s.events().iter().filter(|event| matches!(event, GameEvent::ShuffledIn { .. })).count(),
                    3
                );
                s.expect_events(json!(["shuffledIn", "shuffledIn", "shuffledIn", "drawn"]));
            }

            #[test]
            fn r78_each_card_keeps_its_id_and_its_costmod() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RECYCLE], "library": [X], "graveyard": [{ "def": A, "costMod": 1 }] },
                    "p2": { "hand": [FILLER] },
                }));
                let card = s.card(A).clone();

                s.play(RECYCLE, json!({}));

                assert_eq!(s.card(&card.id).cost_mod, 1);
                assert!([ZoneName::Library, ZoneName::Hand].contains(&s.card(&card.id).zone.z()));
            }

            #[test]
            fn an_empty_graveyard_only_draws() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RECYCLE], "library": [X, Y] }, "p2": { "hand": [FILLER] } }));

                s.play(RECYCLE, json!({}));

                assert_eq!(defs(&s.hand(P1)), vec![X]);
                assert_eq!(defs(&s.pile(P1, "library")), vec![Y]);
                assert_eq!(
                    s.events().iter().filter(|event| matches!(event, GameEvent::ShuffledIn { .. })).count(),
                    0
                );
            }

            #[test]
            fn r80_r316_a_card_the_full_deck_turns_away_stays_in_the_graveyard_reported_by_libraryoverflow() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RECYCLE], "library": nearly_full_library(), "graveyard": [A, B] },
                    "p2": { "hand": [FILLER] },
                }));
                let turned_away = s.card(B).clone();

                s.play(RECYCLE, json!({}));

                assert_eq!(defs(&s.pile(P1, "graveyard")), vec![B, RECYCLE]);
                assert_eq!(
                    s.pile(P1, "graveyard").first().map(|card| card.id.clone()),
                    Some(turned_away.id.clone())
                );
                let overflow: Vec<&GameEvent> = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
                    .collect();
                assert_eq!(
                    serde_json::to_value(&overflow).expect("events serialise"),
                    json!([{ "type": "libraryOverflow", "player": "p1", "instanceId": turned_away.id, "defId": B, "outcome": "graveyard" }])
                );
                assert_eq!(
                    s.events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::EnteredGraveyard { instance_id, .. } if *instance_id == turned_away.id))
                        .count(),
                    0
                );
                // One card went in (60), then the draw took one out.
                assert_eq!(s.pile(P1, "library").len(), (LIBRARY_CAP - 1) as usize);
            }

            #[test]
            fn r97_r311_the_positions_are_blank_in_both_views_and_your_own_deck_list_names_the_cards() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RECYCLE], "library": [X], "graveyard": [A, B] }, "p2": { "hand": [FILLER] } }));

                s.play(RECYCLE, json!({}));

                for viewer in [P1, P2] {
                    let shuffled = view_events_of(&s, viewer, GameEventType::ShuffledIn);
                    assert!(!shuffled.is_empty());
                    for event in &shuffled {
                        let position = match event {
                            GameEvent::ShuffledIn { position, .. } => Some(*position),
                            _ => None,
                        };
                        assert_eq!(position, Some(-1));
                    }
                }
                let listed: Vec<String> = s
                    .view(P1)
                    .you
                    .own_library
                    .map(|library| library.cards.into_iter().map(|entry| entry.def_id).collect())
                    .unwrap_or_default();
                let in_deck = defs(&s.pile(P1, "library"));
                for def_id in &in_deck {
                    assert!(listed.contains(def_id));
                }
                assert!(s.view(P2).opponent.own_library.is_none());
            }

            #[test]
            fn r386_an_upgrade_of_draw_makes_it_draw_2() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RECYCLE], "library": [X, Y, A] }, "p2": { "hand": [FILLER] } }));
                step_param(s.card_mut(RECYCLE), "draw", 1);

                s.play(RECYCLE, json!({}));

                assert_eq!(defs(&s.hand(P1)), vec![X, Y]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r78_each_shuffled_card_costs_1_less_a_costmod_it_keeps() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": [X], "graveyard": [A, B] },
                    "p2": { "hand": [FILLER] },
                }));
                let a = s.card(A).clone();
                let b = s.card(B).clone();

                s.play(RECYCLE, json!({}));

                assert_eq!(s.card(&a.id).cost_mod, -1);
                assert_eq!(s.card(&b.id).cost_mod, -1);
                assert_eq!(s.card(X).cost_mod, 0);
            }

            #[test]
            fn r65_r78_the_discount_adds_to_what_the_card_already_carries_a_card_at_1_ends_at_0_and_one_at_0_at_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": [X], "graveyard": [{ "def": A, "costMod": 1 }, B] },
                    "p2": { "hand": [FILLER] },
                }));
                let a = s.card(A).clone();
                let b = s.card(B).clone();

                s.play(RECYCLE, json!({}));

                assert_eq!(s.card(&a.id).cost_mod, 0);
                assert_eq!(s.card(&b.id).cost_mod, -1);
            }

            #[test]
            fn r177_the_cost_change_inside_the_deck_is_read_by_neither_player() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": [X, Y, C], "graveyard": [A] },
                    "p2": { "hand": [FILLER] },
                }));
                let a = s.card(A).clone();

                s.play(RECYCLE, json!({}));

                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::CostChanged { instance_id, .. } if *instance_id == a.id))
                );
                for viewer in [P1, P2] {
                    for event in view_events_of(&s, viewer, GameEventType::CostChanged) {
                        if let GameEvent::CostChanged { instance_id, .. } = event {
                            assert_ne!(instance_id, a.id);
                        }
                    }
                }
            }

            #[test]
            fn r80_a_card_the_cap_left_in_the_graveyard_is_not_discounted() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": nearly_full_library(), "graveyard": [A, B] },
                    "p2": { "hand": [FILLER] },
                }));
                let left = s.card(B).clone();

                s.play(RECYCLE, json!({}));

                assert_eq!(s.card(&left.id).zone.z(), ZoneName::Graveyard);
                assert_eq!(s.card(&left.id).cost_mod, 0);
            }

            #[test]
            fn then_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": [X] }, "p2": { "hand": [FILLER] } }));

                s.play(RECYCLE, json!({}));

                assert_eq!(defs(&s.hand(P1)), vec![X]);
            }

            #[test]
            fn r386_an_upgrade_of_the_discount_makes_them_cost_2_less() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RECYCLE, "radiant": true }], "library": [X, Y], "graveyard": [A] },
                    "p2": { "hand": [FILLER] },
                }));
                let a = s.card(A).clone();
                step_param(s.card_mut(RECYCLE), "discount", 1);

                s.play(RECYCLE, json!({}));

                assert_eq!(s.card(&a.id).cost_mod, -2);
            }
        }
    }
}
