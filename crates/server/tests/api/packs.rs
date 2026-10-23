//! Buying and opening card packs (`src/economy/packs.rs`, SPEC §14.1, §14.3; #1301): the routes,
//! what each write leaves in the store, and the transaction that takes an opening whole or not at
//! all. The roll itself is `src/economy/roll.rs`'s unit tests; here it runs over the real catalog.
//!
//! Every request goes through the whole app over an empty fake store (`deps::empty_test_app`), as
//! the active account `p1`. An opening the test must steer (a registry with cosmetics a pack may
//! drop, R1301002) calls `open_pack` with that registry; the route opens over the shipped one,
//! whose every entry but the six full arts is a starter (R1313000).

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{CardDefs, SetName};
use jackioh_server::api::collection::{GrantInput, grant_cards, grant_entire_catalog, owned_map};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    CARD_TIER_WEIGHTS, CARD_TOKEN_BAR_POINTS_PER_DUPLICATE, CARD_TOKEN_BAR_SIZE, COSMETIC_TIER_WEIGHTS,
    COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE, COSMETIC_TOKEN_BAR_SIZE, CosmeticTokenStep,
    GLOBAL_COSMETIC_FLOOR_RARITY, GLOBAL_COSMETIC_ROLL_WEIGHT, GLOBAL_PACK_NAME, PACK_FLOOR_RARITY,
    PACK_PRICE_JLORE, PACK_SLOTS, PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS,
    TIER_WEIGHT_TOTAL,
};
use jackioh_server::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, pack_cosmetics, shipped};
use jackioh_server::db::store::{CollectionEntry, StoreError};
use jackioh_server::economy::math::PityCounters;
use jackioh_server::economy::packs::{BarSettle, BarsSettled, OpenedPack, PACK_GRANT_REASON, open_pack};
use jackioh_server::economy::roll::{
    Holdings, OpeningLog, PackKind, Slot, SlotGrant, pack_pool, roll_opening,
};
use jackioh_server::economy::store::{
    Counter, CounterGrant, EconomyReason, InventoryItem, JloreGrant, JloreReason,
};
use jackioh_server::economy::tiers::{card_odds, cosmetic_odds};

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::store::fake;

const PROFILE: &str = "p1";
const USER: &str = "u1";

/// The three routes, as `(method, path)`.
const ROUTES: [(&str, &str); 3] = [
    ("GET", "/api/packs"),
    ("POST", "/api/packs/buy"),
    ("POST", "/api/packs/open"),
];

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// An empty app with the active `p1` and its bearer token.
async fn setup() -> (Arc<App>, String) {
    let app = empty_test_app().await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": PROFILE, "userId": USER, "status": "active" }));
    let token = add_user(&app, USER, "p1@example.test", true);
    (app, token)
}

/// `jlore` more Jlore in `p1`'s wallet, as a game pays it (§14.8).
async fn fund(app: &App, jlore: i64) {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    tx.economy_jlore_grant(&JloreGrant {
        profile_id: PROFILE.to_string(),
        delta: jlore,
        reason: JloreReason::Game,
        reference: None,
        dev: false,
        at: app::now_ms(),
    })
    .await
    .expect("the grant")
    .expect("a grant the wallet takes");
    tx.commit().await.expect("the commit");
}

/// The pack price as a wallet amount.
fn price() -> i64 {
    i64::from(PACK_PRICE_JLORE)
}

/// One pack of `pack` bought through the route.
async fn buy(app: &Arc<App>, token: &str, pack: &str) {
    fund(app, price()).await;
    let (status, _, body) = call(
        app,
        "POST",
        "/api/packs/buy",
        Some(token),
        json!({ "pack": pack }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

/// Opens one pack of `pack` through the route.
async fn open(app: &Arc<App>, token: &str, pack: &str) -> OpenedPack {
    let (status, _, body) = call(
        app,
        "POST",
        "/api/packs/open",
        Some(token),
        json!({ "pack": pack }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    serde_json::from_value(body).expect("an opened pack")
}

/// The store's fault hook: every call named `failing` errors.
async fn fail_on(app: &App, failing: &'static str) {
    fake(app).lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
        if method == failing {
            Err(StoreError::Other("injected fault".to_string()))
        } else {
            Ok(())
        }
    }));
}

async fn heal(app: &App) {
    fake(app).lock().await.on_call = None;
}

/// A store value as JSON.
fn to<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a store value serialises")
}

/// Everything `p1` holds and every ledger row it has, the collection's included, as one value.
async fn everything(app: &App) -> Value {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let state = json!({
        "jlore": tx.economy_jlore_get(PROFILE).await.expect("the wallet"),
        "inventory": to(&tx.economy_inventory_get(PROFILE).await.expect("the inventory")),
        "counters": to(&tx.economy_counters_get(PROFILE).await.expect("the counters")),
        "cosmetics": to(&tx.economy_cosmetics_get(PROFILE).await.expect("the cosmetics")),
        "ledger": to(&tx.economy_ledger_get(PROFILE).await.expect("the ledger")),
        "collection": to(&tx.collection_get(PROFILE).await.expect("the collection")),
    });
    tx.commit().await.expect("the commit");
    let grants = to(&fake(app).lock().await.tables.grants);
    json!({ "state": state, "collectionGrants": grants })
}

/// `p1`'s count of one counter (0 when never written).
async fn counter(app: &App, counter: Counter) -> i64 {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let counters = tx.economy_counters_get(PROFILE).await.expect("the counters");
    tx.commit().await.expect("the commit");
    counters
        .iter()
        .find(|entry| entry.counter == counter && entry.pack.is_empty())
        .map_or(0, |entry| entry.value)
}

/// Sets one counter to `value`, as if earlier openings had left it there.
async fn set_counter(app: &App, which: Counter, value: i64) {
    let current = counter(app, which).await;
    if current == value {
        return;
    }
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    tx.economy_counters_grant(&CounterGrant {
        profile_id: PROFILE.to_string(),
        counter: which,
        pack: String::new(),
        delta: value - current,
        reason: EconomyReason::PackOpen,
        reference: None,
        dev: false,
        at: app::now_ms(),
    })
    .await
    .expect("the grant")
    .expect("a grant the counter takes");
    tx.commit().await.expect("the commit");
}

/// `p1`'s holdings as an opening reads them.
async fn holdings(app: &App) -> Holdings {
    let owned = owned_map(app, PROFILE).await.expect("the collection");
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let cosmetics = tx.economy_cosmetics_get(PROFILE).await.expect("the cosmetics");
    tx.commit().await.expect("the commit");
    let pity = |name| usize::try_from(name).expect("a pity counter fits a usize");
    Holdings {
        owned,
        cosmetics: cosmetics.cards,
        account_cosmetics: cosmetics.account,
        pity: PityCounters {
            epic: pity(counter(app, Counter::PityEpic).await),
            legendary: pity(counter(app, Counter::PityLegendary).await),
            mythic: pity(counter(app, Counter::PityMythic).await),
        },
    }
}

/// How many packs of `item` under `key` `p1` holds.
async fn held(app: &App, item: InventoryItem, key: &str) -> i64 {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let inventory = tx.economy_inventory_get(PROFILE).await.expect("the inventory");
    tx.commit().await.expect("the commit");
    inventory
        .iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count)
}

/// A per-card cosmetic a pack may drop.
fn foil(id: &str, rarity: CosmeticRarity) -> Cosmetic {
    Cosmetic {
        id: id.to_string(),
        kind: CosmeticKind::Foil,
        name: id.to_string(),
        rarity,
        default: false,
        starter: false,
        chase: rarity.is_chase(),
        card: None,
    }
}

fn defs(app: &App) -> &CardDefs {
    &app.catalog.defs
}

fn rarity_of(app: &App, card_id: &str) -> PrintedRarity {
    defs(app)[card_id]
        .rarity
        .as_str()
        .parse()
        .expect("a pool card prints a rarity")
}

fn spent(slots: &[Slot]) -> i64 {
    slots
        .iter()
        .filter(|slot| matches!(slot.grant, SlotGrant::Bars { .. }))
        .count() as i64
}

/// The opening log of `p1`'s last opening, from its inventory row (R1301003).
async fn last_log(app: &App) -> OpeningLog {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    let row = ledger
        .inventory
        .iter()
        .rev()
        .find(|row| row.reason == EconomyReason::PackOpen)
        .expect("an opening's inventory row");
    serde_json::from_str(row.reference.as_deref().expect("the opening's log")).expect("an opening log")
}

// ---------------------------------------------------------------------------------------------
// Buying (§14.1, §14.3)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn buying_debits_exactly_the_pack_price() {
    let (app, token) = setup().await;
    let left = 7;
    fund(&app, price() + left).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "pack": "Core", "jlore": left, "unopened": 1 }));

    fund(&app, price()).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": GLOBAL_PACK_NAME }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "pack": GLOBAL_PACK_NAME, "jlore": left, "unopened": 1 })
    );

    let state = everything(&app).await["state"].clone();
    assert_eq!(state["jlore"], json!(left));
    assert_eq!(
        state["inventory"],
        json!([
            { "item": "global_pack", "key": "", "count": 1 },
            { "item": "set_pack", "key": "Core", "count": 1 },
        ])
    );
    let spends: Vec<(i64, Value, Value)> = state["ledger"]["jlore"]
        .as_array()
        .expect("the Jlore rows")
        .iter()
        .filter(|row| row["reason"] == json!("pack_purchase"))
        .map(|row| {
            (
                row["delta"].as_i64().unwrap_or(0),
                row["reference"].clone(),
                row["dev"].clone(),
            )
        })
        .collect();
    assert_eq!(
        spends,
        vec![
            (-price(), json!("Core"), json!(false)),
            (-price(), json!(GLOBAL_PACK_NAME), json!(false)),
        ]
    );
    let packs: Vec<(Value, Value, Value)> = state["ledger"]["inventory"]
        .as_array()
        .expect("the inventory rows")
        .iter()
        .map(|row| (row["key"].clone(), row["delta"].clone(), row["reason"].clone()))
        .collect();
    assert_eq!(
        packs,
        vec![
            (json!("Core"), json!(1), json!("pack_purchase")),
            (json!(""), json!(1), json!("pack_purchase")),
        ]
    );
}

#[tokio::test]
async fn buying_with_a_short_wallet_is_refused_and_changes_nothing() {
    let (app, token) = setup().await;
    // No wallet at all.
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);

    // A wallet one Jlore short.
    fund(&app, price() - 1).await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);
    assert_eq!(before["state"]["jlore"], json!(price() - 1));
}

// ---------------------------------------------------------------------------------------------
// Opening (§14.3, §14.4)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_opening_grants_exactly_its_slots_one_rare_or_better() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    let opened = open(&app, &token, "Core").await;

    assert_eq!(opened.pack, "Core");
    assert_eq!(opened.unopened, 0);
    assert_eq!(opened.slots.len(), PACK_SLOTS);
    assert!(
        opened.slots.iter().any(|slot| slot.rarity >= PrintedRarity::Rare
            && rarity_of(&app, &slot.card_id) >= PrintedRarity::Rare),
        "{:?}",
        opened.slots
    );
    assert!(
        opened
            .slots
            .iter()
            .all(|slot| defs(&app)[&slot.card_id].set == SetName::Core)
    );
    // Every slot drew its rolled rarity: Core holds all five.
    assert!(
        opened
            .slots
            .iter()
            .all(|slot| rarity_of(&app, &slot.card_id) == slot.rarity)
    );

    // The collection gained exactly the unlocked cards, each a `pack` row; the rest went to the
    // bars or, a full art's card rolling Epic, to its full art (R1301002, R1313000).
    let mut unlocked: IndexMap<String, i64> = IndexMap::new();
    for slot in &opened.slots {
        if slot.grant == SlotGrant::Card {
            *unlocked.entry(slot.card_id.clone()).or_insert(0) += 1;
        }
    }
    assert!(!unlocked.is_empty());
    assert_eq!(owned_map(&app, PROFILE).await.expect("the collection"), unlocked);
    let all = everything(&app).await;
    let reasons: Vec<Value> = all["collectionGrants"]
        .as_array()
        .expect("the collection's rows")
        .iter()
        .map(|row| row["reason"].clone())
        .collect();
    assert_eq!(reasons, vec![json!(PACK_GRANT_REASON); unlocked.len()]);
    assert_eq!(
        counter(&app, Counter::CardTokenBar).await,
        spent(&opened.slots) * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE
    );

    // The pack is spent, and the pity counters stored are the ones answered.
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 0);
    assert_eq!(holdings(&app).await.pity, opened.pity);
}

#[tokio::test]
async fn a_global_pack_draws_from_the_shipped_sets() {
    let (app, token) = setup().await;
    let pool = pack_pool(defs(&app), PackKind::Global);
    let registry = shipped().expect("the registry");
    let mut sets = Vec::new();
    for at in 0..4 {
        buy(&app, &token, GLOBAL_PACK_NAME).await;
        let opened = open_pack(
            &app,
            PROFILE,
            PackKind::Global,
            &registry,
            &format!("global-{at}"),
        )
        .await
        .expect("the opening");
        assert_eq!(opened.slots.len(), PACK_SLOTS);
        for slot in opened.slots {
            assert!(
                pool.iter().any(|card| card.id == slot.card_id),
                "{}",
                slot.card_id
            );
            let set = defs(&app)[&slot.card_id].set;
            if !sets.contains(&set) {
                sets.push(set);
            }
        }
    }
    assert!(sets.len() > 1, "{sets:?}");
    assert_eq!(held(&app, InventoryItem::GlobalPack, "").await, 0);
}

#[tokio::test]
async fn r1301002_an_opening_on_a_full_collection_grants_cosmetics_or_bar_points() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // A Common and a Rare foil a pack may drop, and an Epic starter it may not: an Epic,
    // Legendary or Mythic duplicate has nothing to drop.
    let registry = vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-rare", CosmeticRarity::Rare),
        Cosmetic {
            starter: true,
            ..foil("foil-fixture-epic", CosmeticRarity::Epic)
        },
    ];
    let mut slots = Vec::new();
    for at in 0..4 {
        buy(&app, &token, "Classic").await;
        let opened = open_pack(
            &app,
            PROFILE,
            PackKind::Set(SetName::Classic),
            &registry,
            &format!("r1301002-full-{at}"),
        )
        .await
        .expect("the opening");
        slots.extend(opened.slots);
    }
    assert_eq!(slots.len(), 4 * PACK_SLOTS);

    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let owned = tx
        .economy_cosmetics_get(PROFILE)
        .await
        .expect("the cosmetics")
        .cards;
    tx.commit().await.expect("the commit");
    let mut cosmetics = 0;
    // What the openings granted so far, slot by slot, as (card, cosmetic).
    let mut granted: Vec<(String, String)> = Vec::new();
    for slot in &slots {
        match &slot.grant {
            SlotGrant::Card => panic!("every card was owned: {slot:?}"),
            SlotGrant::Cosmetic {
                cosmetic_id,
                cosmetic_rarity,
            } => {
                cosmetics += 1;
                let entry = registry
                    .iter()
                    .find(|cosmetic| &cosmetic.id == cosmetic_id)
                    .expect("a registry entry");
                assert!(!entry.starter);
                assert_eq!(entry.rarity, *cosmetic_rarity);
                assert!(
                    owned
                        .iter()
                        .any(|held| held.card_id == slot.card_id && &held.cosmetic_id == cosmetic_id),
                    "{slot:?}"
                );
                granted.push((slot.card_id.clone(), cosmetic_id.clone()));
            }
            SlotGrant::Bars { cosmetic_rarity } => {
                // Spent: nothing droppable at its rarity (Epic's is a starter), or an earlier slot
                // already put that rarity's foil on its card.
                let foil = registry
                    .iter()
                    .find(|cosmetic| cosmetic.rarity == *cosmetic_rarity && !cosmetic.starter);
                assert!(
                    foil.is_none_or(|foil| granted.contains(&(slot.card_id.clone(), foil.id.clone()))),
                    "{slot:?}"
                );
            }
        }
    }
    assert_eq!(owned.len(), cosmetics);
    let bars = spent(&slots);
    assert!(cosmetics > 0 && bars > 0, "{slots:?}");
    assert_eq!(
        counter(&app, Counter::CardTokenBar).await,
        bars * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE
    );
    assert_eq!(
        counter(&app, Counter::CosmeticTokenBar).await,
        bars * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE
    );
}

/// The route's answer carries `bars` in camelCase, and it is what the opening left in the store.
#[tokio::test]
async fn an_opening_answers_the_bars_it_left() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let gained = spent(
        &serde_json::from_value::<OpenedPack>(body.clone())
            .expect("an opened pack")
            .slots,
    );
    let bars = &body["bars"];
    assert_eq!(
        bars["card"],
        json!({
            "before": 0,
            "after": gained * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE,
            "size": CARD_TOKEN_BAR_SIZE,
            "pays": 0,
        })
    );
    assert_eq!(
        bars["cosmetic"],
        json!({
            "before": 0,
            "after": gained * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
            "size": COSMETIC_TOKEN_BAR_SIZE,
            "pays": 0,
        })
    );
    assert_eq!(bars["cosmeticPaid"], json!([]));
    assert_eq!(
        counter(&app, Counter::CardTokenBar).await,
        bars["card"]["after"].as_i64().expect("a number")
    );
    assert_eq!(
        counter(&app, Counter::CosmeticTokenBar).await,
        bars["cosmetic"]["after"].as_i64().expect("a number")
    );
}

#[tokio::test]
async fn an_opening_with_no_spent_duplicate_answers_both_bars_unmoved() {
    let (app, token) = setup().await;
    set_counter(&app, Counter::CardTokenBar, 7).await;
    set_counter(&app, Counter::CosmeticTokenBar, 11).await;
    buy(&app, &token, "Core").await;
    // A fresh collection: every slot unlocks a card, unless one card rolls twice in this pack.
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &[], "bars-unspent")
        .await
        .expect("the opening");
    assert_eq!(spent(&opened.slots), 0, "{:?}", opened.slots);
    assert_eq!(
        opened.bars,
        BarsSettled {
            card: BarSettle {
                before: 7,
                after: 7,
                size: CARD_TOKEN_BAR_SIZE,
                pays: 0,
            },
            cosmetic: BarSettle {
                before: 11,
                after: 11,
                size: COSMETIC_TOKEN_BAR_SIZE,
                pays: 0,
            },
            cosmetic_paid: Vec::new(),
        }
    );
    assert_eq!(counter(&app, Counter::CardTokenBar).await, 7);
    assert_eq!(counter(&app, Counter::CosmeticTokenBar).await, 11);
}

#[tokio::test]
async fn spent_duplicates_carrying_the_card_bar_past_its_size_answer_one_pay() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    set_counter(&app, Counter::CardTokenBar, CARD_TOKEN_BAR_SIZE - 1).await;
    buy(&app, &token, "Core").await;
    // An empty registry drops no cosmetic: every slot is spent.
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &[], "bars-card-pays")
        .await
        .expect("the opening");
    assert_eq!(spent(&opened.slots), PACK_SLOTS as i64);
    let card_gain = PACK_SLOTS as i64 * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE;
    let card = BarSettle {
        before: CARD_TOKEN_BAR_SIZE - 1,
        after: CARD_TOKEN_BAR_SIZE - 1 + card_gain - CARD_TOKEN_BAR_SIZE,
        size: CARD_TOKEN_BAR_SIZE,
        pays: 1,
    };
    assert_eq!(opened.bars.card, card);
    // The cosmetic bar only gained, so it paid nothing and no step.
    assert_eq!(
        opened.bars.cosmetic,
        BarSettle {
            before: 0,
            after: PACK_SLOTS as i64 * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
            size: COSMETIC_TOKEN_BAR_SIZE,
            pays: 0,
        }
    );
    assert!(opened.bars.cosmetic_paid.is_empty());

    // The token it paid is held, and the bar reads back where the answer left it.
    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["tokens"]["card"], json!(card.pays));
    assert_eq!(body["cardBar"]["points"], json!(card.after));
    assert_eq!(body["cardBar"]["size"], json!(card.size));
}

#[tokio::test]
async fn the_cosmetic_bar_answers_the_cycle_steps_it_paid_in_order() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    let gain = PACK_SLOTS as i64 * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE;
    // Past two sizes short of this opening's gain, at the cycle's last step: the bar pays twice,
    // the cosmetic pack and, the cycle restarting, a Rare token.
    set_counter(&app, Counter::CosmeticTokenCycle, 5).await;
    set_counter(
        &app,
        Counter::CosmeticTokenBar,
        2 * COSMETIC_TOKEN_BAR_SIZE - gain,
    )
    .await;
    buy(&app, &token, "Core").await;
    let opened = open_pack(
        &app,
        PROFILE,
        PackKind::Set(SetName::Core),
        &[],
        "bars-cosmetic-pays",
    )
    .await
    .expect("the opening");
    assert_eq!(spent(&opened.slots), PACK_SLOTS as i64);
    assert_eq!(
        opened.bars.cosmetic,
        BarSettle {
            before: 2 * COSMETIC_TOKEN_BAR_SIZE - gain,
            after: 0,
            size: COSMETIC_TOKEN_BAR_SIZE,
            pays: 2,
        }
    );
    assert_eq!(
        opened.bars.cosmetic_paid,
        vec![CosmeticTokenStep::CosmeticPack, CosmeticTokenStep::Rare]
    );
    assert_eq!(to(&opened.bars)["cosmeticPaid"], json!(["cosmeticPack", "rare"]));
    assert_eq!(opened.bars.card.pays, 0);

    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["tokens"]["cosmeticPack"], json!(1));
    assert_eq!(body["tokens"]["cosmeticRare"], json!(1));
    assert_eq!(body["tokens"]["cosmeticEpic"], json!(0));
    assert_eq!(body["nextStep"], json!("epic"));
    assert_eq!(body["cosmeticBar"]["points"], json!(0));
}

#[tokio::test]
async fn r1301003_an_opening_replays_from_its_stored_seed() {
    let (app, token) = setup().await;
    // Some cards owned and the counters running, so the replay reads real holdings.
    grant_cards(
        &app,
        GrantInput {
            profile_id: PROFILE.to_string(),
            entries: ["core-001", "core-002", "core-003"]
                .into_iter()
                .map(|card_id| CollectionEntry {
                    card_id: card_id.to_string(),
                    quantity: 1,
                })
                .collect(),
            reason: "admin".to_string(),
        },
    )
    .await
    .expect("a grant");
    buy(&app, &token, "Core").await;
    open(&app, &token, "Core").await;

    // Through the route: the shipped registry and a fresh seed.
    buy(&app, &token, "Core").await;
    let before = holdings(&app).await;
    let opened = open(&app, &token, "Core").await;
    let log = last_log(&app).await;
    assert_eq!(log.pack, "Core");
    assert_eq!(log.catalog, app.catalog.version);
    assert_eq!(log.seed.len(), 32, "16 bytes as hex: {}", log.seed);
    assert_eq!(log.cosmetics, pack_cosmetics(&shipped().expect("the registry")));
    // The result is logged with the grants (§14.11).
    assert_eq!(log.slots, opened.slots);
    assert_eq!(log.pity, opened.pity);
    let pack = PackKind::parse(&log.pack).expect("the logged pack");
    let replayed =
        roll_opening(&pack_pool(defs(&app), pack), &before, &log.cosmetics, &log.seed).expect("the replay");
    assert_eq!(replayed.slots, log.slots);
    assert_eq!(replayed.pity, log.pity);
    assert_eq!(replayed.draws, log.draws);

    // Every economy row of the opening carries its log: the pack spent, each pity counter that
    // moved and each bar that gained.
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    let logged = Some(serde_json::to_string(&log).expect("a log"));
    let pack_rows = ledger
        .inventory
        .iter()
        .filter(|row| row.reference == logged)
        .map(|row| (row.delta, row.reason))
        .collect::<Vec<_>>();
    assert_eq!(pack_rows, vec![(-1, EconomyReason::PackOpen)]);
    let moved = [
        before.pity.epic != opened.pity.epic,
        before.pity.legendary != opened.pity.legendary,
        before.pity.mythic != opened.pity.mythic,
        spent(&opened.slots) * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE != 0,
        spent(&opened.slots) * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE != 0,
    ];
    let counter_rows: Vec<_> = ledger
        .counters
        .iter()
        .filter(|row| row.reference == logged)
        .collect();
    assert_eq!(counter_rows.len(), moved.iter().filter(|moved| **moved).count());
    assert!(
        counter_rows
            .iter()
            .all(|row| row.reason == EconomyReason::PackOpen && !row.dev)
    );

    // On a full collection, with cosmetics to drop: the replay draws the same cosmetics.
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    let registry = vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-rare", CosmeticRarity::Rare),
        foil("foil-fixture-epic", CosmeticRarity::Epic),
        foil("foil-fixture-legendary", CosmeticRarity::Legendary),
    ];
    buy(&app, &token, "Core").await;
    let before = holdings(&app).await;
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &registry, "r1301003")
        .await
        .expect("the opening");
    assert!(
        opened
            .slots
            .iter()
            .any(|slot| matches!(slot.grant, SlotGrant::Cosmetic { .. }))
    );
    let log = last_log(&app).await;
    assert_eq!(log.seed, "r1301003");
    assert_eq!(log.cosmetics, registry);
    assert_eq!(log.slots, opened.slots);
    // The replay reads the cosmetics from the log, not the registry it was opened over.
    let replayed =
        roll_opening(&pack_pool(defs(&app), pack), &before, &log.cosmetics, &log.seed).expect("the replay");
    assert_eq!(replayed.slots, opened.slots);
    assert_eq!(replayed.draws, log.draws);
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    let logged = Some(serde_json::to_string(&log).expect("a log"));
    let cosmetic_rows: Vec<_> = ledger
        .cosmetics
        .iter()
        .filter(|row| row.reference == logged)
        .collect();
    assert_eq!(
        cosmetic_rows.len(),
        opened
            .slots
            .iter()
            .filter(|slot| matches!(slot.grant, SlotGrant::Cosmetic { .. }))
            .count()
    );
}

#[tokio::test]
async fn a_fault_on_the_collection_leaves_no_partial_grant() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    let before = everything(&app).await;
    fail_on(&app, "collection.appendGrants").await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    heal(&app).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(everything(&app).await, before);
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 1);
    assert!(
        before["state"]["ledger"]["inventory"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| row["reason"] != json!("pack_open")))
    );
}

#[tokio::test]
async fn a_fault_on_the_counters_leaves_no_partial_grant() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    buy(&app, &token, "Core").await;
    let before = everything(&app).await;
    fail_on(&app, "economy.counters.grant").await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    heal(&app).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(everything(&app).await, before);
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 1);
    assert_eq!(counter(&app, Counter::CardTokenBar).await, 0);
    // Healed, the same pack opens.
    open(&app, &token, "Core").await;
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 0);
}

#[tokio::test]
async fn opening_without_a_pack_is_refused() {
    let (app, token) = setup().await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);

    // A pack of another kind is not one of this kind.
    buy(&app, &token, GLOBAL_PACK_NAME).await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn an_unknown_pack_is_a_bad_request() {
    let (app, token) = setup().await;
    // A pack held and a price in the wallet, so only the name can refuse either route.
    buy(&app, &token, "Core").await;
    fund(&app, price()).await;
    let before = everything(&app).await;
    for body in [
        json!({ "pack": "Boss" }),
        json!({ "pack": "Custom" }),
        json!({ "pack": 3 }),
        json!({}),
    ] {
        for path in ["/api/packs/buy", "/api/packs/open"] {
            let (status, _, answer) = call(&app, "POST", path, Some(&token), body.clone()).await;
            assert_eq!(
                (status, answer["error"]["code"].clone()),
                (400, json!("bad_request")),
                "{path} {body}: {answer}"
            );
        }
    }
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn the_routes_refuse_a_signed_out_caller() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    fund(&app, price()).await;
    let before = everything(&app).await;
    for (method, path) in ROUTES {
        let (status, _, body) = call(&app, method, path, None, json!({ "pack": "Core" })).await;
        assert_eq!(status, 401, "{method} {path}: {body}");
    }
    assert_eq!(everything(&app).await, before);

    // A pending account is refused too (§9.4): all three are active routes.
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "p2", "userId": "u2", "status": "pending" }));
    let pending = add_user(&app, "u2", "p2@example.test", true);
    for (method, path) in ROUTES {
        let (status, _, body) = call(&app, method, path, Some(&pending), json!({ "pack": "Core" })).await;
        assert_eq!(status, 403, "{method} {path}: {body}");
    }
    assert_eq!(everything(&app).await, before);
    let declared: Vec<(&str, &str, bool)> = app::ROUTES
        .iter()
        .filter(|route| route.1.starts_with("/api/packs"))
        .map(|route| (route.0, route.1, matches!(route.2, AuthLevel::Active)))
        .collect();
    assert_eq!(
        declared,
        ROUTES
            .iter()
            .map(|(method, path)| (*method, *path, true))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn r1339000_get_reads_the_pity_counters_and_the_tables() {
    let (app, token) = setup().await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["pity"], json!({ "epic": 0, "legendary": 0, "mythic": 0 }));
    assert_eq!(body["price"], json!(PACK_PRICE_JLORE));
    assert_eq!(body["slots"], json!(PACK_SLOTS));
    assert_eq!(
        body["thresholds"],
        json!({ "epic": PITY_EPIC_PACKS, "legendary": PITY_LEGENDARY_PACKS, "mythic": PITY_MYTHIC_PACKS })
    );
    assert_eq!(
        body["odds"],
        json!({
            "slot": card_odds(None),
            "flooredSlot": card_odds(Some(PACK_FLOOR_RARITY)),
            "duplicateCosmetic": cosmetic_odds(None),
            "globalCosmetic": {
                "chance": f64::from(GLOBAL_COSMETIC_ROLL_WEIGHT) / f64::from(TIER_WEIGHT_TOTAL),
                "tiers": cosmetic_odds(Some(GLOBAL_COSMETIC_FLOOR_RARITY)),
            },
        })
    );
    // The tables as percentages' fractions, the computed fallback included, and no Common in the
    // floored slot.
    let total = f64::from(TIER_WEIGHT_TOTAL);
    assert_eq!(
        body["odds"]["slot"]["common"],
        json!(f64::from(TIER_WEIGHT_TOTAL - CARD_TIER_WEIGHTS.set_total()) / total)
    );
    assert_eq!(
        body["odds"]["slot"]["epic"],
        json!(f64::from(CARD_TIER_WEIGHTS.epic) / total)
    );
    assert_eq!(body["odds"]["flooredSlot"]["common"], json!(0.0));

    buy(&app, &token, "Core").await;
    let opened = open(&app, &token, "Core").await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["pity"],
        serde_json::to_value(opened.pity).expect("the counters")
    );
}

#[tokio::test]
async fn r1661001_get_answers_the_global_cosmetic_line() {
    let (app, token) = setup().await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    let line = &body["odds"]["globalCosmetic"];
    // The hit's share of one draw, and the floored table's odds, the floor taking what is below it.
    let total = f64::from(TIER_WEIGHT_TOTAL);
    assert_eq!(
        line["chance"],
        json!(f64::from(GLOBAL_COSMETIC_ROLL_WEIGHT) / total)
    );
    assert_eq!(
        line["tiers"],
        serde_json::to_value(cosmetic_odds(Some(GLOBAL_COSMETIC_FLOOR_RARITY))).expect("the odds")
    );
    assert_eq!(line["tiers"]["common"], json!(0.0));
    assert_eq!(line["tiers"]["rare"], json!(0.0));
    let w = COSMETIC_TIER_WEIGHTS;
    assert_eq!(
        line["tiers"]["epic"],
        json!(f64::from(TIER_WEIGHT_TOTAL - w.legendary - w.mythic) / total)
    );
    assert_eq!(line["tiers"]["legendary"], json!(f64::from(w.legendary) / total));
    assert_eq!(line["tiers"]["mythic"], json!(f64::from(w.mythic) / total));
}

#[tokio::test]
async fn r1661001_an_account_cosmetic_a_duplicate_drops_is_owned_on_no_card_and_then_drops_no_more() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // One account cosmetic a pack may drop, and nothing per card: every slot a duplicate, so each
    // takes the global draw until the playmat is owned.
    let playmat = Cosmetic {
        kind: CosmeticKind::Table,
        ..foil("table-fixture-epic", GLOBAL_COSMETIC_FLOOR_RARITY)
    };
    let registry = vec![playmat.clone()];
    let mut dropped = None;
    for at in 0..20 {
        buy(&app, &token, "Core").await;
        let before = holdings(&app).await;
        let seed = format!("r1661001-account-{at}");
        let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &registry, &seed)
            .await
            .expect("the opening");
        let log = last_log(&app).await;
        assert_eq!(log.cosmetics, registry);
        let replayed = roll_opening(
            &pack_pool(defs(&app), PackKind::Set(SetName::Core)),
            &before,
            &log.cosmetics,
            &log.seed,
        )
        .expect("the replay");
        assert_eq!(replayed.slots, opened.slots);
        assert_eq!(replayed.draws, log.draws);
        let hits: Vec<&Slot> = opened
            .slots
            .iter()
            .filter(|slot| matches!(slot.grant, SlotGrant::Cosmetic { .. }))
            .collect();
        if let [slot] = hits.as_slice() {
            assert_eq!(
                slot.grant,
                SlotGrant::Cosmetic {
                    cosmetic_id: playmat.id.clone(),
                    cosmetic_rarity: playmat.rarity,
                }
            );
            dropped = Some(log);
            break;
        }
        assert!(hits.is_empty(), "the playmat drops once: {:?}", opened.slots);
    }
    let log = dropped.expect("a seed that drops the playmat");

    // Owned for the account, on no card, by one `pack_open` row carrying the opening's log.
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let cosmetics = tx.economy_cosmetics_get(PROFILE).await.expect("the cosmetics");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    assert_eq!(cosmetics.account, vec![playmat.id.clone()]);
    assert!(cosmetics.cards.is_empty(), "{:?}", cosmetics.cards);
    let rows: Vec<_> = ledger
        .cosmetics
        .iter()
        .map(|row| {
            (
                row.card_id.clone(),
                row.cosmetic_id.clone(),
                row.delta,
                row.reason,
            )
        })
        .collect();
    assert_eq!(rows, vec![(None, playmat.id.clone(), 1, EconomyReason::PackOpen)]);
    assert_eq!(
        ledger.cosmetics[0].reference,
        Some(serde_json::to_string(&log).expect("a log"))
    );

    // Owned, it leaves nothing to drop: the next opening takes no global draw, each duplicate its
    // tier's, its card's and its cosmetic tier's three, and every slot is spent.
    buy(&app, &token, "Core").await;
    let opened = open_pack(
        &app,
        PROFILE,
        PackKind::Set(SetName::Core),
        &registry,
        "r1661001-owned",
    )
    .await
    .expect("the opening");
    assert_eq!(spent(&opened.slots), PACK_SLOTS as i64);
    assert_eq!(last_log(&app).await.draws as usize, 3 * PACK_SLOTS);
}

#[tokio::test]
async fn get_lists_the_unopened_packs_held() {
    let (app, token) = setup().await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["unopened"], json!([]));

    for pack in ["Classic", "Classic", GLOBAL_PACK_NAME, "Core"] {
        buy(&app, &token, pack).await;
    }
    open(&app, &token, "Core").await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["unopened"],
        json!([
            { "pack": GLOBAL_PACK_NAME, "count": 1 },
            { "pack": "Classic", "count": 2 },
        ])
    );
}
