//! C+ #23 Dropshipping (SPEC §8.7 row 23): add {cards} random cards from every card and token but this
//! (R382, R387), each given Brittle {brittle} (R385); Radiant: each costs (1).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-023";

/// "Each costs (1)" on the Radiant face.
const RADIANT_COST: i32 = 1;

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

fn dropship(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let n = param(&*ctx, "brittle");
            let mut add = json!({
                "query": { "withTokens": true, "excludeDefId": ID },
                "count": param(&*ctx, "cards"),
            });
            if radiant {
                add["costOverride"] = json!(RADIANT_COST);
            }
            vec![
                add_random_from_catalog(json_as(add)),
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

// C+ #23 Dropshipping — SPEC §8.7 row 23, BUILD M9 Classic+ row C+ 23: "Adds 3 random cards drawn from
// every card and token of every set but Dropshipping (R382, R387; repeats allowed, R60), so a Grape, a
// Loser, an AI generated card or a unit-token card (R11) can arrive, each at its printed cost and
// given Brittle 2 (R385): held in your hand, where it does not tick (R638), and started as the card enters
// the field, where it ticks to 1 at the start of your turn t + 2 and crumbles at t + 4 (an ordinary
// destroy that Indestructible ignores, the count staying 0); the count rides the card from hand to
// field; the owner sees the counts, the opponent sees three cards added under the sentinel (R97); a full hand burns
// the rest; card count and Brittle read through `param()`; radiant they cost (1) (`costOverride` 1)".
#[cfg(test)]
mod tests {
    use super::*;
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

    /// The Brittle counts the owner sees on the cards `ids` names, in hand order.
    fn brittles_of(s: &Scenario, ids: &[String]) -> Vec<Option<i32>> {
        own_hand(s)
            .into_iter()
            .filter(|card| ids.contains(&card.instance_id))
            .map(|card| card.brittle)
            .collect()
    }

    /// Two `endTurn`s: the opponent's turn, then p1's next start of turn.
    fn next_own_turn(s: &mut Scenario) -> &mut Scenario {
        s.end_turn().end_turn()
    }

    /// TS wrote through the live instance `s.card(ref)` handed back.
    fn card_mut<'a>(s: &'a mut Scenario, card: &str) -> &'a mut CardInstance {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in no zone")
    }

    /// TS `/-\d{3}-\d/.test(id)`: a token's three-digit index followed by its own number.
    fn has_token_index(id: &str) -> bool {
        id.as_bytes().windows(6).any(|w| {
            w[0] == b'-' && w[1..4].iter().all(u8::is_ascii_digit) && w[4] == b'-' && w[5].is_ascii_digit()
        })
    }

    /// TS `toMatchObject`: every key of `pattern` is in `actual` with a matching value.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches_object(got, value))
            }
            _ => actual == pattern,
        }
    }

    mod c_n23_dropshipping {
        use super::*;

        #[test]
        fn is_a_1_cn_spell_both_faces_add_and_give_brittle() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert!(def.tags.contains(&Tag::Cn));
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

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
            fn adds_three_cards_to_the_hand_each_at_its_printed_cost_with_brittle_2_the_owner_sees() {
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
                }
                assert_eq!(views.iter().map(|card| card.brittle).collect::<Vec<_>>(), vec![Some(2), Some(2), Some(2)]);
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
                crate::register_all();
                let pool = query(&json_as(json!({ "withTokens": true, "excludeDefId": DROP })));
                let grapes: IndexSet<&str> = GRAPE_ODDS.iter().map(|grape| grape.def_id).collect();
                let table: i32 = GRAPE_ODDS.iter().map(|grape| grape.percent).sum();
                let mut differed = 0;
                for seed in ["drop-reroll-3", "drop-reroll-36", "drop-reroll-72", "drop-reroll-76"] {
                    let mut s = shop(seed, false, &[FILLER]);
                    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let mut probe = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let mut expected: Vec<Option<String>> = Vec::new();
                    let mut firsts: Vec<Option<String>> = Vec::new();
                    for _ in 0..3 {
                        let first = probe.pick(&pool).map(|card| card.id.clone());
                        if let Some(id) = &first {
                            if grapes.contains(id.as_str()) {
                                probe.int(table);
                            }
                        }
                        firsts.push(first);
                        expected.push(pick_generated(&mut rng, &pool, None).map(|card| card.id.clone()));
                    }
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
                let thawed: GameState =
                    serde_json::from_value(serde_json::to_value(s.state()).expect("JSON")).expect("a state");
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
                        &serde_json::to_value(event).expect("JSON"),
                        &json!({ "instanceId": "hidden", "defId": "hidden" }),
                    ));
                }
                let text = serde_json::to_string(&s.view(P2)).expect("JSON");
                for id in &ids {
                    assert!(!text.contains(id.as_str()));
                }
            }

            #[test]
            fn r385_r638_the_cards_hold_brittle_2_in_the_hand_it_never_ticks_there_and_nothing_crumbles_however_long_they_wait(
            ) {
                let mut s = shop("drop-hold", false, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                let turn = s.state().turn;
                next_own_turn(&mut s);
                next_own_turn(&mut s);
                assert_eq!(s.state().turn, turn + 4);
                assert_eq!(brittles_of(&s, &ids), vec![Some(2), Some(2), Some(2)]);
                assert!(!s.events().iter().any(|event| matches!(
                    event.event_type(),
                    GameEventType::Crumbled | GameEventType::Discarded
                )));
                for id in &ids {
                    s.expect_in_zone(id, "hand");
                }
            }

            #[test]
            fn r385_the_count_rides_the_card_onto_the_field_and_there_it_is_an_ordinary_destroy_indestructible_ignores() {
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
                    if played.is_err() {
                        continue;
                    }
                    let lane = match s.card(&unit).zone {
                        Zone::Field { lane, .. } => lane,
                        _ => continue,
                    };
                    let at = usize::try_from(lane - 1).expect("a lane");
                    // The Brittle count came with it, and shows on the board to both players.
                    assert_eq!(
                        s.view(P2).opponent.units.get(at).and_then(|view| view.as_ref()).and_then(|view| view.brittle),
                        Some(2)
                    );
                    card_mut(&mut s, &unit).granted_keywords.push(Keyword::Indestructible);
                    next_own_turn(&mut s);
                    next_own_turn(&mut s);
                    // Indestructible ignores the destroy, and the count stays at 0.
                    assert_eq!(s.card(&unit).zone.z(), ZoneName::Field);
                    assert_eq!(
                        s.view(P1)
                            .you
                            .units
                            .get(at)
                            .and_then(|view| view.as_ref())
                            .and_then(|view| view.brittle)
                            .unwrap_or(0),
                        0
                    );
                    return;
                }
                panic!("no seed handed out a playable Unit");
            }

            #[test]
            fn s2_4_r317_a_full_hand_burns_what_doesn_t_fit_and_a_burned_card_takes_no_count() {
                let cap = usize::try_from(HAND_CAP).expect("a hand cap");
                let fillers = vec![FILLER; cap - 1];
                let mut s = shop("drop-full", false, &fillers);
                s.play(DROP, json!({}));
                // Nine cards in hand after the play: one fits, two burn.
                assert_eq!(added(&s).len(), 1);
                let burned: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(burned.len(), 2);
                for id in &burned {
                    let card = s.card(id);
                    assert!(matches!(card.zone.z(), ZoneName::Graveyard | ZoneName::Gone));
                    assert_eq!(card.brittle, None);
                }
                assert_eq!(s.hand(P1).len(), cap);
            }

            #[test]
            fn r386_the_card_count_and_the_brittle_read_through_param() {
                let mut s = shop("drop-tuned", false, &[FILLER]);
                step_param(card_mut(&mut s, DROP), "cards", 1);
                step_param(card_mut(&mut s, DROP), "brittle", 1);
                s.play(DROP, json!({}));
                let ids = added(&s);
                assert_eq!(ids.len(), 4);
                assert_eq!(brittles_of(&s, &ids), vec![Some(3), Some(3), Some(3), Some(3)]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_three_cost_1_costoverride_1_still_brittle_2() {
                let mut s = shop("drop-radiant", true, &[FILLER]);
                s.play(DROP, json!({}));
                let ids = added(&s);
                assert_eq!(ids.len(), 3);
                for id in &ids {
                    assert_eq!(s.card(id).cost_override, Some(1));
                }
                assert_eq!(brittles_of(&s, &ids), vec![Some(2), Some(2), Some(2)]);
            }
        }
    }
}
