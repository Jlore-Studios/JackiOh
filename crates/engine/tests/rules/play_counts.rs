//! Port of `packages/engine/test/playCounts.test.ts`.
//!
//! What every play leaves to be counted (docs/classic-sets.md B5 E4; R451): per player per turn, the
//! plays by type (Classic+ #37 Wardrum), on both players' turns and cleared at every start of turn;
//! per player per game, the plays by tag (Classic+ #64's Fruit, AI Scaling Law's AI); game-wide, the
//! last Spell anyone played (Classic #57 Echo, which records the Spell it copied), and the last face-up
//! card each player played (AI Autocomplete: Traps are set face-down and never count, AI generated cards
//! are passed over). Casts count (R70); countered plays never (R448).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};
use crate::rules::fixtures::play_pipeline_a::{PA, with_play_a};

/// TS's module `let nonce`: unique across the tests, which run on parallel threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in PLAYER_IDS {
        let keep: Vec<String> = ready.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        ready = must(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in PLAYER_IDS {
        ready.players[player].mana.current = 9;
        ready.players[player].mana.max = 9;
    }
    ready
}

/// TS `must`: `body` is the TS `ActionBody` literal; the player and a fresh nonce are added.
fn must(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut body = body;
    body["playerId"] = json!(player_id);
    body["nonce"] = json!(format!("pa-pc-{nonce}"));
    let result = reduce(state, &json_as::<Action>(body));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// TS `hand`: a card put in `player`'s hand with the face asked for; answers its copy as it stands.
fn hand(state: &mut GameState, player: PlayerId, def_id: &str, radiant: bool) -> CardInstance {
    let card = in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("no card");
    find_instance_mut(state, &card.id)
        .expect("the card is in the hand")
        .radiant = radiant;
    find_instance(state, &card.id)
        .cloned()
        .expect("the card is in the hand")
}

/// Play a card that needs no choices (a hero target for the bolts), and return the state after.
fn play(state: &GameState, player: PlayerId, def_id: &str, radiant: bool) -> GameState {
    let mut state = state.clone();
    let card = hand(&mut state, player, def_id, radiant);
    let mut body = json!({ "type": "play", "instanceId": card.id });
    if def_id == PA.bolt.id {
        body["targets"] = json!([{ "pick": "hero", "player": opponent_of(player) }]);
    }
    must(&state, player, body).state
}

const NON_UNIT: [CardType; 3] = [CardType::Spell, CardType::FieldSpell, CardType::Trap];

fn of_type(state: &GameState, player: PlayerId, types: &[CardType]) -> i32 {
    query::played_this_turn_of_type(state, player, types)
}

mod r451_plays_by_type_this_turn_classic_plus_c37 {
    use super::*;

    /// "R451 counts each play by the type it was played as, and a Field Trap is a Trap"
    #[test]
    fn r451_counts_each_play_by_the_type_it_was_played_as_and_a_field_trap_is_a_trap() {
        let mut state = game("r451-types");
        state = play(&state, PlayerId::P1, &PA.field.id, false);
        state = play(&state, PlayerId::P1, &PA.ping.id, false);
        state = play(&state, PlayerId::P1, &PA.crier.id, false);
        state = play(&state, PlayerId::P1, &PA.hidden_trap.id, false);
        state = play(&state, PlayerId::P1, &PA.hidden_field_trap.id, false);
        assert_eq!(of_type(&state, PlayerId::P1, &[CardType::Unit]), 1);
        assert_eq!(of_type(&state, PlayerId::P1, &[CardType::Trap]), 1);
        assert_eq!(
            of_type(&state, PlayerId::P1, &[CardType::Trap, CardType::FieldTrap]),
            2
        );
        let mut non_unit = NON_UNIT.to_vec();
        non_unit.push(CardType::FieldTrap);
        assert_eq!(of_type(&state, PlayerId::P1, &non_unit), 4);
        assert_eq!(of_type(&state, PlayerId::P2, &[CardType::Spell]), 0);
    }

    /// "R451 clears at every start of turn, for both players"
    #[test]
    fn r451_clears_at_every_start_of_turn_for_both_players() {
        let mut state = play(&game("r451-reset"), PlayerId::P1, &PA.ping.id, false);
        assert_eq!(of_type(&state, PlayerId::P1, &[CardType::Spell]), 1);
        state = must(&state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        assert_eq!(state.active, PlayerId::P2);
        assert_eq!(of_type(&state, PlayerId::P1, &[CardType::Spell]), 0);
        assert_eq!(state.players.p1.turn_log.played_by_type, None);
    }

    /// "R451 a cast counts (R70), on the turn it happens, whoever's turn that is"
    #[test]
    fn r451_r70_a_cast_counts_on_the_turn_it_happens_whoevers_turn_that_is() {
        let mut state = game("r451-cast");
        let ping = new_instance(
            &mut state,
            &PA.ping.id,
            PlayerId::P2,
            Zone::Hand { player: PlayerId::P2 },
        );
        // TS `sinkFor(state)`: the rng from the state's cursor; nothing writes the cursor back.
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            resolve::cast_card(&mut sink, &ping, CastOptions::default());
            triggers::settle(&mut sink, SettleOptions::default());
        }
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(of_type(&state, PlayerId::P2, &[CardType::Spell]), 1);
        assert_eq!(
            query::played_this_game_with_tag(&state, PlayerId::P2, Tag::Book),
            0
        );
        assert_eq!(
            query::last_spell_played(&state),
            Some(PlayRecord {
                def_id: PA.ping.id.clone(),
                radiant: false,
            })
        );
    }

    /// "R451 a countered play counts nowhere (R448)"
    #[test]
    fn r451_r448_a_countered_play_counts_nowhere() {
        let mut state = game("r451-countered");
        put(
            &mut state,
            &PA.counter_trap.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let after = play(&state, PlayerId::P1, &PA.apple.id, false);
        assert_eq!(of_type(&after, PlayerId::P1, &[CardType::Spell]), 0);
        assert_eq!(
            query::played_this_game_with_tag(&after, PlayerId::P1, Tag::Fruit),
            0
        );
        assert_eq!(query::last_spell_played(&after), None);
        assert_eq!(query::last_face_up_played(&after, PlayerId::P1), None);
    }
}

mod r451_plays_by_tag_this_game_classic_plus_c64_ai_scaling_law {
    use super::*;

    /// "R451 counts each tag of each play, and never resets"
    #[test]
    fn r451_counts_each_tag_of_each_play_and_never_resets() {
        let mut state = game("r451-tags");
        state = play(&state, PlayerId::P1, &PA.apple.id, false);
        state = play(&state, PlayerId::P1, &PA.pear.id, false);
        state = play(&state, PlayerId::P1, &PA.ai_card.id, false);
        assert_eq!(
            query::played_this_game_with_tag(&state, PlayerId::P1, Tag::Fruit),
            2
        );
        assert_eq!(query::played_this_game_with_tag(&state, PlayerId::P1, Tag::Ai), 1);
        assert_eq!(
            query::played_this_game_with_tag(&state, PlayerId::P1, Tag::Token),
            1
        );
        state = must(&state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        state = must(&state, PlayerId::P2, json!({ "type": "endTurn" })).state;
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(
            query::played_this_game_with_tag(&state, PlayerId::P1, Tag::Fruit),
            2
        );
        assert_eq!(
            query::played_this_game_with_tag(&state, PlayerId::P2, Tag::Fruit),
            0
        );
    }
}

mod r451_the_last_spell_played_game_wide_classic_c57 {
    use super::*;

    /// "R451 records the last Spell either player played, with its face, and no other type overwrites it"
    #[test]
    fn r451_records_the_last_spell_either_player_played_with_its_face_and_no_other_type_overwrites_it() {
        let mut state = game("r451-last-spell");
        assert_eq!(query::last_spell_played(&state), None);
        state = play(&state, PlayerId::P1, &PA.bolt.id, true);
        let bolt = Some(PlayRecord {
            def_id: PA.bolt.id.clone(),
            radiant: true,
        });
        assert_eq!(query::last_spell_played(&state), bolt);
        state = play(&state, PlayerId::P1, &PA.crier.id, false);
        state = play(&state, PlayerId::P1, &PA.field.id, false);
        assert_eq!(query::last_spell_played(&state), bolt);
        state = must(&state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        state = play(&state, PlayerId::P2, &PA.ping.id, false);
        assert_eq!(
            query::last_spell_played(&state),
            Some(PlayRecord {
                def_id: PA.ping.id.clone(),
                radiant: false,
            })
        );
    }

    /// "R451 a played Echo records the Spell it copied, never itself, so it can't copy itself into a
    /// loop"
    #[test]
    fn r451_a_played_echo_records_the_spell_it_copied_never_itself_so_it_cant_copy_itself_into_a_loop() {
        let mut state = game("r451-echo");
        state = play(&state, PlayerId::P1, &PA.bolt.id, true);
        state = play(&state, PlayerId::P1, &PA.echo_copy.id, false);
        assert_eq!(
            query::last_spell_played(&state),
            Some(PlayRecord {
                def_id: PA.bolt.id.clone(),
                radiant: true,
            })
        );
        assert_eq!(
            query::last_face_up_played(&state, PlayerId::P1),
            Some(FaceUpRecord {
                def_id: PA.bolt.id.clone(),
                radiant: true,
                type_: CardType::Spell,
            })
        );
        // With nothing to copy, it records nothing.
        let fresh = play(&game("r451-echo-empty"), PlayerId::P1, &PA.echo_copy.id, false);
        assert_eq!(query::last_spell_played(&fresh), None);
    }
}

mod r451_the_last_face_up_card_each_player_played_ai_autocomplete {
    use super::*;

    fn face_up(def_id: &str, type_: CardType) -> Option<FaceUpRecord> {
        Some(FaceUpRecord {
            def_id: def_id.to_string(),
            radiant: false,
            type_,
        })
    }

    /// "R451 records per player; Traps set face-down and AI generated cards never count"
    #[test]
    fn r451_records_per_player_traps_set_face_down_and_ai_generated_cards_never_count() {
        let mut state = game("r451-face-up");
        state = play(&state, PlayerId::P1, &PA.crier.id, false);
        assert_eq!(
            query::last_face_up_played(&state, PlayerId::P1),
            face_up(&PA.crier.id, CardType::Unit)
        );
        state = play(&state, PlayerId::P1, &PA.hidden_trap.id, false);
        state = play(&state, PlayerId::P1, &PA.hidden_field_trap.id, false);
        state = play(&state, PlayerId::P1, &PA.ai_card.id, false);
        assert_eq!(
            query::last_face_up_played(&state, PlayerId::P1),
            face_up(&PA.crier.id, CardType::Unit)
        );
        state = play(&state, PlayerId::P1, &PA.field.id, false);
        assert_eq!(
            query::last_face_up_played(&state, PlayerId::P1),
            face_up(&PA.field.id, CardType::FieldSpell)
        );
        assert_eq!(query::last_face_up_played(&state, PlayerId::P2), None);
        // The records are copies: writing through one changes nothing.
        let mut read = query::last_face_up_played(&state, PlayerId::P1);
        if let Some(read) = read.as_mut() {
            read.def_id = "nothing".to_string();
        }
        assert_eq!(
            query::last_face_up_played(&state, PlayerId::P1).map(|record| record.def_id),
            Some(PA.field.id.clone())
        );
    }

    /// "R451 every record is plain data: it survives a JSON round trip and hashes the same"
    #[test]
    fn r451_every_record_is_plain_data_it_survives_a_json_round_trip_and_hashes_the_same() {
        let mut state = game("r451-json");
        state = play(&state, PlayerId::P1, &PA.apple.id, false);
        state = play(&state, PlayerId::P1, &PA.crier.id, false);
        let round: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("the state serialises"))
                .expect("the state parses back");
        assert_eq!(round, state);
        assert_eq!(
            query::last_face_up_played(&round, PlayerId::P1),
            query::last_face_up_played(&state, PlayerId::P1)
        );
        assert_eq!(
            query::played_this_game_with_tag(&round, PlayerId::P1, Tag::Fruit),
            1
        );
    }
}
