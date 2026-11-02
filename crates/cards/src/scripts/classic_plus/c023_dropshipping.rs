//! C+ #23 Dropshipping (SPEC §8.7 row 23): add {cards} random cards from every card and token but this
//! (R382, R387), each given Temporary {temporary} (R2179001, issue #2179); Radiant: each costs (1).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-023";

// "Each costs (1)" on the Radiant face: the declared number `setCost` (R386), less being better.

/// The cards this list put in its controller's hand (R136: its own events, from `eventsFrom`).
fn added_by_this_list(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let (from, controller) = (ctx.events_from, ctx.controller);
    ctx.events
        .get(from..)
        .unwrap_or_default()
        .iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand { player, instance_id, .. } if *player == controller => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// The add, `cards` random cards of every card and token but this, at `setCost` on the Radiant face.
fn the_add(ctx: &EffectContext<'_>, radiant: bool) -> Value {
    let mut add = json!({
        "query": { "withTokens": true, "excludeDefId": ID },
        "count": param(ctx, "cards"),
    });
    if radiant {
        add["costOverride"] = json!(param(ctx, "setCost"));
    }
    add
}

fn dropship(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R2179001: "Give each Temporary {temporary}", a keyword each takes as it reaches the hand, so
            // a card the hand cap burns takes none.
            let mut add = the_add(ctx, radiant);
            add["keywords"] = json!([{ "kind": "Temporary", "n": param(&*ctx, "temporary") }]);
            vec![add_random_from_catalog(json_as(add))]
        })),
        ..Script::default()
    }
}

/// v0.5.2's face: each card given Brittle {brittle} (R385) once it is in the hand.
fn dropship_v0_5_2(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let n = param(&*ctx, "brittle");
            vec![
                add_random_from_catalog(json_as(the_add(ctx, radiant))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(added_by_this_list),
                    each: Arc::new(move |instance_id: &str| {
                        give_brittle(json_as(json!({ "instanceId": instance_id, "n": n })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dropship(false),
        radiant: dropship(true),
    }
}

/// R767000: through v0.5.2 each card was given Brittle {brittle}; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts {
        base: dropship_v0_5_2(false),
        radiant: dropship_v0_5_2(true),
    }
}

// C+ #23 Dropshipping — SPEC §8.7 row 23, BUILD M9 Classic+ row C+ 23 (R60, R97).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const DROP: &str = "classicplus-023";
    const FILLER: &str = "core-005";
    const DECK: [&str; 12] = [FILLER; 12];

    fn shop(seed: &str, radiant_face: bool, hand: &[&str]) -> Scenario {
        crate::register_all();
        let mut cards = vec![json!({ "def": DROP, "radiant": radiant_face })];
        cards.extend(hand.iter().map(|card| json!(card)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": DECK },
            "p2": { "hand": [FILLER, FILLER], "library": DECK },
        }))
    }

    /// The cards Dropshipping put in p1's hand, by the `addedToHand` events of its play.
    fn added(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn own_hand(s: &Scenario) -> Vec<CardView> {
        match s.view(P1).you.hand {
            HandView::Cards(cards) => cards,
            HandView::Count { .. } => panic!("own hand is a list"),
        }
    }

    /// The Temporary counts the owner sees on the cards `ids` names, in hand order.
    fn temporaries_of(s: &Scenario, ids: &[String]) -> Vec<Option<i32>> {
        own_hand(s)
            .into_iter()
            .filter(|card| ids.contains(&card.instance_id))
            .map(|card| {
                card.keywords.iter().flatten().find_map(|keyword| match keyword {
                    Keyword::Temporary { n } => Some(n.unwrap_or(1)),
                    _ => None,
                })
            })
            .collect()
    }

    /// The Brittle counts the owner sees on the cards `ids` names, in hand order (v0.5.2's).
    fn brittles_of(s: &Scenario, ids: &[String]) -> Vec<Option<i32>> {
        own_hand(s)
            .into_iter()
            .filter(|card| ids.contains(&card.instance_id))
            .map(|card| card.brittle)
            .collect()
    }

    fn next_own_turn(s: &mut Scenario) -> &mut Scenario {
        s.end_turn().end_turn()
    }

    /// `-\d{3}-\d`: a token's three-digit index followed by its own number.
    fn has_token_index(id: &str) -> bool {
        id.as_bytes().windows(6).any(|w| {
            w[0] == b'-' && w[1..4].iter().all(u8::is_ascii_digit) && w[4] == b'-' && w[5].is_ascii_digit()
        })
    }

    use crate::matches_object;

    mod c_n23_dropshipping {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r382_r387_the_pool_is_every_card_and_token_of_every_set_but_dropshipping() {
                crate::register_all();
                let pool: Vec<String> = query(&json_as(json!({ "withTokens": true, "excludeDefId": DROP })))
                    .into_iter()
                    .map(|card| card.id.clone())
                    .collect();
                assert!(!pool.contains(&DROP.to_string()));
                for id in [
                    "classicplus-065-1",
                    "classicplus-065-5",
                    "classicplus-019-3",
                    "classicplus-t-ai-01",
                    "core-t-rush",
                    "core-t-coin",
                    "core-001",
                    "classic-001",
                ] {
                    assert!(pool.contains(&id.to_string()), "{id}");
                }
                assert_eq!(pool.len(), query(&json_as(json!({ "withTokens": true }))).len() - 1);
            }

            #[test]
            fn adds_three_cards_to_the_hand_each_at_its_printed_cost_with_temporary_2_the_owner_sees() {
                let mut s = shop("drop-1", false, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                assert_eq!(ids.len(), 3);
                let views: Vec<CardView> =
                    own_hand(&s).into_iter().filter(|card| ids.contains(&card.instance_id)).collect();
                assert_eq!(views.len(), 3);
                for id in &ids {
                    let card = s.card(id);
                    assert_eq!(card.zone.z(), ZoneName::Hand);
                    assert_eq!(card.cost_override, None);
                    assert!(!card.radiant);
                    assert_ne!(card.def_id, DROP);
                    assert_eq!(card.brittle, None);
                }
                assert_eq!(temporaries_of(&s, &ids), vec![Some(2), Some(2), Some(2)]);
            }

            #[test]
            fn r60_r382_across_seeds_it_never_makes_itself_may_repeat_and_hands_out_tokens() {
                let mut saw_token = false;
                for seed in 1..=40 {
                    let mut s = shop(&format!("drop-seeds-{seed}"), false, &[FILLER]);
                    s.play(DROP, json!({}));
                    for id in added(&s) {
                        let def_id = s.card(&id).def_id.clone();
                        assert_ne!(def_id, DROP);
                        if def_id.contains("-t-") || has_token_index(&def_id) {
                            saw_token = true;
                        }
                    }
                }
                assert!(saw_token);
            }

            #[test]
            fn r382_a_grape_the_pool_picks_is_re_rolled_on_grape_odds_not_generated_as_picked() {
                // Each seed below picks a Grape mid-pool (read off the probe rng): the card Dropshipping
                // generates for it is the re-roll, so the generated triple equals `pickGenerated` three times
                // over — and differs from the picked triple, which a plain `rng.pick` would generate as is.
                // Which seeds do is a roll of the pool, which a card joining it moves, so each is searched for.
                crate::register_all();
                let pool = query(&json_as(json!({ "withTokens": true, "excludeDefId": DROP })));
                let grapes: IndexSet<&str> = GRAPE_ODDS.iter().map(|grape| grape.def_id).collect();
                let table: i32 = GRAPE_ODDS.iter().map(|grape| grape.percent).sum();
                let rolled = |seed: &str| {
                    let s = shop(seed, false, &[FILLER]);
                    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let mut probe = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let mut expected: Vec<Option<String>> = Vec::new();
                    let mut firsts: Vec<Option<String>> = Vec::new();
                    for _ in 0..3 {
                        let first = probe.pick(&pool).map(|card| card.id.clone());
                        if let Some(id) = &first
                            && grapes.contains(id.as_str())
                        {
                            probe.int(table);
                        }
                        firsts.push(first);
                        expected.push(pick_generated(&mut rng, &pool, None).map(|card| card.id.clone()));
                    }
                    (s, firsts, expected)
                };
                let mut differed = 0;
                for tag in ["drop-reroll-a", "drop-reroll-b", "drop-reroll-c", "drop-reroll-d"] {
                    let seed = seed_where(tag, |seed| {
                        let (_, firsts, expected) = rolled(seed);
                        firsts.iter().any(|id| id.as_deref().is_some_and(|id| grapes.contains(id))) && firsts != expected
                    });
                    let (mut s, firsts, expected) = rolled(&seed);
                    assert!(firsts.iter().any(|id| id.as_deref().is_some_and(|id| grapes.contains(id))));
                    if firsts != expected {
                        differed += 1;
                    }
                    s.play(DROP, json!({}));
                    let made: Vec<Option<String>> =
                        added(&s).iter().map(|id| Some(s.card(id).def_id.clone())).collect();
                    assert_eq!(made, expected);
                }
                assert!(differed > 0);
            }

            #[test]
            fn s9_3_the_three_random_cards_replay_from_a_json_copy_to_the_same_hash() {
                let s = shop("drop-replay", false, &[FILLER]);
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(DROP).id,
                    "playerId": "p1",
                    "nonce": "drop-replay",
                }));
                let thawed: GameState = serde_json::from_value(js(s.state())).expect("a state");
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
            }

            #[test]
            fn r97_the_opponent_sees_three_cards_added_and_nothing_of_which() {
                let mut s = shop("drop-hidden", false, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                let theirs: Vec<GameEvent> = s
                    .view(P2)
                    .events
                    .into_iter()
                    .filter(|event| event.event_type() == GameEventType::AddedToHand)
                    .collect();
                assert_eq!(theirs.len(), 3);
                for event in &theirs {
                    assert!(matches_object(
                        &js(event),
                        &json!({ "instanceId": "hidden", "defId": "hidden" }),
                    ));
                }
                let text = serde_json::to_string(&s.view(P2)).expect("JSON");
                for id in &ids {
                    assert!(!text.contains(id.as_str()));
                }
            }

            #[test]
            fn r2179001_the_cards_stand_at_temporary_1_after_your_end_of_turn_and_go_at_your_next() {
                let mut s = shop("drop-hold", false, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                s.end_turn();
                assert_eq!(temporaries_of(&s, &ids), vec![Some(1), Some(1), Some(1)]);
                for id in &ids {
                    s.expect_in_zone(id, "hand");
                }
                // The opponent's end of turn counts nothing down; the end of p1's next turn discards them.
                s.end_turn();
                assert_eq!(temporaries_of(&s, &ids), vec![Some(1), Some(1), Some(1)]);
                s.end_turn();
                for id in &ids {
                    assert!(!s.hand(P1).iter().any(|card| card.id == *id), "{id} is still in hand");
                }
                let discarded = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Discarded { instance_id, .. } if ids.contains(instance_id)))
                    .count();
                assert_eq!(discarded, 3);
            }

            #[test]
            fn r2179001_a_card_played_before_its_count_runs_out_escapes_it() {
                for seed in 1..=200 {
                    let mut s = shop(&format!("drop-field-{seed}"), false, &[FILLER]);
                    s.play(DROP, json!({}));
                    let unit = added(&s).into_iter().find(|id| {
                        let card = s.card(id);
                        query(&json_as(json!({ "defId": card.def_id, "withTokens": true })))
                            .first()
                            .is_some_and(|printed| {
                                printed.type_ == CardType::Unit
                                    && matches!(printed.cost, CardCost::Fixed(cost) if cost <= 3)
                                    && !printed.base.text.contains("Tribute")
                            })
                    });
                    let Some(unit) = unit else { continue };
                    let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        s.play(&unit, json!({}));
                    }));
                    if played.is_err() || s.card(&unit).zone.z() != ZoneName::Field {
                        continue;
                    }
                    next_own_turn(&mut s);
                    next_own_turn(&mut s);
                    assert_eq!(s.card(&unit).zone.z(), ZoneName::Field);
                    return;
                }
                panic!("no seed handed out a playable Unit");
            }

            #[test]
            fn s2_4_r317_a_full_hand_burns_what_doesn_t_fit_and_a_burned_card_takes_no_temporary() {
                let cap = usize::try_from(HAND_CAP).expect("a hand cap");
                let fillers = vec![FILLER; cap - 1];
                let mut s = shop("drop-full", false, &fillers);
                s.play(DROP, json!({}));
                // Nine cards in hand after the play: one fits, two burn.
                assert_eq!(added(&s).len(), 1);
                let burned: Vec<(String, String)> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Burned { instance_id, def_id, .. } => Some((instance_id.clone(), def_id.clone())),
                        _ => None,
                    })
                    .collect();
                assert_eq!(burned.len(), 2);
                for (id, def_id) in &burned {
                    match find_instance(s.state(), id) {
                        Some(card) => {
                            assert!(matches!(card.zone.z(), ZoneName::Graveyard | ZoneName::Gone));
                            assert!(!card.granted_keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Temporary));
                        }
                        // A burned unit token ceases to exist (R11): it is in no pile, so nothing holds a
                        // count; this seed burns one (C+ #19.5).
                        None => {
                            let def = def_of(Some(s.state()), def_id);
                            assert!(def.token && def.type_ == CardType::Unit, "{def_id}");
                        }
                    }
                }
                assert_eq!(s.hand(P1).len(), cap);
            }

            #[test]
            fn r386_the_card_count_and_the_temporary_read_through_param() {
                let mut s = shop("drop-tuned", false, &[FILLER]);
                step_param(s.card_mut(DROP), "cards", 1);
                step_param(s.card_mut(DROP), "temporary", 1);
                s.play(DROP, json!({}));
                let ids = added(&s);
                assert_eq!(ids.len(), 4);
                assert_eq!(temporaries_of(&s, &ids), vec![Some(3), Some(3), Some(3), Some(3)]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_three_cost_1_costoverride_1_still_temporary_2() {
                let mut s = shop("drop-radiant", true, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                assert_eq!(ids.len(), 3);
                for id in &ids {
                    assert_eq!(s.card(id).cost_override, Some(1));
                }
                assert_eq!(temporaries_of(&s, &ids), vec![Some(2), Some(2), Some(2)]);
            }
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, which gave Brittle 2 (R385).
        mod v0_5_2 {
            use super::*;

            #[test]
            fn r767000_r385_a_game_on_v0_5_2_gives_each_card_brittle_2_and_no_temporary() {
                crate::under_kept_as(
                    DROP,
                    script_v0_5_2(),
                    |def| {
                        for entry in def.params.iter_mut().flatten() {
                            if entry.key == "temporary" {
                                entry.key = "brittle".to_string();
                            }
                        }
                    },
                    || {
                        let mut s = shop("drop-kept", false, &[FILLER]);
                        s.play(DROP, json!({}));
                        let ids = added(&s);
                        assert_eq!(brittles_of(&s, &ids), vec![Some(2), Some(2), Some(2)]);
                        assert_eq!(temporaries_of(&s, &ids), vec![None, None, None]);
                    },
                );
            }
        }

        /// R386 the Radiant face's (1) is declared: an Upgrade makes the three cost (0) and a Degrade (2)
        #[test]
        fn r386_an_upgrade_makes_the_three_cost_0_and_a_degrade_2() {
            for (upgrade, cost) in [(true, 0), (false, 2)] {
                let mut s = shop("drop-radiant", true, &[FILLER]);
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, DROP, "setCost")
                } else {
                    crate::degrade_number(&mut s, DROP, "setCost")
                };
                assert_eq!(moved, cost);
                s.play(DROP, json!({}));
                for id in &added(&s) {
                    assert_eq!(s.card(id).cost_override, Some(cost));
                }
            }
        }
    }
}
