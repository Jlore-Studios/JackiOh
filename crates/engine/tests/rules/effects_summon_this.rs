//! Deck and graveyard triggers, and "summon this from your hand or deck" (docs/classic-sets.md B5 E26):
//! a card in a library answers its `deckTriggers` (Classic+ #37 Wardrum), a card in a graveyard its
//! `graveyardTriggers` (Classic #47 Recurring Felinor), and a hand or deck trigger may summon its own
//! card — no Cry, summoning sick, the leftmost open zone (Classic #66 EU Striker). R464 is their place
//! in R68's order: after the hand's triggers and before the graveyard's, in the order the instances
//! were created and never by library position, which is hidden (§9.1); and a library card's queue
//! entry, like a hand card's, takes no number from the counter both seats read (R177).
//!
//! Port of `packages/engine/test/effects-summonThis.test.ts`.

use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::combat as combat_fx;
use super::fixtures::harness::{events_of_type, in_hand, put, slot};
use super::fixtures::prompt_harness::{
    act, answer_keys, board, expect_replays, hand_card, open_as, replayable, round_trip,
};
use super::fixtures::prompts as prompts_fx;

fn played(state: &GameState, player: PlayerId, def_id: &str) -> GameEvent {
    json_as(json!({
        "type": "cardPlayed",
        "player": player,
        "instanceId": format!("c{}", state.next_id + 1000),
        "defId": def_id,
        "costPaid": 0,
    }))
}

fn resolved(state: &GameState, player: PlayerId, def_id: &str) -> GameEvent {
    json_as(json!({
        "type": "cardResolved",
        "player": player,
        "instanceId": format!("c{}", state.next_id + 1000),
        "defId": def_id,
        "costPaid": 0,
        "permanent": false,
    }))
}

fn grave_card(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does.
fn with_sink<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> R {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

/// TS `{ type: "play", playerId, instanceId }`, as `act` takes it.
fn play_body(player: PlayerId, instance_id: &str) -> Value {
    json!({ "type": "play", "playerId": player, "instanceId": instance_id })
}

/// `state.applied.at(-1)?.events ?? []`.
fn last_applied_events(state: &GameState) -> Vec<GameEvent> {
    state.applied.last().map(|applied| applied.events.clone()).unwrap_or_default()
}

/// The top card of a unit zone, by its index in the row (TS `state.players.p1.units[i]?.[0]`).
fn unit_top(state: &GameState, player: PlayerId, index: usize) -> Option<CardInstance> {
    state.players[player].units.get(index).and_then(|pile| pile.as_ref()).and_then(|pile| pile.first()).cloned()
}

/// TS `list.indexOf(x)`: the first position, or -1.
fn index_of(list: &[String], id: &str) -> i64 {
    list.iter().position(|entry| entry == id).map_or(-1, |at| at as i64)
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn def_ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn summoned_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn played_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

mod e26_deck_and_graveyard_triggers_in_r68_order_r464 {
    use super::*;

    #[test]
    fn r464_deck_triggers_come_after_the_hands_and_before_the_graveyards_in_creation_order_never_library_order() {
        let mut state = board("r464-order");
        let hand = in_hand(&mut state, &prompts_fx::striker().id, PlayerId::P1, 1).remove(0);
        // Created first, then second — and laid in the library the other way round.
        let older = new_instance(
            &mut state,
            &prompts_fx::deck_watcher().id,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        let newer = new_instance(&mut state, &prompts_fx::wardrum().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        let filler = new_instance(&mut state, &combat_fx::plain().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        state.players.p1.library = vec![newer.clone(), filler.clone(), older.clone()];
        let grave = grave_card(&mut state, PlayerId::P1, &prompts_fx::grave_watcher().id);
        let enemy_deck = new_instance(
            &mut state,
            &prompts_fx::deck_watcher().id,
            PlayerId::P2,
            Zone::Library { player: PlayerId::P2 },
        );
        state.players.p2.library = vec![enemy_deck.clone()];

        // The registry: a library card is a holder only when it declares deck triggers.
        let order: Vec<String> = cards_in_trigger_order(&state).iter().map(|holder| holder.card.id.clone()).collect();
        assert!(index_of(&order, &hand.id) < index_of(&order, &older.id));
        assert!(index_of(&order, &older.id) < index_of(&order, &newer.id));
        assert!(index_of(&order, &newer.id) < index_of(&order, &grave.id));
        assert!(!order.contains(&filler.id));
        // Active side first: the other player's deck card comes after all of p1's.
        assert!(index_of(&order, &enemy_deck.id) > index_of(&order, &grave.id));
        let holders = trigger_holders_for_event(&state, GameEventType::CardResolved);
        assert_eq!(
            holders.iter().map(|holder| &holder.zone).collect::<Vec<_>>(),
            vec![
                &TriggerZone::Hand,
                &TriggerZone::Library,
                &TriggerZone::Library,
                &TriggerZone::Graveyard,
                &TriggerZone::Library,
            ]
        );

        // The queue follows it.
        let event = resolved(&state, PlayerId::P1, &prompts_fx::grunt().id);
        with_sink(&mut state, |sink| {
            dispatch_event(sink, &event);
        });
        assert_eq!(
            state.trigger_queue.iter().map(|entry| entry.instance_id.clone()).collect::<Vec<_>>(),
            vec![hand.id, older.id, newer.id, grave.id, enemy_deck.id]
        );
    }

    #[test]
    fn r464_r177_a_library_cards_queue_entry_takes_no_number_from_the_counter_both_seats_read() {
        let mut state = board("r464-hidden");
        let hidden = new_instance(
            &mut state,
            &prompts_fx::deck_watcher().id,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        state.players.p1.library = vec![hidden.clone()];
        let grave = grave_card(&mut state, PlayerId::P1, &prompts_fx::grave_watcher().id);
        let before = state.next_seq;
        let event = played(&state, PlayerId::P2, &prompts_fx::grunt().id);
        with_sink(&mut state, |sink| {
            dispatch_event(sink, &event);
        });
        let deck_entry = state.trigger_queue.first().cloned();
        let grave_entry = state.trigger_queue.get(1).cloned();
        assert_eq!(deck_entry.as_ref().map(|entry| entry.instance_id.clone()), Some(hidden.id.clone()));
        assert_eq!(deck_entry.as_ref().map(|entry| entry.id.starts_with('h')), Some(true));
        // The public graveyard card's entry is numbered; the hidden one's took none.
        assert_eq!(grave_entry.as_ref().map(|entry| entry.instance_id.clone()), Some(grave.id.clone()));
        assert_eq!(state.next_seq, before + 1);
    }
}

mod e26_summon_this_from_your_hand_or_deck {
    use super::*;

    #[test]
    fn e26_a_deck_trigger_summons_its_card_no_cry_summoning_sick_the_leftmost_open_zone() {
        let mut state = board("deck-summon");
        put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1));
        let drum = new_instance(&mut state, &prompts_fx::wardrum().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        let first = new_instance(&mut state, &combat_fx::plain().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        state.players.p1.library = vec![first, drum.clone()];
        let card = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P1, 1).remove(0);
        state = act(&state, json_as(play_body(PlayerId::P1, &card.id)), None);
        let found = unit_top(&state, PlayerId::P1, 1);
        assert_eq!(found.as_ref().map(|unit| unit.id.clone()), Some(drum.id.clone()));
        assert_eq!(found.as_ref().and_then(|unit| unit.summoned_turn), Some(state.turn));
        assert_eq!(state.players.p1.library.len(), 1);
        let events = last_applied_events(&state);
        assert_eq!(summoned_ids(&events), vec![drum.id.clone()]);
        // A summon, not a play: no cardPlayed of its own.
        assert_eq!(played_ids(&events), vec![card.id.clone()]);
        // The deck card is public once it is on the field, and not before.
        let seen = view_for(&state, PlayerId::P2).events;
        let first_summoned = seen.iter().find_map(|event| match event {
            GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
            _ => None,
        });
        assert_eq!(first_summoned, Some(prompts_fx::wardrum().id));
    }

    #[test]
    fn e26_a_hand_trigger_summons_its_card_after_a_unit_is_played_never_on_its_own_play_and_fires_no_cry() {
        let mut state = board("hand-summon");
        let eu = in_hand(&mut state, &prompts_fx::striker().id, PlayerId::P1, 1).remove(0);
        let body = in_hand(&mut state, &prompts_fx::grunt().id, PlayerId::P1, 1).remove(0);
        let zap = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P1, 1).remove(0);
        // A Spell sets nothing off.
        state = act(&state, json_as(play_body(PlayerId::P1, &zap.id)), None);
        assert!(ids_of(&state.players.p1.hand).contains(&eu.id));
        state = act(
            &state,
            json_as(json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": body.id,
                "zone": { "row": "units", "lane": 1 },
            })),
            None,
        );
        assert_eq!(unit_top(&state, PlayerId::P1, 1).map(|unit| unit.id), Some(eu.id.clone()));
        // Its Cry (9 to the enemy hero) did not fire: only the spark's 1 landed.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
    }

    #[test]
    fn e26_with_no_open_zone_the_card_stays_where_it_is() {
        let mut state = board("deck-full");
        for lane in 1..=5 {
            put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, lane));
        }
        let drum = new_instance(&mut state, &prompts_fx::wardrum().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        state.players.p1.library = vec![drum.clone()];
        let card = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P1, 1).remove(0);
        state = act(&state, json_as(play_body(PlayerId::P1, &card.id)), None);
        assert_eq!(ids_of(&state.players.p1.library), vec![drum.id.clone()]);
        assert!(events_of_type(&last_applied_events(&state), GameEventType::Summoned).is_empty());
    }

    #[test]
    fn e26_r113_a_deck_trigger_that_asks_parks_its_tail_under_its_id_and_the_answer_finishes_it() {
        let mut state = board("deck-asks");
        let asker = new_instance(&mut state, &prompts_fx::deck_asker().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        state.players.p1.library = vec![asker.clone()];
        let card = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P1, 1).remove(0);
        state = act(&state, json_as(play_body(PlayerId::P1, &card.id)), None);
        open_as(&state, PromptKind::Mode, PlayerId::P1);
        assert_eq!(
            to_json(view_for(&state, PlayerId::P2))["pending"],
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        // The Spell resolved (its 1 landed) before the trigger answered it; the tail's 1 waits.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        let mut copy = round_trip(&state);
        answer_keys(&mut state, &["mode:come"]);
        answer_keys(&mut copy, &["mode:come"]);
        assert_eq!(hash_state(&copy), hash_state(&state));
        assert_eq!(unit_top(&state, PlayerId::P1, 0).map(|unit| unit.id), Some(asker.id.clone()));
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn e26_a_wardrum_game_replays_from_its_log() {
        let zap = prompts_fx::quickdraw_of(&prompts_fx::spark()).id;
        let wardrum_id = prompts_fx::wardrum().id;
        let replay = replayable("wardrum-replay", &[zap.clone(), wardrum_id.clone()], &[]);
        let (dealt, mut log, decks) = (replay.state, replay.log, replay.decks);
        let mut state = dealt;
        let drum = state
            .players
            .p1
            .hand
            .iter()
            .chain(state.players.p1.library.iter())
            .find(|card| card.def_id == wardrum_id)
            .cloned();
        let zap_id = hand_card(&state, PlayerId::P1, &zap).id.clone();
        state = act(&state, json_as(play_body(PlayerId::P1, &zap_id)), Some(&mut log));
        // From the hand or the deck, wherever the deal left it, it answered the Spell.
        assert_eq!(unit_top(&state, PlayerId::P1, 0).map(|unit| unit.id), drum.map(|card| card.id));
        expect_replays("wardrum-replay", &decks, &log, &state);
    }
}

mod e26_graveyard_triggers_classic_47 {
    use super::*;

    /// TS `trapFires`: p1's Trap fires on p2's play while `recurring` waits in p1's graveyard; the
    /// card comes back as it stands afterwards (in the hand when it returned, else as it was).
    fn trap_fires(seed: &str, radiant: bool) -> (GameState, CardInstance) {
        let mut state = board(seed);
        let mut card = grave_card(&mut state, PlayerId::P1, &prompts_fx::recurring().id);
        card.radiant = radiant;
        find_instance_mut(&mut state, &card.id).expect("in the graveyard").radiant = radiant;
        put(&mut state, &prompts_fx::snare().id, slot(PlayerId::P1, Row::Backrow, 1));
        state.active = PlayerId::P2;
        let zap = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P2, 1).remove(0);
        let state = act(&state, json_as(play_body(PlayerId::P2, &zap.id)), None);
        let moved = state.players.p1.hand.iter().find(|held| held.id == card.id).cloned().unwrap_or(card);
        (state, moved)
    }

    #[test]
    fn e26_when_one_of_your_traps_fires_the_card_in_your_graveyard_returns_to_your_hand() {
        let (state, card) = trap_fires("recur", false);
        // (p2's play ended their turn by itself, R82, so p1 has drawn for the new turn as well.)
        assert!(ids_of(&state.players.p1.hand).contains(&card.id));
        assert_eq!(card.zone.z(), ZoneName::Hand);
        assert_eq!(def_ids_of(&state.players.p1.graveyard), vec![prompts_fx::snare().id]);
        assert_eq!(card.cost_override, None);
    }

    #[test]
    fn e26_radiant_it_returns_costing_0() {
        let (state, card) = trap_fires("recur-radiant", true);
        assert!(ids_of(&state.players.p1.hand).contains(&card.id));
        assert_eq!(card.cost_override, Some(0));
    }

    #[test]
    fn e26_the_other_players_trap_does_not_return_it_and_a_card_elsewhere_has_no_graveyard_trigger() {
        let mut state = board("recur-theirs");
        let card = grave_card(&mut state, PlayerId::P1, &prompts_fx::recurring().id);
        put(&mut state, &prompts_fx::snare().id, slot(PlayerId::P2, Row::Backrow, 1));
        let zap = in_hand(&mut state, &prompts_fx::spark().id, PlayerId::P1, 1).remove(0);
        state = act(&state, json_as(play_body(PlayerId::P1, &zap.id)), None);
        assert_eq!(events_of_type(&last_applied_events(&state), GameEventType::TrapFired).len(), 1);
        assert!(ids_of(&state.players.p1.graveyard).contains(&card.id));

        // In a hand the graveyard trigger is not registered.
        let mut held = board("recur-hand");
        let in_hand_card = in_hand(&mut held, &prompts_fx::recurring().id, PlayerId::P1, 1).remove(0);
        assert!(
            !trigger_holders_for_event(&held, GameEventType::TrapFired)
                .iter()
                .any(|holder| holder.card.id == in_hand_card.id)
        );
    }
}

mod e26_a_fused_card_keeps_its_deck_and_graveyard_triggers_r102 {
    use super::*;

    #[test]
    fn r102_a_fusions_script_carries_each_ingredients_deck_and_graveyard_triggers_namespaced() {
        let mut state = board("fuse-triggers");
        let a = in_hand(&mut state, &prompts_fx::wardrum().id, PlayerId::P1, 1).remove(0);
        let b = in_hand(&mut state, &prompts_fx::recurring().id, PlayerId::P1, 1).remove(0);
        let fused = with_sink(&mut state, |sink| {
            jackioh_engine::subsystems::fuse::fuse(sink, json_as(json!({ "ingredients": [a, b], "toHand": "p1" })))
        });
        let fused_def_id = fused.map(|card| card.def_id).unwrap_or_default();
        let script: Script = scripts_for(&state, &fused_def_id).base;
        assert_eq!(
            script.deck_triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
            vec![format!("{}:wardrum-arrive", prompts_fx::wardrum().id)]
        );
        assert_eq!(
            script.graveyard_triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
            vec![format!("{}:recur", prompts_fx::recurring().id)]
        );
    }
}
