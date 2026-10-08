//! R766 (issue #473): a card that reaches a graveyard or an exile pile is its printed card again, its
//! price included. R78 and R215 already reset it there; since R766 its `costMod` and `costOverride` go
//! with the rest, so it costs its printed cost (R65) wherever it is read or played from there, while its
//! Radiant face, its tuning, its Brittle and times-played counts and its enchantments stay, and a fused
//! card stays fused at its fused definition's cost. A price a card is given on its way to a hand, or one
//! it keeps going from the field to a hand (R78), is untouched.
//!
//! The engine half, on fixture cards. The loop issue #473 found — C+ #54 Book of Books' (0) Book of
//! Stats replayed for free under C #90 In Too Deep's reward L — is proved on the real cards in
//! `crates/cards/src/scripts/classic/c090_in_too_deep.rs`.

use jackioh_engine::effects::{
    ReturnRandomFromGraveyardArgs, bounce, discard, exile, return_random_from_graveyard,
};
use jackioh_engine::mana::effective_cost;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::subsystems::fuse::fuse;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    grave_spell, grave_unit, in_graveyard, only, pb_playing, pb_reduce, plays_of, second_wind, three_spell,
};

const P1: PlayerId = PlayerId::P1;

/// A vanilla (1) Unit of the fixture catalog (`fixtures/catalog.rs`), and a second one to fuse it with.
const VANILLA_A: &str = "fx-1";
const VANILLA_B: &str = "fx-2";

/// The card under this id as it stands in the state now.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("instance {id} in the state"))
}

fn edit(state: &mut GameState, id: &str, change: impl FnOnce(&mut CardInstance)) {
    change(find_instance_mut(state, id).unwrap_or_else(|| panic!("instance {id} in the state")));
}

/// R65: what the card costs now, wherever it is.
fn price(state: &GameState, id: &str) -> i32 {
    effective_cost(state, &live(state, id), Default::default())
}

/// Apply one effect the way p1's script would, aimed at `target` when there is one.
fn run(state: &mut GameState, effect: Effect, target: Option<&str>) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let targets: Vec<Selection> = target
            .map(|id| Selection::Instance {
                instance_id: id.to_string(),
            })
            .into_iter()
            .collect();
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                targets: Some(targets),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
}

/// `{ target: { of: "chosen" } }`.
fn chosen() -> Value {
    json!({ "target": { "of": "chosen" } })
}

fn discard_it(state: &mut GameState, id: &str) {
    run(state, discard(json_as(chosen())), Some(id));
}

fn price_of(card: &CardInstance) -> (i32, Option<i32>) {
    (card.cost_mod, card.cost_override)
}

#[test]
fn r766_a_card_discarded_with_a_discount_or_a_set_price_reaches_the_graveyard_at_its_printed_cost_and_returns_at_it()
 {
    let mut state = pb_playing("r766-discard");
    let three = three_spell().id;
    let discounted = only(&in_hand(&mut state, &three, P1, 1));
    let free = only(&in_hand(&mut state, &three, P1, 1));
    edit(&mut state, &discounted.id, |card| card.cost_mod = -2);
    edit(&mut state, &free.id, |card| card.cost_override = Some(0));
    assert_eq!(price(&state, &discounted.id), 1);
    assert_eq!(price(&state, &free.id), 0);

    discard_it(&mut state, &discounted.id);
    discard_it(&mut state, &free.id);
    for id in [&discounted.id, &free.id] {
        let card = live(&state, id);
        assert_eq!(card.zone, Zone::Graveyard { player: P1 });
        assert_eq!(price_of(&card), (0, None));
        assert_eq!(price(&state, id), 3);
    }

    // Back to the hand (reward C, a Reminisce): the printed card, at its printed (3).
    run(
        &mut state,
        return_random_from_graveyard(ReturnRandomFromGraveyardArgs {
            count: 2,
            player: None,
        }),
        None,
    );
    for id in [&discounted.id, &free.id] {
        assert_eq!(live(&state, id).zone, Zone::Hand { player: P1 });
        assert_eq!(price(&state, id), 3);
    }
}

#[test]
fn r766_a_card_given_0_in_hand_and_discarded_is_played_from_the_graveyard_at_its_printed_cost_so_it_cannot_loop_for_free()
 {
    // Issue #473's loop on fixtures: a (1) Spell priced (0) in a hand, as C+ #54 prices a Book of Stats,
    // in a graveyard its player may play from (C #28 Second Wind's base face, C #90's reward L).
    let mut state = pb_playing("r766-loop");
    put(
        &mut state,
        &second_wind().id,
        slot(P1, Row::Backrow, 1),
        json!({}),
    );
    let book = only(&in_hand(&mut state, &grave_spell().id, P1, 1));
    edit(&mut state, &book.id, |card| card.cost_override = Some(0));
    assert_eq!(price(&state, &book.id), 0);

    discard_it(&mut state, &book.id);
    assert_eq!(price_of(&live(&state, &book.id)), (0, None));
    assert_eq!(price(&state, &book.id), 1);
    assert_eq!(plays_of(&state, &book.id, P1).len(), 1);

    // Each play from the graveyard pays the printed (1), and the card lands back there at (1) again.
    let mana = state.players.p1.mana.current;
    let played = pb_reduce(
        &state,
        json!({ "type": "play", "instanceId": book.id, "playerId": "p1" }),
    );
    assert_eq!(played.error, None);
    let paid: Vec<i32> = played
        .events
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
            _ => None,
        })
        .collect();
    assert_eq!(paid, vec![1]);
    let mut state = played.state;
    assert_eq!(state.players.p1.mana.current, mana - 1);
    assert_eq!(live(&state, &book.id).zone, Zone::Graveyard { player: P1 });
    assert_eq!(price(&state, &book.id), 1);

    // With no mana left it is not offered, where a (0) kept from the hand would have looped for ever.
    state.players.p1.mana.current = 0;
    assert!(plays_of(&state, &book.id, P1).is_empty());
}

#[test]
fn r766_a_buffed_unit_reaches_the_graveyard_unbuffed_at_its_printed_cost_and_a_bounced_one_keeps_its_price_r78()
 {
    let mut state = pb_playing("r766-unit");
    let dying = put(&mut state, &grave_unit().id, slot(P1, Row::Units, 1), json!({}));
    let bounced = put(&mut state, &grave_unit().id, slot(P1, Row::Units, 2), json!({}));
    for id in [&dying.id, &bounced.id] {
        edit(&mut state, id, |card| {
            card.buffs = AttackHealth { attack: 5, health: 5 };
            card.granted_keywords = vec![Keyword::Taunt];
            card.cost_mod = -1;
            card.cost_override = Some(1);
        });
    }
    assert_eq!(price(&state, &dying.id), 0);

    let mut leaving = live(&state, &dying.id);
    assert_eq!(
        move_to_zone(
            &mut state,
            &mut leaving,
            OffFieldZone::Graveyard,
            Default::default()
        ),
        MoveResult::Moved
    );
    let dead = live(&state, &dying.id);
    assert_eq!(dead.buffs, AttackHealth { attack: 0, health: 0 });
    assert!(dead.granted_keywords.is_empty());
    assert_eq!(price_of(&dead), (0, None));
    assert_eq!(price(&state, &dying.id), 2);

    // A bounce is no graveyard: R78 resets the unit and keeps its price in the hand.
    run(&mut state, bounce(json_as(chosen())), Some(&bounced.id));
    let home = live(&state, &bounced.id);
    assert_eq!(home.zone, Zone::Hand { player: P1 });
    assert_eq!(home.buffs, AttackHealth { attack: 0, health: 0 });
    assert_eq!(price_of(&home), (-1, Some(1)));
    assert_eq!(price(&state, &bounced.id), 0);
}

#[test]
fn r766_r77_a_fused_card_reaching_the_graveyard_stays_fused_and_costs_its_fused_definitions_printed_cost() {
    let mut state = pb_playing("r766-fused");
    let a = only(&in_hand(&mut state, VANILLA_A, P1, 1));
    let b = only(&in_hand(&mut state, VANILLA_B, P1, 1));
    let crafted = {
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        fuse(
            &mut sink,
            json_as(json!({ "ingredients": [a, b], "toHand": "p1" })),
        )
        .expect("Craft a Card's fusion")
    };
    // R77: a fresh hand card at (0), its fused definition's cost min(1 + 1, 4) = (2) underneath.
    assert_eq!(crafted.cost_override, Some(0));
    assert_eq!(price(&state, &crafted.id), 0);
    edit(&mut state, &crafted.id, |card| {
        card.buffs = AttackHealth { attack: 2, health: 2 }
    });

    discard_it(&mut state, &crafted.id);
    let landed = live(&state, &crafted.id);
    assert_eq!(landed.zone, Zone::Graveyard { player: P1 });
    // Still the fusion: its definition, never its ingredients again.
    assert_eq!(landed.def_id, crafted.def_id);
    assert!(state.transient_defs.contains_key(&crafted.def_id));
    assert_eq!(
        state
            .players
            .p1
            .graveyard
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>(),
        vec![crafted.id.clone()]
    );
    // That definition's printed card: no buffs, no price.
    assert_eq!(landed.buffs, AttackHealth { attack: 0, health: 0 });
    assert_eq!(price_of(&landed), (0, None));
    assert_eq!(price(&state, &crafted.id), 2);
}

#[test]
fn r766_a_radiant_card_keeps_its_face_its_tuning_its_counts_and_its_enchantments_and_loses_only_its_price() {
    let mut state = pb_playing("r766-radiant");
    let card = only(&in_hand(&mut state, VANILLA_A, P1, 1));
    edit(&mut state, &card.id, |held| {
        held.radiant = true;
        held.cost_mod = -1;
        held.cost_override = Some(0);
        held.buffs = AttackHealth { attack: 1, health: 1 };
        held.tuning = Some(json_as(json!({ "attack": 2 })));
        held.brittle = Some(BrittleCounter {
            count: 2,
            since: 0,
            printed: None,
        });
        held.times_played = Some(3);
        held.enchantments = Some(vec![Enchantment::CastOnDraw]);
    });
    let before = live(&state, &card.id);

    run(&mut state, exile(json_as(chosen())), Some(&card.id));
    let exiled = live(&state, &card.id);
    assert_eq!(exiled.zone, Zone::Exile { player: P1 });
    assert!(exiled.radiant);
    assert_eq!(exiled.tuning, before.tuning);
    assert_eq!(exiled.brittle, before.brittle);
    assert_eq!(exiled.times_played, Some(3));
    assert_eq!(exiled.enchantments, before.enchantments);
    assert_eq!(exiled.buffs, AttackHealth { attack: 0, health: 0 });
    assert_eq!(price_of(&exiled), (0, None));
    assert_eq!(price(&state, &card.id), 1);
}

#[test]
fn r766_a_card_that_goes_from_a_graveyard_to_an_exile_pile_arrives_at_its_printed_cost_too() {
    let mut state = pb_playing("r766-grave-to-exile");
    let card = in_graveyard(&mut state, &three_spell().id, P1);
    // A price the card was given where it lies (no card gives one there yet; the rule reads any zone).
    edit(&mut state, &card.id, |lying| lying.cost_mod = -3);
    assert_eq!(price(&state, &card.id), 0);

    run(&mut state, exile(json_as(chosen())), Some(&card.id));
    let exiled = live(&state, &card.id);
    assert_eq!(exiled.zone, Zone::Exile { player: P1 });
    assert_eq!(price_of(&exiled), (0, None));
    assert_eq!(price(&state, &card.id), 3);
}
