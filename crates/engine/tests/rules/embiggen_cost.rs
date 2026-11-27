//! #492, R81: the view's `embiggenCost` is what a play at the card's embiggen price then charges.
//!
//! An "A embiggen B" card in the viewer's own hand carries `cost` and `embiggenCost` as they stand,
//! read by the function the play is priced by, so the client never works a price out (CLAUDE.md rule 7).
//! The cards crate proves the real Embiggen cards (`crates/cards/tests/cross/embiggen_choices.rs`).
//!
//! R5109000: the Embiggened price is the normal version's finished price plus the Embiggen premium
//! (B − A), so every rung of R65 and R455 moves both prices alike, and `embiggenCost` with them.

use jackioh_engine::modifiers::add_modifier;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{act, act_result, in_hand, sink_for};
use crate::rules::fixtures::play_pipeline_b::{embiggen_field, only, pb_playing};

/// The fixture's printed prices: "2 embiggen 4".
const BASE: i32 = 2;
const EMBIGGEN: i32 = 4;

/// What changes the card's price in a case: its `cost_override`, its `cost_mod` and p1's player
/// modifiers.
struct Prices {
    name: &'static str,
    cost_override: Option<i32>,
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
            cost_override: None,
            cost_mod: 0,
            mods: vec![],
            normal: BASE,
            embiggened: EMBIGGEN,
        },
        Prices {
            name: "the card's own discount (costMod -1)",
            cost_override: None,
            cost_mod: -1,
            mods: vec![],
            normal: BASE - 1,
            embiggened: EMBIGGEN - 1,
        },
        Prices {
            name: "a player's discount of 1",
            cost_override: None,
            cost_mod: 0,
            mods: vec![discount(1, None)],
            normal: BASE - 1,
            embiggened: EMBIGGEN - 1,
        },
        Prices {
            // R363: Professor Curvature's shape reads the price the other steps left, and R5109000: the
            // Embiggened price is the normal one plus the premium, so a (4)+ discount the normal price of
            // 2 does not reach leaves the Embiggened price at 2 + 2 as well.
            name: "a (4)+ Cost discount (R363)",
            cost_override: None,
            cost_mod: 0,
            mods: vec![discount(1, Some(EMBIGGEN))],
            normal: BASE,
            embiggened: EMBIGGEN,
        },
        Prices {
            name: "a surcharge of 2 (R455)",
            cost_override: None,
            cost_mod: 0,
            mods: vec![surcharge(2)],
            normal: BASE + 2,
            embiggened: EMBIGGEN + 2,
        },
        Prices {
            name: "a discount and a surcharge together",
            cost_override: None,
            cost_mod: -1,
            mods: vec![surcharge(2)],
            normal: BASE + 1,
            embiggened: EMBIGGEN + 1,
        },
    ]
}

/// p1's main phase, the fixture in hand on the face asked for, the case's price changes, `mana` to spend.
fn holding(prices: &Prices, radiant: bool, mana: i32) -> (GameState, String) {
    let mut state = pb_playing("embiggen-cost");
    let card = only(&in_hand(&mut state, &embiggen_field().id, PlayerId::P1, 1));
    {
        let held = find_instance_mut(&mut state, &card.id).expect("the card is in hand");
        held.radiant = radiant;
        held.cost_override = prices.cost_override;
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

fn play(state: &GameState, id: &str, embiggen: bool) -> ReduceResult {
    act_result(
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

/// The prices `legal_actions` offers the card at, as its plays' `embiggen` flags.
fn offered(state: &GameState, id: &str) -> Vec<Option<bool>> {
    let mut flags: Vec<Option<bool>> = plays_in(state, PlayerId::P1, id)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play { embiggen, .. } => Some(embiggen),
            _ => None,
        })
        .collect();
    flags.dedup();
    flags
}

/// R5109000's drift cases, each a rung that read the two printed prices apart before #5109: the
/// Embiggened price is now always the normal one plus the premium, EMBIGGEN − BASE.
fn r5109000_cases() -> Vec<Prices> {
    const PREMIUM: i32 = EMBIGGEN - BASE;
    let set_to = |price: i32| ModifierKind::CostRule {
        rule: CostRule {
            set_to: Some(price),
            ..CostRule::default()
        },
    };
    let threshold = |min_cost: i32, amount: i32| ModifierKind::CostRule {
        rule: CostRule {
            min_cost: Some(min_cost),
            amount: Some(amount),
            ..CostRule::default()
        },
    };
    vec![
        Prices {
            name: "a costOverride of (1) (R65)",
            cost_override: Some(1),
            cost_mod: 0,
            mods: vec![],
            normal: 1,
            embiggened: 1 + PREMIUM,
        },
        Prices {
            name: "a \"costs (1)\" price rule (R455)",
            cost_override: None,
            cost_mod: 0,
            mods: vec![set_to(1)],
            normal: 1,
            embiggened: 1 + PREMIUM,
        },
        Prices {
            name: "Professor Curvature's (4)+ discount (R363)",
            cost_override: None,
            cost_mod: 0,
            mods: vec![discount(1, Some(EMBIGGEN))],
            normal: BASE,
            embiggened: BASE + PREMIUM,
        },
        Prices {
            name: "a (4)+ threshold price rule (R455)",
            cost_override: None,
            cost_mod: 0,
            mods: vec![threshold(EMBIGGEN, 1)],
            normal: BASE,
            embiggened: BASE + PREMIUM,
        },
        Prices {
            name: "a discount of 3, past the floor at 0",
            cost_override: None,
            cost_mod: 0,
            mods: vec![discount(3, None)],
            normal: 0,
            embiggened: PREMIUM,
        },
    ]
}

/// The view shows `prices`' normal and Embiggened prices on both faces, both are offered, and each play
/// charges exactly its price.
fn view_offers_and_charges(prices: &Prices) {
    for radiant in [false, true] {
        let what = format!("{} (radiant {radiant})", prices.name);
        let (state, id) = holding(prices, radiant, prices.embiggened);
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
        // And the normal play charges the view's cost.
        let small = charged(&state, &play(&state, &id, false));
        assert_eq!(
            small,
            (prices.normal, prices.normal),
            "{what}: an embiggen: false play charges cost"
        );
    }
}

#[test]
fn r81_the_views_embiggen_cost_is_what_an_embiggened_play_then_charges_with_no_change_and_under_discounts_and_surcharges()
 {
    for prices in cases() {
        view_offers_and_charges(&prices);
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
    // p2's view lists p1's hand as a count: no card of it, and no price, reaches p2.
    assert!(matches!(
        view_for(&state, PlayerId::P2).opponent.hand,
        HandView::Count { .. }
    ));
    assert_eq!(shown(&state, PlayerId::P2, &id), None);

    // R434: once the game is over p2 reads p1's hand in full, and still no embiggenCost on it.
    let over = act(&state, json!({ "type": "concede", "playerId": "p2" }));
    assert!(over.result.is_some());
    let revealed = shown(&over, PlayerId::P2, &id).expect("the hand is revealed at the end");
    assert_eq!(
        revealed.embiggen_cost, None,
        "the opponent's revealed card carries no embiggenCost"
    );
    // Played onto the field at its embiggen price, the card's field view carries none either.
    let after = act(
        &state,
        json!({ "type": "play", "instanceId": id, "zone": { "row": "backrow", "lane": 1 }, "embiggen": true, "playerId": "p1" }),
    );
    let field =
        serde_json::to_value(view_for(&after, PlayerId::P1).you.backrow).expect("the backrow serializes");
    assert!(
        !field.to_string().contains("embiggenCost"),
        "no backrow view carries embiggenCost"
    );
}

#[test]
fn r5109000_the_embiggened_price_is_the_finished_normal_price_plus_the_premium_under_every_rung() {
    for prices in r5109000_cases() {
        view_offers_and_charges(&prices);
    }
}
