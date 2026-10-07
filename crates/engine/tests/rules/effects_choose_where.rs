//! §10.6 A target prompt narrowed by a card's own condition — `chooseTargetWhere`
//! (effects/chooseWhere.ts), the prompt Classic #32 Felinor Feelings' Radiant face asks after its token
//! lands. The real card's test (packages/cards/test/classic/032-felinor-feelings.test.ts) covers the
//! same cases again through the card.
//!
//! Port of `packages/engine/test/effects-choose-where.test.ts`.

use jackioh_engine::effects::{ChooseTargetWhereArgs, choose_target_where};
use std::sync::Arc;

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, put, slot};

/// TS `run(state, effect)`: the effect applied for p1 on a fresh sink over `state`; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(sink, None, HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
        (effect.apply)(&mut ctx);
    }
    events
}

fn choose_where(
    scope: Value,
    where_: impl Fn(&EffectContext<'_>, Option<&CardInstance>) -> bool + Send + Sync + 'static,
) -> Effect {
    choose_target_where(ChooseTargetWhereArgs {
        step: "picked".to_string(),
        scope: Some(json_as(scope)),
        where_: Arc::new(where_),
        prompt: None,
        data: None,
    })
}

fn pending_of(state: &GameState) -> &PendingChoice {
    state.pending.as_ref().expect("a prompt")
}

mod s10_6_choose_target_where_c_32_felinor_feelings {
    use super::*;

    #[test]
    fn offers_the_scope_s_cards_that_the_condition_admits_in_the_scope_s_order_to_the_controller() {
        let mut state = new_game("choose-where", None);
        let a = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let c = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 3), json!({}));

        let events = run(
            &mut state,
            choose_where(json!({ "side": "enemy", "of": ["unit"] }), |_ctx, card| {
                matches!(card.map(|card| &card.zone), Some(Zone::Field { lane, .. }) if *lane != 2)
            }),
        );

        assert_eq!(pending_of(&state).player_id, PlayerId::P1);
        assert_eq!(pending_of(&state).kind, PromptKind::Target);
        assert_eq!(
            pending_of(&state).options.iter().map(|option| option.selection.clone()).collect::<Vec<_>>(),
            vec![Selection::Instance { instance_id: a.id.clone() }, Selection::Instance { instance_id: c.id.clone() }]
        );
        assert_eq!(
            pending_of(&state).options.iter().map(|option| option.key.clone()).collect::<Vec<_>>(),
            vec![format!("instance:{}", a.id), format!("instance:{}", c.id)]
        );
        assert_eq!(events.iter().map(|event| event.event_type()).collect::<Vec<_>>(), vec![GameEventType::PromptOpened]);
    }

    #[test]
    fn names_each_hero_to_the_controller_as_your_hero_or_enemy_hero_keyed_by_seat() {
        let mut state = new_game("choose-where-heroes", None);

        run(&mut state, choose_where(json!({ "side": "any", "of": ["hero"] }), |_ctx, card| card.is_none()));

        assert_eq!(
            pending_of(&state).options.iter().map(|option| option.label.clone()).collect::<Vec<_>>(),
            vec!["Your hero".to_string(), "Enemy hero".to_string()]
        );
        assert_eq!(
            pending_of(&state).options.iter().map(|option| option.key.clone()).collect::<Vec<_>>(),
            vec!["hero:p1".to_string(), "hero:p2".to_string()]
        );
    }

    #[test]
    fn asks_nothing_when_the_condition_admits_no_card_the_effect_fizzles() {
        let mut state = new_game("choose-where-none", None);
        put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));

        let events = run(&mut state, choose_where(json!({ "side": "enemy" }), |_ctx, _card| false));

        assert!(state.pending.is_none());
        assert_eq!(events, Vec::<GameEvent>::new());
    }

    #[test]
    fn the_open_prompt_is_plain_data_it_survives_a_json_round_trip() {
        // §9.3.
        let mut state = new_game("choose-where-json", None);
        put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));

        run(&mut state, choose_where(json!({ "side": "enemy" }), |_ctx, _card| true));

        let revived: GameState = serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
        assert_eq!(revived, state);
    }
}
