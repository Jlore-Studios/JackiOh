//! Port of `packages/engine/test/overflow-events.test.ts`.
//!
//! The three overflows of SPEC §2.4 as the event stream reports them, and what each seat reads of
//! them (§10.3, §10.8): a draw from an empty library (`fatigue`, R315), a card a full library turns
//! away (`libraryOverflow`, R316), and a card a full hand burns (`burned`, R317). The client animates
//! each one on both seats (R318), so the proofs here are about the stream: the event exists, it
//! comes in the order the board plays it, and `viewFor` gives each seat exactly what R97 allows.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, set_library, slot};
use crate::rules::fixtures::scripts::{cn_virus, infinite_reserves};

const TOKEN: &str = "fx-token-rush";

/// TS's module `let nonce`: unique across the tests, which run on parallel threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// TS `step`: one action (`body` carries its `playerId`), refused loudly.
fn step(state: &GameState, body: Value) -> (GameState, Vec<GameEvent>) {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut body = body;
    let kind = body["type"].clone();
    body["nonce"] = json!(format!("ov{nonce}"));
    let result = reduce(state, &json_as::<Action>(body));
    if let Some(error) = &result.error {
        panic!("{kind} refused: {error}");
    }
    (result.state, result.events)
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// Both mulligans kept whole: turn 1, p1's main phase.
fn started(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = hand_ids(&state, PlayerId::P1);
    state = step(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    )
    .0;
    let keep = hand_ids(&state, PlayerId::P2);
    state = step(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    )
    .0;
    state
}

/// Events as JSON, so TS's `toEqual` literals port key for key (an absent optional is TS's
/// `undefined`).
fn values(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// TS `eventsOfType`, over JSON.
fn of_type(events: &[Value], kind: &str) -> Vec<Value> {
    events
        .iter()
        .filter(|event| event["type"] == kind)
        .cloned()
        .collect()
}

/// `eventsOfType(events, kind)[0]?.[field]`: `Null` when there is no such event.
fn first_field(events: &[Value], kind: &str, field: &str) -> Value {
    of_type(events, kind)
        .first()
        .map(|event| event[field].clone())
        .unwrap_or(Value::Null)
}

/// The events both seats' views carry, with the view's window holding exactly `events`.
fn seen(state: &mut GameState, events: &[GameEvent]) -> PerPlayer<Vec<Value>> {
    state.applied = vec![AppliedAction {
        nonce: "overflow".to_string(),
        events: events.to_vec(),
    }];
    PerPlayer {
        p1: values(&view_for(state, PlayerId::P1).events),
        p2: values(&view_for(state, PlayerId::P2).events),
    }
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. It owns the
/// event list and the rng and lends itself, with the state, to one engine call at a time, so a test
/// reads the state between calls as TS did. Nothing writes the cursor back, as TS did not.
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

fn lib(def_ids: &[&str]) -> Vec<String> {
    def_ids.iter().map(|id| id.to_string()).collect()
}

mod r315_fatigue {
    use super::*;

    /// "R315 reports a fatigue draw as `fatigue` with its owner, its count and its hit, just before
    /// the hit, to both seats"
    #[test]
    fn r315_reports_a_fatigue_draw_as_fatigue_with_its_owner_its_count_and_its_hit_just_before_the_hit_to_both_seats()
     {
        let mut state = started("r315");
        state.players.p2.library.clear();
        let (after, events) = step(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        state = after;
        let events = values(&events);

        let at = events
            .iter()
            .position(|event| event["type"] == "fatigue")
            .expect("p2's turn-start draw from an empty library reports fatigue");
        assert_eq!(
            events[at],
            json!({ "type": "fatigue", "player": "p2", "count": 1, "amount": FATIGUE_DAMAGE(1) })
        );
        // The hit follows at once, so the board shows the empty library and then the hero taking it.
        assert_eq!(
            events[at + 1],
            json!({ "type": "damage", "sourceId": null, "targetId": "hero-p2", "amount": 1, "combat": false })
        );
        // No card was drawn, so there is no `drawn` for it (R3).
        let p2_drawn: Vec<Value> = of_type(&events, "drawn")
            .into_iter()
            .filter(|event| event["player"] == "p2")
            .collect();
        assert_eq!(p2_drawn, Vec::<Value>::new());

        // Public on both seats: it names a player and two numbers, and the fatigue count is in the view.
        for viewer in PLAYER_IDS {
            let view = view_for(&state, viewer);
            assert_eq!(
                of_type(&values(&view.events), "fatigue"),
                vec![json!({ "type": "fatigue", "player": "p2", "count": 1, "amount": 1 })]
            );
        }
    }

    /// "R315 counts each fatigue draw of a draw N on, 1, 2, 3"
    #[test]
    fn r315_counts_each_fatigue_draw_of_a_draw_n_on_1_2_3() {
        let mut state = new_game("r315-count", None);
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        let mut sink = Sink::for_state(&state);
        let _ = sink.with(&mut state, |s| draw::draw(s, PlayerId::P1, 3));
        let events = values(&sink.events);
        let pairs: Vec<Value> = of_type(&events, "fatigue")
            .iter()
            .map(|event| json!([event["count"], event["amount"]]))
            .collect();
        assert_eq!(
            Value::Array(pairs),
            json!([
                [1, FATIGUE_DAMAGE(1)],
                [2, FATIGUE_DAMAGE(2)],
                [3, FATIGUE_DAMAGE(3)]
            ])
        );
        // Each report is followed by its own hit.
        let order: Vec<Value> = events
            .iter()
            .filter(|event| event["type"] == "fatigue" || event["type"] == "damage")
            .map(|event| event["type"].clone())
            .collect();
        assert_eq!(
            Value::Array(order),
            json!(["fatigue", "damage", "fatigue", "damage", "fatigue", "damage"])
        );
    }

    /// "R315 still reports a fatigue draw whose whole hit Armor absorbs, ahead of R240's report", which
    /// since patch v0.3.X is the pipeline's `damageAbsorbed` (R1362).
    #[test]
    fn r315_r240_r1362_still_reports_a_fatigue_draw_whose_whole_hit_armor_absorbs_ahead_of_the_report() {
        let mut state = new_game("r315-armor", None);
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        state.players.p1.hero.armor = 3;
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| draw::draw_one(s, PlayerId::P1, None));
        assert!(matches!(outcome, DrawOutcome::Fatigue));
        assert_eq!(
            Value::Array(values(&sink.events)),
            json!([
                { "type": "fatigue", "player": "p1", "count": 1, "amount": 1 },
                { "type": "damageAbsorbed", "sourceId": null, "targetId": "hero-p1", "absorbed": 1, "combat": false },
            ])
        );
    }

    /// "R315 reports no fatigue for a draw #75 Infinite Reserves replaces with a Rush Token card"
    #[test]
    fn r315_reports_no_fatigue_for_a_draw_c75_infinite_reserves_replaces_with_a_rush_token_card() {
        let mut state = new_game("r315-reserves", None);
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        put(
            &mut state,
            &infinite_reserves().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| draw::draw_one(s, PlayerId::P1, None));
        assert!(matches!(outcome, DrawOutcome::Token));
        let events = values(&sink.events);
        assert_eq!(of_type(&events, "fatigue"), Vec::<Value>::new());
        let drawn: Vec<Value> = of_type(&events, "drawn")
            .iter()
            .map(|event| event["defId"].clone())
            .collect();
        assert_eq!(drawn, vec![json!(TOKEN)]);
        assert_eq!(state.players.p1.fatigue_count, 0);
    }
}

mod r316_library_overflow {
    use super::*;

    fn full_library(seed: &str) -> GameState {
        let mut state = new_game(seed, None);
        set_library(
            &mut state,
            PlayerId::P1,
            &vec!["fx-1".to_string(); LIBRARY_CAP as usize],
        );
        state
    }

    /// "R316 reports a copy a full library refuses as `libraryOverflow` notCreated, public to both
    /// seats"
    #[test]
    fn r316_reports_a_copy_a_full_library_refuses_as_library_overflow_not_created_public_to_both_seats() {
        let mut state = full_library("r316-new");
        let mut sink = Sink::for_state(&state);
        let mut fresh = new_instance(
            &mut state,
            "fx-2",
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        let outcome = sink.with(&mut state, |s| {
            draw::shuffle_into_library(s, &mut fresh, false, None)
        });
        assert!(matches!(outcome, ShuffleInOutcome::Dropped));

        let refused = json!({
            "type": "libraryOverflow", "player": "p1", "instanceId": fresh.id, "defId": "fx-2", "outcome": "notCreated"
        });
        assert_eq!(values(&sink.events), vec![refused.clone()]);
        // Never created, so it was never anywhere hidden: both seats read what was turned away.
        let views = seen(&mut state, &sink.events);
        assert_eq!(views.p1, vec![refused.clone()]);
        assert_eq!(views.p2, vec![refused]);
    }

    /// "R316 reports an existing card a full library sends to the graveyard as `graveyard`, then its
    /// enteredGraveyard"
    #[test]
    fn r316_reports_an_existing_card_a_full_library_sends_to_the_graveyard_as_graveyard_then_its_entered_graveyard()
     {
        let mut state = full_library("r316-existing");
        let mut card = in_hand(&mut state, "fx-2", PlayerId::P1, 1)
            .into_iter()
            .next()
            .expect("no card");
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| {
            draw::shuffle_into_library(s, &mut card, true, None)
        });
        assert!(matches!(outcome, ShuffleInOutcome::Dropped));

        assert_eq!(
            Value::Array(values(&sink.events)),
            json!([
                { "type": "libraryOverflow", "player": "p1", "instanceId": card.id, "defId": "fx-2", "outcome": "graveyard" },
                { "type": "enteredGraveyard", "instanceId": card.id, "defId": "fx-2", "owner": "p1" },
            ])
        );
        // It left p1's hand for a public graveyard, so the opponent reads it too (R97: judged by where
        // the card is now).
        let views = seen(&mut state, &sink.events);
        assert_eq!(first_field(&views.p2, "libraryOverflow", "defId"), json!("fx-2"));
    }

    /// "R316 reports an existing unit-token card a full library turns away as `ceased`, with no
    /// graveyard"
    #[test]
    fn r316_reports_an_existing_unit_token_card_a_full_library_turns_away_as_ceased_with_no_graveyard() {
        let mut state = full_library("r316-token");
        let mut token = put(&mut state, TOKEN, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| {
            draw::shuffle_into_library(s, &mut token, true, None)
        });
        assert!(matches!(outcome, ShuffleInOutcome::Dropped));

        assert_eq!(
            values(&sink.events),
            vec![
                json!({ "type": "libraryOverflow", "player": "p1", "instanceId": token.id, "defId": TOKEN, "outcome": "ceased" })
            ]
        );
        // TS read the live object; here the copy the call was handed, which it leaves as it landed.
        assert_eq!(token.zone.z(), ZoneName::Gone);
        let theirs = seen(&mut state, &sink.events).p2;
        assert_eq!(
            first_field(&theirs, "libraryOverflow", "instanceId"),
            json!(token.id)
        );
    }

    /// "R316 judges a copy that was never made by the card it copies: a face-down trap's copy names
    /// nothing to the other seat"
    #[test]
    fn r316_judges_a_copy_that_was_never_made_by_the_card_it_copies_a_face_down_traps_copy_names_nothing_to_the_other_seat()
     {
        // #33 copies whatever its controller plays, a Trap set face-down included (R17, R34).
        let trap: CardDef = json_as(json!({
            "id": "ov-secret-trap",
            "index": "1401",
            "name": "Secret Trap (overflow)",
            "set": "Core",
            "type": "Trap",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "secret" },
            "radiant": { "keywords": [], "text": "secret" },
        }));
        let mut state = full_library("r316-copy-of-trap");
        let mut catalog = registered_catalog().clone();
        catalog.insert(trap.id.clone(), trap.clone());
        register_catalog(catalog);
        let mut set = put(
            &mut state,
            &trap.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let mut sink = Sink::for_state(&state);
        let effect =
            effects::shuffle_into(json_as(json!({ "defId": trap.id, "count": 1, "copyOf": set.id })));
        sink.with(&mut state, |s| {
            let mut ctx = make_context(
                s,
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        });

        let events = values(&sink.events);
        let refused: Vec<Value> = of_type(&events, "libraryOverflow")
            .iter()
            .map(|event| json!([event["defId"], event["outcome"], event["copyOf"]]))
            .collect();
        assert_eq!(Value::Array(refused), json!([[trap.id, "notCreated", set.id]]));
        let views = seen(&mut state, &sink.events);
        // Its controller reads the trap, and so reads its copy; the other seat reads neither.
        assert_eq!(first_field(&views.p1, "libraryOverflow", "defId"), json!(trap.id));
        assert_eq!(
            of_type(&views.p2, "libraryOverflow"),
            vec![
                json!({ "type": "libraryOverflow", "player": "p1", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "outcome": "notCreated" })
            ]
        );
        // `copyOf` is bookkeeping: it names the face-down card's id, so no view carries it.
        for viewer in PLAYER_IDS {
            let text = serde_json::to_string(&views[viewer]).expect("the view serialises");
            assert!(!text.contains("copyOf"), "{viewer}: {text}");
        }

        // Once the trap is public (fired into its graveyard), the copy it would have made is too.
        zones::move_to_zone(
            &mut state,
            &mut set,
            OffFieldZone::Graveyard,
            MoveToZoneOptions::default(),
        );
        let theirs = seen(&mut state, &sink.events).p2;
        assert_eq!(first_field(&theirs, "libraryOverflow", "defId"), json!(trap.id));
    }

    /// "R316 carries a refused Radiant copy's face with its identity, and hides both together"
    #[test]
    fn r316_carries_a_refused_radiant_copys_face_with_its_identity_and_hides_both_together() {
        let mut state = full_library("r316-radiant");
        let mut sink = Sink::for_state(&state);
        let mut fresh = new_instance(
            &mut state,
            "fx-2",
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        fresh.radiant = true;
        let _ = sink.with(&mut state, |s| {
            draw::shuffle_into_library(s, &mut fresh, false, None)
        });
        assert_eq!(
            first_field(&values(&sink.events), "libraryOverflow", "radiant"),
            json!(true)
        );
        let theirs = seen(&mut state, &sink.events).p2;
        assert_eq!(first_field(&theirs, "libraryOverflow", "radiant"), json!(true));

        // Copying a face-down trap: the other seat reads neither the card nor its face.
        let mut hidden = full_library("r316-radiant-hidden");
        let mut trap = put(&mut hidden, "fx-2", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut hidden_sink = Sink::for_state(&hidden);
        let mut copy = new_instance(
            &mut hidden,
            "fx-2",
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        copy.radiant = true;
        zones::move_to_zone(
            &mut hidden,
            &mut trap,
            OffFieldZone::Hand,
            MoveToZoneOptions::default(),
        );
        let trap_id = trap.id.clone();
        let _ = hidden_sink.with(&mut hidden, |s| {
            draw::shuffle_into_library(s, &mut copy, false, Some(&trap_id))
        });
        let theirs = seen(&mut hidden, &hidden_sink.events).p2;
        let theirs = of_type(&theirs, "libraryOverflow")
            .first()
            .cloned()
            .unwrap_or(Value::Null);
        assert_eq!(
            theirs,
            json!({ "type": "libraryOverflow", "player": "p1", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "outcome": "notCreated" })
        );
    }

    /// "R316 reports nothing below the cap, and a CN-Virus chain at the cap turns its second copy away"
    #[test]
    fn r316_reports_nothing_below_the_cap_and_a_cn_virus_chain_at_the_cap_turns_its_second_copy_away() {
        let mut roomy = new_game("r316-roomy", None);
        set_library(&mut roomy, PlayerId::P1, &lib(&["fx-1"]));
        let mut roomy_sink = Sink::for_state(&roomy);
        let mut fresh = new_instance(
            &mut roomy,
            "fx-2",
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        let _ = roomy_sink.with(&mut roomy, |s| {
            draw::shuffle_into_library(s, &mut fresh, false, None)
        });
        assert_eq!(
            of_type(&values(&roomy_sink.events), "libraryOverflow"),
            Vec::<Value>::new()
        );

        // A full library with a CN-Virus on top: the draw takes it (59 left), the cast shuffles two
        // copies in, the first fills the library to 60 and the second is never created.
        let mut state = new_game("r316-virus", None);
        let mut library = vec![cn_virus().id];
        library.extend(vec!["fx-1".to_string(); (LIBRARY_CAP - 1) as usize]);
        set_library(&mut state, PlayerId::P1, &library);
        let mut sink = Sink::for_state(&state);
        let _ = sink.with(&mut state, |s| draw::draw_one(s, PlayerId::P1, None));
        let events = values(&sink.events);
        assert_eq!(of_type(&events, "shuffledIn").len(), 1);
        let refused: Vec<Value> = of_type(&events, "libraryOverflow")
            .iter()
            .map(|event| json!([event["player"], event["defId"], event["outcome"]]))
            .collect();
        assert_eq!(
            Value::Array(refused),
            json!([["p1", cn_virus().id, "notCreated"]])
        );
        assert!(state.players.p1.library.len() <= LIBRARY_CAP as usize);
    }
}

mod r317_a_full_hand_burns {
    use super::*;

    /// "R317 burns the card a full hand cannot take, and both seats read it: drawn, burned, then its
    /// graveyard"
    #[test]
    fn r317_burns_the_card_a_full_hand_cannot_take_and_both_seats_read_it_drawn_burned_then_its_graveyard() {
        let mut state = started("r317");
        // p2's hand is full, so the draw at the start of p2's turn is burned.
        state.players.p2.hand.clear();
        in_hand(&mut state, "fx-30", PlayerId::P2, HAND_CAP);
        let top = state
            .players
            .p2
            .library
            .first()
            .cloned()
            .expect("p2 has no library");
        let (after, events) = step(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        state = after;

        let mine: Vec<Value> = values(&events)
            .into_iter()
            .filter(|event| {
                (event["type"] == "drawn" || event["type"] == "burned" || event["type"] == "enteredGraveyard")
                    && event["instanceId"] == top.id.as_str()
            })
            .map(|event| event["type"].clone())
            .collect();
        assert_eq!(Value::Array(mine), json!(["drawn", "burned", "enteredGraveyard"]));
        assert_eq!(state.players.p2.hand.len(), HAND_CAP as usize);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == top.id));

        // It never reached the hand: it sits in a public graveyard, so the opponent reads the card that
        // burned as well as its owner does, the draw included (R97).
        for viewer in PLAYER_IDS {
            let seen_events = values(&view_for(&state, viewer).events);
            assert_eq!(
                of_type(&seen_events, "burned"),
                vec![json!({ "type": "burned", "instanceId": top.id, "defId": top.def_id, "owner": "p2" })],
                "{viewer}"
            );
            let drawn: Vec<Value> = of_type(&seen_events, "drawn")
                .into_iter()
                .filter(|event| event["instanceId"] == top.id.as_str())
                .map(|event| event["defId"].clone())
                .collect();
            assert_eq!(drawn, vec![json!(top.def_id)], "{viewer}");
        }
    }

    /// "R317 hides a burned card from the opponent once it returns to its owner's hand"
    #[test]
    fn r317_hides_a_burned_card_from_the_opponent_once_it_returns_to_its_owners_hand() {
        let mut state = new_game("r317-returned", None);
        set_library(&mut state, PlayerId::P1, &lib(&["fx-2"]));
        in_hand(&mut state, "fx-30", PlayerId::P1, HAND_CAP);
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| draw::draw_one(s, PlayerId::P1, None));
        assert!(matches!(outcome, DrawOutcome::Burned));
        let mut card = state
            .players
            .p1
            .graveyard
            .first()
            .cloned()
            .expect("nothing burned");

        // #72 Reminisce's move: graveyard to hand. R97 judges the card by where it is now.
        state.players.p1.hand.remove(0);
        zones::move_to_zone(
            &mut state,
            &mut card,
            OffFieldZone::Hand,
            MoveToZoneOptions::default(),
        );
        let views = seen(&mut state, &sink.events);
        assert_eq!(first_field(&views.p1, "burned", "defId"), json!("fx-2"));
        assert_eq!(
            of_type(&views.p2, "burned"),
            vec![json!({ "type": "burned", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "owner": "p1" })]
        );
    }

    /// "R317 burns a unit-token card out of existence: public, and no graveyard"
    #[test]
    fn r317_burns_a_unit_token_card_out_of_existence_public_and_no_graveyard() {
        let mut state = new_game("r317-token", None);
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        put(
            &mut state,
            &infinite_reserves().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        in_hand(&mut state, "fx-30", PlayerId::P1, HAND_CAP);
        let mut sink = Sink::for_state(&state);
        let outcome = sink.with(&mut state, |s| draw::draw_one(s, PlayerId::P1, None));
        assert!(matches!(outcome, DrawOutcome::Token));
        let events = values(&sink.events);
        let burned: Vec<Value> = of_type(&events, "burned")
            .iter()
            .map(|event| event["defId"].clone())
            .collect();
        assert_eq!(burned, vec![json!(TOKEN)]);
        assert_eq!(of_type(&events, "enteredGraveyard"), Vec::<Value>::new());
        let theirs = seen(&mut state, &sink.events).p2;
        assert_eq!(first_field(&theirs, "burned", "defId"), json!(TOKEN));
    }
}
