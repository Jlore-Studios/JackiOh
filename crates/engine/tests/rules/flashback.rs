//! Flashback (#783, R783012, R3039001): a card with it in its owner's graveyard may be played from there,
//! at its cost, as a play for every rule, and keeps the keyword, so it is offered again whenever it lies
//! there; R783013, which had the play spend it for good, is superseded. `fixtures/keywords_783.rs` stands
//! in for the cards #781 prints.

use jackioh_engine::testkit::*;

use super::fixtures::harness::{act, act_result, apply_now, put, slot};
use super::fixtures::keywords_783::{
    flashback_spell, flashback_unit, picky_wind, plague_field, plain_spell, plain_unit,
    register_keywords_783, second_wind,
};
use super::fixtures::play_pipeline_b::{in_graveyard, plays_of};
use super::fixtures::prompt_harness::board;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// p1's main phase on turn 3 with 4 mana a side and #783's cards registered.
fn playing(seed: &str) -> GameState {
    let mut state = board(seed);
    register_keywords_783();
    // A card still to play, so the turn does not end by itself.
    let filler = new_instance(&mut state, &plain_unit().id, P1, Zone::Hand { player: P1 });
    state.players.p1.hand.push(filler);
    state
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn graveyard_card(state: &GameState, id: &str) -> Option<CardInstance> {
    state
        .players
        .p1
        .graveyard
        .iter()
        .find(|card| card.id == id)
        .cloned()
}

fn has_flashback(state: &GameState, card: &CardInstance) -> bool {
    unit_has(state, card, KeywordKind::Flashback)
}

fn cast_spell(state: &GameState, id: &str) -> ReduceResult {
    act_result(
        state,
        json!({ "type": "play", "instanceId": id, "playerId": "p1" }),
    )
}

mod r783012_flashback_play {
    use super::*;

    #[test]
    fn r783012_offered_from_the_graveyard_at_its_cost_cry_and_counts() {
        let mut state = playing("r783012-play");
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        let plain = in_graveyard(&mut state, &plain_spell().id, P1);
        let theirs = in_graveyard(&mut state, &flashback_spell().id, P2);

        // Offered from its owner's graveyard with no permission; not a card without Flashback, nor theirs.
        assert!(has_flashback(&state, &spell));
        assert!(!plays_of(&state, &spell.id, P1).is_empty());
        assert!(plays_of(&state, &plain.id, P1).is_empty());
        assert!(plays_of(&state, &theirs.id, P1).is_empty());

        // At its cost: with less mana, neither offered nor allowed.
        let mut poor = state.clone();
        poor.players.p1.mana.current = 1;
        assert!(plays_of(&poor, &spell.id, P1).is_empty());
        assert!(cast_spell(&poor, &spell.id).error.is_some());

        // A play: paid, its Cry run, counted, announced as from the graveyard (R454).
        let mana = state.players.p1.mana.current;
        let before = hero_health(&state, P2);
        let result = cast_spell(&state, &spell.id);
        assert_eq!(result.error, None);
        assert_eq!(result.state.players.p1.mana.current, mana - 2);
        assert_eq!(hero_health(&result.state, P2), before - 2);
        assert_eq!(result.state.players.p1.turn_log.cards_played, 1);
        assert!(result.events.iter().any(|event| matches!(
            event,
            GameEvent::CardPlayed { instance_id, from: Some(PlayedFrom::Graveyard), .. } if *instance_id == spell.id
        )));
    }
}

/// R3039001 (superseding R783013): a play by Flashback spends nothing, so the card keeps the keyword and
/// is offered by it again whenever it lies in its owner's graveyard.
mod r3039001_r783013_flashback_kept {
    use super::*;

    fn play_unit(state: &GameState, id: &str) -> GameState {
        act(
            state,
            json!({ "type": "play", "instanceId": id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
        )
    }

    /// Back in p1's graveyard with its Flashback, never written spent, and offered by it again.
    fn kept_in_graveyard(state: &GameState, id: &str) -> CardInstance {
        let landed = must(graveyard_card(state, id), "the card in the graveyard");
        assert_eq!(landed.flashback_spent, None);
        assert!(has_flashback(state, &landed));
        assert!(!plays_of(state, id, P1).is_empty(), "offered again");
        landed
    }

    #[test]
    fn r3039001_r783013_kept_as_replayed_and_flashed_back_twice_unit_and_spell() {
        // A Spell: back in the graveyard as any Spell goes, Flashback kept, so flashed back again.
        let mut state = playing("r3039001-spell");
        state.players.p1.mana.current = 10;
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        let before = hero_health(&state, P2);
        for _ in 0..2 {
            let result = cast_spell(&state, &spell.id);
            assert_eq!(result.error, None);
            state = result.state;
            kept_in_graveyard(&state, &spell.id);
        }
        assert_eq!(hero_health(&state, P2), before - 4, "its Cry ran twice");
        assert_eq!(state.players.p1.turn_log.cards_played, 2);

        // A Unit: it keeps the keyword on the field, and once it dies it is flashed back again.
        let mut units = playing("r3039001-unit");
        units.players.p1.mana.current = 10;
        let body = in_graveyard(&mut units, &flashback_unit().id, P1);
        let before = hero_health(&units, P2);
        for _ in 0..2 {
            units = play_unit(&units, &body.id);
            let standing = must(find_instance(&units, &body.id).cloned(), "the Unit on the field");
            assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }));
            assert_eq!(standing.flashback_spent, None);
            assert!(has_flashback(&units, &standing));
            apply_now(
                &mut units,
                jackioh_engine::effects::destroy(json_as(
                    json!({ "target": { "of": "instance", "instanceId": body.id } }),
                )),
            );
            kept_in_graveyard(&units, &body.id);
        }
        assert_eq!(hero_health(&units, P2), before - 2, "its Cry ran twice");
    }

    #[test]
    fn r3039001_r783013_a_permission_play_and_a_flashback_play_alike_keep_it() {
        let mut state = playing("r3039001-permission");
        state.players.p1.mana.current = 10;
        let wind = put(&mut state, &second_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);

        // Second Wind allows the play, which keeps Flashback.
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        kept_in_graveyard(&state, &spell.id);

        // Without the permission, Flashback's own play keeps it as well.
        let mut lone = wind.clone();
        jackioh_engine::zones::remove_from_any_zone(&mut state, &mut lone);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        kept_in_graveyard(&state, &spell.id);
    }

    #[test]
    fn r3039001_r783013_a_play_only_flashback_allows_keeps_it_paid_in_mana_or_plague() {
        // A floor above its price (Radiant Second Wind's shape at (3)): the play is Flashback's, which keeps it.
        let mut state = playing("r3039001-floor");
        state.players.p1.mana.current = 10;
        put(&mut state, &picky_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        kept_in_graveyard(&state, &spell.id);
        assert_eq!(cast_spell(&state, &spell.id).error, None, "and plays again");

        // A Plague-only permission (Corpse Plantation's shape): a play paid in mana is Flashback's, one paid in
        // its Plague Counters the permission's, and both keep it.
        let mut grave = playing("r3039001-plague");
        let field = put(
            &mut grave,
            &plague_field().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        );
        find_instance_mut(&mut grave, &field.id)
            .expect("the permission stands")
            .counters
            .plague = Some(2);
        let body = in_graveyard(&mut grave, &flashback_unit().id, P1);

        let in_mana = play_unit(&grave, &body.id);
        let in_plague = act(
            &grave,
            json!({
                "type": "play",
                "instanceId": body.id,
                "zone": { "row": "units", "lane": 1 },
                "plague": { "from": field.id, "tokens": 1 },
                "playerId": "p1",
            }),
        );
        for played in [&in_mana, &in_plague] {
            let standing = must(find_instance(played, &body.id).cloned(), "the Unit on the field");
            assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }));
            assert_eq!(standing.flashback_spent, None);
            assert!(has_flashback(played, &standing));
        }
    }
}
