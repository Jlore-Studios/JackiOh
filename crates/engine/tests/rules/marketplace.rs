//! Stinkyscape's Marketplace (#1853, R1853000–R1853005; #2250, R2250002; #3047, R3047000–R3047004):
//! opened for both players by the first Resource or Credits either gains; at the start of each player's
//! turn `MARKETPLACE_OFFERS_PER_TURN` (4) offers of their own, one copy each, rolled from a table of
//! lots weighted by their counts and replacing the ones before, each perhaps Buffed or Nerfed; the
//! designer's prices, moved as they are rolled, and the Resources' quotes, which a Resource played or
//! discarded fetches and which it shows in braces in its owner's hand; `marketBuy` and `marketSell` in
//! the player's own main phase, for Credits and no mana, listed, refused and shown by one function
//! each, the card bought costing (0) and carrying its lot's Buffs and Nerfs; and a lot an effect adds
//! for the rest of the game.

use jackioh_engine::effects::{
    TuneDirection, TuneRow, add_market_lot, applicable_changes, discard_hand, discard_resource,
    gain_market_credits, gain_resources,
};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};

use crate::rules::fixtures::harness::{new_game, next_nonce, run_effect, setup_catalog, spread, types_of};
use crate::rules::fixtures::marketplace::*;
use crate::rules::fixtures::resources::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

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

/// The engine's own turn start for `player` (`turn::start_turn`), made the active player: the turn
/// counter goes up by one and the Marketplace's turn-start work runs for them.
fn start_turn_of(state: &mut GameState, player: PlayerId) {
    state.active = player;
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        jackioh_engine::turn::start_turn(&mut sink, player);
    }
    state.rng_cursor = rng.cursor();
}

fn market(state: &GameState) -> &Marketplace {
    state.marketplace.as_ref().expect("the Marketplace is open")
}

/// `player`'s rolled offers, each with the index a buy names it by (its place among the lots offered to
/// `player`, `offered_to`), in order (R2250002).
fn own_offers(state: &GameState, player: PlayerId) -> Vec<(i32, MarketOffer)> {
    offered_to(market(state), player)
        .enumerate()
        .filter(|(_, (_, offer))| offer.fixed.is_none())
        .map(|(at, (_, offer))| (at as i32, offer.clone()))
        .collect()
}

/// The index a buy names the lot an effect added for `player` by (R1853005).
fn fixed_offer(state: &GameState, player: PlayerId) -> i32 {
    offered_to(market(state), player)
        .position(|(_, offer)| offer.fixed == Some(true))
        .expect("the lot an effect added") as i32
}

/// Where `player`'s lot `offer` sits in the Marketplace's own list.
fn slot_of(state: &GameState, player: PlayerId, offer: i32) -> usize {
    offered_to(market(state), player)
        .nth(offer as usize)
        .map(|(at, _)| at)
        .expect("a lot offered to the player")
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

fn resource_ids() -> Vec<&'static str> {
    RESOURCES.iter().map(|(id, _)| *id).collect()
}

fn is_fixture_resource(def_id: &str) -> bool {
    resource_ids().contains(&def_id)
}

#[test]
fn r1853000_r2250002_the_first_resource_gained_opens_it_and_rolls_each_player_their_offers() {
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
        vec![GameEventType::AddedToHand],
        "opening reports nothing of its own"
    );
    for player in [P1, P2] {
        let own = own_offers(&state, player);
        assert_eq!(own.len() as i32, MARKETPLACE_OFFERS_PER_TURN, "{player:?}");
        // Each view lists that player's own offers, the ones they may buy, and no other's.
        let shown: Vec<i32> = view_for(&state, player)
            .marketplace
            .expect("both players' views carry it")
            .offers
            .into_iter()
            .map(|view| view.offer)
            .collect();
        assert_eq!(
            shown,
            own.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
            "{player:?}"
        );
    }
    assert_eq!(market(&state).offers[0].owner, Some(P1));
    assert_eq!(
        market(&state).offers.last().map(|offer| offer.owner),
        Some(Some(P2))
    );
}

#[test]
fn r1853000_credits_open_it_and_nothing_else_does() {
    let fresh = new_game("r1853000-fresh", None);
    let json = serde_json::to_value(&fresh).expect("a state serializes");
    assert!(json.get("marketplace").is_none(), "a new game has no Marketplace");

    // A Resource laid out by the setup is no gain, and a gain of no Credits is none either.
    let mut state = board("r1853000-credits", "p1", json!({ "hand": [COMMON_A] }), json!({}));
    assert!(credits(&mut state, P1, 0).is_empty());
    let json = serde_json::to_value(&state).expect("a state serializes");
    assert!(json.get("marketplace").is_none());
    for viewer in [P1, P2] {
        let view = serde_json::to_value(view_for(&state, viewer)).expect("a view serializes");
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
fn r1853001_r2250002_a_roll_draws_four_different_cards_by_the_lots_counts() {
    let state = board("r2250002-roll", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r2250002-roll-draws", 0);
    const ROLLS: i32 = 4_000;
    let mut first: IndexMap<String, i32> = IndexMap::new();
    for _ in 0..ROLLS {
        let offers = roll_offers(&state, &mut rng, MARKET_TABLE, P1);
        assert_eq!(offers.len() as i32, MARKETPLACE_OFFERS_PER_TURN);
        let ids: IndexSet<&str> = offers.iter().map(|offer| offer.def_id.as_str()).collect();
        assert_eq!(ids.len(), offers.len(), "the offers differ: {ids:?}");
        for offer in &offers {
            assert!(MARKET_CARDS.contains(&offer.def_id.as_str()), "{offer:?}");
            assert!(!is_fixture_resource(&offer.def_id), "no Resource is offered");
            assert_eq!(offer.owner, Some(P1));
            assert_eq!(offer.stock, MARKETPLACE_OFFER_STOCK);
            assert_eq!(offer.bought, PerPlayer::new(0, 0));
            assert_eq!(offer.fixed, None);
        }
        *first.entry(offers[0].def_id.clone()).or_default() += 1;
    }
    // The first draw: the named three by 2, the (4) Unit alone by 1, the four Boss-X cards by 1. The
    // fixture pool holds four cards and a roll offers four, so each roll offers every one (R3047000).
    let total = f64::from(4);
    let share = |lot: f64| lot / total;
    let expected = [
        (FREE, share(2.0) / 3.0 + share(1.0) / 4.0),
        (FOUR, share(2.0) / 3.0 + share(1.0) + share(1.0) / 4.0),
        (FIVE, share(2.0) / 3.0 + share(1.0) / 4.0),
        (CHAOS, share(1.0) / 4.0),
    ];
    for (id, chance) in expected {
        let seen = f64::from(first.get(id).copied().unwrap_or(0)) / f64::from(ROLLS);
        assert!(
            (seen - chance).abs() <= 0.03,
            "{id} comes first {seen} of the time against {chance}"
        );
    }
}

#[test]
fn r1853001_a_lot_is_radiant_one_time_in_five() {
    let state = board("r1853001-radiant", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r1853001-radiant-rolls", 0);
    let (mut lots, mut radiant) = (0, 0);
    for _ in 0..2_000 {
        let offers = roll_offers(&state, &mut rng, MARKET_TABLE, P1);
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
fn r2250002_each_turn_start_rolls_that_player_new_offers_and_leaves_the_others() {
    let mut state = board("r2250002-turns", "p1", json!({}), json!({}));
    credits(&mut state, P1, 500);
    let (first, _) = own_offers(&state, P1)[0].clone();
    let bought = buy(&state, P1, first);
    assert_eq!(bought.error, None);
    let mut state = bought.state;
    let p1_before = own_offers(&state, P1);
    assert_eq!(p1_before[0].1.bought[P1], 1);

    // p2's turn starts: p2's offers are rolled again, p1's stay as they are, bought copy and all.
    let p2_before = own_offers(&state, P2);
    start_turn_of(&mut state, P2);
    let p1_kept: Vec<MarketOffer> = own_offers(&state, P1)
        .into_iter()
        .map(|(_, offer)| offer)
        .collect();
    assert_eq!(
        p1_kept,
        p1_before
            .iter()
            .map(|(_, offer)| offer.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(own_offers(&state, P2).len() as i32, MARKETPLACE_OFFERS_PER_TURN);
    assert_ne!(
        own_offers(&state, P2)
            .into_iter()
            .map(|(_, offer)| offer)
            .collect::<Vec<_>>(),
        p2_before.into_iter().map(|(_, offer)| offer).collect::<Vec<_>>(),
        "p2's offers are fresh"
    );

    // p1's turn starts: p1's old offers go, four fresh ones with no copy bought follow.
    start_turn_of(&mut state, P1);
    let fresh = own_offers(&state, P1);
    assert_eq!(fresh.len() as i32, MARKETPLACE_OFFERS_PER_TURN);
    assert!(
        fresh
            .iter()
            .all(|(_, offer)| offer.bought == PerPlayer::new(0, 0))
    );
    assert_eq!(
        market(&state).offers.len() as i32,
        2 * MARKETPLACE_OFFERS_PER_TURN,
        "each player's four, no more"
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
fn r1853002_the_price_rule_and_an_offer_moves_within_a_tenth_as_it_is_rolled() {
    register();
    let price = |id: &str, radiant: bool| market_card_price(def_of(None, id), radiant);
    assert_eq!(price(FREE, false), 8, "a (0) card (R3047000)");
    assert_eq!(price(CHAOS, false), 15);
    assert_eq!(price(FOUR, false), 45);
    assert_eq!(price(FIVE, false), 60, "(5) is 45 and 15 for the mana past (4)");
    assert_eq!(price(FIVE, true), 120, "twice that on a Radiant lot");

    let state = board("r1853002-rule", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r1853002-rule-rolls", 0);
    let mut moved = false;
    for _ in 0..50 {
        for offer in roll_offers(&state, &mut rng, MARKET_TABLE, P1) {
            let base = tuned_price(
                price(&offer.def_id, offer.radiant == Some(true)),
                offer.buffs.unwrap_or(0),
                offer.nerfs.unwrap_or(0),
            );
            assert_eq!(offer.base, base, "{offer:?}");
            let spread = base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
            assert!(
                (base - spread..=base + spread).contains(&offer.price),
                "{offer:?} moves within a tenth of its base"
            );
            moved |= offer.price != offer.base;
        }
    }
    assert!(moved, "prices move as they are rolled");

    // A rolled offer's price stays until the offer goes.
    let mut state = board("r1853002-kept", "p1", json!({}), json!({}));
    credits(&mut state, P1, 20);
    let mine = own_offers(&state, P1);
    start_turn_of(&mut state, P2);
    assert_eq!(own_offers(&state, P1), mine, "p2's turn moves no price of p1's");
}

#[test]
fn r1853002_r2250001_quotes_move_each_turn_and_a_discarded_resource_fetches_this_turns() {
    let mut state = board(
        "r1853002-quotes",
        "p1",
        json!({ "hand": ["fx-1", RARE, { "def": RARE, "radiant": true }] }),
        json!({}),
    );
    credits(&mut state, P2, 20);
    let mut sorted = resource_ids();
    sorted.sort();
    let mut rare_quotes: Vec<(i32, i32)> = Vec::new();
    for turn in 0..4 {
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
        start_turn_of(&mut state, if turn % 2 == 0 { P2 } else { P1 });
    }
    assert!(
        rare_quotes.iter().any(|quote| *quote != rare_quotes[0]),
        "sell prices move from turn to turn: {rare_quotes:?}"
    );

    // While the Marketplace is open, a Resource discarded from the hand pays this turn's quote.
    let quote = market(&state)
        .quotes
        .iter()
        .find(|quote| quote.def_id == RARE)
        .cloned()
        .expect("the Rare is quoted");
    let before = state.players[P1].market_credits.unwrap_or(0);
    run_effect(
        &mut state,
        discard_resource(json_as(json!({ "defIds": [RARE] }))),
        P1,
    );
    let after_base = state.players[P1].market_credits.unwrap_or(0);
    assert_eq!(after_base - before, quote.base, "the base face's quote");
    run_effect(
        &mut state,
        discard_resource(json_as(json!({ "defIds": [RARE] }))),
        P1,
    );
    assert_eq!(
        state.players[P1].market_credits.unwrap_or(0) - after_base,
        quote.radiant,
        "the Radiant face's quote"
    );
}

#[test]
fn r1853003_a_buy_pays_credits_not_mana_and_arrives_hidden() {
    let mut state = board("r1853003-buy", "p1", json!({}), json!({}));
    credits(&mut state, P1, 200);
    let (at, offer) = own_offers(&state, P1)[0].clone();
    let mana = state.players[P1].mana.current;
    assert!(buys_listed(&state, P1).contains(&at));
    let out = buy(&state, P1, at);
    assert_eq!(out.error, None);
    // The lot's Buffs and Nerfs follow the card's arrival, one event each (R3047003).
    let mut expected = vec![
        GameEventType::MarketBought,
        GameEventType::MarketCreditsChanged,
        GameEventType::AddedToHand,
    ];
    expected.extend((0..offer.buffs.unwrap_or(0)).map(|_| GameEventType::Upgraded));
    expected.extend((0..offer.nerfs.unwrap_or(0)).map(|_| GameEventType::Degraded));
    assert_eq!(types_of(&out.events), expected);
    let after = &out.state;
    assert_eq!(after.players[P1].market_credits, Some(200 - offer.price));
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
        (offer.def_id.as_str(), offer.price, offer.radiant)
    );
    let card = find_instance(after, instance_id).expect("the card bought exists");
    assert_eq!(card.zone, Zone::Hand { player: P1 });
    assert_eq!(card.radiant, offer.radiant == Some(true), "on the offer's face");
    assert_eq!(
        offer_view(after, P1, at).map(|view| view.left),
        Some(0),
        "its one copy"
    );
    // p2's view lists p2's own offers, never p1's.
    let theirs: Vec<(String, i32)> = view_for(after, P2)
        .marketplace
        .expect("open")
        .offers
        .into_iter()
        .map(|view| (view.def_id, view.price))
        .collect();
    assert_eq!(
        theirs,
        own_offers(after, P2)
            .into_iter()
            .map(|(_, offer)| (offer.def_id, offer.price))
            .collect::<Vec<_>>()
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
fn r1853003_r2250002_the_listing_the_refusal_and_the_view_agree() {
    let closed = board("r1853003-agree", "p1", json!({}), json!({}));
    refused_everywhere(&closed, 0, "the Marketplace is not open");

    let mut state = closed.clone();
    credits(&mut state, P1, 1000);
    let (at, lot) = own_offers(&state, P1)[0].clone();
    let name = def_of(Some(&state), &lot.def_id).name.clone();
    assert!(buys_listed(&state, P1).contains(&at));
    let shown = offer_view(&state, P1, at).expect("the lot is shown");
    assert_eq!(
        (shown.reason, shown.price),
        (None, lot.price),
        "an affordable lot has no reason"
    );

    let mut theirs = state.clone();
    theirs.active = P2;
    refused_everywhere(&theirs, at, "it is not your turn");
    assert_eq!(offer_view(&theirs, P1, at).and_then(|view| view.reason), None);

    refused_everywhere(&state, 99, "no offer 99 at the Marketplace");
    refused_everywhere(&state, -1, "no offer -1 at the Marketplace");
    // p2's four follow p1's in the Marketplace's list, yet p1 is offered four alone.
    let past = own_offers(&state, P1).len() as i32;
    refused_everywhere(&state, past, &format!("no offer {past} at the Marketplace"));

    let mut spent = state.clone();
    let slot = slot_of(&state, P1, at);
    spent.marketplace.as_mut().expect("open").offers[slot].bought[P1] = lot.stock;
    refused_everywhere(&spent, at, &format!("you have bought {name}"));

    let mut short = state.clone();
    short.players[P1].market_credits = Some(lot.price - 1);
    refused_everywhere(
        &short,
        at,
        &format!(
            "{name} costs {} Credits and you have {}",
            lot.price,
            lot.price - 1
        ),
    );

    let mut full_hand = state.clone();
    while (full_hand.players[P1].hand.len() as i32) < HAND_CAP {
        let card = new_instance(&mut full_hand, "fx-5", P1, Zone::Hand { player: P1 });
        full_hand.players[P1].hand.push(card);
    }
    refused_everywhere(&full_hand, at, "your hand is full");
}

#[test]
fn r1853003_r2250001_a_resource_is_not_sold_and_the_view_lists_no_sale_of_it() {
    let mut state = board("r1853003-no-sale", "p1", json!({ "hand": [COMMON_A] }), json!({}));
    credits(&mut state, P1, 10);
    let id = resources_in_hand_of(&state, P1)[0].id.clone();
    assert!(sales_listed(&state, P1).is_empty());
    assert_eq!(
        sell(&state, P1, &id).error,
        Some(format!("no card {id} in your hand sells at the Marketplace"))
    );
    assert!(view_for(&state, P1).marketplace.expect("open").sales.is_empty());
}

/// p1 may do one thing only: buy their first offer that is a Spell, with exactly its price in Credits, no
/// mana, an empty hand and no other offer left to them. A Spell, so that once it is bought and played no
/// Unit is left to switch position (R3047001: the card bought costs (0), so it is something to play).
fn only_a_buy(seed: &str) -> (GameState, i32) {
    let mut state = board(seed, "p1", json!({ "hand": [] }), json!({}));
    credits(&mut state, P1, 1);
    state.players[P1].mana.current = 0;
    let own = own_offers(&state, P1);
    let (chosen, lot) = own
        .iter()
        .find(|(_, offer)| [FREE, CHAOS].contains(&offer.def_id.as_str()))
        .cloned()
        .expect("a roll offers each of the four fixture cards, two of them Spells");
    let slots: Vec<usize> = own
        .iter()
        .filter(|(at, _)| *at != chosen)
        .map(|(at, _)| slot_of(&state, P1, *at))
        .collect();
    let market = state.marketplace.as_mut().expect("open");
    for slot in slots {
        market.offers[slot].bought[P1] = market.offers[slot].stock;
    }
    let price = lot.price;
    state.players[P1].market_credits = Some(price);
    assert_eq!(buys_listed(&state, P1), vec![chosen]);
    (state, chosen)
}

#[test]
fn r1853004_a_deal_left_keeps_the_turn_from_ending_by_itself() {
    let (state, chosen) = only_a_buy("r1853004-auto");
    let poke = Action::new(ActionBody::SetAutoEndTurn { enabled: true }, P1, "p0");
    let kept = reduce(&state, &poke);
    assert_eq!(kept.error, None);
    assert_eq!(kept.state.active, P1, "a buy left is something to do (R82)");
    assert!(!types_of(&kept.events).contains(&GameEventType::TurnAutoEnded));
    // The card bought costs (0) (R3047001), so it is something to play: the turn waits for its play, and
    // once that is made and nothing is left, the turn ends by itself.
    let bought = buy(&kept.state, P1, chosen);
    assert_eq!(bought.error, None);
    assert!(!types_of(&bought.events).contains(&GameEventType::TurnAutoEnded));
    assert_eq!(bought.state.active, P1, "the card bought is playable at no mana");
    let card = bought.state.players[P1].hand[0].id.clone();
    let play = legal_actions(&bought.state, P1)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card))
        .expect("the card bought is listed to play");
    let played = reduce(&bought.state, &Action::new(play, P1, next_nonce()));
    assert_eq!(played.error, None);
    assert!(types_of(&played.events).contains(&GameEventType::TurnAutoEnded));
    assert_eq!(played.state.active, P2);
}

#[test]
fn r1853004_the_random_policy_may_deal_and_the_timeout_never_does() {
    let (state, chosen) = only_a_buy("r1853004-policy");
    let offered = jackioh_engine::subsystems::ai_policy::policy_actions(&state, P1, Default::default());
    assert!(
        offered.contains(&buy_of(chosen)),
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
fn r1853005_r2250002_a_fixed_lot_is_its_casters_alone_at_a_fixed_price_renewed_each_turn() {
    let mut state = board("r1853005-fixed", "p1", json!({}), json!({}));
    let events = run_effect(
        &mut state,
        add_market_lot(json_as(json!({ "pool": [CHAOS], "price": CHAOS_PRICE }))),
        P1,
    );
    assert!(events.is_empty(), "adding a lot reports nothing");
    let fixed = fixed_offer(&state, P1);
    assert_eq!(
        fixed, MARKETPLACE_OFFERS_PER_TURN,
        "the Marketplace opened, and the lot follows p1's rolled ones"
    );
    assert_eq!(
        market(&state).offers[slot_of(&state, P1, fixed)],
        MarketOffer {
            def_id: CHAOS.to_string(),
            radiant: None,
            base: CHAOS_PRICE,
            price: CHAOS_PRICE,
            stock: MARKETPLACE_FIXED_LOT_STOCK,
            bought: PerPlayer::new(0, 0),
            owner: Some(P1),
            fixed: Some(true),
            buffs: None,
            nerfs: None,
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

    // Its price never moves, a second copy before its owner's next turn is refused, and each of their
    // turn starts keeps it and renews its copy.
    let mut state = buy(&state, P1, fixed).state;
    assert_eq!(
        buy(&state, P1, fixed).error.as_deref(),
        Some("you have bought Fixture Chaos")
    );
    for player in [P2, P1, P2, P1] {
        start_turn_of(&mut state, player);
        let kept: Vec<&MarketOffer> = market(&state)
            .offers
            .iter()
            .filter(|offer| offer.fixed == Some(true))
            .collect();
        assert_eq!(kept.len(), 1, "kept for the rest of the game");
        assert_eq!(kept[0].price, CHAOS_PRICE, "on turn {}", state.turn);
    }
    let fixed = fixed_offer(&state, P1);
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
        json!({ "hand": ["fx-1", HAND_SALE, { "def": HAND_SALE, "radiant": true }, COMMON_A] }),
        json!({ "hand": ["fx-1", HAND_SALE] }),
    );
    let hand: Vec<String> = state.players[P1]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect();
    let (plain, radiant) = (hand[1].clone(), hand[2].clone());
    credits(&mut state, P1, 10);

    // Quoted after every Resource, each face in its range, turn after turn.
    let mut expected = resource_ids();
    expected.sort();
    expected.push(HAND_SALE);
    for player in [P2, P1, P2] {
        let ids: Vec<&str> = market(&state)
            .quotes
            .iter()
            .map(|quote| quote.def_id.as_str())
            .collect();
        assert_eq!(ids, expected, "the Resources sorted, then the hand-sale card");
        let quote = market(&state).quotes.last().cloned().expect("quoted");
        assert!(in_range(HAND_SALE_PRICE, quote.base), "{quote:?}");
        assert!(in_range(HAND_SALE_RADIANT_PRICE, quote.radiant), "{quote:?}");
        start_turn_of(&mut state, player);
    }
    state.active = P1;
    let quote = market(&state).quotes.last().cloned().expect("quoted");

    // Listed and shown in hand order, the Resource not among them; the other player's is theirs alone.
    assert_eq!(sales_listed(&state, P1), vec![plain.clone(), radiant.clone()]);
    let sales = |viewer: PlayerId, state: &GameState| -> Vec<(String, i32)> {
        view_for(state, viewer)
            .marketplace
            .expect("open")
            .sales
            .into_iter()
            .map(|sale| (sale.instance_id, sale.credits))
            .collect()
    };
    assert_eq!(
        sales(P1, &state),
        vec![(plain.clone(), quote.base), (radiant.clone(), quote.radiant)]
    );
    let theirs = state.players[P2].hand[1].id.clone();
    assert_eq!(
        sales(P2, &state),
        vec![(theirs.clone(), quote.base)],
        "p2 sees its own hand-sale card, not p1's"
    );
    assert_eq!(
        sell(&state, P1, &theirs).error,
        Some(format!("no card {theirs} in your hand sells at the Marketplace"))
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
    assert_eq!(
        create_invariant_monitor(&out.state).hidden(&out.state),
        Vec::<String>::new(),
        "I6: the sale named it to both seats as it went, though it is no token"
    );
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
    let hand: Vec<String> = state.players[P1]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect();
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
        Some(format!(
            "no card {} in your hand sells at the Marketplace",
            hand[0]
        )),
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

// ---------------------------------------------------------------------------------------------
// Issue #3047: four offers a turn, the new price curve, a card bought at (0), Buffed and Nerfed
// offers, and a Resource's quote in braces.

/// The card `offer` names, bought into p1's hand by `out`, from its `marketBought`.
fn bought_in(out: &ReduceResult) -> CardInstance {
    let Some(GameEvent::MarketBought { instance_id, .. }) = out.events.first() else {
        panic!("the buy reports marketBought first: {:?}", out.events);
    };
    find_instance(&out.state, instance_id)
        .expect("the card bought exists")
        .clone()
}

/// p1's main phase with the Marketplace open (p1 holds 1000 Credits) on the first seed `<prefix>-<n>`
/// in which an offer p1 is rolled satisfies `pick`: the position, the offer's index and the offer.
fn opened_with(prefix: &str, pick: impl Fn(&MarketOffer) -> bool) -> (GameState, i32, MarketOffer) {
    let open = |seed: &str| -> GameState {
        let mut state = board(seed, "p1", json!({}), json!({}));
        credits(&mut state, P1, 1000);
        state
    };
    let seed = seed_where(prefix, |seed| {
        own_offers(&open(seed), P1).iter().any(|(_, offer)| pick(offer))
    });
    let state = open(&seed);
    let (at, offer) = own_offers(&state, P1)
        .into_iter()
        .find(|(_, offer)| pick(offer))
        .expect("the seed holds the offer");
    (state, at, offer)
}

/// p1's hand cards as the view lists them.
fn hand_of(state: &GameState, viewer: PlayerId) -> Vec<CardView> {
    match view_for(state, viewer).you.hand {
        HandView::Cards(cards) => cards,
        HandView::Count { count } => panic!("the viewer's own hand is cards, not a count of {count}"),
    }
}

#[test]
fn r3047000_a_turn_rolls_four_offers_of_their_own() {
    assert_eq!(MARKETPLACE_OFFERS_PER_TURN, 4);
    let mut state = board("r3047000-four", "p1", json!({}), json!({}));
    credits(&mut state, P1, 20);
    for player in [P1, P2] {
        assert_eq!(own_offers(&state, player).len(), 4, "{player:?}");
        let view = view_for(&state, player).marketplace.expect("open");
        assert_eq!(view.offers.len(), 4, "{player:?}'s view");
    }
    // The fixture's Boss-X pool holds four cards and a roll offers four, so each roll draws every one.
    let mut cards: Vec<String> = MARKET_CARDS.iter().map(|id| (*id).to_string()).collect();
    cards.sort();
    for player in [P2, P1, P2, P1] {
        start_turn_of(&mut state, player);
        let mut offered: Vec<String> = own_offers(&state, player)
            .into_iter()
            .map(|(_, offer)| offer.def_id)
            .collect();
        offered.sort();
        assert_eq!(offered, cards, "{player:?} on turn {}", state.turn);
    }
    // A table with more cards than a roll takes still offers four of them, all different.
    let rolling = board("r3047000-nine", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r3047000-nine-rolls", 0);
    for _ in 0..50 {
        let offers = roll_offers(&rolling, &mut rng, COST_TABLE, P1);
        let ids: IndexSet<&str> = offers.iter().map(|offer| offer.def_id.as_str()).collect();
        assert_eq!((offers.len(), ids.len()), (4, 4));
    }
}

#[test]
fn r3047000_the_base_price_at_each_cost_from_0_to_8_within_the_spread() {
    register();
    let curve = [8, 15, 25, 35, 45, 60, 75, 90, 105];
    for (cost, (id, price)) in COST_CARDS.iter().zip(curve).enumerate() {
        let def = def_of(None, id);
        let mana = cost as i32;
        assert_eq!(market_card_price(def, false), price, "a ({mana}) card");
        assert_eq!(market_card_price(def, true), 2 * price, "a Radiant ({mana}) card");
        let by_rule = if mana == 0 {
            MARKETPLACE_ZERO_COST_PRICE
        } else {
            MARKETPLACE_PRICE_BASE
                + MARKETPLACE_CREDITS_PER_MANA * mana.min(MARKETPLACE_FULL_PRICE_MANA)
                + MARKETPLACE_CREDITS_PER_MANA_PAST * (mana - MARKETPLACE_FULL_PRICE_MANA).max(0)
        };
        assert_eq!(price, by_rule, "({mana}) by the rule");
    }
    // Rolled, an untuned offer's base is that price and its price moves within a tenth of it; every
    // cost is offered.
    let state = board("r3047000-curve", "p1", json!({}), json!({}));
    let mut rng = Rng::new("r3047000-curve-rolls", 0);
    let mut seen = [false; 9];
    for _ in 0..400 {
        for offer in roll_offers(&state, &mut rng, COST_TABLE, P1) {
            let cost = COST_CARDS
                .iter()
                .position(|id| *id == offer.def_id)
                .expect("a cost card");
            seen[cost] = true;
            let radiant = offer.radiant == Some(true);
            let designer = curve[cost]
                * if radiant {
                    MARKETPLACE_RADIANT_PRICE_MULTIPLIER
                } else {
                    1
                };
            assert_eq!(
                offer.base,
                tuned_price(designer, offer.buffs.unwrap_or(0), offer.nerfs.unwrap_or(0)),
                "{offer:?}"
            );
            if offer.buffs.is_none() && offer.nerfs.is_none() {
                assert_eq!(offer.base, designer, "{offer:?}");
            }
            let spread = offer.base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
            assert!(
                (offer.base - spread..=offer.base + spread).contains(&offer.price),
                "{offer:?} moves within a tenth of its base"
            );
        }
    }
    assert_eq!(seen, [true; 9], "each cost from (0) to (8) was offered");
}

#[test]
fn r3047001_a_bought_card_costs_0_in_hand_and_its_printed_cost_again_in_the_graveyard() {
    let mut state = board("r3047001-free", "p1", json!({}), json!({}));
    credits(&mut state, P1, 1000);
    // Every roll offers each of the four fixture cards, so the (4) Unit is one of p1's offers.
    let (at, offer) = own_offers(&state, P1)
        .into_iter()
        .find(|(_, offer)| offer.def_id == FOUR)
        .expect("the (4) Unit is offered");
    let out = buy(&state, P1, at);
    assert_eq!(out.error, None);
    let card = bought_in(&out);
    assert_eq!(card.zone, Zone::Hand { player: P1 });
    assert_eq!(card.cost_override, Some(MARKETPLACE_BOUGHT_COST));
    assert_eq!(MARKETPLACE_BOUGHT_COST, 0);
    assert_eq!(play_cost(&out.state, &card), 0, "it costs (0) in hand, not (4)");
    let shown = hand_of(&out.state, P1)
        .into_iter()
        .find(|shown| shown.instance_id == card.id)
        .expect("the hand shows it");
    assert_eq!(shown.cost, 0, "the view shows (0)");
    assert_eq!(
        out.state.players[P1].market_credits,
        Some(1000 - offer.price),
        "the Credits are the price, and the mana nothing"
    );

    // In a graveyard it costs its printed cost again (R766).
    let mut state = out.state;
    run_effect(&mut state, discard_hand(json_as(json!({}))), P1);
    let card = find_instance(&state, &card.id).expect("it is in the graveyard");
    assert!(matches!(card.zone, Zone::Graveyard { .. }), "{:?}", card.zone);
    assert_eq!(card.cost_override, None);
    assert_eq!(play_cost(&state, card), 4);

    // A Spell bought is free in hand as well.
    let mut state = board("r3047001-spell", "p1", json!({}), json!({}));
    credits(&mut state, P1, 1000);
    let (at, _) = own_offers(&state, P1)
        .into_iter()
        .find(|(_, offer)| offer.def_id == CHAOS)
        .expect("the (1) Spell is offered");
    let spell = bought_in(&buy(&state, P1, at));
    assert_eq!(spell.cost_override, Some(0));
}

#[test]
fn r3047002_a_price_compounds_each_step_and_rounds_down_once() {
    assert_eq!(tuned_price(100, 0, 0), 100);
    assert_eq!(tuned_price(100, 1, 0), MARKETPLACE_BUFF_PRICE_PERCENT);
    assert_eq!(tuned_price(100, 2, 0), 144, "120% of 120%, not 140%");
    assert_eq!(tuned_price(100, 0, 1), MARKETPLACE_NERF_PRICE_PERCENT);
    assert_eq!(tuned_price(100, 0, 2), 72, "85% of 85% is 72.25, rounded down");
    // Rounded once, at the end: 8 × 1.2³ is 13.824, where rounding each step would give 9, 10, 12.
    assert_eq!(tuned_price(8, 3, 0), 13);
    assert_eq!(tuned_price(45, 0, 3), 27, "45 × 0.85³ is 27.6");
    // Radiant first, then the percent: a Radiant (5) lot, 120, Nerfed once.
    assert_eq!(tuned_price(2 * 60, 0, 1), 102);
    // A Nerfed lot is never free within the bound, whatever it costs; a Buffed one never overflows.
    for base in MARKETPLACE_ZERO_COST_PRICE..=2 * market_card_price(&COST_DEFS[8], false) {
        for steps in 0..=MARKETPLACE_TUNE_MAX {
            assert!(tuned_price(base, 0, steps) >= 1, "{base} after {steps} Nerfs");
            assert!(tuned_price(base, steps, 0) >= base, "{base} after {steps} Buffs");
        }
    }
}

#[test]
fn r3047002_a_run_of_rolls_stops_at_the_bound() {
    let mut rng = Rng::new("r3047002-bound", 0);
    assert_eq!(
        repeated_rolls(&mut rng, 1.0),
        MARKETPLACE_TUNE_MAX,
        "a roll that always lands stops"
    );
    assert_eq!(
        rng.cursor(),
        MARKETPLACE_TUNE_MAX as u32,
        "one draw for each roll made, none past the bound"
    );
    let mut rng = Rng::new("r3047002-bound", 0);
    assert_eq!(repeated_rolls(&mut rng, 0.0), 0);
    assert_eq!(
        rng.cursor(),
        1,
        "a roll that fails ends the run after its one draw"
    );
}

#[test]
fn r3047002_a_roll_draws_the_card_radiant_buffs_nerfs_then_the_spread_in_that_order() {
    let state = board("r3047002-order", "p1", json!({}), json!({}));
    let four = def_of(None, FOUR).clone();
    let (mut buffed, mut nerfed, mut plain) = (0, 0, 0);
    for n in 0..400 {
        let seed = format!("r3047002-order-{n}");
        let mut rng = Rng::new(&seed, 0);
        let offers = roll_offers(&state, &mut rng, ONE_CARD_TABLE, P1);

        // The same draws by hand: the lot, its card, Radiant, Buffs, Nerfs only with none, the spread.
        let mut by_hand = Rng::new(&seed, 0);
        by_hand.int(1);
        by_hand.int(1);
        let radiant = by_hand.chance(MARKETPLACE_RADIANT_CHANCE);
        let mut buffs = 0;
        while buffs < MARKETPLACE_TUNE_MAX && by_hand.chance(MARKETPLACE_BUFF_CHANCE) {
            buffs += 1;
        }
        let mut nerfs = 0;
        if buffs == 0 {
            while nerfs < MARKETPLACE_TUNE_MAX && by_hand.chance(MARKETPLACE_NERF_CHANCE) {
                nerfs += 1;
            }
        }
        let base = tuned_price(market_card_price(&four, radiant), buffs, nerfs);
        let spread = base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
        let price = if spread > 0 {
            base - spread + by_hand.int(2 * spread + 1)
        } else {
            base
        };
        assert_eq!(
            offers,
            vec![MarketOffer {
                def_id: FOUR.to_string(),
                radiant: radiant.then_some(true),
                base,
                price,
                stock: MARKETPLACE_OFFER_STOCK,
                bought: PerPlayer::new(0, 0),
                owner: Some(P1),
                fixed: None,
                buffs: (buffs > 0).then_some(buffs),
                nerfs: (nerfs > 0).then_some(nerfs),
            }],
            "{seed}"
        );
        assert_eq!(
            rng.cursor(),
            by_hand.cursor(),
            "{seed}: the same draws and no more"
        );
        match (buffs, nerfs) {
            (0, 0) => plain += 1,
            (0, _) => nerfed += 1,
            _ => buffed += 1,
        }
    }
    assert!(buffed > 0 && nerfed > 0 && plain > 0, "{buffed} {nerfed} {plain}");
}

#[test]
fn r3047002_over_many_seeds_an_offer_is_never_both_buffed_and_nerfed() {
    let state = board("r3047002-many", "p1", json!({}), json!({}));
    let (mut offers_seen, mut buffed, mut nerfed, mut twice, mut once) = (0, 0, 0, 0, 0);
    for n in 0..3_000 {
        let mut rng = Rng::new(&format!("r3047002-many-{n}"), 0);
        for offer in roll_offers(&state, &mut rng, MARKET_TABLE, P1) {
            offers_seen += 1;
            assert!(
                offer.buffs.is_none() || offer.nerfs.is_none(),
                "never both: {offer:?}"
            );
            for count in [offer.buffs, offer.nerfs].into_iter().flatten() {
                assert!(
                    (1..=MARKETPLACE_TUNE_MAX).contains(&count),
                    "absent, never 0: {offer:?}"
                );
            }
            let designer = market_card_price(def_of(None, &offer.def_id), offer.radiant == Some(true));
            let tuned = tuned_price(designer, offer.buffs.unwrap_or(0), offer.nerfs.unwrap_or(0));
            assert_eq!(offer.base, tuned, "{offer:?}");
            if offer.buffs.is_some() {
                buffed += 1;
                assert!(offer.base > designer, "a Buff raises the price: {offer:?}");
            }
            if offer.nerfs.is_some() {
                nerfed += 1;
                assert!(offer.base < designer, "a Nerf lowers the price: {offer:?}");
            }
            twice += i32::from(offer.buffs == Some(2));
            once += i32::from(offer.nerfs == Some(1));
        }
    }
    let share = |count: i32| f64::from(count) / f64::from(offers_seen);
    // A Buff is 0.2 of the offers; a Nerf 0.1 of those with no Buff; Buffed twice 0.2² × 0.8; Nerfed once
    // 0.8 × 0.1 × 0.9.
    assert!((0.17..=0.23).contains(&share(buffed)), "Buffed {}", share(buffed));
    assert!((0.06..=0.10).contains(&share(nerfed)), "Nerfed {}", share(nerfed));
    assert!(
        (0.02..=0.045).contains(&share(twice)),
        "Buffed twice {}",
        share(twice)
    );
    assert!(
        (0.05..=0.095).contains(&share(once)),
        "Nerfed once {}",
        share(once)
    );
}

#[test]
fn r3047003_a_buy_lands_each_buff_of_the_lot_on_its_card_after_marketbought() {
    let (state, at, offer) = opened_with("r3047003-buff", |offer| {
        offer.buffs == Some(2) && [FOUR, FIVE].contains(&offer.def_id.as_str())
    });
    let designer = market_card_price(def_of(None, &offer.def_id), offer.radiant == Some(true));
    assert_eq!(offer.base, tuned_price(designer, 2, 0));
    assert!(offer.base > designer, "two Buffs raise the price");
    let shown = offer_view(&state, P1, at).expect("shown");
    assert_eq!(
        (shown.buffs, shown.nerfs, shown.price),
        (Some(2), None, offer.price)
    );
    assert_eq!(
        offer_view(&state, P2, 0).map(|view| (view.buffs, view.nerfs)),
        own_offers(&state, P2)
            .first()
            .map(|(_, theirs)| (theirs.buffs, theirs.nerfs)),
        "each viewer reads the counts of their own lots"
    );

    let out = buy(&state, P1, at);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![
            GameEventType::MarketBought,
            GameEventType::MarketCreditsChanged,
            GameEventType::AddedToHand,
            GameEventType::Upgraded,
            GameEventType::Upgraded,
        ],
        "each Buff is a Buff, after the buy's own events"
    );
    let card = bought_in(&out);
    assert_eq!(out.state.players[P1].market_credits, Some(1000 - offer.price));
    for event in &out.events[3..] {
        let GameEvent::Upgraded {
            instance_id,
            change,
            hidden_from,
            ..
        } = event
        else {
            panic!("an upgrade: {event:?}");
        };
        assert_eq!(*instance_id, card.id);
        assert_ne!(*change, TuningChange::None, "a Unit has a row to Buff");
        assert_eq!(
            *hidden_from,
            Some(vec![P2]),
            "the other seat reads nothing of it (R177)"
        );
    }
    assert!(card.tuning.is_some(), "the card carries the Buffs");
    assert_eq!(play_cost(&out.state, &card), 0, "and still costs (0)");
}

#[test]
fn r3047003_a_buy_lands_each_nerf_of_the_lot_sparing_the_cost_so_the_card_stays_free() {
    let (state, at, offer) = opened_with("r3047003-nerf", |offer| {
        offer.nerfs == Some(1) && [FOUR, FIVE].contains(&offer.def_id.as_str())
    });
    let designer = market_card_price(def_of(None, &offer.def_id), offer.radiant == Some(true));
    assert_eq!(offer.base, tuned_price(designer, 0, 1));
    assert!(offer.base < designer, "a Nerf lowers the price");
    let shown = offer_view(&state, P1, at).expect("shown");
    assert_eq!((shown.buffs, shown.nerfs), (None, Some(1)));
    let out = buy(&state, P1, at);
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![
            GameEventType::MarketBought,
            GameEventType::MarketCreditsChanged,
            GameEventType::AddedToHand,
            GameEventType::Degraded,
        ]
    );
    let card = bought_in(&out);
    assert_eq!(play_cost(&out.state, &card), 0);
    // A plain Nerf could lift the (0) card's cost, as the cost row is open to a card that costs (0).
    assert!(applicable_changes(&out.state, &card, TuneDirection::Degrade).contains(&TuneRow::Cost));

    // Over many Nerfed lots: no Nerf ever draws the cost row, so every card bought stays at (0).
    let mut landed = 0;
    for n in 0..120 {
        let mut state = board(&format!("r3047003-nerfs-{n}"), "p1", json!({}), json!({}));
        credits(&mut state, P1, 1000);
        for (at, offer) in own_offers(&state, P1) {
            if offer.nerfs.is_none() {
                continue;
            }
            let out = buy(&state, P1, at);
            assert_eq!(out.error, None);
            let card = bought_in(&out);
            assert_eq!(play_cost(&out.state, &card), 0, "{offer:?}");
            for event in &out.events {
                if let GameEvent::Degraded { change, .. } = event {
                    assert!(!matches!(change, TuningChange::Cost { .. }), "{change:?}");
                    landed += 1;
                }
            }
        }
    }
    assert!(landed >= 10, "the seeds Nerfed {landed} cards");
}

#[test]
fn r3047002_a_lot_an_effect_adds_is_never_buffed_or_nerfed_yet_its_card_costs_0() {
    let mut state = board("r3047002-fixed", "p1", json!({}), json!({}));
    run_effect(
        &mut state,
        add_market_lot(json_as(json!({ "pool": [CHAOS], "price": CHAOS_PRICE }))),
        P1,
    );
    credits(&mut state, P1, 100);
    let fixed = fixed_offer(&state, P1);
    let lot = market(&state).offers[slot_of(&state, P1, fixed)].clone();
    assert_eq!((lot.buffs, lot.nerfs, lot.price), (None, None, CHAOS_PRICE));
    let shown = offer_view(&state, P1, fixed).expect("shown");
    assert_eq!((shown.buffs, shown.nerfs), (None, None));
    // It stays that way through a turn start, which renews its copy.
    start_turn_of(&mut state, P1);
    let renewed = market(&state).offers[slot_of(&state, P1, fixed_offer(&state, P1))].clone();
    assert_eq!(
        (renewed.buffs, renewed.nerfs, renewed.price),
        (None, None, CHAOS_PRICE)
    );
    let out = buy(&state, P1, fixed_offer(&state, P1));
    assert_eq!(out.error, None);
    assert_eq!(
        types_of(&out.events),
        vec![
            GameEventType::MarketBought,
            GameEventType::MarketCreditsChanged,
            GameEventType::AddedToHand
        ]
    );
    assert_eq!(
        bought_in(&out).cost_override,
        Some(0),
        "bought at (0) like any lot"
    );
}

#[test]
fn r3047004_a_resource_in_its_owners_hand_shows_what_it_sells_for_while_the_marketplace_is_open() {
    let mut state = board(
        "r3047004-quote",
        "p1",
        json!({ "hand": ["fx-1", COMMON_A, { "def": COMMON_A, "radiant": true }] }),
        json!({ "hand": ["fx-1", COMMON_A] }),
    );
    let shown = |state: &GameState, viewer: PlayerId| -> Vec<(String, Option<Vec<PreviewValue>>)> {
        hand_of(state, viewer)
            .into_iter()
            .map(|card| (card.def_id, card.preview))
            .collect()
    };
    // Closed, nothing is quoted: no card of the hand carries a preview.
    assert!(state.marketplace.is_none());
    assert!(
        shown(&state, P1).iter().all(|(_, preview)| preview.is_none()),
        "no quote while the Marketplace is closed"
    );

    credits(&mut state, P2, 1);
    for turn in [P1, P2, P1, P2, P1] {
        let quote = market(&state)
            .quotes
            .iter()
            .find(|quote| quote.def_id == COMMON_A)
            .cloned()
            .expect("quoted");
        let value = |credits: i32| {
            Some(vec![PreviewValue {
                label: SELL_QUOTE_LABEL.to_string(),
                value: credits,
                display: None,
                ids: None,
            }])
        };
        let hand = shown(&state, P1);
        assert_eq!(hand[0].1, None, "a card that is no Resource shows no quote");
        assert_eq!(
            hand[1].1,
            value(quote.base),
            "the base face's quote, turn {}",
            state.turn
        );
        assert_eq!(hand[2].1, value(quote.radiant), "the Radiant face's quote");
        let theirs = shown(&state, P2);
        assert_eq!(
            (theirs[0].1.clone(), theirs[1].1.clone()),
            (None, value(quote.base)),
            "p2's own hand"
        );

        // The opponent's hand is a count, so it shows p2 nothing of p1's.
        assert!(matches!(
            view_for(&state, P2).opponent.hand,
            HandView::Count { .. }
        ));
        start_turn_of(&mut state, turn);
    }

    // What it shows is what it pays as it is discarded (R2250001).
    let before = state.players[P1].market_credits.unwrap_or(0);
    let shown_base = shown(&state, P1)[1].1.clone().expect("the quote").remove(0).value;
    run_effect(
        &mut state,
        discard_resource(json_as(json!({ "defIds": [COMMON_A] }))),
        P1,
    );
    assert_eq!(state.players[P1].market_credits.unwrap_or(0) - before, shown_base);
}
