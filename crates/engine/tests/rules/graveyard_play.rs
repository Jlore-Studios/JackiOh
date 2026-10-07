//! Port of `packages/engine/test/graveyard-play.test.ts`.
//!
//! Playing cards from the graveyard (docs/classic-sets.md B5 E11; R454): the permissions, what
//! `legalActions` offers under them, §10.5 taking the card out of the graveyard, R65's prices, the
//! Plague Counter payment, a pause mid-play surviving JSON, and what the other seat sees.

use jackioh_engine::mana::effective_cost;
use jackioh_engine::modifiers::add_modifier;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    QUEST_REWARD_KEY, discover_spell, grave_spell, grave_trap, grave_unit, in_graveyard, lobbyist, monkey,
    only, pb_act, pb_playing, pb_reduce, plantation, plays_of, quest_card, round_trip, second_wind,
    three_spell, titan, zero_spell,
};

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

/// `playsOf(state, instanceId, player)`, each offered play as its JSON (TS's `{ type: "play", … }`).
fn plays_json(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<Value> {
    plays_of(state, instance_id, player)
        .iter()
        .map(|play| serde_json::to_value(play).expect("a play serialises"))
        .collect()
}

/// TS `{ ...play, playerId }`.
fn with_player(body: Value, player: PlayerId) -> Value {
    let mut body = body;
    body["playerId"] = json!(player);
    body
}

/// `eventsOfType(events, type)`, each event as its JSON.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

fn view_events_of(state: &GameState, viewer: PlayerId, kind: GameEventType) -> Vec<Value> {
    of_type(&view_for(state, viewer).events, kind)
}

/// Jest's `toMatchObject` over JSON: objects by subset, arrays by length and element.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// A refusal's text, for TS's `toMatch` (`""` when the action was not refused, which no pattern matches).
fn refusal(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// TS's `/no card .* in p1's hand/`: the one pattern here that is not a literal.
fn no_card_in_hand(text: &str, player: &str) -> bool {
    let head = "no card ";
    let tail = format!(" in {player}'s hand");
    text.find(head)
        .is_some_and(|at| text[at + head.len()..].contains(&tail))
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// TS `sinkFor(state)` / `{ state, events: [] }`: a sink lent with the state to one engine call.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn add_modifier_to(state: &mut GameState, player: PlayerId, expiry: ModifierExpiry, kind: ModifierKind) {
    let mut bench = Bench::new(state);
    add_modifier(&mut bench.sink(state), player, expiry, kind);
}

fn cost_discount(amount: i32, only_type: Option<CardType>) -> ModifierKind {
    ModifierKind::CostDiscount {
        amount,
        only_type,
        min_current_cost: None,
        once_per_turn: None,
    }
}

mod r454_e11_play_from_the_graveyard {
    use super::*;

    #[test]
    fn r454_a_permission_on_the_field_offers_and_plays_a_graveyard_card_as_a_play_counted_its_script_run_the_card_out_of_the_graveyard()
     {
        let mut state = pb_playing("r454-offer");
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);

        // No permission: nothing offered, and the play is refused.
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1), Vec::<Value>::new());
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })
            ))
            .contains("may not play that card from your graveyard")
        );

        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let offered = plays_json(&state, &spell.id, PlayerId::P1);
        assert_eq!(offered, vec![json!({ "type": "play", "instanceId": spell.id })]);

        let before = hero_health(&state, PlayerId::P2);
        let played = state.counters.played;
        let result = pb_reduce(&state, with_player(only(&offered), PlayerId::P1));
        assert_eq!(result.error, None);
        let after = &result.state;
        // Its script ran (1 damage), it was counted as a play, and it paid its price.
        assert_eq!(hero_health(after, PlayerId::P2), before - 1);
        assert_eq!(after.counters.played, played + 1);
        assert!(after.players.p1.turn_log.played_ids.contains(&spell.id));
        assert_eq!(after.players.p1.mana.current, 3);
        // The play said where it came from; the Spell resolved and landed in the graveyard again.
        let card_played = only(&of_type(&result.events, GameEventType::CardPlayed));
        assert_eq!(card_played["from"], json!("graveyard"));
        assert_eq!(card_played["costPaid"], json!(1));
        let resolved: Vec<Value> = of_type(&result.events, GameEventType::CardResolved)
            .iter()
            .map(|event| event["instanceId"].clone())
            .collect();
        assert_eq!(resolved, vec![json!(spell.id)]);
        assert!(ids(&after.players.p1.graveyard).contains(&spell.id));
    }

    #[test]
    fn r454_r1_a_unit_played_from_the_graveyard_is_placed_summoning_sick_and_its_cry_fires_played_from_anywhere()
     {
        let mut state = pb_playing("r454-unit");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let body = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1);
        let before = hero_health(&state, PlayerId::P2);

        let after = pb_act(
            &state,
            json!({ "type": "play", "instanceId": body.id, "zone": { "row": "units", "lane": 3 }, "playerId": "p1" }),
        );
        let standing: Vec<_> = active_units_of(&after, PlayerId::P1)
            .into_iter()
            .filter(|unit| unit.id == body.id)
            .collect();
        let placed = only(&standing);
        assert_eq!(
            placed.zone.clone(),
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 3
            }
        );
        assert_eq!(placed.summoned_turn, Some(after.turn));
        assert_eq!(hero_health(&after, PlayerId::P2), before - 2);
        assert!(!ids(&after.players.p1.graveyard).contains(&body.id));
    }

    #[test]
    fn r454_only_the_players_own_graveyard_is_reached_and_only_in_their_main_phase_with_nothing_open() {
        let mut state = pb_playing("r454-own");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let theirs = in_graveyard(&mut state, &grave_spell().id, PlayerId::P2);
        assert_eq!(plays_json(&state, &theirs.id, PlayerId::P1), Vec::<Value>::new());
        assert!(no_card_in_hand(
            &refusal(&pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": theirs.id, "playerId": "p1" })
            )),
            "p1"
        ));

        // p2's own Second Wind would let p2 — on p2's turn, not p1's.
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        assert_eq!(plays_json(&state, &theirs.id, PlayerId::P2), Vec::<Value>::new());
    }

    #[test]
    fn r454_r65_player_prices_reach_a_graveyard_play_a_next_spell_discount_prices_and_is_spent_and_an_auras_surcharge_prices_it()
     {
        let mut state = pb_playing("r454-prices");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &three_spell().id, PlayerId::P1);
        state.players.p1.mana.current = 2;
        // (3) is more than 2 mana: not offered.
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1), Vec::<Value>::new());

        add_modifier_to(
            &mut state,
            PlayerId::P1,
            ModifierExpiry::Used,
            cost_discount(1, Some(CardType::Spell)),
        );
        let offered = plays_json(&state, &spell.id, PlayerId::P1);
        assert_eq!(offered.len(), 1);
        let result = pb_reduce(&state, with_player(only(&offered), PlayerId::P1));
        assert_eq!(result.error, None);
        assert_eq!(
            only(&of_type(&result.events, GameEventType::CardPlayed))["costPaid"],
            json!(2)
        );
        assert!(
            !result
                .state
                .players
                .p1
                .mods
                .iter()
                .any(|modifier| matches!(modifier.kind, ModifierKind::CostDiscount { .. }))
        );

        // An aura's surcharge prices it too (Classic #77's shape: Spells cost (1) more).
        let mut taxed = pb_playing("r454-aura");
        put(
            &mut taxed,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        put(
            &mut taxed,
            &monkey().id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let again = in_graveyard(&mut taxed, &grave_spell().id, PlayerId::P1);
        let paid = pb_reduce(
            &taxed,
            json!({ "type": "play", "instanceId": again.id, "playerId": "p1" }),
        );
        assert_eq!(
            only(&of_type(&paid.events, GameEventType::CardPlayed))["costPaid"],
            json!(2)
        );
    }

    #[test]
    fn r454_r65_a_graveyard_card_is_priced_as_a_play_wherever_it_is_read_while_a_permission_lets_its_player_play_it_and_at_its_own_cost_otherwise()
     {
        let mut state = pb_playing("r454-read");
        put(
            &mut state,
            &monkey().id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        ); // Spells cost (1) more
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(effective_cost(&state, &spell, Default::default()), 1);
        let wind = put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        assert_eq!(effective_cost(&state, &spell, Default::default()), 2);
        // The view prints that price.
        let shown: Vec<CardView> = view_for(&state, PlayerId::P1)
            .you
            .graveyard
            .into_iter()
            .filter(|card| card.instance_id == spell.id)
            .collect();
        assert_eq!(only(&shown).cost, 2);
        find_instance_mut(&mut state, &wind.id)
            .expect("Second Wind stands")
            .vanilla = true;
        assert_eq!(effective_cost(&state, &spell, Default::default()), 1);
    }

    #[test]
    fn r454_second_winds_radiant_face_needs_a_price_of_1_or_more_as_it_would_be_paid() {
        let mut state = pb_playing("r454-min-price");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json_as(json!({ "radiant": true })),
        );
        let free = in_graveyard(&mut state, &zero_spell().id, PlayerId::P1);
        let one = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(plays_json(&state, &free.id, PlayerId::P1), Vec::<Value>::new());
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": free.id, "playerId": "p1" })
            ))
            .contains("must cost (1) or more")
        );
        assert_eq!(plays_json(&state, &one.id, PlayerId::P1).len(), 1);

        // A discount that takes the (1) Spell to (0) takes it below the floor: the price as it would be paid.
        let turn = state.turn;
        add_modifier_to(
            &mut state,
            PlayerId::P1,
            ModifierExpiry::ThisTurn { turn },
            cost_discount(1, None),
        );
        assert_eq!(plays_json(&state, &one.id, PlayerId::P1), Vec::<Value>::new());
    }

    #[test]
    fn r454_corpse_plantation_units_only_paid_with_its_plague_counters_at_least_1_at_most_the_tokens_and_the_price_the_rest_in_mana()
     {
        let mut state = pb_playing("r454-plague");
        let field = put(
            &mut state,
            &plantation().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        find_instance_mut(&mut state, &field.id)
            .expect("the Plantation stands")
            .counters
            .plague = Some(2);
        let body = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1); // (2)
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        state.players.p1.mana.current = 1;

        // Units only.
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1), Vec::<Value>::new());
        // 1 token + 1 mana, or 2 tokens; never mana alone under this permission.
        let payments: Vec<Value> = plays_json(&state, &body.id, PlayerId::P1)
            .into_iter()
            .filter(|play| play["zone"]["lane"] == json!(1))
            .map(|play| play["plague"].clone())
            .collect();
        assert_eq!(
            payments,
            vec![
                json!({ "from": field.id, "tokens": 1 }),
                json!({ "from": field.id, "tokens": 2 })
            ]
        );
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": body.id, "playerId": "p1" })
            ))
            .contains("spending Plague Counters")
        );
        assert!(refusal(&pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": body.id, "plague": { "from": field.id, "tokens": 0 }, "playerId": "p1" })
        ))
        .contains("at least 1 Plague Counter"));
        assert!(refusal(&pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": body.id, "plague": { "from": field.id, "tokens": 3 }, "playerId": "p1" })
        ))
        .contains("not that many"));
        // A hand card never spends tokens.
        let hand_card = only(&state.players.p1.hand);
        let hand_play = pb_reduce(
            &state,
            json!({
                "type": "play",
                "instanceId": hand_card.id,
                "plague": { "from": field.id, "tokens": 1 },
                "playerId": "p1",
            }),
        );
        assert!(refusal(&hand_play).contains("only a play from your graveyard can spend Plague Counters"));

        let result = pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": body.id, "plague": { "from": field.id, "tokens": 1 }, "playerId": "p1" }),
        );
        assert_eq!(result.error, None);
        let after = &result.state;
        // The price was the whole (2): one token and one mana.
        assert_eq!(
            only(&of_type(&result.events, GameEventType::CardPlayed))["costPaid"],
            json!(2)
        );
        assert_eq!(after.players.p1.mana.current, 0);
        assert_eq!(
            find_instance(after, &field.id).and_then(|card| card.counters.plague),
            Some(1)
        );
        assert!(
            of_type(&result.events, GameEventType::CounterChanged).contains(&json!({
                "type": "counterChanged",
                "instanceId": field.id,
                "counter": "plague",
                "value": 1,
            }))
        );
        assert!(
            active_units_of(after, PlayerId::P1)
                .iter()
                .any(|unit| unit.id == body.id)
        );
    }

    #[test]
    fn r454_two_permissions_a_unit_may_be_paid_in_mana_under_one_and_with_tokens_under_the_other() {
        let mut state = pb_playing("r454-both");
        let field = put(
            &mut state,
            &plantation().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        find_instance_mut(&mut state, &field.id)
            .expect("the Plantation stands")
            .counters
            .plague = Some(1);
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let body = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1);
        let payments: Vec<Value> = plays_json(&state, &body.id, PlayerId::P1)
            .into_iter()
            .filter(|play| play["zone"]["lane"] == json!(1))
            .map(|play| play.get("plague").cloned().unwrap_or(Value::Null))
            .collect();
        assert_eq!(
            payments,
            vec![Value::Null, json!({ "from": field.id, "tokens": 1 })]
        );
    }

    #[test]
    fn r454_in_too_deeps_reward_l_is_the_same_permission_held_while_the_card_says_it_was_earned() {
        let mut state = pb_playing("r454-quest");
        let quest = put(
            &mut state,
            &quest_card().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1), Vec::<Value>::new());
        find_instance_mut(&mut state, &quest.id)
            .expect("the quest card stands")
            .memory
            .insert(QUEST_REWARD_KEY.to_string(), json!(true));
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1).len(), 1);
    }

    #[test]
    fn r454_a_trap_played_from_the_graveyard_is_set_face_down_under_a_fresh_id_and_the_other_seat_reads_no_more_than_a_zone()
     {
        let mut state = pb_playing("r454-trap");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let trap = in_graveyard(&mut state, &grave_trap().id, PlayerId::P1);
        let result = pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": trap.id, "zone": { "row": "backrow", "lane": 4 }, "playerId": "p1" }),
        );
        assert_eq!(result.error, None);
        let placed = result.state.players.p1.backrow[3].clone();
        assert_eq!(
            placed.as_ref().map(|card| card.def_id.clone()),
            Some(grave_trap().id)
        );
        assert_ne!(placed.as_ref().map(|card| card.id.clone()), Some(trap.id.clone()));

        let theirs = view_for(&result.state, PlayerId::P2);
        let zone = serde_json::to_value(&theirs.opponent.backrow[3]).expect("a backrow view serialises");
        assert!(matches_object(&zone, &json!({ "faceDown": true })));
        // The raw event names the card and the id it had; p2's copy is the face-down redaction (R97,
        // R227) — no identity, no former id — with the pile it came from, which the public graveyard
        // already showed.
        assert_eq!(
            only(&of_type(&result.events, GameEventType::CardPlayed))["formerId"],
            json!(trap.id)
        );
        let shown = only(&of_type(&theirs.events, GameEventType::CardPlayed));
        assert!(matches_object(
            &shown,
            &json!({ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "from": "graveyard" })
        ));
        assert!(shown.get("formerId").is_none());
        // Its controller reads it (R33).
        let mine = only(&view_events_of(
            &result.state,
            PlayerId::P1,
            GameEventType::CardPlayed,
        ));
        assert_eq!(mine["defId"], json!(grave_trap().id));
    }

    #[test]
    fn r454_r226_a_card_no_longer_in_the_graveyard_at_step_4_is_not_played_the_rule_from_the_graveyard() {
        let mut state = pb_playing("r454-lost");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let titan_card = in_graveyard(&mut state, &titan().id, PlayerId::P1);
        // A body whose Death exiles its owner's graveyard: the Tribute paid at step 2 takes the Titan
        // out of the graveyard before step 4 can.
        let exiler = put(
            &mut state,
            "fx-1",
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let scripts = registered_scripts().clone();
        let mut patched = scripts.clone();
        let titan_id = titan_card.id.clone();
        patched.insert(
            "fx-1".to_string(),
            CardScripts {
                base: Script {
                    death: Some(hook(move |_ctx| {
                        let titan_id = titan_id.clone();
                        vec![Effect::new("test:exileGraveyard", move |ctx| {
                            let graveyard: Vec<CardInstance> = ctx.state.players.p1.graveyard.clone();
                            for mut card in graveyard {
                                if card.id == titan_id {
                                    move_to_zone(
                                        &mut *ctx.state,
                                        &mut card,
                                        OffFieldZone::Exile,
                                        Default::default(),
                                    );
                                }
                            }
                        })]
                    })),
                    ..Script::default()
                },
                radiant: Script::default(),
            },
        );
        register_scripts(patched);
        // TS's `try … finally` restores the registry; here the override is this test thread's own, so
        // a failing assertion cannot leak it, and the restore runs after the assertions as in TS.
        let result = pb_reduce(
            &state,
            json!({
                "type": "play",
                "instanceId": titan_card.id,
                "zone": { "row": "units", "lane": 2 },
                "tributes": [exiler.id],
                "playerId": "p1",
            }),
        );
        assert_eq!(result.error, None);
        assert_eq!(
            of_type(&result.events, GameEventType::CardPlayed),
            Vec::<Value>::new()
        );
        assert!(ids(&result.state.players.p1.exile).contains(&titan_card.id));
        assert_eq!(result.state.counters.played, state.counters.played);
        register_scripts(scripts);
    }

    #[test]
    fn r454_r455_a_ban_on_the_field_refuses_a_graveyard_play_as_it_refuses_a_hand_play() {
        let mut state = pb_playing("r454-ban");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        put(
            &mut state,
            &lobbyist().id,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        let spell = in_graveyard(&mut state, &three_spell().id, PlayerId::P1);
        assert_eq!(plays_json(&state, &spell.id, PlayerId::P1), Vec::<Value>::new());
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })
            ))
            .contains("can't play (3)+ Cost cards")
        );
    }

    #[test]
    fn r454_a_graveyard_play_that_asks_pauses_survives_json_and_finishes_the_same_way_live_and_round_tripped()
    {
        let mut state = pb_playing("r454-pause");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let asker = in_graveyard(&mut state, &discover_spell().id, PlayerId::P1);
        let paused = pb_act(
            &state,
            json!({ "type": "play", "instanceId": asker.id, "playerId": "p1" }),
        );
        let pending = paused.pending.as_ref();
        assert_eq!(pending.map(|open| open.kind), Some(PromptKind::Discover));
        assert_eq!(pending.map(|open| open.player_id), Some(PlayerId::P1));
        // The other seat sees a prompt is open, never its options (§10.8).
        assert_eq!(
            serde_json::to_value(&view_for(&paused, PlayerId::P2).pending)
                .expect("a pending view serialises"),
            json!({ "forYou": false, "pendingFor": "p1" })
        );

        let round = round_trip(&paused);
        assert_eq!(hash_state(&round), hash_state(&paused));
        let answers: Vec<ActionBody> = legal_actions(&paused, PlayerId::P1)
            .into_iter()
            .filter(|action| action.action_type() == ActionType::Answer)
            .collect();
        let answer = serde_json::to_value(only(&answers)).expect("an answer serialises");
        let live = pb_act(&paused, with_player(answer.clone(), PlayerId::P1));
        let replayed = pb_act(&round, with_player(answer, PlayerId::P1));
        assert_eq!(hash_state(&replayed), hash_state(&live));
        assert_eq!(live.pending, None);
        assert!(ids(&live.players.p1.graveyard).contains(&asker.id));
    }

    #[test]
    fn r454_a_graveyard_plays_card_played_reaches_the_other_seat_with_its_source_and_nothing_hidden() {
        let mut state = pb_playing("r454-view");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        let result = pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        let event = only(&view_events_of(
            &result.state,
            PlayerId::P2,
            GameEventType::CardPlayed,
        ));
        assert!(matches_object(
            &event,
            &json!({ "instanceId": spell.id, "defId": grave_spell().id, "from": "graveyard" })
        ));
        // Engine bookkeeping never travels (R119, R174).
        assert!(event.get("exitsFrom").is_none());
    }
}
