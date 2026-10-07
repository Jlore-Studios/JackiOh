//! Port of `packages/engine/test/announce.test.ts`.
//!
//! The announce window and Counter (docs/classic-sets.md B5 E1, E2; R448): §10.5 gains a step between 3
//! and 4 — the paid-for card moves to its player's resolving zone, `cardAnnounced` goes out, and a
//! window runs in which Counters answer it, traps first and then the other triggers the announce woke.
//! The first Counter cancels the play, which then never resolves and is never counted; the card goes to
//! its owner's graveyard, to exile, or to the countering player's hand as theirs (E2).
//!
//! Pinned here, on fixture cards of the shapes Classic #4, #10, #17, #72 and #87 and AI Refusal have
//! (`fixtures/playPipelineA.ts`): the timing, what a countered play does not do, the second Counter that
//! stays set, a non-trap response, a cast's announce, the card out of a discard's reach, a question in
//! the window (pause, JSON round trip, replay from the log), the steal and its hand cap, and the
//! redaction of a face-down card's announce.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::announce::open_announces;
use jackioh_engine::testkit::*;
use jackioh_engine::zones::card_at;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, setup_catalog, slot};
use crate::rules::fixtures::play_pipeline_a::{register_play_a, with_play_a, PA};

static NONCE: AtomicU32 = AtomicU32::new(0);

/// A game in p1's first main phase, both mulligans kept, with this file's fixtures registered.
fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = ready.players[player].hand.iter().map(|card| card.id.clone()).collect();
        ready = must(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in [PlayerId::P1, PlayerId::P2] {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
    }
    ready
}

/// TS `must(state, playerId, body)`: `{ ...body, playerId, nonce }` reduced, and a refusal throws.
fn must(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let n = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["playerId"] = json!(player_id);
    action["nonce"] = json!(format!("pa-an-{n}"));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn types_of(events: &[GameEvent]) -> Vec<String> {
    events.iter().map(|event| event.event_type().as_str().to_string()).collect()
}

/// TS `order.indexOf(type)`: -1 when absent.
fn index_in(order: &[String], kind: &str) -> i64 {
    order.iter().position(|seen| seen == kind).map_or(-1, |at| at as i64)
}

fn hand(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    in_hand(state, def_id, player, 1).into_iter().next().expect("no card")
}

/// `eventsOfType(events, type)`, each event as its JSON, so TS's `toEqual` literals compare key for key.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// `eventsOfType(events, type).map((event) => event[key])`.
fn pluck(events: &[GameEvent], kind: GameEventType, key: &str) -> Vec<Value> {
    of_type(events, kind).iter().map(|event| event[key].clone()).collect()
}

/// `eventsOfType(events, type)[0]?.[key]`.
fn first_field(events: &[GameEvent], kind: GameEventType, key: &str) -> Option<Value> {
    of_type(events, kind).first().map(|event| event[key].clone())
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// TS `sinkFor(state)`: a sink's events and rng (from the state's cursor), lent with the state to one
/// engine call at a time.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn card_id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

mod r448_the_announce_between_s10_5_steps_3_and_4 {
    use super::*;

    #[test]
    fn r448_announces_a_paid_for_play_before_it_moves_and_a_play_nobody_answers_goes_on_as_before() {
        let mut state = game("r448-plain");
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);

        let result = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": crier.id, "zone": { "row": "units", "lane": 2 } }),
        );
        let (after, events) = (&result.state, &result.events);

        let order = types_of(events);
        assert!(index_in(&order, "manaChanged") < index_in(&order, "cardAnnounced"));
        assert!(index_in(&order, "cardAnnounced") < index_in(&order, "cardPlayed"));
        assert_eq!(
            of_type(events, GameEventType::CardAnnounced),
            vec![json!({
                "type": "cardAnnounced",
                "player": "p1",
                "instanceId": crier.id,
                "defId": PA.crier.id,
                "cardType": "Unit",
                "costPaid": 1,
                "targets": [],
                "row": "units",
                "lane": 2,
            })]
        );
        // The Cry resolved, the card stands where it was played, and no window is left open.
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 3);
        assert_eq!(card_id_at(after, slot(PlayerId::P1, Row::Units, 2)), Some(crier.id.clone()));
        assert!(open_announces(after).is_empty());
        assert_eq!(after.announcing, None);
    }

    #[test]
    fn r448_names_the_declared_targets_a_hero_as_hero_player() {
        let mut state = game("r448-targets");
        let bolt = hand(&mut state, PlayerId::P1, &PA.bolt.id);
        let unit = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1), Default::default());

        let first = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        assert_eq!(first_field(&first.events, GameEventType::CardAnnounced, "targets"), Some(json!(["hero-p2"])));

        let mut first_state = first.state.clone();
        let second = hand(&mut first_state, PlayerId::P1, &PA.bolt.id);
        let next = must(
            &first_state,
            PlayerId::P1,
            json!({
                "type": "play",
                "instanceId": second.id,
                "targets": [{ "pick": "instance", "instanceId": unit.id }],
            }),
        );
        assert_eq!(first_field(&next.events, GameEventType::CardAnnounced, "targets"), Some(json!([unit.id])));
        assert_eq!(first_field(&next.events, GameEventType::CardAnnounced, "cardType"), Some(json!("Spell")));
    }
}

mod r448_a_countered_play_never_resolves_and_counts_for_nothing {
    use super::*;

    #[test]
    fn r448_a_counter_trap_cancels_the_play_no_cry_no_card_played_no_card_resolved_mana_spent_nothing_counted() {
        let mut state = game("r448-counter");
        let trap = put(&mut state, &PA.counter_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);
        let mana = state.players.p1.mana.current;

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(
            of_type(events, GameEventType::Countered),
            vec![json!({
                "type": "countered",
                "player": "p1",
                "instanceId": crier.id,
                "defId": PA.crier.id,
                "byInstanceId": trap.id,
                "to": "graveyard",
            })]
        );
        for absent in [GameEventType::CardPlayed, GameEventType::Summoned, GameEventType::CardResolved] {
            assert_eq!(of_type(events, absent), Vec::<Value>::new());
        }
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH);
        assert!(ids(&after.players.p1.graveyard).contains(&crier.id));
        assert_eq!(after.players.p1.resolving, Vec::<CardInstance>::new());
        assert_eq!(after.players.p1.mana.current, mana - 1);
        // §10.5 step 4 never ran, so no count moved: the turn log, the game counter, E4's records.
        assert_eq!(after.players.p1.turn_log.cards_played, 0);
        assert_eq!(after.players.p1.turn_log.played_ids, Vec::<String>::new());
        assert_eq!(after.players.p1.turn_log.played_by_type, None);
        assert_eq!(after.counters.played, 0);
        assert_eq!(after.players.p1.game_log, None);
        // The trap fired and was consumed (§5.1).
        assert!(ids(&after.players.p2.graveyard).contains(&trap.id));
    }

    #[test]
    fn r448_the_first_counter_cancels_the_play_and_a_second_finds_no_card_and_stays_set() {
        let mut state = game("r448-second");
        let first = put(&mut state, &PA.counter_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let second = put(&mut state, &PA.counter_trap2.id, slot(PlayerId::P2, Row::Backrow, 2), Default::default());
        let ping = hand(&mut state, PlayerId::P1, &PA.ping.id);

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(pluck(events, GameEventType::TrapFired, "instanceId"), vec![json!(first.id)]);
        assert_eq!(of_type(events, GameEventType::Countered).len(), 1);
        let standing = card_at(after, slot(PlayerId::P2, Row::Backrow, 2));
        assert_eq!(standing.map(|card| card.id.clone()), Some(second.id.clone()));
        assert_ne!(standing.and_then(|card| card.face_up), Some(true));
    }

    #[test]
    fn r448_a_countered_spells_echo_repeats_never_happen() {
        let mut state = game("r448-echo");
        put(&mut state, &PA.counter_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let target = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1), Default::default());
        let echo = hand(&mut state, PlayerId::P1, &PA.echo_bolt.id);

        let result = must(
            &state,
            PlayerId::P1,
            json!({
                "type": "play",
                "instanceId": echo.id,
                "targets": [{ "pick": "instance", "instanceId": target.id }],
            }),
        );
        let (after, events) = (&result.state, &result.events);

        assert_eq!(of_type(events, GameEventType::Damage), Vec::<Value>::new());
        assert_eq!(after.echo_queue, Vec::<EchoItem>::new());
        assert_eq!(after.pending, None);
    }

    #[test]
    fn r448_a_counter_that_is_not_a_trap_answers_in_the_window_after_the_traps_classic_87s_shape() {
        let mut state = game("r448-chalice");
        put(&mut state, &PA.chalice.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(pluck(events, GameEventType::Countered, "instanceId"), vec![json!(crier.id)]);
        assert_eq!(of_type(events, GameEventType::CardPlayed), Vec::<Value>::new());
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH);

        // A play that paid 0 is not the chalice's: it resolves.
        let mut after = after.clone();
        let field = hand(&mut after, PlayerId::P1, &PA.field.id);
        let next = must(&after, PlayerId::P1, json!({ "type": "play", "instanceId": field.id }));
        assert_eq!(of_type(&next.events, GameEventType::Countered), Vec::<Value>::new());
        assert_eq!(pluck(&next.events, GameEventType::CardPlayed, "instanceId"), vec![json!(field.id)]);
    }

    #[test]
    fn r448_r55_a_countered_card_goes_to_exile_when_the_counter_says_so_classic_10_and_exile_counts_it() {
        let mut state = game("r448-exile");
        put(&mut state, &PA.exile_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let ping = hand(&mut state, PlayerId::P1, &PA.ping.id);

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(first_field(events, GameEventType::Countered, "to"), Some(json!("exile")));
        assert_eq!(ids(&after.players.p1.exile), vec![ping.id.clone()]);
        assert_eq!(after.counters.exiled, 1);
        assert_eq!(pluck(events, GameEventType::Exiled, "instanceId"), vec![json!(ping.id)]);
    }

    #[test]
    fn r448_a_response_that_counters_a_spell_by_its_declared_targets_reads_them_off_the_announce_ai_refusal() {
        let mut state = game("r448-refusal");
        put(&mut state, &PA.refusal.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let mine = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1), Default::default());
        let bolt = hand(&mut state, PlayerId::P1, &PA.bolt.id);
        let other = hand(&mut state, PlayerId::P1, &PA.bolt.id);

        // At the hero: not a Spell that targets one of p2's units, so it resolves.
        let first = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        assert_eq!(of_type(&first.events, GameEventType::Countered), Vec::<Value>::new());
        let second = must(
            &first.state,
            PlayerId::P1,
            json!({
                "type": "play",
                "instanceId": other.id,
                "targets": [{ "pick": "instance", "instanceId": mine.id }],
            }),
        );
        assert_eq!(pluck(&second.events, GameEventType::Countered, "instanceId"), vec![json!(other.id)]);
        assert_eq!(
            second.state.players.p2.units[0].as_ref().and_then(|pile| pile.first()).map_or(0, |card| card.damage),
            0
        );
    }
}

mod r448_where_the_card_waits_the_resolving_zone_out_of_every_hands_reach {
    use super::*;

    #[test]
    fn r448_a_discard_of_the_players_hand_in_the_window_does_not_reach_the_card_being_played() {
        let mut state = game("r448-shred");
        put(&mut state, &PA.shredder.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);
        let others: Vec<String> =
            state.players.p1.hand.iter().filter(|card| card.id != crier.id).map(|card| card.id.clone()).collect();

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id }));
        let (after, events) = (&result.state, &result.events);

        let mut discarded: Vec<String> = pluck(events, GameEventType::Discarded, "instanceId")
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect();
        assert!(!discarded.contains(&crier.id));
        discarded.sort();
        let mut expected = others.clone();
        expected.sort();
        assert_eq!(discarded, expected);
        assert_eq!(after.players.p1.hand, Vec::<CardInstance>::new());
        assert_eq!(card_id_at(after, slot(PlayerId::P1, Row::Units, 1)), Some(crier.id.clone()));
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 3);
    }
}

mod r448_r70_a_cast_is_announced_and_can_be_countered {
    use super::*;

    #[test]
    fn r448_a_card_an_effect_casts_is_announced_from_the_resolving_zone_and_a_counter_cancels_it() {
        let mut state = game("r448-cast");
        let trap = put(&mut state, &PA.counter_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let ping = new_instance(&mut state, &PA.ping.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &ping, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        let announced: Vec<Value> = of_type(&bench.events, GameEventType::CardAnnounced)
            .iter()
            .map(|event| json!([event["instanceId"], event["costPaid"]]))
            .collect();
        assert_eq!(announced, vec![json!([ping.id, 0])]);
        assert_eq!(pluck(&bench.events, GameEventType::Countered, "byInstanceId"), vec![json!(trap.id)]);
        assert_eq!(of_type(&bench.events, GameEventType::CardPlayed), Vec::<Value>::new());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert!(ids(&state.players.p1.graveyard).contains(&ping.id));
        assert_eq!(state.players.p1.turn_log.cards_played, 0);
        assert_eq!(state.announcing, None);
    }

    #[test]
    fn r448_an_uncountered_cast_is_announced_and_then_played_as_before() {
        let mut state = game("r448-cast-plain");
        let ping = new_instance(&mut state, &PA.ping.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &ping, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        let order: Vec<String> = types_of(&bench.events)
            .into_iter()
            .filter(|kind| kind == "cardAnnounced" || kind == "cardPlayed")
            .collect();
        assert_eq!(order, vec!["cardAnnounced".to_string(), "cardPlayed".to_string()]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        assert_eq!(state.players.p1.turn_log.cards_played, 1);
    }
}

mod e2_a_counter_that_steals_the_card_goes_to_the_thiefs_hand_and_the_thief_owns_it {
    use super::*;

    #[test]
    fn r448_the_stolen_card_is_the_thiefs_own_in_their_hand_classic_72_radiant() {
        let mut state = game("r448-steal");
        put(&mut state, &PA.steal_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(first_field(events, GameEventType::Countered, "to"), Some(json!("hand")));
        assert_eq!(
            of_type(events, GameEventType::Stolen),
            vec![json!({
                "type": "stolen",
                "instanceId": crier.id,
                "defId": PA.crier.id,
                "from": "p1",
                "to": "p2",
                "zone": "resolving",
                // R466: announced face up, so both seats read the card it was.
                "readableFrom": ["p1", "p2"],
            })]
        );
        let taken = after.players.p2.hand.iter().find(|card| card.id == crier.id);
        assert_eq!(taken.map(|card| card.owner), Some(PlayerId::P2));
        assert_eq!(taken.map(|card| card.controller), Some(PlayerId::P2));
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH);
        // R97: the thief reads its own hand card. (What the victim reads of a card taken off the stack
        // is the `stolen` redaction's own rule, which the prompts workstream settles.)
        assert_eq!(
            first_field(&view_for(after, PlayerId::P2).events, GameEventType::Stolen, "instanceId"),
            Some(json!(crier.id))
        );
    }

    #[test]
    fn r448_a_thief_with_a_full_hand_burns_the_stolen_card_into_their_own_graveyard_s2_4() {
        let mut state = game("r448-steal-burn");
        put(&mut state, &PA.steal_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        while (state.players.p2.hand.len() as i32) < HAND_CAP {
            in_hand(&mut state, "fx-5", PlayerId::P2, 1);
        }
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);

        let result = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id }));
        let (after, events) = (&result.state, &result.events);

        assert_eq!(first_field(events, GameEventType::Countered, "to"), Some(json!("graveyard")));
        assert_eq!(pluck(events, GameEventType::Burned, "instanceId"), vec![json!(crier.id)]);
        assert_eq!(
            after.players.p2.graveyard.iter().find(|card| card.id == crier.id).map(|card| card.owner),
            Some(PlayerId::P2)
        );
        assert!(!after.players.p1.graveyard.iter().any(|card| card.id == crier.id));
    }
}

mod r448_a_question_in_the_window_classic_4_palantirs_shape {
    use super::*;

    /// TS `paused(seed)`: `{ state, bolt, palantir }`.
    fn paused(seed: &str) -> (GameState, CardInstance, CardInstance) {
        let mut state = game(seed);
        let palantir = put(&mut state, &PA.palantir.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let bolt = hand(&mut state, PlayerId::P1, &PA.bolt.id);
        let waiting = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p2" }] }),
        )
        .state;
        (waiting, bolt, palantir)
    }

    fn choice_id(state: &GameState) -> String {
        state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default()
    }

    #[test]
    fn r448_pauses_the_play_with_its_card_in_the_resolving_zone_and_the_rest_of_the_play_owed() {
        let (state, bolt, _) = paused("r448-pause");
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert_eq!(ids(&state.players.p1.resolving), vec![bolt.id.clone()]);
        assert_eq!(
            serde_json::to_value(&state.announcing).expect("the announces serialise"),
            json!([{ "instanceId": bolt.id, "player": "p1" }])
        );
        assert!(state.work.iter().any(|item| item.resume.hook == "play" && item.resume.step == "announce"));
        // p1 may only wait; p2 answers.
        let kinds: Vec<ActionType> = legal_actions(&state, PlayerId::P1).iter().map(|action| action.action_type()).collect();
        assert_eq!(kinds, vec![ActionType::Concede]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r448_the_answer_steal_sacrifices_the_field_spell_and_takes_the_spell_which_never_resolves() {
        let (state, bolt, palantir) = paused("r448-steal-answer");
        let result = must(
            &state,
            PlayerId::P2,
            json!({ "type": "answer", "choiceId": choice_id(&state), "selection": [{ "pick": "mode", "option": "steal" }] }),
        );
        let (after, events) = (&result.state, &result.events);
        assert_eq!(after.pending, None);
        assert!(ids(&after.players.p2.graveyard).contains(&palantir.id));
        assert_eq!(
            after.players.p2.hand.iter().find(|card| card.id == bolt.id).map(|card| card.owner),
            Some(PlayerId::P2)
        );
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(of_type(events, GameEventType::CardPlayed), Vec::<Value>::new());
        assert_eq!(after.work, Vec::<WorkItem>::new());
        assert_eq!(after.announcing, None);
    }

    #[test]
    fn r448_the_answer_pass_lets_the_play_go_on_to_step_4_and_resolve() {
        let (state, bolt, _) = paused("r448-pass");
        let result = must(
            &state,
            PlayerId::P2,
            json!({ "type": "answer", "choiceId": choice_id(&state), "selection": [{ "pick": "mode", "option": "pass" }] }),
        );
        let (after, events) = (&result.state, &result.events);
        assert_eq!(pluck(events, GameEventType::CardPlayed, "instanceId"), vec![json!(bolt.id)]);
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 2);
        assert!(ids(&after.players.p1.graveyard).contains(&bolt.id));
        assert_eq!(after.players.p1.turn_log.cards_played, 1);
    }

    #[test]
    fn r448_the_paused_window_survives_json_parse_json_stringify_state_and_answers_the_same_way() {
        let (state, _, _) = paused("r448-json");
        let round: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("a state serialises")).expect("and parses");
        assert_eq!(round, state);
        assert_eq!(hash_state(&round), hash_state(&state));
        let answer: Action = json_as(json!({
            "type": "answer",
            "choiceId": choice_id(&state),
            "selection": [{ "pick": "mode", "option": "steal" }],
            "playerId": "p2",
            "nonce": "json-live",
        }));
        let live = reduce(&state, &answer);
        let revived = reduce(&round, &answer);
        assert_eq!(revived.error, None);
        assert_eq!(hash_state(&revived.state), hash_state(&live.state));
        assert_eq!(revived.events, live.events);
    }

    #[test]
    fn r448_a_game_with_a_question_in_a_window_replays_from_its_log_to_the_same_hash() {
        let seed = "r448-fold";
        let deck_one: Vec<String> = std::iter::once(PA.q_bolt.id.clone()).chain(vanilla_deck(DECK_SIZE - 1, 1)).collect();
        let deck_two: Vec<String> =
            std::iter::once(PA.q_palantir.id.clone()).chain(vanilla_deck(DECK_SIZE - 1, 21)).collect();
        setup_catalog();
        register_play_a();
        let start = create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: (deck_one.clone(), deck_two.clone()),
            ..Default::default()
        });
        let mut state = begin_game(&start).state;
        let mut log: Vec<Action> = Vec::new();
        let step = |state: &mut GameState, log: &mut Vec<Action>, player_id: PlayerId, body: Value| {
            let kind = body["type"].as_str().unwrap_or_default().to_string();
            let mut action = body;
            action["playerId"] = json!(player_id);
            action["nonce"] = json!(format!("fold-{}", log.len()));
            let action: Action = json_as(action);
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{kind}: {error}");
            }
            log.push(action);
            *state = result.state;
        };
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            step(&mut state, &mut log, player, json!({ "type": "mulligan", "keep": keep }));
        }
        // p1's first turn passes; p2 sets down its Palantir; p1 plays the Bolt into its question.
        step(&mut state, &mut log, PlayerId::P1, json!({ "type": "endTurn" }));
        let palantir = state.players.p2.hand.iter().find(|card| card.def_id == PA.q_palantir.id).cloned();
        let palantir_id = palantir.map(|card| card.id).unwrap_or_default();
        step(&mut state, &mut log, PlayerId::P2, json!({ "type": "play", "instanceId": palantir_id }));
        step(&mut state, &mut log, PlayerId::P2, json!({ "type": "endTurn" }));
        let bolt = state.players.p1.hand.iter().find(|card| card.def_id == PA.q_bolt.id).cloned();
        let bolt_id = bolt.as_ref().map(|card| card.id.clone()).unwrap_or_default();
        step(
            &mut state,
            &mut log,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt_id, "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        let paused_hash = hash_state(&state);
        let choice = choice_id(&state);
        step(
            &mut state,
            &mut log,
            PlayerId::P2,
            json!({ "type": "answer", "choiceId": choice, "selection": [{ "pick": "mode", "option": "steal" }] }),
        );
        assert!(state.players.p2.hand.iter().any(|card| Some(&card.id) == bolt.as_ref().map(|held| &held.id)));

        let replayed = fold(&json_as(json!({ "seed": seed, "decks": [deck_one, deck_two], "log": log })));
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
        let partial = fold(&json_as(json!({ "seed": seed, "decks": [deck_one, deck_two], "log": &log[..log.len() - 1] })));
        assert_eq!(hash_state(&partial.state), paused_hash);
    }
}

mod r448_r97_r227_a_card_being_set_face_down_is_its_players_alone_while_it_waits {
    use super::*;

    /// TS `setting(seed, trapDef)`: `{ state, trap }`.
    fn setting(seed: &str, trap_def: &str) -> (GameState, CardInstance) {
        let mut state = game(seed);
        put(&mut state, &PA.watcher.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let trap = hand(&mut state, PlayerId::P1, trap_def);
        let waiting = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": trap.id, "zone": { "row": "backrow", "lane": 3 } }),
        )
        .state;
        (waiting, trap)
    }

    #[test]
    fn r448_the_other_player_reads_only_the_zone_of_the_announce_a_card_back_in_the_resolving_zone_and_no_type() {
        let (state, trap) = setting("r448-hidden", &PA.hidden_field_trap.id);
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));

        let theirs = view_for(&state, PlayerId::P2);
        assert_eq!(
            of_type(&theirs.events, GameEventType::CardAnnounced),
            vec![json!({
                "type": "cardAnnounced",
                "player": "p1",
                "instanceId": HIDDEN_ID,
                "defId": HIDDEN_ID,
                "cardType": "Trap",
                "costPaid": 0,
                "targets": [],
                "row": "backrow",
                "lane": 3,
                "faceDown": true,
            })]
        );
        assert_eq!(
            serde_json::to_value(&theirs.opponent.resolving).expect("a resolving view serialises"),
            json!([{ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "radiant": false, "cost": -1 }])
        );
        let shown = serde_json::to_string(&theirs).expect("a view serialises");
        assert!(!shown.contains(&trap.id));
        assert!(!shown.contains(&PA.hidden_field_trap.id));

        let own = view_for(&state, PlayerId::P1);
        assert_eq!(first_field(&own.events, GameEventType::CardAnnounced, "defId"), Some(json!(PA.hidden_field_trap.id)));
        assert_eq!(first_field(&own.events, GameEventType::CardAnnounced, "cardType"), Some(json!("Field Trap")));
        let resolving: Vec<String> = own.you.resolving.iter().map(|card| card.instance_id.clone()).collect();
        assert_eq!(resolving, vec![trap.id.clone()]);
    }

    #[test]
    fn r448_once_set_the_trap_is_face_down_under_a_fresh_id_and_the_announce_stays_unread() {
        let (state, trap) = setting("r448-hidden-set", &PA.hidden_trap.id);
        let choice = state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
        let after = must(
            &state,
            PlayerId::P2,
            json!({ "type": "answer", "choiceId": choice, "selection": [{ "pick": "mode", "option": "noted" }] }),
        )
        .state;
        let set = card_at(&after, slot(PlayerId::P1, Row::Backrow, 3));
        assert_eq!(set.map(|card| card.def_id.clone()), Some(PA.hidden_trap.id.clone()));
        assert_ne!(set.map(|card| card.id.clone()), Some(trap.id.clone()));
        let theirs = view_for(&after, PlayerId::P2);
        assert_eq!(first_field(&theirs.events, GameEventType::CardAnnounced, "instanceId"), Some(json!(HIDDEN_ID)));
        assert!(!serde_json::to_string(&theirs.events).expect("events serialise").contains(&PA.hidden_trap.id));
        assert_eq!(theirs.opponent.resolving, Vec::<CardView>::new());
    }

    #[test]
    fn r448_a_face_up_play_waiting_in_the_window_is_public_to_both_players() {
        let mut state = game("r448-public");
        put(&mut state, &PA.watcher.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let crier = hand(&mut state, PlayerId::P1, &PA.crier.id);
        let waiting = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id })).state;
        let theirs = view_for(&waiting, PlayerId::P2);
        let resolving: Vec<String> = theirs.opponent.resolving.iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(resolving, vec![PA.crier.id.clone()]);
        assert_eq!(first_field(&theirs.events, GameEventType::CardAnnounced, "defId"), Some(json!(PA.crier.id)));
    }
}
