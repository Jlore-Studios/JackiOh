//! Flashback (#783, R783012, R783013): a card with it in its owner's graveyard may be played from there
//! once, at its cost, as a play for every rule, spending the keyword for good; a play another permission
//! allows spends nothing. `fixtures/keywords_783.rs` stands in for the cards #781 prints. Since issue #4285 a
//! play by Flashback costs at least (1) after every discount (R4285000).

use jackioh_engine::testkit::*;

use super::fixtures::harness::{act, act_result, apply_now, put, slot};
use super::fixtures::keywords_783::{
    flashback_spell, flashback_unit, picky_wind, plague_field, plain_spell, plain_unit,
    register_keywords_783, second_wind,
};
use super::fixtures::play_pipeline_b::{in_graveyard, register_pipeline_b, x_spell};
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
        assert!(!plays_in(&state, P1, &spell.id).is_empty());
        assert!(plays_in(&state, P1, &plain.id).is_empty());
        assert!(plays_in(&state, P1, &theirs.id).is_empty());

        // At its cost: with less mana, neither offered nor allowed.
        let mut poor = state.clone();
        poor.players.p1.mana.current = 1;
        assert!(plays_in(&poor, P1, &spell.id).is_empty());
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

mod r783013_flashback_spent {
    use super::*;

    #[test]
    fn r783013_r3039001_spent_as_replayed_never_offered_again_unit_and_spell() {
        // A Spell: back in the graveyard as any Spell goes, spent.
        let mut state = playing("r783013-spell");
        state.players.p1.mana.current = 10;
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(!has_flashback(&state, &landed));
        assert!(plays_in(&state, P1, &spell.id).is_empty());
        assert!(cast_spell(&state, &spell.id).error.is_some());

        // A Unit: spent on the field, and still once it dies.
        let mut units = playing("r783013-unit");
        let body = in_graveyard(&mut units, &flashback_unit().id, P1);
        let before = hero_health(&units, P2);
        units = act(
            &units,
            json!({ "type": "play", "instanceId": body.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
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
        assert!(plays_in(&units, P1, &body.id).is_empty());
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
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, None);
        assert!(has_flashback(&state, &landed));

        // Without the permission, Flashback still offers it once.
        let mut lone = wind.clone();
        jackioh_engine::zones::remove_from_any_zone(&mut state, &mut lone);
        assert!(!plays_in(&state, P1, &spell.id).is_empty());
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(plays_in(&state, P1, &spell.id).is_empty());
    }

    #[test]
    fn r783013_a_permission_that_cannot_cover_the_play_leaves_it_to_flashback_which_is_spent() {
        // A floor above its price (Radiant Second Wind's shape at (3)): the play is Flashback's, so spent.
        let mut state = playing("r783013-floor");
        state.players.p1.mana.current = 10;
        put(&mut state, &picky_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        let landed = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert_eq!(landed.flashback_spent, Some(true));
        assert!(!has_flashback(&state, &landed));
        assert!(plays_in(&state, P1, &spell.id).is_empty());
        assert!(cast_spell(&state, &spell.id).error.is_some());

        // A Plague-only permission (Corpse Plantation's shape): a play paid in mana is Flashback's, spending
        // it; one paid in its Plague Counters is the permission's and keeps it.
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

/// R4285000: "You must spend at least (1) to cast a card from your graveyard by Flashback": the price after
/// every discount, printed or lent, an X card's too; a permission's play and a hand's are unfloored.
mod r4285000_flashback_floor {
    use super::*;
    use jackioh_engine::config::FLASHBACK_MIN_PRICE;
    use jackioh_engine::mana::{CostOptions, effective_cost};

    fn price(state: &GameState, id: &str) -> i32 {
        let card = must(find_instance(state, id).cloned(), "the card");
        effective_cost(state, &card, CostOptions { as_play: Some(true) })
    }

    /// A (2) card discounted to (0), `cost_mod` -2.
    fn discounted(state: &mut GameState, id: &str) {
        must(find_instance_mut(state, id), "the card").cost_mod = -2;
    }

    #[test]
    fn r4285000_a_flashback_play_costs_at_least_1_after_discounts_and_the_same_card_in_hand_0() {
        let mut state = playing("r4285000-floor");
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        discounted(&mut state, &spell.id);
        assert_eq!(FLASHBACK_MIN_PRICE, 1);
        assert_eq!(price(&state, &spell.id), 1);

        // Not offered at 0 mana, offered at 1, and the play pays 1.
        let mut broke = state.clone();
        broke.players.p1.mana.current = 0;
        assert!(plays_in(&broke, P1, &spell.id).is_empty());
        assert!(cast_spell(&broke, &spell.id).error.is_some());
        state.players.p1.mana.current = 1;
        assert!(!plays_in(&state, P1, &spell.id).is_empty());
        let result = cast_spell(&state, &spell.id);
        assert_eq!(result.error, None);
        assert_eq!(result.state.players.p1.mana.current, 0);

        // The same card in a hand is a hand's play and costs (0).
        let mut hand = playing("r4285000-hand");
        let held = new_instance(&mut hand, &flashback_spell().id, P1, Zone::Hand { player: P1 });
        hand.players.p1.hand.push(held.clone());
        discounted(&mut hand, &held.id);
        assert_eq!(price(&hand, &held.id), 0);
    }

    #[test]
    fn r4285000_a_lent_flashback_is_floored_too() {
        let mut state = playing("r4285000-lent");
        let spell = in_graveyard(&mut state, &plain_spell().id, P1);
        let live = must(find_instance_mut(&mut state, &spell.id), "the Spell");
        live.granted_keywords.push(Keyword::Flashback);
        live.cost_mod = -1;
        let lent = must(graveyard_card(&state, &spell.id), "the Spell in the graveyard");
        assert!(has_flashback(&state, &lent));
        assert_eq!(price(&state, &spell.id), 1);
        state.players.p1.mana.current = 0;
        assert!(plays_in(&state, P1, &spell.id).is_empty());
    }

    #[test]
    fn r4285000_an_x_card_played_by_flashback_for_x_0_costs_1() {
        let mut state = playing("r4285000-x");
        register_pipeline_b();
        let spell = in_graveyard(&mut state, &x_spell().id, P1);
        let live = must(find_instance_mut(&mut state, &spell.id), "the X Spell");
        live.granted_keywords.push(Keyword::Flashback);
        live.x = Some(0);
        assert_eq!(price(&state, &spell.id), 1);
        must(find_instance_mut(&mut state, &spell.id), "the X Spell").x = Some(3);
        assert_eq!(price(&state, &spell.id), 3);
    }

    #[test]
    fn r4285000_a_play_a_field_permission_covers_takes_no_floor_and_keeps_flashback() {
        let mut state = playing("r4285000-permission");
        put(&mut state, &second_wind().id, slot(P1, Row::Units, 1), json!({}));
        let spell = in_graveyard(&mut state, &flashback_spell().id, P1);
        discounted(&mut state, &spell.id);
        assert_eq!(price(&state, &spell.id), 0);
        state.players.p1.mana.current = 0;
        let result = cast_spell(&state, &spell.id);
        assert_eq!(result.error, None);
        let landed = must(
            graveyard_card(&result.state, &spell.id),
            "the Spell in the graveyard",
        );
        assert_eq!(landed.flashback_spent, None);
    }

    #[test]
    fn r4285000_a_plague_payment_meets_the_unfloored_price_so_no_counter_pays_for_a_0_flashback_unit() {
        // Corpse Plantation's shape: a Plague-only permission with two counters on it.
        let mut grave = playing("r4285000-plague");
        let field = put(
            &mut grave,
            &plague_field().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        );
        must(find_instance_mut(&mut grave, &field.id), "the permission")
            .counters
            .plague = Some(2);
        let body = in_graveyard(&mut grave, &flashback_unit().id, P1);
        discounted(&mut grave, &body.id);
        let plague_play = json!({
            "type": "play",
            "instanceId": body.id,
            "zone": { "row": "units", "lane": 1 },
            "plague": { "from": field.id, "tokens": 1 },
            "playerId": "p1",
        });

        // Its mana play is Flashback's and floored at (1); no play is offered in its Plague Counters,
        // as before the floor, and one is refused.
        assert_eq!(price(&grave, &body.id), 1);
        let offered = plays_in(&grave, P1, &body.id);
        assert!(!offered.is_empty());
        assert!(
            offered
                .iter()
                .all(|action| matches!(action, ActionBody::Play { plague: None, .. }))
        );
        assert!(act_result(&grave, plague_play.clone()).error.is_some());

        // At (1) before any floor, one counter still pays it and the play keeps Flashback.
        must(find_instance_mut(&mut grave, &body.id), "the Unit").cost_mod = -1;
        assert_eq!(price(&grave, &body.id), 1);
        assert!(
            plays_in(&grave, P1, &body.id)
                .iter()
                .any(|action| matches!(action, ActionBody::Play { plague: Some(_), .. }))
        );
        let paid = act_result(&grave, plague_play);
        assert_eq!(paid.error, None);
        let standing = must(
            find_instance(&paid.state, &body.id).cloned(),
            "the Unit on the field",
        );
        assert!(has_flashback(&paid.state, &standing));
    }
}
