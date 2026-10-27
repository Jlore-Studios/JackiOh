//! Stinkyscape's Marketplace (#1853, R1853000–R1853005): opened for both players by the first Resource
//! or Credits either gains, one offering rolled from a table of lots and re-rolled every
//! `MARKETPLACE_CYCLE_TURNS`, with a stock of each lot for each player; the designer's prices, moving
//! each turn, and the Resources' quotes; `marketBuy` and `marketSell` in the player's own main phase, for
//! Credits and no mana, listed, refused and shown by one function each; and a lot an effect adds for the
//! rest of the game. Proved on the fixtures of `fixtures/marketplace.rs` and `fixtures/resources.rs`,
//! with the fixture table registered; the effect harness is `resources.rs`'.

use jackioh_engine::effects::{add_market_lot, gain_market_credits, gain_resources};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};

use crate::rules::fixtures::harness::{new_game, next_nonce, run_effect, setup_catalog, spread};
use crate::rules::fixtures::marketplace::*;
use crate::rules::fixtures::resources::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The fixture catalog with the Resources and the Marketplace's cards, their scripts, and the fixture
/// table.
fn register() {
    setup_catalog();
    register_catalog(marketplace_catalog(resources_catalog(
        registered_catalog().clone(),
    )));
    let mut scripts = registered_scripts();
    scripts.extend(RESOURCE_SCRIPTS.clone());
    register_scripts(scripts);
    register_marketplace_table(MARKET_TABLE);
}

/// Turn 9, `active`'s main phase. Each side holds a vanilla Unit and a library to draw, so no turn
/// ends by itself (§2.5); `p1` and `p2` add to each side's setup.
fn board(seed: &str, active: &str, p1: Value, p2: Value) -> GameState {
    register();
    let side = |extra: Value| {
        spread(
            json!({ "hand": ["fx-1"], "library": ["fx-2", "fx-3", "fx-4"] }),
            extra,
        )
    };
    scenario(json!({ "seed": seed, "active": active, "p1": side(p1), "p2": side(p2) }))
        .state()
        .clone()
}

/// `player` gains `amount` Credits, which opens the Marketplace.
fn credits(state: &mut GameState, player: PlayerId, amount: i32) -> Vec<GameEvent> {
    run_effect(
        state,
        gain_market_credits(json_as(json!({ "amount": amount }))),
        player,
    )
}

/// The engine's own turn start for the active player (`turn::start_turn`): the turn counter goes up by
/// one and the Marketplace's turn-start work runs.
fn start_turn(state: &mut GameState) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let active = sink.state.active;
        jackioh_engine::turn::start_turn(&mut sink, active);
    }
    state.rng_cursor = rng.cursor();
}

fn market(state: &GameState) -> &Marketplace {
    state.marketplace.as_ref().expect("the Marketplace is open")
}

fn buy_of(offer: i32) -> ActionBody {
    ActionBody::MarketBuy { offer }
}

fn sell_of(id: &str) -> ActionBody {
    ActionBody::MarketSell {
        instance_id: id.to_string(),
    }
}

fn deal(state: &GameState, player: PlayerId, body: ActionBody) -> ReduceResult {
    reduce(state, &Action::new(body, player, next_nonce()))
}

fn buy(state: &GameState, player: PlayerId, offer: i32) -> ReduceResult {
    deal(state, player, buy_of(offer))
}

fn sell(state: &GameState, player: PlayerId, id: &str) -> ReduceResult {
    deal(state, player, sell_of(id))
}

fn buys_listed(state: &GameState, player: PlayerId) -> Vec<i32> {
    legal_actions(state, player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::MarketBuy { offer } => Some(offer),
            _ => None,
        })
        .collect()
}

fn sales_listed(state: &GameState, player: PlayerId) -> Vec<String> {
    legal_actions(state, player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::MarketSell { instance_id } => Some(instance_id),
            _ => None,
        })
        .collect()
}

/// The lot `offer` as `viewer`'s view shows it; `None` while the Marketplace is closed or the lot is not
/// offered to them.
fn offer_view(state: &GameState, viewer: PlayerId, offer: i32) -> Option<MarketOfferView> {
    view_for(state, viewer)
        .marketplace?
        .offers
        .into_iter()
        .find(|view| view.offer == offer)
}

fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

fn tray_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    resource_tray_of(state, player)
        .into_iter()
        .map(|card| card.id)
        .collect()
}

/// The fixture Resources' ids.
fn resource_ids() -> Vec<&'static str> {
    RESOURCES.iter().map(|(id, _)| *id).collect()
}

fn is_fixture_resource(def_id: &str) -> bool {
    resource_ids().contains(&def_id)
}

/// The index of the table's fixed-stock (4) Unit lot, the fifth offer.
const FOUR_LOT: i32 = 4;

#[test]
fn r1853000_the_first_resource_gained_opens_it_for_both_players() {
    let mut state = board("r1853000-resource", "p1", json!({}), json!({}));
    assert!(state.marketplace.is_none(), "closed before anything is gained");
    assert!(view_for(&state, P1).marketplace.is_none());
    let events = run_effect(
        &mut state,
        gain_resources(json_as(json!({ "count": 1, "pool": [COMMON_A] }))),
        P1,
    );
    assert_eq!(
        types_of(&events),
        vec![GameEventType::ResourceGained],
        "opening reports nothing of its own"
    );
    assert_eq!(market(&state).rolled_turn, state.turn);
    let offering = |viewer: PlayerId| -> Vec<(String, i32)> {
        view_for(&state, viewer)
            .marketplace
            .expect("both players' views carry it")
            .offers
            .into_iter()
            .map(|offer| (offer.def_id, offer.price))
            .collect()
    };
    assert_eq!(offering(P1).len(), 6, "the fixture table's six lots");
    assert_eq!(offering(P1), offering(P2), "one offering for both players");
}

#[test]
fn r1853000_credits_open_it_and_nothing_else_does() {
    let fresh = new_game("r1853000-fresh", None);
    let json = serde_json::to_value(&fresh).expect("a state serialises");
    assert!(json.get("marketplace").is_none(), "a new game has no Marketplace");

    // A tray laid out by the setup is no gain, and a gain of no Credits is none either.
    let mut state = board("r1853000-credits", "p1", json!({ "tray": [COMMON_A] }), json!({}));
    assert!(credits(&mut state, P1, 0).is_empty());
    let json = serde_json::to_value(&state).expect("a state serialises");
    assert!(json.get("marketplace").is_none());
    for viewer in [P1, P2] {
        let view = serde_json::to_value(view_for(&state, viewer)).expect("a view serialises");
        assert!(view.get("marketplace").is_none(), "{viewer:?}'s view");
    }

    let events = credits(&mut state, P2, 20);
    assert_eq!(
        events,
        vec![GameEvent::MarketCreditsChanged {
            player: P2,
            value: 20
        }]
    );
    assert!(state.marketplace.is_some(), "the first Credits open it");
    assert!(view_for(&state, P1).marketplace.is_some(), "for both players");
}

#[test]
fn r1853001_the_offering_follows_the_table() {
    let mut state = board("r1853001-table", "p1", json!({}), json!({}));
    credits(&mut state, P1, 20);
    let offers = &market(&state).offers;
    assert_eq!(offers.len(), 6, "2 + 2 + 1 + 1 lots");
    let ids: Vec<&str> = offers.iter().map(|offer| offer.def_id.as_str()).collect();
    assert!(ids[0..2].iter().all(|id| is_fixture_resource(id)), "{ids:?}");
    assert!(
        ids[2..4].iter().all(|id| [FREE, FOUR, FIVE].contains(id)),
        "{ids:?}"
    );
    assert_ne!(ids[0], ids[1], "a lot's cards differ");
    assert_ne!(ids[2], ids[3], "a lot's cards differ");
    assert_eq!(ids[4], FOUR);
    assert!(
        MARKET_CARDS.contains(&ids[5]),
        "the set lot draws a Boss-X card: {ids:?}"
    );
    let stocks: Vec<i32> = offers.iter().map(|offer| offer.stock).collect();
    assert!(
        stocks[0..2].iter().all(|stock| (4..=8).contains(stock)),
        "{stocks:?}"
    );
    assert!(
        stocks[2..4].iter().all(|stock| (1..=3).contains(stock)),
        "{stocks:?}"
    );
    assert_eq!(stocks[4..], [6, 2]);
    for offer in offers {
        assert_eq!(offer.bought, PerPlayer::new(0, 0));
        assert_eq!(offer.owner, None);
    }
}

#[test]
fn r1853001_a_lot_is_radiant_one_time_in_five() {
    let state = board("r1853001-radiant", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r1853001-radiant-rolls", 0);
    let (mut lots, mut radiant) = (0, 0);
    for _ in 0..2_000 {
        let offers = roll_offers(&state, &mut rng, MARKET_TABLE);
        lots += offers.len();
        radiant += offers.iter().filter(|offer| offer.radiant == Some(true)).count();
    }
    let share = radiant as f64 / lots as f64;
    assert!(
        (0.17..=0.23).contains(&share),
        "a lot is Radiant {MARKETPLACE_RADIANT_CHANCE} of the time, got {share}"
    );
}

#[test]
fn r1853001_it_rerolls_every_cycle_and_restocks() {
    let mut state = board("r1853001-cycle", "p1", json!({}), json!({}));
    assert_eq!(state.turn, 9);
    credits(&mut state, P1, 500);
    let bought = buy(&state, P1, FOUR_LOT);
    assert_eq!(bought.error, None);
    let mut state = bought.state;
    while state.turn < 9 + MARKETPLACE_CYCLE_TURNS - 1 {
        start_turn(&mut state);
    }
    assert_eq!(state.turn, 14);
    assert_eq!(market(&state).rolled_turn, 9, "the cycle has not run out");
    assert_eq!(market(&state).offers[FOUR_LOT as usize].bought[P1], 1);
    start_turn(&mut state);
    assert_eq!(state.turn, 15);
    assert_eq!(market(&state).rolled_turn, 15, "six turns on, it re-rolls");
    assert_eq!(market(&state).offers.len(), 6);
    assert!(
        market(&state)
            .offers
            .iter()
            .all(|offer| offer.bought == PerPlayer::new(0, 0)),
        "and restocks"
    );
    assert_eq!(
        view_for(&state, P1).marketplace.map(|view| view.rerolls_on),
        Some(21)
    );
}

#[test]
fn r1853001_the_same_seed_rolls_the_same_offering() {
    let open = |seed: &str| -> GameState {
        let mut state = board(seed, "p1", json!({}), json!({}));
        let cursor = state.rng_cursor;
        credits(&mut state, P1, 20);
        assert!(state.rng_cursor > cursor, "the roll draws from the match rng");
        state
    };
    let (one, two) = (open("r1853001-seed"), open("r1853001-seed"));
    assert_eq!(market(&one), market(&two));
}

#[test]
fn r1853002_the_price_rule() {
    register();
    let price = |id: &str, radiant: bool| market_card_price(def_of(None, id), radiant);
    assert_eq!(price(FREE, false), 5, "a (0) card");
    assert_eq!(price(CHAOS, false), 10);
    assert_eq!(price(FOUR, false), 40);
    assert_eq!(price(FIVE, false), 45, "(5) is 40 and 5 for the mana past (4)");
    assert_eq!(price(FIVE, true), 90, "twice that on a Radiant lot");

    // A lot starts at its base: a card's by the rule, a Resource's printed buy on its own face.
    let state = board("r1853002-rule", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r1853002-rule-rolls", 0);
    let (mut base_resource, mut radiant_resource) = (false, false);
    for _ in 0..50 {
        for offer in roll_offers(&state, &mut rng, MARKET_TABLE) {
            let radiant = offer.radiant == Some(true);
            let expected = if !is_fixture_resource(&offer.def_id) {
                price(&offer.def_id, radiant)
            } else if radiant {
                radiant_resource = true;
                RADIANT_BUY
            } else {
                base_resource = true;
                BASE_BUY
            };
            assert_eq!((offer.base, offer.price), (expected, expected), "{offer:?}");
        }
    }
    assert!(
        base_resource && radiant_resource,
        "both faces of a Resource lot were rolled"
    );
}

#[test]
fn r1853002_prices_move_within_a_tenth_and_resource_prices_never_move() {
    let mut state = board("r1853002-moves", "p1", json!({}), json!({}));
    credits(&mut state, P1, 20);
    let mut moved = false;
    for _ in 0..MARKETPLACE_CYCLE_TURNS - 1 {
        start_turn(&mut state);
        assert_eq!(market(&state).rolled_turn, 9, "no re-roll inside the cycle");
        for offer in &market(&state).offers {
            if is_fixture_resource(&offer.def_id) {
                assert_eq!(offer.price, offer.base, "a Resource's buy price never moves");
            } else {
                let spread = offer.base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
                assert!(
                    (offer.base - spread..=offer.base + spread).contains(&offer.price),
                    "{offer:?} moves within a tenth of its base"
                );
                moved |= offer.price != offer.base;
            }
        }
    }
    assert!(moved, "card prices move from turn to turn");
}

#[test]
fn r1853002_sell_quotes_move_in_range_and_the_full_tray_sale_fetches_the_quote() {
    let full: Vec<&str> = (0..RESOURCE_TRAY_CAP).map(|_| COMMON_A).collect();
    let mut state = board("r1853002-quotes", "p1", json!({ "tray": full }), json!({}));
    credits(&mut state, P2, 20);
    let mut sorted = resource_ids();
    sorted.sort();
    let mut rare_quotes: Vec<(i32, i32)> = Vec::new();
    for _ in 0..MARKETPLACE_CYCLE_TURNS - 1 {
        let quotes = &market(&state).quotes;
        let ids: Vec<&str> = quotes.iter().map(|quote| quote.def_id.as_str()).collect();
        assert_eq!(ids, sorted, "every Resource is quoted, in sorted order");
        for quote in quotes {
            assert!((BASE_SELL.0..=BASE_SELL.1).contains(&quote.base), "{quote:?}");
            assert!(
                (RADIANT_SELL.0..=RADIANT_SELL.1).contains(&quote.radiant),
                "{quote:?}"
            );
            if quote.def_id == RARE {
                rare_quotes.push((quote.base, quote.radiant));
            }
        }
        start_turn(&mut state);
    }
    assert!(
        rare_quotes.iter().any(|quote| *quote != rare_quotes[0]),
        "sell prices move from turn to turn: {rare_quotes:?}"
    );

    // R1840001's sale of a Resource past the cap fetches this turn's quote while the Marketplace is open.
    let quoted = |state: &GameState, radiant: bool| -> i32 {
        let quote = market(state)
            .quotes
            .iter()
            .find(|quote| quote.def_id == RARE)
            .expect("the Rare is quoted");
        if radiant { quote.radiant } else { quote.base }
    };
    for radiant in [false, true] {
        let expected = quoted(&state, radiant);
        let events = run_effect(
            &mut state,
            gain_resources(json_as(json!({ "count": 1, "pool": [RARE], "radiant": radiant }))),
            P1,
        );
        let sold: Vec<i32> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::ResourceSold { credits, .. } => Some(*credits),
                _ => None,
            })
            .collect();
        assert_eq!(sold, vec![expected], "radiant: {radiant}");
    }
}

#[test]
fn r1853003_a_buy_pays_credits_not_mana_and_arrives_hidden() {
    let mut state = board("r1853003-buy", "p1", json!({}), json!({}));
    credits(&mut state, P1, 100);
    let offer = market(&state).offers[FOUR_LOT as usize].clone();
    let mana = state.players[P1].mana.current;
    assert!(buys_listed(&state, P1).contains(&FOUR_LOT));
    let out = buy(&state, P1, FOUR_LOT);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![
            GameEventType::MarketBought,
            GameEventType::MarketCreditsChanged,
            GameEventType::AddedToHand
        ]
    );
    let after = &out.state;
    assert_eq!(after.players[P1].market_credits, Some(100 - offer.price));
    assert_eq!(after.players[P1].mana.current, mana, "no mana is paid");
    let GameEvent::MarketBought {
        instance_id,
        def_id,
        radiant,
        credits: paid,
        ..
    } = &out.events[0]
    else {
        panic!("the buy first");
    };
    assert_eq!(
        (def_id.as_str(), *paid, *radiant),
        (FOUR, offer.price, offer.radiant)
    );
    let card = find_instance(after, instance_id).expect("the card bought exists");
    assert_eq!(card.zone, Zone::Hand { player: P1 });
    assert_eq!(card.radiant, offer.radiant == Some(true), "on the lot's face");
    assert_eq!(
        offer_view(after, P1, FOUR_LOT).map(|view| view.left),
        Some(offer.stock - 1)
    );
    assert_eq!(
        offer_view(after, P2, FOUR_LOT).map(|view| view.left),
        Some(offer.stock),
        "a stock each"
    );

    let bought_in = |viewer: PlayerId| -> GameEvent {
        view_for(after, viewer)
            .events
            .into_iter()
            .find(|event| event.event_type() == GameEventType::MarketBought)
            .expect("the view carries the buy")
    };
    assert_eq!(bought_in(P1), out.events[0]);
    assert_eq!(
        bought_in(P2),
        GameEvent::MarketBought {
            player: P1,
            instance_id: HIDDEN_ID.to_string(),
            def_id: HIDDEN_ID.to_string(),
            radiant: None,
            credits: offer.price,
        },
        "the other player learns that a card was bought, not which"
    );
}

#[test]
fn r1853003_a_resource_bought_goes_to_the_tray() {
    let mut state = board("r1853003-resource", "p1", json!({}), json!({}));
    credits(&mut state, P1, 100);
    let offer = market(&state).offers[0].clone();
    assert!(is_fixture_resource(&offer.def_id));
    let hand = state.players[P1].hand.clone();
    let out = buy(&state, P1, 0);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![
            GameEventType::MarketBought,
            GameEventType::MarketCreditsChanged,
            GameEventType::ResourceGained
        ]
    );
    let tray = resource_tray_of(&out.state, P1);
    assert_eq!(tray.len(), 1);
    assert_eq!(tray[0].def_id, offer.def_id);
    assert_eq!(tray[0].radiant, offer.radiant == Some(true));
    assert_eq!(out.state.players[P1].hand, hand, "never the hand");
}

/// Each refusal of a buy of `offer` by p1 is the same three times: `legal_actions` lists no buy of it,
/// the reducer refuses it in `why`'s words, and, while p1 may act, the view gives that lot `why` as its
/// reason.
fn refused_everywhere(state: &GameState, offer: i32, why: &str) {
    assert!(!buys_listed(state, P1).contains(&offer), "{why}: not listed");
    assert_eq!(buy(state, P1, offer).error.as_deref(), Some(why));
    let may_act = state.active == P1 && state.phase == Phase::Main && state.pending.is_none();
    if may_act && let Some(view) = offer_view(state, P1, offer) {
        assert_eq!(view.reason.as_deref(), Some(why), "the view's reason");
    }
}

#[test]
fn r1853003_the_listing_the_refusal_and_the_view_agree() {
    let closed = board("r1853003-agree", "p1", json!({}), json!({}));
    refused_everywhere(&closed, 0, "the Marketplace is not open");

    let mut state = closed.clone();
    credits(&mut state, P1, 1000);
    let lot = market(&state).offers[FOUR_LOT as usize].clone();
    assert!(buys_listed(&state, P1).contains(&FOUR_LOT));
    let shown = offer_view(&state, P1, FOUR_LOT).expect("the lot is shown");
    assert_eq!(
        (shown.reason, shown.price),
        (None, lot.price),
        "an affordable lot has no reason"
    );

    let mut theirs = state.clone();
    theirs.active = P2;
    refused_everywhere(&theirs, FOUR_LOT, "it is not your turn");
    assert_eq!(
        offer_view(&theirs, P1, FOUR_LOT).and_then(|view| view.reason),
        None
    );

    refused_everywhere(&state, 99, "no offer 99 at the Marketplace");
    refused_everywhere(&state, -1, "no offer -1 at the Marketplace");

    let mut spent = state.clone();
    spent.marketplace.as_mut().expect("open").offers[FOUR_LOT as usize].bought[P1] = lot.stock;
    refused_everywhere(
        &spent,
        FOUR_LOT,
        &format!("you have bought this cycle's {} Fixture Four", lot.stock),
    );

    let mut short = state.clone();
    short.players[P1].market_credits = Some(lot.price - 1);
    refused_everywhere(
        &short,
        FOUR_LOT,
        &format!(
            "Fixture Four costs {} Credits and you have {}",
            lot.price,
            lot.price - 1
        ),
    );

    let mut full_hand = state.clone();
    while (full_hand.players[P1].hand.len() as i32) < HAND_CAP {
        let card = new_instance(&mut full_hand, "fx-5", P1, Zone::Hand { player: P1 });
        full_hand.players[P1].hand.push(card);
    }
    refused_everywhere(&full_hand, FOUR_LOT, "your hand is full");
    assert!(
        buys_listed(&full_hand, P1).contains(&0),
        "a Resource needs no hand slot"
    );

    let mut full_tray = state.clone();
    for _ in 0..RESOURCE_TRAY_CAP {
        let mut card = new_instance(&mut full_tray, COMMON_B, P1, Zone::Tray { player: P1 });
        put_in_tray(&mut full_tray, &mut card);
    }
    refused_everywhere(&full_tray, 0, "your Resource tray is full");
    assert!(buys_listed(&full_tray, P1).contains(&FOUR_LOT));
}

#[test]
fn r1853003_a_sale_fetches_the_quote_and_is_public() {
    let mut state = board(
        "r1853003-sale",
        "p1",
        json!({ "tray": [COMMON_A, { "def": RARE, "radiant": true }] }),
        json!({}),
    );
    let tray = tray_ids(&state, P1);
    let hand_card = state.players[P1].hand[0].id.clone();
    assert_eq!(
        sell(&state, P1, &tray[0]).error.as_deref(),
        Some("the Marketplace is not open")
    );
    credits(&mut state, P1, 10);
    assert_eq!(
        sales_listed(&state, P1),
        tray,
        "each tray Resource, in tray order"
    );
    let quote = |def_id: &str| {
        market(&state)
            .quotes
            .iter()
            .find(|quote| quote.def_id == def_id)
            .cloned()
            .expect("quoted")
    };
    let (common, rare) = (quote(COMMON_A).base, quote(RARE).radiant);
    let sales = view_for(&state, P1)
        .marketplace
        .expect("open")
        .sales
        .into_iter()
        .map(|sale| (sale.instance_id, sale.credits))
        .collect::<Vec<_>>();
    assert_eq!(sales, vec![(tray[0].clone(), common), (tray[1].clone(), rare)]);

    let out = sell(&state, P1, &tray[0]);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![GameEventType::MarketSold, GameEventType::MarketCreditsChanged]
    );
    assert_eq!(out.state.players[P1].market_credits, Some(10 + common));
    assert!(
        find_instance(&out.state, &tray[0]).is_none(),
        "it ceased to exist"
    );
    let sold_in_p2 = view_for(&out.state, P2)
        .events
        .into_iter()
        .find(|event| event.event_type() == GameEventType::MarketSold)
        .expect("the other view carries the sale");
    assert_eq!(
        sold_in_p2,
        GameEvent::MarketSold {
            player: P1,
            instance_id: tray[0].clone(),
            def_id: COMMON_A.to_string(),
            radiant: None,
            credits: common,
        },
        "a sale is public"
    );

    let out = sell(&out.state, P1, &tray[1]);
    assert_eq!(out.error, None);
    assert_eq!(out.state.players[P1].market_credits, Some(10 + common + rare));
    assert_eq!(
        out.state.players[P1].resource_tray, None,
        "an empty tray is absent"
    );

    assert_eq!(
        sell(&state, P1, &hand_card).error,
        Some(format!("no Resource {hand_card} in your Resource tray"))
    );
    let mut theirs = state.clone();
    theirs.active = P2;
    assert!(sales_listed(&theirs, P1).is_empty());
    assert_eq!(
        sell(&theirs, P1, &tray[0]).error.as_deref(),
        Some("it is not your turn")
    );
}

#[test]
fn r1853003_the_view_sends_the_viewers_own_stock_and_sales_only() {
    let mut state = board(
        "r1853003-own",
        "p1",
        json!({ "tray": [COMMON_A] }),
        json!({ "tray": [COMMON_B, RARE] }),
    );
    credits(&mut state, P1, 100);
    let stock = market(&state).offers[FOUR_LOT as usize].stock;
    let state = buy(&state, P1, FOUR_LOT).state;
    let p1 = view_for(&state, P1).marketplace.expect("open");
    let p2 = view_for(&state, P2).marketplace.expect("open");
    assert_eq!(p1.offers[FOUR_LOT as usize].left, stock - 1);
    assert_eq!(
        p2.offers[FOUR_LOT as usize].left, stock,
        "p1's purchases are not p2's to read"
    );
    let sale_ids = |view: &MarketplaceView| -> Vec<String> {
        view.sales.iter().map(|sale| sale.instance_id.clone()).collect()
    };
    assert_eq!(sale_ids(&p1), tray_ids(&state, P1));
    assert_eq!(sale_ids(&p2), tray_ids(&state, P2));
    assert!(
        p2.offers.iter().all(|offer| offer.reason.is_none()),
        "not p2's turn"
    );
}

/// p1 may do one thing only: buy the (4) Unit lot, with exactly its price in Credits, no mana, an empty
/// hand and no other lot left to them.
fn only_a_buy(seed: &str) -> GameState {
    let mut state = board(seed, "p1", json!({ "hand": [] }), json!({}));
    credits(&mut state, P1, 1);
    state.players[P1].mana.current = 0;
    let market = state.marketplace.as_mut().expect("open");
    for (at, offer) in market.offers.iter_mut().enumerate() {
        if at as i32 != FOUR_LOT {
            offer.bought[P1] = offer.stock;
        }
    }
    let price = market.offers[FOUR_LOT as usize].price;
    state.players[P1].market_credits = Some(price);
    assert_eq!(buys_listed(&state, P1), vec![FOUR_LOT]);
    state
}

#[test]
fn r1853004_a_deal_left_keeps_the_turn_from_ending_by_itself() {
    let state = only_a_buy("r1853004-auto");
    let poke = Action::new(ActionBody::SetAutoEndTurn { enabled: true }, P1, "p0");
    let kept = reduce(&state, &poke);
    assert_eq!(kept.error, None);
    assert_eq!(kept.state.active, P1, "a buy left is something to do (R82)");
    assert!(!types_of(&kept.events).contains(&GameEventType::TurnAutoEnded));
    // Once it is made and nothing is left, the turn ends by itself.
    let bought = buy(&kept.state, P1, FOUR_LOT);
    assert_eq!(bought.error, None);
    assert!(types_of(&bought.events).contains(&GameEventType::TurnAutoEnded));
    assert_eq!(bought.state.active, P2);
}

#[test]
fn r1853004_the_random_policy_may_deal_and_the_timeout_never_does() {
    let state = only_a_buy("r1853004-policy");
    let offered = jackioh_engine::subsystems::ai_policy::policy_actions(&state, P1, Default::default());
    assert!(
        offered.contains(&buy_of(FOUR_LOT)),
        "the random policy draws from the buys too"
    );
    let timed_out = reduce(&state, &Action::new(ActionBody::Timeout, P1, "srv-t"));
    assert_eq!(timed_out.error, None);
    assert!(!types_of(&timed_out.events).contains(&GameEventType::MarketBought));
    assert_eq!(
        timed_out.state.active, P2,
        "the clock ends the turn and buys nothing"
    );
}

#[test]
fn r1853005_a_fixed_lot_is_its_casters_alone_at_a_fixed_price() {
    let mut state = board("r1853005-fixed", "p1", json!({}), json!({}));
    let events = run_effect(
        &mut state,
        add_market_lot(json_as(json!({ "pool": [CHAOS], "price": CHAOS_PRICE }))),
        P1,
    );
    assert!(events.is_empty(), "adding a lot reports nothing");
    let fixed = market(&state).offers.len() as i32 - 1;
    assert_eq!(
        fixed, 6,
        "the Marketplace opened, and the lot follows the rolled ones"
    );
    let lot = market(&state).offers[fixed as usize].clone();
    assert_eq!(
        lot,
        MarketOffer {
            def_id: CHAOS.to_string(),
            radiant: None,
            base: CHAOS_PRICE,
            price: CHAOS_PRICE,
            stock: MARKETPLACE_FIXED_LOT_STOCK,
            bought: PerPlayer::new(0, 0),
            owner: Some(P1),
        }
    );
    credits(&mut state, P1, 100);
    credits(&mut state, P2, 100);
    let shown = offer_view(&state, P1, fixed).expect("p1 is offered it");
    assert_eq!(
        (shown.fixed, shown.price, shown.left),
        (Some(true), CHAOS_PRICE, 1)
    );
    assert!(offer_view(&state, P2, fixed).is_none(), "p2 is not");
    let mut theirs = state.clone();
    theirs.active = P2;
    assert!(!buys_listed(&theirs, P2).contains(&fixed));
    assert_eq!(
        buy(&theirs, P2, fixed).error,
        Some(format!("no offer {fixed} at the Marketplace"))
    );

    // Its price never moves, and a second copy in one cycle is refused.
    let mut state = buy(&state, P1, fixed).state;
    assert_eq!(
        buy(&state, P1, fixed).error.as_deref(),
        Some("you have bought this cycle's 1 Fixture Chaos")
    );
    while state.turn < 9 + MARKETPLACE_CYCLE_TURNS {
        start_turn(&mut state);
        let kept: Vec<&MarketOffer> = market(&state)
            .offers
            .iter()
            .filter(|offer| offer.owner.is_some())
            .collect();
        assert_eq!(kept.len(), 1, "kept for the rest of the game");
        assert_eq!(kept[0].price, CHAOS_PRICE, "on turn {}", state.turn);
    }
    // The re-roll keeps it after the fresh lots and renews its stock.
    assert_eq!(market(&state).rolled_turn, 15);
    assert_eq!(market(&state).offers[fixed as usize].owner, Some(P1));
    assert_eq!(offer_view(&state, P1, fixed).map(|view| view.left), Some(1));
    assert!(buys_listed(&state, P1).contains(&fixed));

    // A pool with nothing in the catalog adds nothing.
    let before = market(&state).offers.len();
    run_effect(
        &mut state,
        add_market_lot(json_as(json!({ "pool": ["fx-mkt-none"], "price": CHAOS_PRICE }))),
        P1,
    );
    assert_eq!(market(&state).offers.len(), before);
}

/// `board`, with the hand-sale card and its script registered too (R1861000).
fn hand_sale_board(seed: &str, p1: Value, p2: Value) -> GameState {
    register();
    let mut defs = registered_catalog().clone();
    defs.insert(HAND_SALE.to_string(), HAND_SALE_DEF.clone());
    register_catalog(defs);
    let mut scripts = registered_scripts();
    scripts.extend(HAND_SALE_SCRIPTS.clone());
    register_scripts(scripts);
    let side = |extra: Value| {
        spread(
            json!({ "hand": ["fx-1"], "library": ["fx-2", "fx-3", "fx-4"] }),
            extra,
        )
    };
    scenario(json!({ "seed": seed, "active": "p1", "p1": side(p1), "p2": side(p2) }))
        .state()
        .clone()
}

fn in_range(price: ResourcePrice, credits: i32) -> bool {
    (price.sell_min..=price.sell_max).contains(&credits)
}

#[test]
fn r1861000_a_hand_sale_card_is_quoted_after_the_resources_listed_shown_and_sold_from_the_hand() {
    let mut state = hand_sale_board(
        "r1861000-sale",
        json!({ "hand": ["fx-1", HAND_SALE, { "def": HAND_SALE, "radiant": true }], "tray": [COMMON_A] }),
        json!({ "hand": ["fx-1", HAND_SALE] }),
    );
    let tray = tray_ids(&state, P1);
    let hand: Vec<String> = state.players[P1].hand.iter().map(|card| card.id.clone()).collect();
    let (plain, radiant) = (hand[1].clone(), hand[2].clone());
    credits(&mut state, P1, 10);

    // Quoted after every Resource, each face in its range, turn after turn.
    let mut expected = resource_ids();
    expected.sort();
    expected.push(HAND_SALE);
    for _ in 0..3 {
        let ids: Vec<&str> = market(&state).quotes.iter().map(|quote| quote.def_id.as_str()).collect();
        assert_eq!(ids, expected, "the Resources sorted, then the hand-sale card");
        let quote = market(&state).quotes.last().cloned().expect("quoted");
        assert!(in_range(HAND_SALE_PRICE, quote.base), "{quote:?}");
        assert!(in_range(HAND_SALE_RADIANT_PRICE, quote.radiant), "{quote:?}");
        start_turn(&mut state);
    }
    let quote = market(&state).quotes.last().cloned().expect("quoted");

    // Listed and shown after the tray's Resources, in hand order; the other player's is theirs alone.
    assert_eq!(
        sales_listed(&state, P1),
        vec![tray[0].clone(), plain.clone(), radiant.clone()]
    );
    let sales = |viewer: PlayerId, state: &GameState| -> Vec<(String, i32)> {
        view_for(state, viewer)
            .marketplace
            .expect("open")
            .sales
            .into_iter()
            .map(|sale| (sale.instance_id, sale.credits))
            .collect()
    };
    let shown = sales(P1, &state);
    assert_eq!(shown.len(), 3);
    assert_eq!(shown[1..], [(plain.clone(), quote.base), (radiant.clone(), quote.radiant)]);
    let theirs = state.players[P2].hand[1].id.clone();
    assert_eq!(
        sales(P2, &state),
        vec![(theirs.clone(), quote.base)],
        "p2 sees its own hand-sale card, not p1's"
    );
    assert_eq!(
        sell(&state, P1, &theirs).error,
        Some(format!("no Resource {theirs} in your Resource tray"))
    );

    // The sale: public, the quote in Credits, and the card gone from the hand.
    let out = sell(&state, P1, &plain);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![GameEventType::MarketSold, GameEventType::MarketCreditsChanged]
    );
    assert_eq!(out.state.players[P1].market_credits, Some(10 + quote.base));
    assert!(find_instance(&out.state, &plain).is_none(), "it ceased to exist");
    assert_eq!(out.state.players[P1].hand.len(), state.players[P1].hand.len() - 1);
    let sold_in_p2 = view_for(&out.state, P2)
        .events
        .into_iter()
        .find(|event| event.event_type() == GameEventType::MarketSold)
        .expect("the other view carries the sale");
    assert_eq!(
        sold_in_p2,
        GameEvent::MarketSold {
            player: P1,
            instance_id: plain,
            def_id: HAND_SALE.to_string(),
            radiant: None,
            credits: quote.base,
        },
        "a sale is public"
    );
    let out = sell(&out.state, P1, &radiant);
    assert_eq!(out.error, None);
    assert_eq!(
        out.state.players[P1].market_credits,
        Some(10 + quote.base + quote.radiant),
        "a Radiant one sells at the Radiant quote"
    );
}

#[test]
fn r1861000_closed_on_the_opponents_turn_or_unflagged_it_does_not_sell() {
    let mut state = hand_sale_board(
        "r1861000-refused",
        json!({ "hand": ["fx-1", HAND_SALE] }),
        json!({}),
    );
    let hand: Vec<String> = state.players[P1].hand.iter().map(|card| card.id.clone()).collect();
    assert!(sales_listed(&state, P1).is_empty());
    assert_eq!(
        sell(&state, P1, &hand[1]).error.as_deref(),
        Some("the Marketplace is not open")
    );
    assert_eq!(view_for(&state, P1).marketplace, None);

    credits(&mut state, P1, 10);
    assert_eq!(sales_listed(&state, P1), vec![hand[1].clone()]);
    assert_eq!(
        sell(&state, P1, &hand[0]).error,
        Some(format!("no Resource {} in your Resource tray", hand[0])),
        "a hand card that does not sell from the hand"
    );
    let mut theirs = state.clone();
    theirs.active = P2;
    assert!(sales_listed(&theirs, P1).is_empty());
    assert_eq!(
        sell(&theirs, P1, &hand[1]).error.as_deref(),
        Some("it is not your turn")
    );
}
