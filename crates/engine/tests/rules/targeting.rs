//! The targeting point and what a declaration may pick (docs/classic-sets.md B5 E5, E9, E35; R450).
//!
//! "Targeting" is choosing: a declared `target` pick of a play, a cast or an activation, and a `target`
//! prompt's answer (random picks, "all" effects, Tributes, hand and zone picks target nothing). At that
//! point a targeting cost is owed (Classic #89 Paul Allen's Ghost: random discards at pay time, R682,
//! and with too few other cards the card is no legal target at all), and an interceptor answers
//! (Classic #33 Joro, from its owner's hand: summoned, no Cry, summoning sick, and the pick moves to
//! it). A Spell's declarations never offer a card Immune to Spells, and the v0.2.0 filter fields — a
//! graveyard pick, a cost range, damaged, Plague Counters, a named predicate — narrow what a declaration
//! offers.
//!
//! Port of `packages/engine/test/targeting.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, sink_for, slot};
use crate::rules::fixtures::play_pipeline_a::{PA, with_play_a};

static NONCE: AtomicU32 = AtomicU32::new(0);

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = ids(&ready.players[player].hand);
        ready = must(&ready, player, json!({ "type": "mulligan", "keep": keep })).0;
    }
    for player in [PlayerId::P1, PlayerId::P2] {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
        ready.players[player].hand = vec![];
    }
    ready
}

fn attempt(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    reduce(
        state,
        &Action::new(json_as(body), player_id, format!("pa-tg-{nonce}")),
    )
}

fn must(state: &GameState, player_id: PlayerId, body: Value) -> (GameState, Vec<GameEvent>) {
    let result = attempt(state, player_id, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

fn hand(state: &mut GameState, player: PlayerId, def_id: &str, count: i32) -> Vec<CardInstance> {
    in_hand(state, def_id, player, count)
}

fn one(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    hand(state, player, def_id, 1)
        .into_iter()
        .next()
        .expect("no card")
}

fn at(card: &CardInstance) -> Selection {
    Selection::Instance {
        instance_id: card.id.clone(),
    }
}

fn offered(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<Selection> {
    let decl = declared_targets(state, card)
        .into_iter()
        .next()
        .expect("no declaration");
    legal_selections_for(state, player, card, &decl)
}

fn answer(state: &GameState, player_id: PlayerId, selection: Vec<Selection>) -> (GameState, Vec<GameEvent>) {
    let choice_id = state
        .pending
        .as_ref()
        .map(|pending| pending.id.clone())
        .unwrap_or_default();
    must(
        state,
        player_id,
        json!({ "type": "answer", "choiceId": choice_id, "selection": selection }),
    )
}

/// The card as it stands in the state now (TS held the live object).
fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is in the state")
}

/// `state.players[p].units[i]?.[0]`: the top card of a unit pile.
fn top(pile: &Option<Pile>) -> Option<&CardInstance> {
    pile.as_ref().and_then(|cards| cards.first())
}

/// `eventsOfType(events, kind)`, each event as its JSON.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// `eventsOfType(events, kind).map((event) => event[field])`.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    of_type(events, kind)
        .into_iter()
        .map(|event| event[field].clone())
        .collect()
}

/// `toMatchObject`: every key of `expected` is in `actual` with a matching value.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

/// `toMatch(/text/)` on an error that must be there.
fn says(error: &Option<String>, text: &str) -> bool {
    error.as_deref().is_some_and(|message| message.contains(text))
}

/// `Array.prototype.indexOf` / `lastIndexOf`: -1 when absent.
fn index_of(order: &[GameEventType], wanted: GameEventType) -> i64 {
    order
        .iter()
        .position(|kind| *kind == wanted)
        .map_or(-1, |at| at as i64)
}
fn last_index_of(order: &[GameEventType], wanted: GameEventType) -> i64 {
    order
        .iter()
        .rposition(|kind| *kind == wanted)
        .map_or(-1, |at| at as i64)
}

/// A play as its JSON, so its `targets` and the absence of `discards` read as TS reads them.
fn play_json(play: &impl serde::Serialize) -> Value {
    serde_json::to_value(play).expect("a play serialises")
}

fn names_first(play: &Value, card: &CardInstance) -> bool {
    play["targets"][0]["pick"] == json!("instance") && play["targets"][0]["instanceId"] == json!(card.id)
}

mod r450_the_v0_2_0_filter_fields_10_6 {
    use super::*;

    #[test]
    fn r450_a_graveyard_pick_offers_either_sides_graveyard_cards_the_filter_admits() {
        let mut state = game("r450-graveyard");
        let mut mine = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut theirs = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({}));
        // Move them to graveyards, with a Spell beside them that the type filter refuses.
        state.players.p1.units[0] = None;
        state.players.p2.units[0] = None;
        mine.zone = Zone::Graveyard { player: PlayerId::P1 };
        theirs.zone = Zone::Graveyard { player: PlayerId::P2 };
        state.players.p1.graveyard.push(mine.clone());
        state.players.p2.graveyard.push(theirs.clone());
        let mut spell = one(&mut state, PlayerId::P1, &PA.ping.id);
        state.players.p1.hand.retain(|card| card.id != spell.id);
        spell.zone = Zone::Graveyard { player: PlayerId::P1 };
        state.players.p1.graveyard.push(spell);
        let raiser = one(&mut state, PlayerId::P1, &PA.grave_raiser.id);

        assert_eq!(
            offered(&state, PlayerId::P1, &raiser),
            vec![at(&mine), at(&theirs)]
        );
    }

    #[test]
    fn r450_a_cost_range_reads_r65s_cost_where_the_card_is_now_and_no_hero_has_a_cost() {
        let mut state = game("r450-cost");
        let cheap = one(&mut state, PlayerId::P1, "fx-3");
        let dear = one(&mut state, PlayerId::P1, &PA.ghost.id);
        let discounted = one(&mut state, PlayerId::P1, &PA.ghost.id);
        live_mut(&mut state, &discounted).cost_mod = -1;
        let field = put(&mut state, "fx-4", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let hunter = one(&mut state, PlayerId::P1, &PA.cheap_hunter.id);

        let picks = offered(&state, PlayerId::P1, &hunter);
        assert!(picks.contains(&at(&cheap)));
        assert!(picks.contains(&at(&discounted)));
        assert!(picks.contains(&at(&field)));
        assert!(!picks.contains(&at(&dear)));
        assert!(!picks.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
    }

    #[test]
    fn r450_damaged_and_plague_counter_filters_offer_only_the_cards_that_match() {
        let mut state = game("r450-damaged");
        let hurt = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let whole = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 2), json!({}));
        live_mut(&mut state, &hurt).damage = 1;
        let plagued = put(
            &mut state,
            &PA.field.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        live_mut(&mut state, &plagued).counters.plague = Some(2);
        let clean = put(
            &mut state,
            &PA.chalice.id,
            slot(PlayerId::P2, Row::Backrow, 2),
            json!({}),
        );
        let medic = one(&mut state, PlayerId::P1, &PA.medic.id);
        let hunter = one(&mut state, PlayerId::P1, &PA.plague_hunter.id);

        assert_eq!(offered(&state, PlayerId::P1, &medic), vec![at(&hurt)]);
        assert_eq!(offered(&state, PlayerId::P1, &hunter), vec![at(&plagued)]);
        assert!(!offered(&state, PlayerId::P1, &hunter).contains(&at(&clean)));
        assert!(!offered(&state, PlayerId::P1, &hunter).contains(&at(&whole)));
    }

    #[test]
    fn r450_a_named_predicate_is_asked_for_cards_and_zones_and_a_hero_it_does_not_admit_is_not_offered() {
        let mut state = game("r450-check");
        let lane1 = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let lane2 = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let hunter = one(&mut state, PlayerId::P1, &PA.lane_hunter.id);

        let picks = offered(&state, PlayerId::P1, &hunter);
        assert!(picks.contains(&at(&lane2)));
        assert!(!picks.contains(&at(&lane1)));
        assert!(picks.contains(&Selection::Zone {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 2
        }));
        assert!(picks.contains(&Selection::Zone {
            player: PlayerId::P1,
            row: Row::Backrow,
            lane: 2
        }));
        assert!(picks.iter().all(|pick| !matches!(pick, Selection::Hero { .. })));
        assert!(picks.iter().all(|pick| match pick {
            Selection::Zone { lane, .. } => *lane == 2,
            _ => true,
        }));
    }
}

mod r450_immune_to_spells_e35_a_spells_declarations_never_offer_it {
    use super::*;

    #[test]
    fn r450_a_spell_cannot_name_an_immune_unit_and_a_units_cry_can() {
        let mut state = game("r450-immune");
        let immune = put(
            &mut state,
            &PA.immune.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let zapper = one(&mut state, PlayerId::P1, &PA.zapper.id);

        assert!(!offered(&state, PlayerId::P1, &bolt).contains(&at(&immune)));
        assert!(offered(&state, PlayerId::P1, &zapper).contains(&at(&immune)));
        assert!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&immune)] })
            )
            .error
            .is_some()
        );
        let (after, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": zapper.id, "targets": [at(&immune)] }),
        );
        assert_eq!(top(&after.players.p2.units[0]).map(|card| card.damage), Some(1));
    }
}

mod r450_a_targeting_cost_classic_89_paul_allens_ghost {
    use super::*;

    #[test]
    fn r450_r682_a_declared_target_naming_it_lists_one_play_with_no_discards_carried_offered_only_when_payable()
     {
        let mut state = game("r450-ghost-list");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 3);

        assert_eq!(targeting_discards_of(&state, &ghost), 2);
        let plays: Vec<Value> = play_actions_for(&state, PlayerId::P1, &bolt)
            .iter()
            .map(play_json)
            .collect();
        let at_ghost: Vec<&Value> = plays.iter().filter(|play| names_first(play, &ghost)).collect();
        let at_other: Vec<&Value> = plays.iter().filter(|play| names_first(play, &other)).collect();
        // R682: the discards are random at pay time, so one play, carrying none.
        assert_eq!(at_ghost.len(), 1);
        assert_eq!(at_other.len(), 1);
        assert!(at_ghost.iter().all(|play| play.get("discards").is_none()));
        assert!(at_other.iter().all(|play| play.get("discards").is_none()));
        assert_eq!(
            legal_actions(&state, PlayerId::P1)
                .iter()
                .filter(
                    |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == bolt.id)
                )
                .count(),
            plays.len()
        );
    }

    #[test]
    fn r682_a_targeting_cost_is_never_a_choice_with_exactly_the_cost_held_the_play_pays_both_with_no_prompt()
    {
        let mut state = game("r640-exact-cost");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let spares = hand(&mut state, PlayerId::P1, "fx-5", 2);
        let (after, events) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] }),
        );
        assert_eq!(after.pending, None);
        let mut discarded: Vec<String> = field_of(&events, GameEventType::Discarded, "instanceId")
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        discarded.sort();
        let mut wanted = vec![spares[0].id.clone(), spares[1].id.clone()];
        wanted.sort();
        assert_eq!(discarded, wanted);
        assert_eq!(after.players.p1.hand.len(), 0);
    }

    #[test]
    fn r450_r682_refuses_a_play_naming_it_when_too_few_other_cards_are_held_and_takes_none_otherwise() {
        let mut state = game("r450-ghost-refuse");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 1);
        // Bolt plus one other: only one card outside the played card, fewer than the two owed — no legal target.
        assert!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] })
            )
            .error
            .is_some()
        );
        assert_eq!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p2" }] })
            )
            .error,
            None
        );

        let mut rich = game("r450-ghost-afford");
        let rich_ghost = put(
            &mut rich,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let rich_bolt = one(&mut rich, PlayerId::P1, &PA.bolt.id);
        hand(&mut rich, PlayerId::P1, "fx-5", 2);
        assert_eq!(
            attempt(
                &rich,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": rich_bolt.id, "targets": [at(&rich_ghost)] })
            )
            .error,
            None
        );
    }

    #[test]
    fn r450_r682_two_costly_picks_each_payable_alone_but_not_together_are_refused_as_a_cost() {
        let mut state = game("r450-ghost-sum");
        let first = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let twin = one(&mut state, PlayerId::P1, &PA.twin.id);
        hand(&mut state, PlayerId::P1, "fx-5", 3);
        // Each pick is offered alone (three others pay either two), but the pair owes four.
        assert!(offered(&state, PlayerId::P1, &twin).contains(&at(&first)));
        assert!(offered(&state, PlayerId::P1, &twin).contains(&at(&second)));
        let refused = attempt(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": twin.id, "targets": [at(&first), at(&second)] }),
        );
        assert!(says(&refused.error, "discard"));
    }

    #[test]
    fn r450_with_fewer_than_two_other_cards_in_hand_it_is_no_legal_target() {
        let mut state = game("r450-ghost-few");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 1);
        assert!(!can_pay_to_target(&state, PlayerId::P1, &ghost, Some(&bolt.id)));
        assert!(!offered(&state, PlayerId::P1, &bolt).contains(&at(&ghost)));
    }

    #[test]
    fn r450_r682_two_random_other_cards_are_paid_at_step_2_with_the_mana_and_the_play_resolves_at_the_card() {
        let mut state = game("r450-ghost-pay");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let spares = hand(&mut state, PlayerId::P1, "fx-5", 3);
        let spare_ids = ids(&spares);

        let (after, events) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] }),
        );

        let order: Vec<GameEventType> = events.iter().map(GameEvent::event_type).collect();
        let discarded: Vec<String> = field_of(&events, GameEventType::Discarded, "instanceId")
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        assert_eq!(discarded.len(), 2);
        // Random, but never the card being played: both come from the three spares.
        for id in &discarded {
            assert!(spare_ids.contains(id));
        }
        assert_eq!(discarded.iter().collect::<IndexSet<_>>().len(), 2);
        assert!(index_of(&order, GameEventType::ManaChanged) < index_of(&order, GameEventType::Discarded));
        assert!(
            last_index_of(&order, GameEventType::Discarded) < index_of(&order, GameEventType::CardAnnounced)
        );
        assert_eq!(after.players.p1.hand.len(), 1);
        assert!(
            after
                .players
                .p1
                .hand
                .first()
                .is_some_and(|card| spare_ids.contains(&card.id))
        );
        assert_eq!(
            card_at(&after, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(2)
        );
    }

    #[test]
    fn r450_r682_it_binds_its_own_controller_too() {
        let mut state = game("r450-ghost-own");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 1);
        // Bolt plus one other: unpayable even for its controller — no legal target.
        assert!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] })
            )
            .error
            .is_some()
        );
        assert!(!offered(&state, PlayerId::P1, &bolt).contains(&at(&ghost)));

        let mut rich = game("r450-ghost-own-rich");
        let own_ghost = put(
            &mut rich,
            &PA.ghost.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let rich_bolt = one(&mut rich, PlayerId::P1, &PA.bolt.id);
        hand(&mut rich, PlayerId::P1, "fx-5", 2);
        // Payable — and the two random discards land on its controller all the same.
        let (_, events) = must(
            &rich,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": rich_bolt.id, "targets": [at(&own_ghost)] }),
        );
        assert_eq!(events_of_type(&events, GameEventType::Discarded).len(), 2);
    }

    #[test]
    fn r450_r682_a_prompt_answer_naming_it_pays_two_random_discards_at_once_and_goes_on() {
        let mut state = game("r450-ghost-prompt");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let chooser = one(&mut state, PlayerId::P1, &PA.chooser.id);
        let spares = hand(&mut state, PlayerId::P1, "fx-5", 2);
        let spare_ids = ids(&spares);

        let (asked, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": chooser.id }),
        );
        assert_eq!(
            asked.pending.as_ref().map(|pending| pending.kind),
            Some(PromptKind::Target)
        );
        assert!(asked.pending.as_ref().is_some_and(|pending| {
            pending
                .options
                .iter()
                .any(|option| option.selection == at(&ghost))
        }));

        let (done, events) = answer(&asked, PlayerId::P1, vec![at(&ghost)]);
        // R682: no follow-up hand prompt — the cost is paid at once, at random.
        assert_eq!(done.pending, None);
        assert!(done.work.is_empty());
        let discarded: Vec<String> = field_of(&events, GameEventType::Discarded, "instanceId")
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        assert_eq!(discarded.len(), 2);
        for id in &discarded {
            assert!(spare_ids.contains(id));
        }
        assert_eq!(
            card_at(&done, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(2)
        );
    }

    #[test]
    fn r450_a_prompt_never_offers_it_to_a_chooser_who_cannot_pay() {
        let mut state = game("r450-ghost-prompt-few");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let chooser = one(&mut state, PlayerId::P1, &PA.chooser.id);
        hand(&mut state, PlayerId::P1, "fx-5", 1);
        let (asked, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": chooser.id }),
        );
        let picks: Vec<Selection> = asked
            .pending
            .as_ref()
            .map(|pending| {
                pending
                    .options
                    .iter()
                    .map(|option| option.selection.clone())
                    .collect()
            })
            .unwrap_or_default();
        assert!(picks.contains(&at(&other)));
        assert!(!picks.contains(&at(&ghost)));
    }

    #[test]
    fn r450_r682_an_echo_repeats_fresh_pick_of_it_pays_the_discards_too_the_pipelines_own_prompt() {
        let mut state = game("r450-ghost-echo");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let echo = one(&mut state, PlayerId::P1, &PA.echo_bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 2);

        let (repeat, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": echo.id, "targets": [at(&other)] }),
        );
        assert_eq!(
            repeat
                .pending
                .as_ref()
                .map(|pending| pending.resume.hook.as_str()),
            Some("play")
        );
        let (done, events) = answer(&repeat, PlayerId::P1, vec![at(&ghost)]);
        assert_eq!(done.pending, None);
        assert_eq!(events_of_type(&events, GameEventType::Discarded).len(), 2);
        assert_eq!(
            card_at(&done, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(1)
        );
        assert_eq!(
            card_at(&done, slot(PlayerId::P2, Row::Units, 2)).map(|card| card.damage),
            Some(1)
        );
        assert!(ids(&done.players.p1.graveyard).contains(&echo.id));
    }

    #[test]
    fn r450_an_answer_whose_picks_cost_more_cards_than_its_chooser_holds_is_refused() {
        let mut state = game("r450-ghost-sum");
        let first = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        hand(&mut state, PlayerId::P1, "fx-5", 3);
        let pending = PendingChoice {
            id: "q-test".to_string(),
            player_id: PlayerId::P1,
            kind: PromptKind::Target,
            prompt: "two".to_string(),
            options: vec![],
            min: 2,
            max: 2,
            budget: None,
            resume: Resume {
                def_id: String::new(),
                hook: "resume".to_string(),
                step: String::new(),
                radiant: false,
                instance_id: None,
                data: IndexMap::new(),
            },
        };
        // TS `string | null`: a refusal is an `Err` with its text (SURFACE §4.4.9).
        assert!(why_target_answer_refused(&state, &pending, &[at(&first), at(&second)]).is_err());
        assert!(why_target_answer_refused(&state, &pending, &[at(&first)]).is_ok());
    }
}

mod r450_an_interception_classic_33_joro {
    use super::*;

    #[test]
    fn r450_summons_it_from_its_owners_hand_no_cry_summoning_sick_and_moves_the_declared_pick_to_it() {
        let mut state = game("r450-joro");
        let unit = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let joro = one(&mut state, PlayerId::P2, &PA.joro.id);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        let (after, events) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&unit)] }),
        );

        assert_eq!(
            Value::Array(of_type(&events, GameEventType::Redirected)),
            json!([{ "type": "redirected", "what": "target", "fromId": unit.id, "toId": joro.id, "byInstanceId": joro.id }])
        );
        let summoned = of_type(&events, GameEventType::Summoned)
            .into_iter()
            .find(|event| event["instanceId"] == json!(joro.id));
        assert!(summoned.is_some_and(|event| matches_object(
            &event,
            &json!({ "player": "p2", "row": "units", "lane": 2 })
        )));
        // Its Cry would have hit p1's hero for 5: a summon fires none (R1).
        assert_eq!(after.players.p1.hero.health, HERO_HEALTH);
        assert_eq!(
            of_type(&events, GameEventType::CardAnnounced)
                .first()
                .map(|event| event["targets"].clone()),
            Some(json!([joro.id]))
        );
        assert_eq!(
            field_of(&events, GameEventType::Damage, "targetId"),
            vec![json!(joro.id)]
        );
        assert_eq!(
            card_at(&after, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(0)
        );
        // The redirect is public on both seats.
        assert_eq!(
            field_of(
                &view_for(&after, PlayerId::P1).events,
                GameEventType::Redirected,
                "toId"
            )
            .first(),
            Some(&json!(joro.id))
        );
    }

    #[test]
    fn r450_the_interceptor_is_summoning_sick() {
        let mut state = game("r450-joro-sick");
        let unit = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let joro = one(&mut state, PlayerId::P2, &PA.joro.id);
        let zapper = one(&mut state, PlayerId::P1, &PA.zapper.id);
        let (after, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": zapper.id, "targets": [at(&unit)] }),
        );
        let standing = card_at(&after, slot(PlayerId::P2, Row::Units, 2)).cloned();
        assert_eq!(
            standing.as_ref().map(|card| card.id.clone()),
            Some(joro.id.clone())
        );
        assert_eq!(
            standing.as_ref().and_then(|card| card.summoned_turn),
            Some(after.turn)
        );
        assert_eq!(standing.as_ref().map(|card| card.damage), Some(1));
        assert_eq!(
            top(&after.players.p2.units[1]).map(|card| card.id.clone()),
            Some(joro.id.clone())
        );
    }

    #[test]
    fn r450_nothing_answers_a_hero_target_a_players_own_unit_a_full_row_or_a_declaration_it_does_not_fit() {
        let mut hero = game("r450-joro-hero");
        put(&mut hero, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        one(&mut hero, PlayerId::P2, &PA.joro.id);
        let bolt = one(&mut hero, PlayerId::P1, &PA.bolt.id);
        let to_hero = must(
            &hero,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        assert!(events_of_type(&to_hero.1, GameEventType::Redirected).is_empty());

        let mut own = game("r450-joro-own");
        let mine = put(&mut own, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        one(&mut own, PlayerId::P1, &PA.joro.id);
        let zap = one(&mut own, PlayerId::P1, &PA.zapper.id);
        let zapped = must(
            &own,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": zap.id, "targets": [at(&mine)] }),
        );
        assert!(events_of_type(&zapped.1, GameEventType::Redirected).is_empty());

        let mut full = game("r450-joro-full");
        let units: Vec<CardInstance> = (1..=5)
            .map(|lane| put(&mut full, "fx-1", slot(PlayerId::P2, Row::Units, lane), json!({})))
            .collect();
        one(&mut full, PlayerId::P2, &PA.joro.id);
        let full_bolt = one(&mut full, PlayerId::P1, &PA.bolt.id);
        let target = units[0].clone();
        let blocked = must(
            &full,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": full_bolt.id, "targets": [at(&target)] }),
        );
        assert!(events_of_type(&blocked.1, GameEventType::Redirected).is_empty());
        assert_eq!(
            field_of(&blocked.1, GameEventType::Damage, "targetId"),
            vec![json!(target.id)]
        );

        let mut unfit = game("r450-joro-unfit");
        let hurt = put(&mut unfit, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        live_mut(&mut unfit, &hurt).damage = 1;
        one(&mut unfit, PlayerId::P2, &PA.joro.id);
        let medic = one(&mut unfit, PlayerId::P1, &PA.medic.id);
        let healed = must(
            &unfit,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": medic.id, "targets": [at(&hurt)] }),
        );
        assert!(events_of_type(&healed.1, GameEventType::Redirected).is_empty());
    }

    #[test]
    fn r450_one_interceptor_answers_one_targeting_a_play_naming_two_units_redirects_the_first_only() {
        let mut state = game("r450-joro-first");
        let first = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let second = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let joros = hand(&mut state, PlayerId::P2, &PA.joro.id, 2);
        let (joro, spare) = (joros[0].clone(), joros[1].clone());
        let twin = one(&mut state, PlayerId::P1, &PA.twin.id);

        let (after, events) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": twin.id, "targets": [at(&first), at(&second)] }),
        );

        assert_eq!(
            of_type(&events, GameEventType::Redirected)
                .into_iter()
                .map(|event| json!([event["fromId"], event["toId"]]))
                .collect::<Vec<_>>(),
            vec![json!([first.id, joro.id])]
        );
        assert_eq!(
            after
                .players
                .p2
                .hand
                .iter()
                .filter(|card| card.def_id == PA.joro.id)
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![spare.id.clone()]
        );
        assert_eq!(
            card_at(&after, slot(PlayerId::P2, Row::Units, 2)).map(|card| card.damage),
            Some(1)
        );
    }

    #[test]
    fn r450_answers_a_prompted_pick_too() {
        let mut state = game("r450-joro-prompt");
        let unit = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let joro = one(&mut state, PlayerId::P2, &PA.joro.id);
        let chooser = one(&mut state, PlayerId::P1, &PA.chooser.id);
        let (asked, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": chooser.id }),
        );
        let (after, events) = answer(&asked, PlayerId::P1, vec![at(&unit)]);
        assert_eq!(
            field_of(&events, GameEventType::Redirected, "toId"),
            vec![json!(joro.id)]
        );
        assert_eq!(
            field_of(&events, GameEventType::Damage, "targetId"),
            vec![json!(joro.id)]
        );
        assert_eq!(
            card_at(&after, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(0)
        );
    }

    #[test]
    fn r450_r682_a_cost_already_owed_for_the_first_pick_stays_paid_when_an_interceptor_takes_the_pick() {
        let mut state = game("r450-joro-ghost");
        let ghost = put(
            &mut state,
            &PA.ghost.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let joro = one(&mut state, PlayerId::P2, &PA.joro.id);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        hand(&mut state, PlayerId::P1, "fx-5", 2);
        let (after, events) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] }),
        );
        assert_eq!(events_of_type(&events, GameEventType::Discarded).len(), 2);
        assert_eq!(
            field_of(&events, GameEventType::Redirected, "toId").first(),
            Some(&json!(joro.id))
        );
        assert_eq!(
            card_at(&after, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(0)
        );
    }

    #[test]
    fn r450_the_attack_half_calls_the_same_interception_and_reports_an_attack_moved() {
        let mut state = game("r450-joro-attack");
        let unit = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let joro = one(&mut state, PlayerId::P2, &PA.joro.id);
        let mut sink = sink_for(&mut state);
        let picks = intercept_targeting(
            &mut sink,
            InterceptArgs {
                chooser: PlayerId::P1,
                picks: vec![at(&unit)],
                targeting: None,
                accepts: None,
                what: Some(RedirectKind::Attack),
                source: None,
            },
        );
        assert_eq!(picks, vec![at(&joro)]);
        assert_eq!(
            Value::Array(of_type(&sink.events[..], GameEventType::Redirected)),
            json!([{ "type": "redirected", "what": "attack", "fromId": unit.id, "toId": joro.id, "byInstanceId": joro.id }])
        );
    }
}
