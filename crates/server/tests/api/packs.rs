//! Buying and opening card packs (`src/economy/packs.rs`, SPEC §14.1, §14.3; #1301): the routes,
//! what each write leaves in the store, and the transaction that takes an opening whole or not at
//! all. The roll itself is `src/economy/roll.rs`'s unit tests; here it runs over the real catalog.
//!
//! Every request goes through the whole app over an empty fake store (`deps::empty_test_app`), as
//! the active account `p1`. An opening the test must steer (a registry with cosmetics a pack may
//! drop, R1301002) calls `open_pack` with that registry; the route opens over the shipped one,
//! whose every entry but the six full arts (R1313000) and the five portraits but vanilla
//! (R1827000) is a starter. A spend sent while the account's other spends run waits its turn
//! (R2378000): the test takes the turns with spends of its own, held until it lets them go.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};
use tokio::sync::watch;

use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{CardDefs, SetName};
use jackioh_server::api::collection::{GrantInput, grant_cards, grant_entire_catalog, owned_map};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    CARD_TIER_WEIGHTS, CARD_TOKEN_BAR_SIZE, COSMETIC_TIER_WEIGHTS, COSMETIC_TOKEN_BAR_SIZE_EPIC,
    COSMETIC_TOKEN_BAR_SIZE_LEGENDARY, COSMETIC_TOKEN_BAR_SIZE_RARE, CosmeticTokenStep,
    ECONOMY_SPENDS_RUNNING_MAX, ECONOMY_SPENDS_WAITING_MAX, GLOBAL_COSMETIC_FLOOR_RARITY,
    GLOBAL_COSMETIC_ROLL_WEIGHT, GLOBAL_PACK_NAME, PACK_FLOOR_RARITY, PACK_PRICE_JLORE, PACK_SLOTS,
    PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, SPEND_BUSY_RETRY_AFTER_MS, TIER_WEIGHT_TOTAL,
};
use jackioh_server::cosmetics::{
    Cosmetic, CosmeticKind, CosmeticRarity, global_cosmetics, pack_cosmetics, portrait_of, shipped,
};
use jackioh_server::db::store::CollectionEntry;
use jackioh_server::economy::math::PityCounters;
use jackioh_server::economy::packs::{
    BarSettle, BarsSettled, OpenedPack, PACK_GRANT_REASON, open_pack, slot_bar_points,
};
use jackioh_server::economy::roll::{
    Holdings, OpeningLog, PackKind, Slot, SlotGrant, pack_pool, roll_opening,
};
use jackioh_server::economy::spend_slots::SPEND_BUSY_MESSAGE;
use jackioh_server::economy::store::{
    Counter, CounterGrant, EconomyReason, InventoryItem, JloreGrant, JloreReason,
};
use jackioh_server::economy::tiers::{card_odds, cosmetic_odds};
use jackioh_server::economy::tokens::card_bar_points;

use crate::support::deps::{add_user, call, call_with_key, empty_test_app};
use crate::support::store::{fail_on, fake, stop_failing, uuid};

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
    fund_of(app, PROFILE, jlore).await;
}

/// `jlore` more Jlore in `profile_id`'s wallet, as a game pays it (§14.8).
async fn fund_of(app: &App, profile_id: &str, jlore: i64) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a transaction");
    tx.economy_jlore_grant(&JloreGrant {
        profile_id: profile_id.to_string(),
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

/// A store value as JSON.
fn to<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a store value serializes")
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

/// The points an opening's slots add to each bar, each its slot's rarity's (R1953000, R1999000).
fn points(slots: &[Slot]) -> i64 {
    slot_bar_points(slots).iter().sum()
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
    assert_eq!(counter(&app, Counter::CardTokenBar).await, points(&opened.slots));

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
    let bars = points(&slots);
    assert!(cosmetics > 0 && spent(&slots) > 0, "{slots:?}");
    assert_eq!(counter(&app, Counter::CardTokenBar).await, bars);
    assert_eq!(counter(&app, Counter::CosmeticTokenBar).await, bars);
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
    let slots = serde_json::from_value::<OpenedPack>(body.clone())
        .expect("an opened pack")
        .slots;
    let gained = points(&slots);
    let bars = &body["bars"];
    assert_eq!(
        bars["card"],
        json!({
            "before": 0,
            "after": gained,
            "size": CARD_TOKEN_BAR_SIZE,
            "pays": 0,
        })
    );
    assert_eq!(
        bars["cosmetic"],
        json!({
            "before": 0,
            "after": gained,
            "size": COSMETIC_TOKEN_BAR_SIZE_RARE,
            "pays": 0,
        })
    );
    assert_eq!(bars["cosmeticPaid"], json!([]));
    assert_eq!(bars["cosmeticSizes"], json!([COSMETIC_TOKEN_BAR_SIZE_RARE]));
    assert_eq!(bars["slotPoints"], json!(slot_bar_points(&slots)));
    assert_eq!(
        counter(&app, Counter::CardTokenBar).await,
        bars["card"]["after"].as_i64().expect("a number")
    );
    assert_eq!(
        counter(&app, Counter::CosmeticTokenBar).await,
        bars["cosmetic"]["after"].as_i64().expect("a number")
    );
}

/// R1999000: a slot that unlocks a new card still adds its rolled rarity's points to both bars, so
/// an opening of new cards alone moves both, and the answer carries every slot's points.
#[tokio::test]
async fn r1999000_an_opening_of_new_cards_adds_every_slot_s_points_to_both_bars() {
    let (app, token) = setup().await;
    set_counter(&app, Counter::CardTokenBar, 7).await;
    set_counter(&app, Counter::CosmeticTokenBar, 11).await;
    buy(&app, &token, "Core").await;
    // A fresh collection: every slot unlocks a card, unless one card rolls twice in this pack.
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &[], "bars-unspent")
        .await
        .expect("the opening");
    assert!(
        opened.slots.iter().all(|slot| slot.grant == SlotGrant::Card),
        "{:?}",
        opened.slots
    );
    let expected: Vec<i64> = opened
        .slots
        .iter()
        .map(|slot| card_bar_points(slot.rarity))
        .collect();
    assert!(expected.iter().all(|points| *points > 0), "{expected:?}");
    let gained: i64 = expected.iter().sum();
    assert_eq!(
        opened.bars,
        BarsSettled {
            card: BarSettle {
                before: 7,
                after: 7 + gained,
                size: CARD_TOKEN_BAR_SIZE,
                pays: 0,
            },
            cosmetic: BarSettle {
                before: 11,
                after: 11 + gained,
                size: COSMETIC_TOKEN_BAR_SIZE_RARE,
                pays: 0,
            },
            cosmetic_paid: Vec::new(),
            cosmetic_sizes: vec![COSMETIC_TOKEN_BAR_SIZE_RARE],
            slot_points: expected,
        }
    );
    assert_eq!(counter(&app, Counter::CardTokenBar).await, 7 + gained);
    assert_eq!(counter(&app, Counter::CosmeticTokenBar).await, 11 + gained);

    // On the wire, through the route, every slot's points are `bars.slotPoints`.
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
    let slots = serde_json::from_value::<OpenedPack>(body.clone())
        .expect("an opened pack")
        .slots;
    let points: Vec<i64> = slots.iter().map(|slot| card_bar_points(slot.rarity)).collect();
    assert_eq!(body["bars"]["slotPoints"], json!(points));
}

/// R1999000: a duplicate that becomes a new per-card cosmetic, and one whose global roll drops an
/// account cosmetic (R1661001), each still add their slot's card rarity's points to both bars.
#[tokio::test]
async fn r1999000_a_duplicate_s_cosmetic_and_a_global_roll_s_account_cosmetic_add_their_slot_s_points() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // A foil at every tier, so every duplicate that misses the global roll drops one, and one
    // account cosmetic the global roll may drop.
    let playmat = Cosmetic {
        kind: CosmeticKind::Table,
        ..foil("table-fixture-epic", GLOBAL_COSMETIC_FLOOR_RARITY)
    };
    let mut registry: Vec<Cosmetic> = CosmeticRarity::ALL
        .into_iter()
        .map(|rarity| foil(&format!("foil-fixture-{rarity}"), rarity))
        .collect();
    registry.push(playmat.clone());
    // The roll is pure, so a seed whose pack holds both is found before the opening, over the
    // holdings it will read.
    let pool = pack_pool(defs(&app), PackKind::Set(SetName::Core));
    let before = holdings(&app).await;
    let is_playmat = |slot: &Slot| matches!(&slot.grant, SlotGrant::Cosmetic { cosmetic_id, .. } if *cosmetic_id == playmat.id);
    let is_foil = |slot: &Slot| matches!(&slot.grant, SlotGrant::Cosmetic { cosmetic_id, .. } if *cosmetic_id != playmat.id);
    let seed = (0..10_000)
        .map(|n| format!("r1999000-dupes-{n}"))
        .find(|seed| {
            let slots = roll_opening(&pool, &before, &registry, seed)
                .expect("the roll")
                .slots;
            slots.iter().any(is_playmat) && slots.iter().any(is_foil)
        })
        .expect("a seed dropping the playmat and a foil");
    buy(&app, &token, "Core").await;
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &registry, &seed)
        .await
        .expect("the opening");
    assert!(opened.slots.iter().any(is_playmat), "{:?}", opened.slots);
    assert!(opened.slots.iter().any(is_foil), "{:?}", opened.slots);
    let expected: Vec<i64> = opened
        .slots
        .iter()
        .map(|slot| card_bar_points(slot.rarity))
        .collect();
    assert!(expected.iter().all(|points| *points > 0), "{expected:?}");
    assert_eq!(opened.bars.slot_points, expected);
    let gained: i64 = expected.iter().sum();
    assert_eq!((opened.bars.card.before, opened.bars.card.after), (0, gained));
    assert_eq!(
        (opened.bars.cosmetic.before, opened.bars.cosmetic.after),
        (0, gained)
    );
    assert_eq!(counter(&app, Counter::CardTokenBar).await, gained);
    assert_eq!(counter(&app, Counter::CosmeticTokenBar).await, gained);
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
    let card_gain = points(&opened.slots);
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
            after: card_gain,
            size: COSMETIC_TOKEN_BAR_SIZE_RARE,
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

/// R1953000: a spent duplicate adds its slot's rarity's points to each bar, so a pack whose spent
/// duplicates rolled one of each rarity adds 1 + 2 + 3 + 5 + 10 = 21 to both, and the answer
/// carries each slot's points in slot order.
#[tokio::test]
async fn r1953000_spent_duplicates_of_each_rarity_add_21_to_each_bar() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // The roll is pure, so a seed whose five slots rolled one of each rarity is found before the
    // opening, over the holdings it will read and an empty registry, which drops no cosmetic.
    let pool = pack_pool(defs(&app), PackKind::Set(SetName::Core));
    let before = holdings(&app).await;
    let every = [
        PrintedRarity::Common,
        PrintedRarity::Rare,
        PrintedRarity::Epic,
        PrintedRarity::Legendary,
        PrintedRarity::Mythic,
    ];
    let seed = (0..200_000)
        .map(|n| format!("r1953000-{n}"))
        .find(|seed| {
            let mut rarities: Vec<PrintedRarity> = roll_opening(&pool, &before, &[], seed)
                .expect("the roll")
                .slots
                .iter()
                .map(|slot| slot.rarity)
                .collect();
            rarities.sort();
            rarities == every
        })
        .expect("a seed rolling one of each rarity");
    buy(&app, &token, "Core").await;
    let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &[], &seed)
        .await
        .expect("the opening");
    assert_eq!(spent(&opened.slots), PACK_SLOTS as i64, "{:?}", opened.slots);
    let expected: Vec<i64> = opened
        .slots
        .iter()
        .map(|slot| card_bar_points(slot.rarity))
        .collect();
    assert_eq!(opened.bars.slot_points, expected);
    let mut sorted = opened.bars.slot_points.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 2, 3, 5, 10]);
    assert_eq!(opened.bars.slot_points.iter().sum::<i64>(), 21);
    assert_eq!((opened.bars.card.before, opened.bars.card.after), (0, 21));
    assert_eq!((opened.bars.cosmetic.before, opened.bars.cosmetic.after), (0, 21));
    assert_eq!(counter(&app, Counter::CardTokenBar).await, 21);
    assert_eq!(counter(&app, Counter::CosmeticTokenBar).await, 21);
}

#[tokio::test]
async fn r2728000_one_opening_pays_every_step_it_crosses_each_at_its_own_size() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // This opening's gain, every slot spent: its roll is pure, so it is known before it opens.
    let rolled = roll_opening(
        &pack_pool(defs(&app), PackKind::Set(SetName::Core)),
        &holdings(&app).await,
        &[],
        "bars-cosmetic-pays",
    )
    .expect("the roll");
    let gain = points(&rolled.slots);
    // At the cycle's Epic step, short of this opening's gain from the Epic, Legendary and Rare
    // steps' sizes together: the bar pays all three in order, each at its own size, and rests on
    // the next Epic step (R2728000).
    let crossed =
        COSMETIC_TOKEN_BAR_SIZE_EPIC + COSMETIC_TOKEN_BAR_SIZE_LEGENDARY + COSMETIC_TOKEN_BAR_SIZE_RARE;
    set_counter(&app, Counter::CosmeticTokenCycle, 1).await;
    set_counter(&app, Counter::CosmeticTokenBar, crossed - gain).await;
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
    assert_eq!(points(&opened.slots), gain);
    assert_eq!(
        opened.bars.cosmetic,
        BarSettle {
            before: crossed - gain,
            after: 0,
            size: COSMETIC_TOKEN_BAR_SIZE_EPIC,
            pays: 3,
        }
    );
    assert_eq!(
        opened.bars.cosmetic_paid,
        vec![
            CosmeticTokenStep::Epic,
            CosmeticTokenStep::Legendary,
            CosmeticTokenStep::Rare
        ]
    );
    assert_eq!(opened.bars.cosmetic_sizes, vec![75, 100, 50, 75]);
    assert_eq!(
        opened.bars.cosmetic_sizes,
        vec![
            COSMETIC_TOKEN_BAR_SIZE_EPIC,
            COSMETIC_TOKEN_BAR_SIZE_LEGENDARY,
            COSMETIC_TOKEN_BAR_SIZE_RARE,
            COSMETIC_TOKEN_BAR_SIZE_EPIC
        ]
    );
    assert_eq!(
        to(&opened.bars)["cosmeticPaid"],
        json!(["epic", "legendary", "rare"])
    );
    assert_eq!(to(&opened.bars)["cosmeticSizes"], json!([75, 100, 50, 75]));
    assert_eq!(opened.bars.card.pays, 0);
    assert_eq!(counter(&app, Counter::CosmeticTokenCycle).await, 4);

    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["tokens"]["cosmeticRare"], json!(1));
    assert_eq!(body["tokens"]["cosmeticEpic"], json!(1));
    assert_eq!(body["tokens"]["cosmeticLegendary"], json!(1));
    assert_eq!(body["tokens"]["cosmeticPack"], json!(0));
    assert_eq!(body["nextStep"], json!("epic"));
    assert_eq!(
        body["cosmeticBar"],
        json!({ "points": 0, "size": COSMETIC_TOKEN_BAR_SIZE_EPIC })
    );
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
    let registry = shipped().expect("the registry");
    assert_eq!(
        log.cosmetics,
        [pack_cosmetics(&registry), global_cosmetics(&registry)].concat()
    );
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
        points(&opened.slots) != 0,
        points(&opened.slots) != 0,
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
    fail_on(&app, "collection.appendGrants", "injected fault").await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    stop_failing(&app).await;
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
    fail_on(&app, "economy.counters.grant", "injected fault").await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/open",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    stop_failing(&app).await;
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

/// R1953000: the global cosmetic roll is 20 of 1,000, and `/api/packs` publishes it as 2%.
#[tokio::test]
async fn r1953000_get_publishes_a_two_percent_global_roll() {
    let (app, token) = setup().await;
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(GLOBAL_COSMETIC_ROLL_WEIGHT, 20);
    assert_eq!(body["odds"]["globalCosmetic"]["chance"], json!(0.02));
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
async fn r1827000_r1833000_a_duplicate_drops_each_account_cosmetic_on_no_card_until_all_are_owned() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // The shipped registry, whose global roll drops the five portraits but vanilla (R1827000) and
    // #1833's six playmats, six backdrops and six card backs (R1833000): every slot a duplicate, so
    // each takes the global draw while one is left unowned.
    let registry = shipped().expect("the registry");
    let global: Vec<String> = global_cosmetics(&registry)
        .into_iter()
        .map(|cosmetic| cosmetic.id)
        .collect();
    assert_eq!(global.len(), 5 + 18);
    assert_eq!(global.iter().filter(|id| portrait_of(id).is_some()).count(), 5);
    let pool = pack_pool(defs(&app), PackKind::Set(SetName::Core));
    let mut dropped = 0;
    // A duplicate's global roll hits GLOBAL_COSMETIC_ROLL_WEIGHT times in TIER_WEIGHT_TOTAL, so a
    // few dozen openings own them all.
    for at in 0..400 {
        let before = holdings(&app).await;
        if global.iter().all(|id| before.account_cosmetics.contains(id)) {
            break;
        }
        buy(&app, &token, "Core").await;
        let seed = format!("r1827000-{at}");
        let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &registry, &seed)
            .await
            .expect("the opening");
        // The log holds the per-card cosmetics, then the global ones, and replays (R1301003).
        let log = last_log(&app).await;
        assert_eq!(
            log.cosmetics,
            [pack_cosmetics(&registry), global_cosmetics(&registry)].concat()
        );
        let replayed = roll_opening(&pool, &before, &log.cosmetics, &log.seed).expect("the replay");
        assert_eq!(replayed.slots, opened.slots);
        assert_eq!(replayed.draws, log.draws);
        let after = holdings(&app).await;
        for slot in &opened.slots {
            let SlotGrant::Cosmetic {
                cosmetic_id,
                cosmetic_rarity,
            } = &slot.grant
            else {
                continue;
            };
            let Some(entry) = registry
                .iter()
                .find(|cosmetic| cosmetic.id == *cosmetic_id && !cosmetic.kind.is_per_card())
            else {
                continue;
            };
            // An account cosmetic at the floor or above, at its own rarity, owned on the account
            // and on no card.
            dropped += 1;
            assert_eq!(*cosmetic_rarity, entry.rarity, "{slot:?}");
            assert!(*cosmetic_rarity >= GLOBAL_COSMETIC_FLOOR_RARITY, "{slot:?}");
            assert!(after.account_cosmetics.contains(cosmetic_id), "{slot:?}");
            assert!(
                !before.account_cosmetics.contains(cosmetic_id),
                "{cosmetic_id} drops once"
            );
            assert!(
                after
                    .cosmetics
                    .iter()
                    .all(|held| &held.cosmetic_id != cosmetic_id),
                "{slot:?}"
            );
        }
    }
    assert_eq!(dropped, global.len());

    // All of them owned, each by one `pack_open` row on no card.
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let cosmetics = tx.economy_cosmetics_get(PROFILE).await.expect("the cosmetics");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    let mut owned: Vec<&String> = cosmetics.account.iter().collect();
    owned.sort();
    let mut expected: Vec<&String> = global.iter().collect();
    expected.sort();
    assert_eq!(owned, expected);
    for id in &global {
        let rows: Vec<_> = ledger
            .cosmetics
            .iter()
            .filter(|row| &row.cosmetic_id == id)
            .map(|row| (row.card_id.clone(), row.delta, row.reason))
            .collect();
        assert_eq!(rows, vec![(None, 1, EconomyReason::PackOpen)], "{id}");
    }

    // Nothing is left to own: a later duplicate takes no global draw, so an opening rolls as it
    // would over the per-card cosmetics alone (R1661001).
    for at in 0..3 {
        buy(&app, &token, "Core").await;
        let before = holdings(&app).await;
        let seed = format!("r1827000-owned-{at}");
        let opened = open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &registry, &seed)
            .await
            .expect("the opening");
        let alone = roll_opening(&pool, &before, &pack_cosmetics(&registry), &seed).expect("the roll");
        assert_eq!(opened.slots, alone.slots, "{seed}");
        assert_eq!(last_log(&app).await.draws, alone.draws, "{seed}");
    }
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

// ---------------------------------------------------------------------------------------------
// R2124000: a buy or an opening sent again under its Idempotency-Key runs once
// ---------------------------------------------------------------------------------------------

/// `profile_id`'s wallet and the packs it holds, each kind with its count.
async fn wallet_and_packs(app: &App, profile_id: &str) -> (i64, Value) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a transaction");
    let jlore = tx.economy_jlore_get(profile_id).await.expect("the wallet");
    let inventory = to(&tx.economy_inventory_get(profile_id).await.expect("the inventory"));
    tx.commit().await.expect("the commit");
    (jlore, inventory)
}

/// How many idempotency keys the store holds, every account's.
async fn keys_held(app: &App) -> usize {
    fake(app).lock().await.tables.economy.idempotency_keys.len()
}

/// The ledger's `pack_purchase` Jlore rows of `p1`.
fn purchases(everything: &Value) -> usize {
    everything["state"]["ledger"]["jlore"]
        .as_array()
        .expect("the Jlore rows")
        .iter()
        .filter(|row| row["reason"] == json!("pack_purchase"))
        .count()
}

#[tokio::test]
async fn r2124000_the_same_key_twice_buys_one_pack_and_answers_the_same() {
    let (app, token) = setup().await;
    fund(&app, price() * 2).await;
    let key = uuid(2124001);
    let buy = || {
        call_with_key(
            &app,
            "POST",
            "/api/packs/buy",
            Some(&token),
            json!({ "pack": "Core" }),
            &key,
        )
    };

    let (first_status, _, first) = buy().await;
    assert_eq!(first_status, 200, "{first}");
    assert_eq!(first, json!({ "pack": "Core", "jlore": price(), "unopened": 1 }));
    let (again_status, _, again) = buy().await;
    assert_eq!(again_status, 200, "{again}");
    assert_eq!(again, first, "the retry answers what the first try answered");

    let after = everything(&app).await;
    assert_eq!(after["state"]["jlore"], json!(price()), "one price spent");
    assert_eq!(
        after["state"]["inventory"],
        json!([{ "item": "set_pack", "key": "Core", "count": 1 }])
    );
    assert_eq!(purchases(&after), 1);
    assert_eq!(keys_held(&app).await, 1);
}

#[tokio::test]
async fn r2124000_the_same_key_with_another_body_or_route_is_refused_and_writes_nothing() {
    let (app, token) = setup().await;
    fund(&app, price() * 3).await;
    let key = uuid(2124002);
    let (status, _, body) = call_with_key(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": "Core" }),
        &key,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let before = everything(&app).await;

    for (path, pack) in [("/api/packs/buy", GLOBAL_PACK_NAME), ("/api/packs/open", "Core")] {
        let (status, _, body) =
            call_with_key(&app, "POST", path, Some(&token), json!({ "pack": pack }), &key).await;
        assert_eq!(
            (status, body["error"]["code"].clone()),
            (422, json!("idempotency_key_reused")),
            "{path} {pack}: {body}"
        );
        assert_eq!(everything(&app).await, before, "{path} {pack}");
        assert_eq!(keys_held(&app).await, 1, "{path} {pack}");
    }
}

#[tokio::test]
async fn r2124000_a_buy_with_no_key_buys_again_as_today() {
    let (app, token) = setup().await;
    fund(&app, price() * 2).await;
    for unopened in [1, 2] {
        let (status, _, body) = call(
            &app,
            "POST",
            "/api/packs/buy",
            Some(&token),
            json!({ "pack": "Core" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["unopened"], json!(unopened));
    }
    let after = everything(&app).await;
    assert_eq!(after["state"]["jlore"], json!(0));
    assert_eq!(purchases(&after), 2);
    assert_eq!(keys_held(&app).await, 0, "a keyless spend keeps nothing");
}

#[tokio::test]
async fn r2124000_the_same_key_under_two_accounts_makes_two_purchases() {
    let (app, token) = setup().await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "p2", "userId": "u2", "status": "active" }));
    let other = add_user(&app, "u2", "p2@example.test", true);
    let key = uuid(2124003);
    for (profile_id, bearer) in [(PROFILE, &token), ("p2", &other)] {
        fund_of(&app, profile_id, price() * 2).await;
        let (status, _, body) = call_with_key(
            &app,
            "POST",
            "/api/packs/buy",
            Some(bearer),
            json!({ "pack": "Core" }),
            &key,
        )
        .await;
        assert_eq!(status, 200, "{profile_id}: {body}");
        assert_eq!(
            body,
            json!({ "pack": "Core", "jlore": price(), "unopened": 1 }),
            "{profile_id}"
        );
    }
    for profile_id in [PROFILE, "p2"] {
        assert_eq!(
            wallet_and_packs(&app, profile_id).await,
            (
                price(),
                json!([{ "item": "set_pack", "key": "Core", "count": 1 }])
            ),
            "{profile_id} is debited once and holds one pack"
        );
    }
    assert_eq!(keys_held(&app).await, 2);
}

#[tokio::test]
async fn r2124000_a_key_that_is_not_a_uuid_is_refused_and_buys_nothing() {
    let (app, token) = setup().await;
    fund(&app, price()).await;
    let before = everything(&app).await;
    for key in ["", "not-a-uuid", "2124", "00000000-0000-4000-8000-00000000000g"] {
        let (status, _, body) = call_with_key(
            &app,
            "POST",
            "/api/packs/buy",
            Some(&token),
            json!({ "pack": "Core" }),
            key,
        )
        .await;
        assert_eq!(
            (status, body["error"]["code"].clone()),
            (400, json!("bad_request")),
            "{key:?}: {body}"
        );
    }
    assert_eq!(everything(&app).await, before);
    assert_eq!(keys_held(&app).await, 0);
}

#[tokio::test]
async fn r2124000_the_same_key_twice_opens_one_pack_and_answers_the_same_slots() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    buy(&app, &token, "Core").await;
    let key = uuid(2124004);
    let open = || {
        call_with_key(
            &app,
            "POST",
            "/api/packs/open",
            Some(&token),
            json!({ "pack": "Core" }),
            &key,
        )
    };

    let (first_status, _, first) = open().await;
    assert_eq!(first_status, 200, "{first}");
    assert_eq!(first["unopened"], json!(1));
    let after_first = everything(&app).await;
    let (again_status, _, again) = open().await;
    assert_eq!(again_status, 200, "{again}");
    assert_eq!(again, first, "the retry answers the first try's slots");
    assert_eq!(
        everything(&app).await,
        after_first,
        "the retry rolls and grants nothing"
    );
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 1);
}

// ---------------------------------------------------------------------------------------------
// R2378000: an account runs one spend at a time, the rest waiting their turn
// ---------------------------------------------------------------------------------------------

/// How many times `until_claims` lets the spawned requests run before it gives up.
const YIELDS: usize = 1_000;

/// Lets the spawned requests run until `profile_id` has `claims` spends running or waiting.
async fn until_claims(app: &App, profile_id: &str, claims: usize) {
    for _ in 0..YIELDS {
        if app.spends.claims(profile_id) == claims {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!(
        "{profile_id} has {} spends, not {claims}",
        app.spends.claims(profile_id)
    );
}

/// `count` spends of `profile_id`'s own, running or waiting their turn, each held until the
/// sender this answers is dropped.
async fn hold_turns(app: &Arc<App>, profile_id: &'static str, count: usize) -> watch::Sender<()> {
    let (release, held) = watch::channel(());
    for _ in 0..count {
        let (app, mut held) = (Arc::clone(app), held.clone());
        tokio::spawn(async move {
            let _turn = app.spends.claim(profile_id).await;
            // Errs once the sender is dropped.
            held.changed().await.ok();
        });
    }
    until_claims(app, profile_id, count).await;
    release
}

/// As many spends as an account runs and keeps waiting at once.
fn full() -> usize {
    ECONOMY_SPENDS_RUNNING_MAX + ECONOMY_SPENDS_WAITING_MAX
}

/// The refusal of a spend sent past the waiting ones.
fn assert_busy(status: u16, body: &Value) {
    assert_eq!(status, 429, "{body}");
    assert_eq!(body["error"]["code"], json!("rate_limited"), "{body}");
    assert_eq!(body["error"]["message"], json!(SPEND_BUSY_MESSAGE), "{body}");
    assert_eq!(
        body["error"]["details"]["retryAfterMs"],
        json!(SPEND_BUSY_RETRY_AFTER_MS),
        "{body}"
    );
}

/// A spend of `{ pack: "Core" }` on `path`, sent from its own task, with `key` when given.
fn spawn_spend(
    app: &Arc<App>,
    token: &str,
    path: &'static str,
    key: Option<&str>,
) -> tokio::task::JoinHandle<(u16, axum::http::HeaderMap, Value)> {
    let (app, token, key) = (Arc::clone(app), token.to_string(), key.map(str::to_string));
    tokio::spawn(async move {
        let body = json!({ "pack": "Core" });
        match key {
            Some(key) => call_with_key(&app, "POST", path, Some(&token), body, &key).await,
            None => call(&app, "POST", path, Some(&token), body).await,
        }
    })
}

#[tokio::test]
async fn r2378000_a_buy_and_an_open_sent_while_a_spend_runs_wait_their_turn_then_run() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    fund(&app, price()).await;
    let before = everything(&app).await;
    let release = hold_turns(&app, PROFILE, ECONOMY_SPENDS_RUNNING_MAX).await;

    let opened = spawn_spend(&app, &token, "/api/packs/open", None);
    let bought = spawn_spend(&app, &token, "/api/packs/buy", None);
    until_claims(&app, PROFILE, ECONOMY_SPENDS_RUNNING_MAX + 2).await;
    assert!(
        !opened.is_finished() && !bought.is_finished(),
        "both wait their turn"
    );
    assert_eq!(everything(&app).await, before, "and neither has run");

    drop(release);
    for (what, request) in [("the open", opened), ("the buy", bought)] {
        let (status, _, body) = request.await.expect("the request's task");
        assert_eq!(status, 200, "{what}: {body}");
    }
    until_claims(&app, PROFILE, 0).await;
    assert_eq!(purchases(&everything(&app).await), 2);
    assert_eq!(held(&app, InventoryItem::SetPack, "Core").await, 1);
}

#[tokio::test]
async fn r2378000_a_buy_or_an_open_sent_past_the_waiting_ones_is_refused_at_once_and_writes_nothing() {
    let (app, token) = setup().await;
    buy(&app, &token, "Core").await;
    fund(&app, price()).await;
    let before = everything(&app).await;
    let release = hold_turns(&app, PROFILE, full()).await;

    for path in ["/api/packs/buy", "/api/packs/open"] {
        let (status, _, body) = call(&app, "POST", path, Some(&token), json!({ "pack": "Core" })).await;
        assert_busy(status, &body);
    }
    assert_eq!(everything(&app).await, before, "a refused spend writes nothing");
    assert_eq!(app.spends.claims(PROFILE), full(), "and takes no place");
    let (status, _, body) = call(&app, "GET", "/api/packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "a read takes no turn: {body}");

    drop(release);
    until_claims(&app, PROFILE, 0).await;
    assert_eq!(open(&app, &token, "Core").await.unopened, 0);
}

#[tokio::test]
async fn r2378000_a_refused_or_failed_spend_gives_its_turn_back() {
    let (app, token) = setup().await;
    let open_core = || {
        call(
            &app,
            "POST",
            "/api/packs/open",
            Some(&token),
            json!({ "pack": "Core" }),
        )
    };
    let (status, _, body) = open_core().await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(app.spends.claims(PROFILE), 0, "a refusal gives its turn back");

    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    buy(&app, &token, "Core").await;
    fail_on(&app, "economy.counters.grant", "injected fault").await;
    let (status, _, body) = open_core().await;
    stop_failing(&app).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(app.spends.claims(PROFILE), 0, "so does a failure");
    open(&app, &token, "Core").await;
}

#[tokio::test]
async fn r2378000_a_retry_racing_its_first_try_waits_its_turn_and_answers_as_the_first_did() {
    let (app, token) = setup().await;
    fund(&app, price() * 2).await;
    let key = uuid(2378001);
    let release = hold_turns(&app, PROFILE, ECONOMY_SPENDS_RUNNING_MAX).await;

    let first = spawn_spend(&app, &token, "/api/packs/buy", Some(&key));
    let retry = spawn_spend(&app, &token, "/api/packs/buy", Some(&key));
    until_claims(&app, PROFILE, ECONOMY_SPENDS_RUNNING_MAX + 2).await;
    drop(release);
    let (first_status, _, first) = first.await.expect("the first try's task");
    let (retry_status, _, retry) = retry.await.expect("the retry's task");
    assert_eq!((first_status, retry_status), (200, 200), "{first} {retry}");
    assert_eq!(retry, first, "the retry answers what its first try answered");
    assert_eq!(purchases(&everything(&app).await), 1);
    assert_eq!(keys_held(&app).await, 1);
}

#[tokio::test]
async fn r2378000_a_keyed_buy_refused_past_the_waiting_ones_keeps_nothing_and_runs_when_sent_again() {
    let (app, token) = setup().await;
    fund(&app, price()).await;
    let key = uuid(2378002);
    let buy_keyed = || {
        call_with_key(
            &app,
            "POST",
            "/api/packs/buy",
            Some(&token),
            json!({ "pack": "Core" }),
            &key,
        )
    };
    let release = hold_turns(&app, PROFILE, full()).await;
    let (status, _, body) = buy_keyed().await;
    assert_busy(status, &body);
    assert_eq!(keys_held(&app).await, 0, "a refusal keeps no key");

    drop(release);
    until_claims(&app, PROFILE, 0).await;
    let (first_status, _, first) = buy_keyed().await;
    assert_eq!(first_status, 200, "{first}");
    let (again_status, _, again) = buy_keyed().await;
    assert_eq!(again_status, 200, "{again}");
    assert_eq!(again, first);
    assert_eq!(purchases(&everything(&app).await), 1);
}

#[tokio::test]
async fn r2378000_another_account_spends_while_one_account_s_turns_are_full() {
    let (app, token) = setup().await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "p2", "userId": "u2", "status": "active" }));
    let other = add_user(&app, "u2", "p2@example.test", true);
    fund_of(&app, PROFILE, price()).await;
    fund_of(&app, "p2", price()).await;
    let release = hold_turns(&app, PROFILE, full()).await;

    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&other),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(status, 200, "p2: {body}");
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/packs/buy",
        Some(&token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_busy(status, &body);
    drop(release);
}
