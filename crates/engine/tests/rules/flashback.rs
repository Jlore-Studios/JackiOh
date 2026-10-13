//! Flashback (#783, R783012, R783013): a card with Flashback in its owner's graveyard may be played from
//! there once, at its cost, as a play for every rule; the play spends the keyword for good. A play from
//! the graveyard another permission allows spends nothing. The fixtures in `fixtures/keywords_783.rs`
//! stand in for the cards #781 prints.

use jackioh_engine::testkit::*;

use super::fixtures::harness::{put, slot};
use super::fixtures::keywords_783::{
    flashback_spell, flashback_unit, picky_wind, plague_field, plain_spell, plain_unit,
    register_keywords_783, second_wind,
};
use super::fixtures::play_pipeline_b::{in_graveyard, pb_reduce, plays_of};
use super::fixtures::prompt_harness::{act, board, must};

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
    pb_reduce(
        state,
        json!({ "type": "play", "instanceId": id, "playerId": "p1" }),
    )
}

/// Apply one effect for p1 and settle, as a resolving card's would.
fn apply_now(state: &mut GameState, effect: Effect) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
}

mod r783012_flashback_play {
    use super::*;

    #[test]
    fn r783012_offered_from_the_graveyard_at_its_cost_cry_and_counts() {
        let mut state = playing("r783012-play");
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        let plain = in_graveyard(&mut state, &plain_spell().id, P1);
        let theirs = in_graveyard(&mut state, &flashback_spell().id, P2);

        // Offered from its owner's graveyard with no permission on the field; a card without
        // Flashback is not, nor is the opponent's.
        assert!(has_flashback(&state, &spell));
        assert!(!plays_of(&state, &spell.id, P1).is_empty());
        assert!(plays_of(&state, &plain.id, P1).is_empty());
        assert!(plays_of(&state, &theirs.id, P1).is_empty());

        // At its cost: with less mana than that, neither offered nor allowed.
        let mut poor = state.clone();
        poor.players.p1.mana.current = 1;
        assert!(plays_of(&poor, &spell.id, P1).is_empty());
        assert!(cast_spell(&poor, &spell.id).error.is_some());

        // A play: its price paid, its Cry run, counted, announced as from the graveyard (R454).
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

mod r783013_flashback_spent {
    use super::*;

    #[test]
    fn r783013_spent_as_replayed_never_offered_again_unit_and_spell() {
        // A Spell: back in the graveyard as any Spell goes, with no Flashback, never offered again.
        let mut state = playing("r783013-spell");
        state.players.p1.mana.current = 10;
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
            None,
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(!has_flashback(&state, &landed));
        assert!(plays_of(&state, &spell.id, P1).is_empty());
        assert!(cast_spell(&state, &spell.id).error.is_some());

        // A Unit: played into a zone with its Cry, no Flashback on the field, none once it dies.
        let mut units = playing("r783013-unit");
        let body = in_graveyard(&mut units, &flashback_unit().id, P1);
        let before = hero_health(&units, P2);
        units = act(
            &units,
            json!({ "type": "play", "instanceId": body.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
            None,
        );
        assert_eq!(hero_health(&units, P2), before - 1);
        let standing = must(find_instance(&units, &body.id).cloned(), "the Unit on the field");
        assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }));
        assert!(!has_flashback(&units, &standing));
        apply_now(
            &mut units,
            jackioh_engine::effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": body.id } }),
            )),
        );
        let dead = must(graveyard_card(&units, &body.id), "the Unit in the graveyard");
        assert_eq!(dead.flashback_spent, Some(true));
        assert!(!has_flashback(&units, &dead));
        assert!(plays_of(&units, &body.id, P1).is_empty());
    }

    #[test]
    fn r783013_a_permission_play_spends_no_flashback() {
        let mut state = playing("r783013-permission");
        state.players.p1.mana.current = 10;
        let wind = put(&mut state, &second_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);

        // Second Wind allows the play, so Flashback is kept.
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
            None,
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, None);
        assert!(has_flashback(&state, &landed));

        // Without the permission, Flashback still offers it once.
        let mut lone = wind.clone();
        jackioh_engine::zones::remove_from_any_zone(&mut state, &mut lone);
        assert!(!plays_of(&state, &spell.id, P1).is_empty());
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
            None,
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(plays_of(&state, &spell.id, P1).is_empty());
    }

    #[test]
    fn r783013_a_permission_that_cannot_cover_the_play_leaves_it_to_flashback_which_is_spent() {
        // A floor above the card's price (Radiant Second Wind's shape at (3)): the play is Flashback's
        // alone, so it spends it, and nothing offers the card again.
        let mut state = playing("r783013-floor");
        state.players.p1.mana.current = 10;
        put(&mut state, &picky_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
            None,
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(!has_flashback(&state, &landed));
        assert!(plays_of(&state, &spell.id, P1).is_empty());
        assert!(cast_spell(&state, &spell.id).error.is_some());

        // A Plague-only permission (Corpse Plantation's shape): a play paid in mana is Flashback's and
        // spends it; one paid with the permission's Plague Counters is the permission's and keeps it.
        let mut grave = playing("r783013-plague");
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

        let in_mana = act(
            &grave,
            json!({ "type": "play", "instanceId": body.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
            None,
        );
        let standing = must(
            find_instance(&in_mana, &body.id).cloned(),
            "the Unit on the field",
        );
        assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }));
        assert_eq!(standing.flashback_spent, Some(true));
        assert!(!has_flashback(&in_mana, &standing));

        let in_plague = act(
            &grave,
            json!({
                "type": "play",
                "instanceId": body.id,
                "zone": { "row": "units", "lane": 1 },
                "plague": { "from": field.id, "tokens": 1 },
                "playerId": "p1",
            }),
            None,
        );
        let standing = must(
            find_instance(&in_plague, &body.id).cloned(),
            "the Unit on the field",
        );
        assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }));
        assert_eq!(standing.flashback_spent, None);
        assert!(has_flashback(&in_plague, &standing));
    }
}
