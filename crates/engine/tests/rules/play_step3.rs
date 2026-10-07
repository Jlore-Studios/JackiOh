//! Port of `packages/engine/test/play-step3.test.ts`.
//!
//! §10.5 step 3's two v0.2.0 rules (R449): a play replaced by another card (Classic #23 Devil's Pact)
//! and plays made Radiant by tag (Classic+ #68 Organic Produce: R213's rule by tag, read at step 1 as
//! R214 reads Gifted Program).
//!
//! R449: a live `replacePlays` modifier replaces each card its player plays — a cast included (R70) —
//! by a new instance of the named definition (Radiant per the modifier). The old card ceases to exist,
//! the price paid was the old card's, and the new card makes its own choices as a cast does, before
//! the announce, so a Counter reads them; it takes its zone at step 4 by R64.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};
use crate::rules::fixtures::play_pipeline_a::{PA, with_play_a};

/// TS's module `let nonce`: unique across the tests, which run on parallel threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in PLAYER_IDS {
        let keep: Vec<String> = ready.players[player].hand.iter().map(|card| card.id.clone()).collect();
        ready = must(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in PLAYER_IDS {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
    }
    ready
}

/// TS `must`: `body` is the TS `ActionBody` literal; the player and a fresh nonce are added.
fn must(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut body = body;
    body["playerId"] = json!(player_id);
    body["nonce"] = json!(format!("pa-s3-{nonce}"));
    let result = reduce(state, &json_as::<Action>(body));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// TS `one`: a card put in `player`'s hand with the face asked for; answers its copy as it stands.
fn one(state: &mut GameState, player: PlayerId, def_id: &str, radiant: bool) -> CardInstance {
    let card = in_hand(state, def_id, player, 1).into_iter().next().expect("no card");
    find_instance_mut(state, &card.id).expect("the card is in the hand").radiant = radiant;
    find_instance(state, &card.id).cloned().expect("the card is in the hand")
}

const ENEMY_HERO: Selection = Selection::Hero { player: PlayerId::P2 };

/// A game in which p1 has played a Devil's Pact this turn (its modifier live), and the state after.
fn pacted(seed: &str, radiant: bool) -> GameState {
    let mut state = game(seed);
    let pact = one(&mut state, PlayerId::P1, &PA.pact.id, radiant);
    must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": pact.id })).state
}

fn values(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// TS `eventsOfType`, over JSON.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    values(events).into_iter().filter(|event| event["type"] == kind).collect()
}

/// `eventsOfType(events, kind)[0]?.[field]`: `Null` when there is no such event.
fn first_field(events: &[GameEvent], kind: &str, field: &str) -> Value {
    of_type(events, kind)
        .first()
        .map(|event| event[field].clone())
        .unwrap_or(Value::Null)
}

/// `events.map((e) => e[field])` over JSON.
fn pluck(events: &[Value], field: &str) -> Vec<Value> {
    events.iter().map(|event| event[field].clone()).collect()
}

/// TS `toMatchObject`: every key `expected` names matches, objects by subset, arrays element by
/// element and of the same length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected.iter().all(|(key, want)| {
            actual.get(key).is_some_and(|got| matches_object(got, want))
        }),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn pending_id(state: &GameState) -> String {
    state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default()
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does, lent with the
/// state to one engine call at a time. Nothing writes the cursor back, as TS did not.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Sink {
    fn for_state(state: &GameState) -> Sink {
        Sink {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn with<T>(&mut self, state: &mut GameState, run: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
        let mut sink = EngineSink::new(state, &mut self.events, &mut self.rng);
        run(&mut sink)
    }
}

/// `castCard(sink, card); settle(sink);`
fn cast_and_settle(state: &mut GameState, card: &CardInstance) -> Vec<GameEvent> {
    let mut sink = Sink::for_state(state);
    sink.with(state, |s| {
        let _ = resolve::cast_card(s, card, CastOptions::default());
        let _ = triggers::settle(s, SettleOptions::default());
    });
    sink.events
}

mod r449_a_play_replaced_at_step_3_classic_c23_devils_pact {
    use super::*;

    /// "R449 the card played becomes the named card, which asks its target and resolves as that play"
    #[test]
    fn r449_the_card_played_becomes_the_named_card_which_asks_its_target_and_resolves_as_that_play() {
        let mut state = pacted("r449-replace", false);
        let crier = one(&mut state, PlayerId::P1, &PA.crier.id, false);
        let mana = state.players.p1.mana.current;

        let played = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": crier.id, "zone": { "row": "units", "lane": 3 } }),
        );
        let (asked, events) = (played.state, played.events);

        let replaced = of_type(&events, "transformed").first().cloned().unwrap_or(Value::Null);
        assert!(
            matches_object(
                &replaced,
                &json!({ "instanceId": crier.id, "fromDefId": PA.crier.id, "toDefId": PA.book.id, "hiddenFrom": ["p2"] })
            ),
            "{replaced}"
        );
        let book_id = replaced["newInstanceId"].as_str().unwrap_or("").to_string();
        // The price paid was the old card's, and the old card has ceased to exist.
        assert_eq!(asked.players.p1.mana.current, mana - 1);
        assert!(!asked.players.p1.hand.iter().any(|card| card.id == crier.id));
        // The Book of Flame asks for its target now, before the announce.
        assert_eq!(asked.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
        assert!(asked.pending.as_ref().is_some_and(|pending| pending.prompt.contains("Play:")));
        assert_eq!(of_type(&events, "cardAnnounced"), Vec::<Value>::new());
        let resolving: Vec<String> = asked.players.p1.resolving.iter().map(|card| card.id.clone()).collect();
        assert_eq!(resolving, vec![book_id.clone()]);

        let answered = must(
            &asked,
            PlayerId::P1,
            json!({ "type": "answer", "choiceId": pending_id(&asked), "selection": [ENEMY_HERO] }),
        );
        let (after, resolved) = (answered.state, answered.events);
        let announced = of_type(&resolved, "cardAnnounced").first().cloned().unwrap_or(Value::Null);
        assert!(
            matches_object(
                &announced,
                &json!({
                    "instanceId": book_id,
                    "defId": PA.book.id,
                    "cardType": "Spell",
                    "costPaid": 1,
                    "targets": ["hero-p2"],
                })
            ),
            "{announced}"
        );
        let played: Vec<Value> = of_type(&resolved, "cardPlayed")
            .iter()
            .map(|event| json!([event["instanceId"], event["costPaid"]]))
            .collect();
        assert_eq!(Value::Array(played), json!([[book_id, 1]]));
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 4);
        // The crier's Cry never happened and nothing stands in lane 3.
        assert!(zones::card_at(&after, slot(PlayerId::P1, Row::Units, 3)).is_none());
        assert_eq!(json!(after.players.p1.turn_log.played_by_type), json!({ "Spell": 2 }));
        assert!(after.players.p1.graveyard.iter().any(|card| card.id == book_id));
    }

    /// "R449 R177 the other player never reads the card that was replaced in the hand"
    #[test]
    fn r449_r177_the_other_player_never_reads_the_card_that_was_replaced_in_the_hand() {
        let mut state = pacted("r449-hidden", false);
        let crier = one(&mut state, PlayerId::P1, &PA.crier.id, false);
        let asked = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id })).state;
        let theirs = view_for(&asked, PlayerId::P2).events;
        assert_eq!(first_field(&theirs, "transformed", "fromDefId"), json!(HIDDEN_ID));
        let text = serde_json::to_string(&view_for(&asked, PlayerId::P2).events).expect("the events serialise");
        assert!(!text.contains(PA.crier.id.as_str()), "{text}");
    }

    /// "R449 the Radiant modifier replaces with the Radiant card"
    #[test]
    fn r449_the_radiant_modifier_replaces_with_the_radiant_card() {
        let mut state = pacted("r449-radiant", true);
        let ping = one(&mut state, PlayerId::P1, &PA.ping.id, false);
        let asked = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id })).state;
        let after = must(
            &asked,
            PlayerId::P1,
            json!({ "type": "answer", "choiceId": pending_id(&asked), "selection": [ENEMY_HERO] }),
        )
        .state;
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 8);
        assert_eq!(
            after.last_spell,
            Some(PlayRecord {
                def_id: PA.book.id.clone(),
                radiant: true,
            })
        );
    }

    /// "R449 a cast is a play and is replaced too (R70)"
    #[test]
    fn r449_r70_a_cast_is_a_play_and_is_replaced_too() {
        let mut state = pacted("r449-cast", false);
        let ping = new_instance(&mut state, &PA.ping.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        let events = cast_and_settle(&mut state, &ping);
        assert_eq!(first_field(&events, "transformed", "toDefId"), json!(PA.book.id));
        assert!(state.pending.as_ref().is_some_and(|pending| pending.prompt.contains("Cast:")));
        let after = must(
            &state,
            PlayerId::P1,
            json!({ "type": "answer", "choiceId": pending_id(&state), "selection": [ENEMY_HERO] }),
        )
        .state;
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 4);
    }

    /// "R449 the named card itself is not replaced by itself"
    #[test]
    fn r449_the_named_card_itself_is_not_replaced_by_itself() {
        let mut state = pacted("r449-self", false);
        let book = one(&mut state, PlayerId::P1, &PA.book.id, false);
        let played = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": book.id, "targets": [ENEMY_HERO] }),
        );
        assert_eq!(of_type(&played.events, "transformed"), Vec::<Value>::new());
        assert_eq!(pluck(&of_type(&played.events, "cardPlayed"), "instanceId"), vec![json!(book.id)]);
        assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH - 4);
    }

    /// "R449 a replacement that is a permanent takes the leftmost open zone at step 4 (R64)"
    #[test]
    fn r449_r64_a_replacement_that_is_a_permanent_takes_the_leftmost_open_zone_at_step_4() {
        let mut state = game("r449-permanent");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1));
        let turn = state.turn;
        state.players.p1.mods.push(PlayerModifier {
            id: "m-test".to_string(),
            expiry: ModifierExpiry::ThisTurn { turn },
            kind: ModifierKind::ReplacePlays {
                def_id: PA.crier.id.clone(),
                radiant: false,
            },
        });
        let ping = one(&mut state, PlayerId::P1, &PA.ping.id, false);
        let after = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id })).state;
        assert_eq!(
            zones::card_at(&after, slot(PlayerId::P1, Row::Units, 2)).map(|card| card.def_id.clone()),
            Some(PA.crier.id.clone())
        );
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 3);
    }

    /// "R449 the modifier lasts this turn only"
    #[test]
    fn r449_the_modifier_lasts_this_turn_only() {
        let state = pacted("r449-expiry", false);
        let mut after = must(&state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        after = must(&after, PlayerId::P2, json!({ "type": "endTurn" })).state;
        after.players.p1.mana.current = 8;
        let ping = one(&mut after, PlayerId::P1, &PA.ping.id, false);
        let played = must(&after, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id }));
        assert_eq!(of_type(&played.events, "transformed"), Vec::<Value>::new());
        assert!(
            !after
                .players
                .p1
                .mods
                .iter()
                .any(|modifier| matches!(modifier.kind, ModifierKind::ReplacePlays { .. }))
        );
    }

    /// "R449 the replacement's own question pauses the play: JSON round trip and replay agree"
    #[test]
    fn r449_the_replacements_own_question_pauses_the_play_json_round_trip_and_replay_agree() {
        /// `{ paused, done, revived }`: the hashes of the paused state, of the live answer and of the
        /// answer from the round-tripped state.
        fn run() -> (String, String, String) {
            let mut state = pacted("r449-pause", false);
            let crier = one(&mut state, PlayerId::P1, &PA.crier.id, false);
            let asked = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": crier.id })).state;
            let action: Action = json_as(json!({
                "type": "answer",
                "choiceId": pending_id(&asked),
                "selection": [ENEMY_HERO],
                "playerId": "p1",
                "nonce": "pause-answer",
            }));
            let round: GameState =
                serde_json::from_value(serde_json::to_value(&asked).expect("the state serialises"))
                    .expect("the state parses back");
            let live = reduce(&asked, &action);
            let revived = reduce(&round, &action);
            (hash_state(&asked), hash_state(&live.state), hash_state(&revived.state))
        }
        let first = run();
        let second = run();
        assert_eq!(first.2, first.1);
        assert_eq!(second, first);
    }

    /// "R449 a Counter reads the replacement's announce and cancels it"
    #[test]
    fn r449_a_counter_reads_the_replacements_announce_and_cancels_it() {
        let mut state = pacted("r449-counter", false);
        put(&mut state, &PA.refusal.id, slot(PlayerId::P2, Row::Backrow, 1));
        let mine = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1));
        let ping = one(&mut state, PlayerId::P1, &PA.ping.id, false);
        let asked = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id })).state;
        let answered = must(
            &asked,
            PlayerId::P1,
            json!({
                "type": "answer",
                "choiceId": pending_id(&asked),
                "selection": [{ "pick": "instance", "instanceId": mine.id }],
            }),
        );
        assert_eq!(first_field(&answered.events, "countered", "defId"), json!(PA.book.id));
        assert_eq!(
            zones::card_at(&answered.state, slot(PlayerId::P2, Row::Units, 1)).map(|card| card.damage),
            Some(0)
        );
    }
}

mod r449_r213_by_tag_plays_made_radiant_at_step_3_classic_plus_c68_organic_produce {
    use super::*;

    /// "R449 every Fruit its controller plays becomes Radiant as it is played, and nothing else does"
    #[test]
    fn r449_every_fruit_its_controller_plays_becomes_radiant_as_it_is_played_and_nothing_else_does() {
        let mut state = game("r213-produce");
        put(&mut state, &PA.produce.id, slot(PlayerId::P1, Row::Backrow, 1));
        let pear = one(&mut state, PlayerId::P1, &PA.pear.id, false);
        let ping = one(&mut state, PlayerId::P1, &PA.ping.id, false);

        let first = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": pear.id }));
        assert_eq!(pluck(&of_type(&first.events, "radiantSet"), "instanceId"), vec![json!(pear.id)]);
        assert_eq!(
            zones::card_at(&first.state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.radiant),
            Some(true)
        );
        let second = must(&first.state, PlayerId::P1, json!({ "type": "play", "instanceId": ping.id }));
        assert_eq!(of_type(&second.events, "radiantSet"), Vec::<Value>::new());
    }

    /// "R449 step 1 reads the Radiant face (R214), so the play carries that face's choices"
    #[test]
    fn r449_r214_step_1_reads_the_radiant_face_so_the_play_carries_that_faces_choices() {
        let mut state = game("r214-produce");
        put(&mut state, &PA.produce.id, slot(PlayerId::P1, Row::Backrow, 1));
        let apple = one(&mut state, PlayerId::P1, &PA.apple.id, false);
        assert!(play_choices::resolving_face(&state, PlayerId::P1, &apple, 1).radiant);
        let plays = play_choices::play_actions_for(&state, PlayerId::P1, &apple);
        assert!(plays
            .iter()
            .all(|play| play.targets.as_ref().map_or(0, |targets| targets.len()) == 1));
        let wanted: ActionBody = json_as(json!({ "type": "play", "instanceId": apple.id, "targets": [ENEMY_HERO] }));
        assert!(legal_actions(&state, PlayerId::P1).contains(&wanted));
        let after = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": apple.id, "targets": [ENEMY_HERO] }),
        )
        .state;
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 3);
    }

    /// "R449 the opponent's Organic Produce does nothing for this player, and a cast Fruit is Radiant
    /// too (R70)"
    #[test]
    fn r449_r70_the_opponents_organic_produce_does_nothing_for_this_player_and_a_cast_fruit_is_radiant_too() {
        let mut state = game("r213-produce-theirs");
        put(&mut state, &PA.produce.id, slot(PlayerId::P2, Row::Backrow, 1));
        let pear = one(&mut state, PlayerId::P1, &PA.pear.id, false);
        let mut after = must(&state, PlayerId::P1, json!({ "type": "play", "instanceId": pear.id })).state;
        assert_eq!(
            zones::card_at(&after, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.radiant),
            Some(false)
        );

        put(&mut after, &PA.produce.id, slot(PlayerId::P1, Row::Backrow, 2));
        let cast = new_instance(&mut after, &PA.pear.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        let events = cast_and_settle(&mut after, &cast);
        assert_eq!(pluck(&of_type(&events, "radiantSet"), "instanceId"), vec![json!(cast.id)]);
    }
}
