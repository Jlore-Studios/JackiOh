//! Meditative #40 Feng Shui (R980–R987): ME-ELEMENT and ME-LUCK, proved on the `fs-*`
//! fixture cards (`rules/fixtures/feng_shui.rs`).
//!
//! The units' indices end in 1–5 on purpose (R980: 水 火 木 金 土); the Trap ends in 2 (火) and the
//! judge in 0 (土). The record is written in every game (R982, D14).

use jackioh_engine::subsystems::feng_shui::{element_of, element_of_index, last_element, reaction};
use jackioh_engine::testkit::*;

use super::fixtures::feng_shui::{FS, with_feng_shui};
use super::fixtures::harness::{events_json, in_hand, new_game, next_nonce, put, slot};

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_feng_shui(new_game(seed, None))).state;
    for player in PLAYER_IDS {
        let keep: Vec<String> = ready.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        ready = reduce_ok(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in PLAYER_IDS {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
    }
    ready
}

/// `body` reduced with the player and a fresh nonce added; panics on a refusal.
fn reduce_ok(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let mut body = body;
    body["playerId"] = json!(player_id);
    body["nonce"] = json!(next_nonce());
    let result = reduce(state, &json_as::<Action>(body));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// A card put in `player`'s hand on the face asked for, as it stands.
fn one(state: &mut GameState, player: PlayerId, def_id: &str, radiant: bool) -> CardInstance {
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

fn judge(state: &mut GameState, radiant: bool) -> CardInstance {
    put(
        state,
        &FS.judge.id,
        slot(PlayerId::P1, Row::Backrow, 1),
        if radiant {
            json!({ "radiant": true })
        } else {
            json!({})
        },
    )
}

fn play(state: &GameState, player: PlayerId, instance_id: &str) -> ReduceResult {
    reduce_ok(
        state,
        player,
        json!({ "type": "play", "instanceId": instance_id }),
    )
}

fn values(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serializes"))
        .collect()
}

/// The element fixture's catalog id, by its Hetu name.
fn id_for(element: &str) -> String {
    let fs = &*FS;
    match element {
        "water" => fs.water.id.clone(),
        "fire" => fs.fire.id.clone(),
        "wood" => fs.wood.id.clone(),
        "metal" => fs.metal.id.clone(),
        "earth" => fs.earth.id.clone(),
        _ => panic!("no such element"),
    }
}

/// The relation the rules read, hardcoded from MD-C6 rather than computed: positive when the last
/// element generates the new one, negative when it overcomes it, neutral otherwise.
fn expected(last: &str, played: &str) -> Option<FengShuiOutcome> {
    const POSITIVE: [(&str, &str); 5] = [
        ("wood", "fire"),
        ("fire", "earth"),
        ("earth", "metal"),
        ("metal", "water"),
        ("water", "wood"),
    ];
    const NEGATIVE: [(&str, &str); 5] = [
        ("wood", "earth"),
        ("earth", "water"),
        ("water", "fire"),
        ("fire", "metal"),
        ("metal", "wood"),
    ];
    if POSITIVE.contains(&(last, played)) {
        Some(FengShuiOutcome::Positive)
    } else if NEGATIVE.contains(&(last, played)) {
        Some(FengShuiOutcome::Negative)
    } else {
        None
    }
}

const ELEMENTS: [&str; 5] = ["water", "fire", "wood", "metal", "earth"];

mod r980_an_index_s_last_digit_is_its_element {
    use super::*;

    #[test]
    fn r980_an_index_s_last_digit_is_its_element() {
        assert_eq!(element_of_index("1"), CardElement::Water);
        assert_eq!(element_of_index("10"), CardElement::Earth);
        assert_eq!(element_of_index("39.2"), CardElement::Fire);
        assert_eq!(element_of_index("T-coin"), CardElement::Earth);
        assert_eq!(element_of_index("T-AI-10"), CardElement::Earth);
        // A fused card takes its first ingredient's element (R77's order).
        let state = with_feng_shui(new_game("r980-fused", None));
        assert_eq!(element_of(&state, "t-1:fs-fire+fs-water"), CardElement::Fire);
        assert_eq!(element_of(&state, &FS.water.id), CardElement::Water);
        assert_eq!(element_of(&state, &FS.judge.id), CardElement::Earth);
    }
}

mod r981_every_element_pair_through_the_pipeline {
    use super::*;

    #[test]
    fn r981_every_element_pair_through_the_pipeline() {
        for last in ELEMENTS {
            for played in ELEMENTS {
                let mut state = game(&format!("r981-{last}-{played}"));
                let judge_card = judge(&mut state, false);
                let first = one(&mut state, PlayerId::P1, &id_for(last), false);
                let first_play = play(&state, PlayerId::P1, &first.id);
                assert!(
                    events_json(&first_play.events, "fengShui").is_empty(),
                    "a first play is neutral"
                );
                let mut state = first_play.state;
                let second = one(&mut state, PlayerId::P1, &id_for(played), false);
                let health_before = state.players[PlayerId::P1].hero.health;
                let played_out = play(&state, PlayerId::P1, &second.id);
                let after = played_out.state;
                let verdicts = events_json(&played_out.events, "fengShui");
                let live = find_instance(&after, &second.id).expect("the played card is in play");
                match expected(last, played) {
                    Some(FengShuiOutcome::Positive) => {
                        assert!(live.radiant, "{last} generates {played}");
                        assert_eq!(
                            verdicts,
                            vec![json!({
                                "type": "fengShui",
                                "instanceId": second.id,
                                "sourceId": judge_card.id,
                                "player": "p1",
                                "outcome": "positive",
                            })],
                            "{last} generates {played}"
                        );
                        assert_eq!(after.players[PlayerId::P1].hero.health, health_before);
                        assert_eq!(live.brittle.as_ref().map(|brittle| brittle.count), None);
                    }
                    Some(FengShuiOutcome::Negative) => {
                        assert!(!live.radiant, "{last} overcomes {played}");
                        assert_eq!(
                            verdicts,
                            vec![json!({
                                "type": "fengShui",
                                "instanceId": second.id,
                                "sourceId": judge_card.id,
                                "player": "p1",
                                "outcome": "negative",
                            })],
                            "{last} overcomes {played}"
                        );
                        assert_eq!(
                            live.brittle.as_ref().map(|brittle| brittle.count),
                            Some(2),
                            "{last} overcomes {played}"
                        );
                        assert_eq!(
                            after.players[PlayerId::P1].hero.health,
                            health_before - 10,
                            "{last} overcomes {played}"
                        );
                    }
                    None => {
                        assert!(!live.radiant, "{last} then {played} is neutral");
                        assert!(
                            verdicts.is_empty(),
                            "{last} then {played} is neutral: {verdicts:?}"
                        );
                        assert_eq!(after.players[PlayerId::P1].hero.health, health_before);
                        assert_eq!(live.brittle.as_ref().map(|brittle| brittle.count), None);
                    }
                }
                // The record holds each play's element in turn; the subsystem agrees with the pairs.
                assert_eq!(
                    last_element(&after, PlayerId::P1),
                    Some(element_of(&after, &id_for(played))),
                    "{last} then {played}"
                );
                assert_eq!(
                    reaction(
                        Some(element_of(&after, &id_for(last))),
                        element_of(&after, &id_for(played))
                    ),
                    expected(last, played),
                    "{last} then {played}"
                );
            }
        }
    }

    #[test]
    fn r981_a_first_play_is_neutral() {
        let mut state = game("r981-first");
        judge(&mut state, false);
        let first = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let played_out = play(&state, PlayerId::P1, &first.id);
        assert!(events_json(&played_out.events, "fengShui").is_empty());
        let live = find_instance(&played_out.state, &first.id).expect("the played card is in play");
        assert!(!live.radiant);
        assert_eq!(
            last_element(&played_out.state, PlayerId::P1),
            Some(CardElement::Wood)
        );
    }
}

mod r982_a_face_down_play_is_neither_judged_nor_recorded {
    use super::*;

    #[test]
    fn r982_a_face_down_play_is_neither_judged_nor_recorded() {
        let mut state = game("r982-trap");
        judge(&mut state, false);
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        assert_eq!(last_element(&state, PlayerId::P1), Some(CardElement::Wood));
        let mut state = state;
        let trap = one(&mut state, PlayerId::P1, &FS.trap.id, false);
        let health_before = state.players[PlayerId::P1].hero.health;
        let played_out = reduce_ok(
            &state,
            PlayerId::P1,
            json!({
                "type": "play",
                "instanceId": trap.id,
                "zone": { "row": "backrow", "lane": 2 },
            }),
        );
        assert!(events_json(&played_out.events, "fengShui").is_empty());
        // The record still holds the last face-up play's element.
        assert_eq!(
            last_element(&played_out.state, PlayerId::P1),
            Some(CardElement::Wood)
        );
        assert_eq!(played_out.state.players[PlayerId::P1].hero.health, health_before);
    }
}

mod r982_the_record_is_written_in_every_game_since_the_set_shipped {
    use super::*;

    /// With nothing previewed, the first play is recorded and the second is judged against it.
    #[test]
    fn r982_the_record_is_written_in_every_game_since_the_set_shipped() {
        assert!(set_ships(SetName::Meditative));
        let mut state = game("r982-shipped");
        let judge_card = judge(&mut state, false);
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        assert_eq!(last_element(&state, PlayerId::P1), Some(CardElement::Wood));
        // Wood generates fire: the second play is judged positive and recorded in turn.
        let mut state = state;
        let fire = one(&mut state, PlayerId::P1, &id_for("fire"), false);
        let played_out = play(&state, PlayerId::P1, &fire.id);
        assert_eq!(
            events_json(&played_out.events, "fengShui"),
            vec![json!({
                "type": "fengShui",
                "instanceId": fire.id,
                "sourceId": judge_card.id,
                "player": "p1",
                "outcome": "positive",
            })]
        );
        assert_eq!(
            last_element(&played_out.state, PlayerId::P1),
            Some(CardElement::Fire)
        );
    }
}

mod r983_the_damage_lands_after_card_resolved {
    use super::*;

    #[test]
    fn r983_the_damage_lands_after_card_resolved() {
        let mut state = game("r983-order");
        judge(&mut state, false);
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        let mut state = state;
        // Wood is overcome by Earth: punished.
        let earth = one(&mut state, PlayerId::P1, &id_for("earth"), false);
        let played_out = play(&state, PlayerId::P1, &earth.id);
        let kinds: Vec<String> = values(&played_out.events)
            .iter()
            .map(|event| event["type"].as_str().unwrap_or_default().to_string())
            .collect();
        let resolved = kinds.iter().rposition(|kind| kind == "cardResolved");
        let hit = kinds.iter().position(|kind| kind == "damage");
        assert!(
            resolved.is_some_and(|at| hit.is_some_and(|hit| at < hit)),
            "the hit lands after the play has resolved: {kinds:?}"
        );
        let damage = events_json(&played_out.events, "damage");
        assert_eq!(damage.len(), 1);
        assert_eq!(damage[0]["amount"], json!(10));
        assert_eq!(damage[0]["targetId"], json!("hero-p1"));
    }
}

mod r984_two_judges_punish_twice {
    use super::*;

    #[test]
    fn r984_two_judges_punish_twice() {
        let mut state = game("r984-twice");
        let first_judge = judge(&mut state, false);
        let second_judge = put(
            &mut state,
            &FS.judge.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        let mut state = state;
        let earth = one(&mut state, PlayerId::P1, &id_for("earth"), false);
        let health_before = state.players[PlayerId::P1].hero.health;
        let played_out = play(&state, PlayerId::P1, &earth.id);
        let mut sources: Vec<String> = events_json(&played_out.events, "fengShui")
            .iter()
            .map(|event| event["sourceId"].as_str().unwrap_or_default().to_string())
            .collect();
        sources.sort();
        let mut wanted = vec![first_judge.id.clone(), second_judge.id.clone()];
        wanted.sort();
        assert_eq!(sources, wanted);
        assert_eq!(
            played_out.state.players[PlayerId::P1].hero.health,
            health_before - 20
        );
    }
}

mod r984_the_radiant_judge_rewards_only_its_controller_and_punishes_only_the_opponent_for_20 {
    use super::*;

    #[test]
    fn r984_the_radiant_judge_rewards_only_its_controller_and_punishes_only_the_opponent_for_20() {
        let mut state = game("r984-radiant");
        judge(&mut state, true);
        // p1's plays: a generating pair is rewarded, an overcoming pair is not punished.
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        let mut state = state;
        let fire = one(&mut state, PlayerId::P1, &id_for("fire"), false);
        let played_out = play(&state, PlayerId::P1, &fire.id);
        let live = find_instance(&played_out.state, &fire.id).expect("the rewarded card is in play");
        assert!(live.radiant);
        assert_eq!(events_json(&played_out.events, "fengShui").len(), 1);
        let mut state = played_out.state;
        let metal = one(&mut state, PlayerId::P1, &id_for("metal"), false);
        let health_before = state.players[PlayerId::P1].hero.health;
        let played_out = play(&state, PlayerId::P1, &metal.id);
        let live = find_instance(&played_out.state, &metal.id).expect("the spared card is in play");
        assert!(!live.radiant);
        assert!(events_json(&played_out.events, "fengShui").is_empty());
        assert_eq!(live.brittle.as_ref().map(|brittle| brittle.count), None);
        assert_eq!(played_out.state.players[PlayerId::P1].hero.health, health_before);
        // p2's plays: a generating pair is not rewarded, an overcoming pair is punished for 20.
        let state = reduce_ok(&played_out.state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        let mut state = state;
        let wood = one(&mut state, PlayerId::P2, &id_for("wood"), false);
        let state = play(&state, PlayerId::P2, &wood.id).state;
        let mut state = state;
        let fire = one(&mut state, PlayerId::P2, &id_for("fire"), false);
        let played_out = play(&state, PlayerId::P2, &fire.id);
        let live = find_instance(&played_out.state, &fire.id).expect("the unrewarded card is in play");
        assert!(!live.radiant);
        assert!(events_json(&played_out.events, "fengShui").is_empty());
        let mut state = played_out.state;
        let metal = one(&mut state, PlayerId::P2, &id_for("metal"), false);
        let health_before = state.players[PlayerId::P2].hero.health;
        let played_out = play(&state, PlayerId::P2, &metal.id);
        let live = find_instance(&played_out.state, &metal.id).expect("the punished card is in play");
        assert_eq!(live.brittle.as_ref().map(|brittle| brittle.count), Some(2));
        assert_eq!(
            played_out.state.players[PlayerId::P2].hero.health,
            health_before - 20
        );
        assert_eq!(events_json(&played_out.events, "fengShui").len(), 1);
    }

    #[test]
    fn r984_the_base_face_judges_the_opponent() {
        let mut state = game("r984-base");
        judge(&mut state, false);
        let state = reduce_ok(&state, PlayerId::P1, json!({ "type": "endTurn" })).state;
        let mut state = state;
        let wood = one(&mut state, PlayerId::P2, &id_for("wood"), false);
        let state = play(&state, PlayerId::P2, &wood.id).state;
        let mut state = state;
        let earth = one(&mut state, PlayerId::P2, &id_for("earth"), false);
        let health_before = state.players[PlayerId::P2].hero.health;
        let played_out = play(&state, PlayerId::P2, &earth.id);
        assert_eq!(events_json(&played_out.events, "fengShui").len(), 1);
        assert_eq!(
            played_out.state.players[PlayerId::P2].hero.health,
            health_before - 10
        );
    }
}

mod r985_the_hidden_numbers_are_config {
    use super::*;

    #[test]
    fn r985_the_hidden_numbers_are_config() {
        assert_eq!(FENG_SHUI_DAMAGE.base, 10);
        assert_eq!(FENG_SHUI_DAMAGE.radiant, 20);
        assert_eq!(FENG_SHUI_BRITTLE, 2);
        // The play's punishment reads those same numbers: a negative pair from a base judge.
        let mut state = game("r985-config");
        judge(&mut state, false);
        let wood = one(&mut state, PlayerId::P1, &id_for("wood"), false);
        let state = play(&state, PlayerId::P1, &wood.id).state;
        let mut state = state;
        let earth = one(&mut state, PlayerId::P1, &id_for("earth"), false);
        let health_before = state.players[PlayerId::P1].hero.health;
        let played_out = play(&state, PlayerId::P1, &earth.id);
        let live = find_instance(&played_out.state, &earth.id).expect("the punished card is in play");
        assert_eq!(
            live.brittle.as_ref().map(|brittle| brittle.count),
            Some(FENG_SHUI_BRITTLE)
        );
        assert_eq!(
            played_out.state.players[PlayerId::P1].hero.health,
            health_before - FENG_SHUI_DAMAGE.on(false)
        );
    }
}

mod r986_no_tag_is_an_element {
    use super::*;

    #[test]
    fn r986_no_tag_is_an_element() {
        for glyph in ["水", "火", "木", "金", "土"] {
            assert!(glyph.parse::<Tag>().is_err(), "{glyph} is no tag");
        }
    }
}
