//! #492, R81: the view's `embiggenCost` is what a play at the card's embiggen price then charges.
//!
//! An "A embiggen B" card in the viewer's own hand carries `cost`, its normal price as it stands, and
//! `embiggenCost`, the embiggen price as it stands (`view_for::with_embiggen_cost`), so the client can
//! show the price the picker's Embiggened card plays at without working one out (CLAUDE.md rule 7).
//! Both are read by the function the play itself is priced by (`play_choices::embiggen_play_cost`,
//! `mana::play_cost`), and this proves it from the outside: the number the view shows is the
//! `costPaid` of the `embiggen: true` play `reduce` then accepts, the mana it takes, and the line
//! `legalActions` offers that play at, with no cost change and under a discount (the card's own, a
//! player's, and R363's "(4)+ Cost" threshold, which reaches the embiggen price and not the base one)
//! and a surcharge, on both faces. Driven with the engine's fixture embiggen Field Spell (`pb-`); the
//! cards crate proves the real Embiggen cards again (`crates/cards/tests/cross/embiggen_choices.rs`).

use jackioh_engine::modifiers::add_modifier;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, sink_for};
use crate::rules::fixtures::play_pipeline_b::{
    embiggen_field, only, pb_act, pb_playing, pb_reduce, plays_of,
};

/// The fixture's printed prices: "2 embiggen 4".
const BASE: i32 = 2;
const EMBIGGEN: i32 = 4;

/// What changes the card's price in a case: its own `costMod`, and the player modifiers on p1.
struct Prices {
    name: &'static str,
    cost_mod: i32,
    mods: Vec<ModifierKind>,
    /// What the normal and the embiggen price then come to (§10.5 step 1, R65, R363, R455).
    normal: i32,
    embiggened: i32,
}

fn discount(amount: i32, min_current_cost: Option<i32>) -> ModifierKind {
    ModifierKind::CostDiscount {
        amount,
        only_type: None,
        min_current_cost,
        once_per_turn: None,
    }
}

fn surcharge(amount: i32) -> ModifierKind {
    ModifierKind::CostRule {
        rule: CostRule {
            amount: Some(amount),
            ..CostRule::default()
        },
    }
}

fn cases() -> Vec<Prices> {
    vec![
        Prices {
            name: "no cost change",
            cost_mod: 0,
            mods: vec![],
            normal: BASE,
            embiggened: EMBIGGEN,
        },
        Prices {
            name: "the card's own discount (costMod -1)",
            cost_mod: -1,
            mods: vec![],
            normal: BASE - 1,
            embiggened: EMBIGGEN - 1,
        },
        Prices {
            name: "a player's discount of 1",
            cost_mod: 0,
            mods: vec![discount(1, None)],
            normal: BASE - 1,
            embiggened: EMBIGGEN - 1,
        },
        Prices {
            // R363: Professor Curvature's shape reads the price the other steps left, so it reaches
            // the embiggen price of 4 and not the base price of 2.
            name: "a (4)+ Cost discount (R363)",
            cost_mod: 0,
            mods: vec![discount(1, Some(EMBIGGEN))],
            normal: BASE,
            embiggened: EMBIGGEN - 1,
        },
        Prices {
            name: "a surcharge of 2 (R455)",
            cost_mod: 0,
            mods: vec![surcharge(2)],
            normal: BASE + 2,
            embiggened: EMBIGGEN + 2,
        },
        Prices {
            name: "a discount and a surcharge together",
            cost_mod: -1,
            mods: vec![surcharge(2)],
            normal: BASE + 1,
            embiggened: EMBIGGEN + 1,
        },
    ]
}

/// p1's main phase with the fixture in hand on the face asked for, the case's price changes in place
/// and `mana` to spend. Returns the state and the card's id.
fn holding(prices: &Prices, radiant: bool, mana: i32) -> (GameState, String) {
    let mut state = pb_playing("embiggen-cost");
    let card = only(&in_hand(&mut state, &embiggen_field().id, PlayerId::P1, 1));
    {
        let held = find_instance_mut(&mut state, &card.id).expect("the card is in hand");
        held.radiant = radiant;
        held.cost_mod = prices.cost_mod;
    }
    let turn = state.turn;
    for kind in &prices.mods {
        add_modifier(
            &mut sink_for(&mut state),
            PlayerId::P1,
            ModifierExpiry::ThisTurn { turn },
            kind.clone(),
        );
    }
    state.players.p1.mana.current = mana;
    (state, card.id)
}

/// The card as `view_for` shows it to `viewer`, when that viewer's view lists it.
fn shown(state: &GameState, viewer: PlayerId, id: &str) -> Option<CardView> {
    let view = view_for(state, viewer);
    let hand = if viewer == PlayerId::P1 {
        view.you.hand
    } else {
        view.opponent.hand
    };
    match hand {
        HandView::Cards(cards) => cards.into_iter().find(|card| card.instance_id == id),
        HandView::Count { .. } => None,
    }
}

/// A play of the card into backrow lane 1 at the price asked for.
fn play(state: &GameState, id: &str, embiggen: bool) -> ReduceResult {
    pb_reduce(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": "backrow", "lane": 1 },
            "embiggen": embiggen,
            "playerId": "p1",
        }),
    )
}

/// What an accepted play charged: its `cardPlayed`'s `costPaid`, and the mana it took.
fn charged(before: &GameState, result: &ReduceResult) -> (i32, i32) {
    assert_eq!(result.error, None, "the play is accepted");
    let paid = result
        .events
        .iter()
        .find_map(|event| match event {
            GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
            _ => None,
        })
        .expect("the play announces its price");
    let spent = before.players.p1.mana.current - result.state.players.p1.mana.current;
    (paid, spent)
}

/// The prices `legalActions` offers the card at, as its plays' `embiggen` flags.
fn offered(state: &GameState, id: &str) -> Vec<Option<bool>> {
    let mut flags: Vec<Option<bool>> = plays_of(state, id, PlayerId::P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play { embiggen, .. } => Some(embiggen),
            _ => None,
        })
        .collect();
    flags.dedup();
    flags
}

#[test]
fn r81_the_views_embiggen_cost_is_what_an_embiggened_play_then_charges_with_no_change_and_under_discounts_and_surcharges()
 {
    for prices in cases() {
        for radiant in [false, true] {
            let what = format!("{} (radiant {radiant})", prices.name);
            let (state, id) = holding(&prices, radiant, prices.embiggened);
            let card = shown(&state, PlayerId::P1, &id).expect("p1 sees its own hand card");
            assert_eq!(
                card.cost, prices.normal,
                "{what}: the view's cost is the normal price"
            );
            assert_eq!(
                card.embiggen_cost,
                Some(prices.embiggened),
                "{what}: the view's embiggenCost"
            );

            // The embiggened play is offered at that price, and charges exactly it.
            assert_eq!(
                offered(&state, &id),
                vec![Some(false), Some(true)],
                "{what}: both prices offered"
            );
            let big = charged(&state, &play(&state, &id, true));
            assert_eq!(
                big,
                (prices.embiggened, prices.embiggened),
                "{what}: an embiggen: true play charges embiggenCost"
            );
            // And the normal play charges the view's cost, as before.
            let small = charged(&state, &play(&state, &id, false));
            assert_eq!(
                small,
                (prices.normal, prices.normal),
                "{what}: an embiggen: false play charges cost"
            );
        }
    }
}

#[test]
fn r81_one_mana_short_of_the_views_embiggen_cost_the_embiggened_play_is_neither_offered_nor_accepted() {
    for prices in cases() {
        for radiant in [false, true] {
            let what = format!("{} (radiant {radiant})", prices.name);
            let (state, id) = holding(&prices, radiant, prices.embiggened - 1);
            let card = shown(&state, PlayerId::P1, &id).expect("p1 sees its own hand card");
            assert_eq!(
                card.embiggen_cost,
                Some(prices.embiggened),
                "{what}: the view still shows the price"
            );
            assert_eq!(
                offered(&state, &id),
                vec![Some(false)],
                "{what}: only the normal price is offered"
            );
            assert!(
                play(&state, &id, true).error.is_some(),
                "{what}: the embiggened play is refused"
            );
        }
    }
}

#[test]
fn r81_embiggen_cost_rides_only_the_viewers_own_hand_cards_with_an_embiggen_price() {
    let prices = &cases()[0];
    let (mut state, id) = holding(prices, false, EMBIGGEN);
    // A card with one price has none.
    let plain = only(&in_hand(&mut state, "pb-three-spell", PlayerId::P1, 1));
    assert_eq!(
        shown(&state, PlayerId::P1, &plain.id).map(|card| card.embiggen_cost),
        Some(None),
        "a one-price card carries no embiggenCost"
    );
    // The opponent's view lists p1's hand as a count, so no card of it, and no price, reaches p2.
    assert!(matches!(
        view_for(&state, PlayerId::P2).opponent.hand,
        HandView::Count { .. }
    ));
    assert_eq!(shown(&state, PlayerId::P2, &id), None);

    // R434: once the game is over p2 reads p1's hand in full, and still no embiggenCost on it.
    let over = pb_act(&state, json!({ "type": "concede", "playerId": "p2" }));
    assert!(over.result.is_some());
    let revealed = shown(&over, PlayerId::P2, &id).expect("the hand is revealed at the end");
    assert_eq!(
        revealed.embiggen_cost, None,
        "the opponent's revealed card carries no embiggenCost"
    );
    // The card is played onto the field at its embiggen price: the field's view carries none either.
    let after = pb_act(
        &state,
        json!({ "type": "play", "instanceId": id, "zone": { "row": "backrow", "lane": 1 }, "embiggen": true, "playerId": "p1" }),
    );
    let field =
        serde_json::to_value(view_for(&after, PlayerId::P1).you.backrow).expect("the backrow serialises");
    assert!(
        !field.to_string().contains("embiggenCost"),
        "no backrow view carries embiggenCost"
    );
}
