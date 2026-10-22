//! Cosmetic packs and the Chase Shard bar (`src/economy/cosmetic_packs.rs`, SPEC §14.6; #1306):
//! the slots an opening gives, the cosmetics' fall to a lower tier and to the bars, the bar
//! counting, paying and holding, what it buys and refuses, the read-back and the replay.
//!
//! Every request goes through the whole app over an empty fake store (`deps::empty_test_app`), as
//! the active account `p1`. Openings the test must steer (a fixture registry, so a tier is empty
//! or a cosmetic is for an unowned card) call `open_cosmetic_pack` with that registry and a seed,
//! as `tokens.rs` calls `open_pack`; the opening through the route, the read-back and the shipped
//! registry's cases go through the routes.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::api::collection::{GrantInput, grant_cards, owned_map};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    CARD_TOKEN_BAR_POINTS_PER_DUPLICATE, CARD_TOKEN_BAR_SIZE, CHASE_SHARD_PACKS, COSMETIC_PACK_FLOOR_RARITY,
    COSMETIC_PACK_SLOTS, COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE, COSMETIC_TOKEN_BAR_SIZE, CosmeticTokenStep,
};
use jackioh_server::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, shipped};
use jackioh_server::db::store::CollectionEntry;
use jackioh_server::economy::cosmetic_packs::{
    CosmeticHoldings, CosmeticOpeningLog, CosmeticSlotGrant, OpenedCosmeticPack, buy_chase,
    open_cosmetic_pack, roll_cosmetic_pack,
};
use jackioh_server::economy::packs::{BarSettle, BarsSettled};
use jackioh_server::economy::roll::{PackKind, pack_pool};
use jackioh_server::economy::store::{
    CosmeticGrant, Counter, CounterGrant, EconomyLedger, EconomyReason, InventoryGrant, InventoryItem,
};
use jackioh_server::economy::tiers::cosmetic_odds;

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::store::fake;

const PROFILE: &str = "p1";
const USER: &str = "u1";

// A cosmetic pack has a slot for each of the two full arts a test below fixes.
const _: () = assert!(COSMETIC_PACK_SLOTS >= 2);

/// The three routes, as `(method, path)`.
const ROUTES: [(&str, &str); 3] = [
    ("GET", "/api/cosmetic-packs"),
    ("POST", "/api/cosmetic-packs/open"),
    ("POST", "/api/cosmetic-packs/chase"),
];

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// An empty app with the active `p1` and its Bearer [REDACTED]
async fn setup() -> (Arc<App>, String) {
    let app = empty_test_app().await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": PROFILE, "userId": USER, "status": "active" }));
    let token = add_user(&app, USER, "p1@example.test", true);
    (app, token)
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
async fn counter_of(app: &App, counter: Counter) -> i64 {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let counters = tx.economy_counters_get(PROFILE).await.expect("the counters");
    tx.commit().await.expect("the commit");
    counters
        .iter()
        .find(|entry| entry.counter == counter && entry.pack.is_empty())
        .map_or(0, |entry| entry.value)
}

/// Sets one counter to `value`, as if earlier openings had left it there.
async fn set_counter(app: &App, counter: Counter, value: i64) {
    let current = counter_of(app, counter).await;
    if current == value {
        return;
    }
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    tx.economy_counters_grant(&CounterGrant {
        profile_id: PROFILE.to_string(),
        counter,
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

/// Grants `delta` of one inventory item, as if a bar had paid it.
async fn grant_item(app: &App, item: InventoryItem, key: &str, delta: i64) {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    tx.economy_inventory_grant(&InventoryGrant {
        profile_id: PROFILE.to_string(),
        item,
        key: key.to_string(),
        delta,
        reason: EconomyReason::TokenBar,
        reference: None,
        dev: false,
        at: app::now_ms(),
    })
    .await
    .expect("the grant")
    .expect("a grant the inventory takes");
    tx.commit().await.expect("the commit");
}

/// How many of `item` under `key` `p1` holds.
async fn held(app: &App, item: InventoryItem, key: &str) -> i64 {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let inventory = tx.economy_inventory_get(PROFILE).await.expect("the inventory");
    tx.commit().await.expect("the commit");
    inventory
        .iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count)
}

/// `p1`'s economy ledger.
async fn ledger(app: &App) -> EconomyLedger {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let ledger = tx.economy_ledger_get(PROFILE).await.expect("the ledger");
    tx.commit().await.expect("the commit");
    ledger
}

/// `p1`'s holdings as an opening reads them.
async fn holdings(app: &App) -> CosmeticHoldings {
    let owned = owned_map(app, PROFILE).await.expect("the collection");
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let cosmetics = tx.economy_cosmetics_get(PROFILE).await.expect("the cosmetics");
    tx.commit().await.expect("the commit");
    CosmeticHoldings::of(owned, &cosmetics)
}

/// The first three non-token cards of the catalog.
fn three_cards(app: &App) -> Vec<String> {
    app.catalog
        .card_ids
        .iter()
        .filter(|id| !app.catalog.is_token(id))
        .take(3)
        .cloned()
        .collect()
}

/// Owns `cards` of `p1`, as a reward would.
async fn own(app: &App, cards: &[String]) {
    grant_cards(
        app,
        GrantInput {
            profile_id: PROFILE.to_string(),
            entries: cards
                .iter()
                .map(|card_id| CollectionEntry {
                    card_id: card_id.clone(),
                    quantity: 1,
                })
                .collect(),
            reason: "admin".to_string(),
        },
    )
    .await
    .expect("a grant");
}

/// Owns `cosmetic_id` on `card_id` (or on the account), as an earlier opening would have.
async fn give(app: &App, card_id: Option<&str>, cosmetic_id: &str) {
    let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
    let granted = tx
        .economy_cosmetics_grant(&CosmeticGrant {
            profile_id: PROFILE.to_string(),
            card_id: card_id.map(str::to_string),
            cosmetic_id: cosmetic_id.to_string(),
            delta: 1,
            reason: EconomyReason::PackOpen,
            reference: None,
            dev: false,
            at: app::now_ms(),
        })
        .await
        .expect("the grant");
    assert!(granted, "{cosmetic_id} was already owned");
    tx.commit().await.expect("the commit");
}

/// `p1` owns `cosmetic_id` on `card_id` (or on the account).
async fn owns_cosmetic(app: &App, card_id: Option<&str>, cosmetic_id: &str) -> bool {
    let held = holdings(app).await;
    match card_id {
        Some(card_id) => held
            .card_cosmetics
            .contains(&(card_id.to_string(), cosmetic_id.to_string())),
        None => held.account_cosmetics.contains(cosmetic_id),
    }
}

/// An opening answered by the route, read back as the type the route serialises.
fn opened(body: &Value) -> OpenedCosmeticPack {
    serde_json::from_value(body.clone()).unwrap_or_else(|error| panic!("{error}: {body}"))
}

// ---------------------------------------------------------------------------------------------
// A fixture registry (per-card cosmetics at every tier, and one of each refusal)
// ---------------------------------------------------------------------------------------------

fn cosmetic(id: &str, kind: CosmeticKind, rarity: CosmeticRarity) -> Cosmetic {
    Cosmetic {
        id: id.to_string(),
        kind,
        name: id.to_string(),
        rarity,
        default: false,
        starter: false,
        chase: rarity.is_chase(),
        card: None,
    }
}

fn foil(id: &str, rarity: CosmeticRarity) -> Cosmetic {
    cosmetic(id, CosmeticKind::Foil, rarity)
}

fn full_art(id: &str, rarity: CosmeticRarity, card_id: &str) -> Cosmetic {
    Cosmetic {
        card: Some(card_id.to_string()),
        ..cosmetic(id, CosmeticKind::FullArt, rarity)
    }
}

/// What a Chase Shard purchase's tests choose from.
fn registry(card_id: &str) -> Vec<Cosmetic> {
    vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-rare", CosmeticRarity::Rare),
        foil("foil-fixture-epic", CosmeticRarity::Epic),
        foil("foil-fixture-legendary", CosmeticRarity::Legendary),
        foil("foil-fixture-owned-legendary", CosmeticRarity::Legendary),
        Cosmetic {
            starter: true,
            ..foil("foil-fixture-starter-legendary", CosmeticRarity::Legendary)
        },
        Cosmetic {
            chase: true,
            ..foil("foil-fixture-chase", CosmeticRarity::Legendary)
        },
        cosmetic(
            "table-fixture-mythic",
            CosmeticKind::Table,
            CosmeticRarity::Mythic,
        ),
        full_art("full-art-fixture-legendary", CosmeticRarity::Legendary, card_id),
    ]
}

/// A full bar's worth of openings short of full, as earlier packs left it.
async fn nearly_full(app: &App) {
    set_counter(app, Counter::ChaseShardBar, CHASE_SHARD_PACKS - 1).await;
}

/// The rows an opening wrote past `before`, as `(inventory, counters, cosmetics)` references.
fn new_references(before: &EconomyLedger, after: &EconomyLedger) -> Vec<Option<String>> {
    after.inventory[before.inventory.len()..]
        .iter()
        .map(|row| row.reference.clone())
        .chain(
            after.counters[before.counters.len()..]
                .iter()
                .map(|row| row.reference.clone()),
        )
        .chain(
            after.cosmetics[before.cosmetics.len()..]
                .iter()
                .map(|row| row.reference.clone()),
        )
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Opening (R1306000, R1306001)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1306000_a_cosmetic_pack_opens_its_slots_one_epic_or_better() {
    let (app, token) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 2).await;

    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let pack = opened(&body);
    assert_eq!(pack.slots.len(), COSMETIC_PACK_SLOTS);
    assert!(
        pack.slots[0].rarity >= CosmeticRarity::Epic,
        "{:?}",
        pack.slots[0]
    );
    assert_eq!(pack.unopened, 1);
    assert_eq!(held(&app, InventoryItem::CosmeticPack, "").await, 1);
    // The first slot's cosmetic is Epic or better, whatever the registry still held.
    match &pack.slots[0].grant {
        CosmeticSlotGrant::Cosmetic { cosmetic_rarity, .. } => {
            assert!(*cosmetic_rarity >= CosmeticRarity::Epic, "{:?}", pack.slots[0]);
        }
        CosmeticSlotGrant::Bars => panic!("the shipped full arts are Epic and open: {:?}", pack.slots[0]),
    }
    // Every cosmetic granted is owned, on its card or the account; the shipped registry's
    // droppable ones are the six Epic full arts.
    for slot in &pack.slots {
        if let CosmeticSlotGrant::Cosmetic {
            cosmetic_id,
            card_id,
            cosmetic_rarity,
            ..
        } = &slot.grant
        {
            assert_eq!(*cosmetic_rarity, CosmeticRarity::Epic, "{slot:?}");
            assert!(card_id.is_some(), "{slot:?}");
            assert!(
                owns_cosmetic(&app, card_id.as_deref(), cosmetic_id).await,
                "{slot:?}"
            );
        }
    }
    // The pack's bar moved by one.
    assert_eq!(pack.shard_bar.points, 1);
    assert_eq!(pack.shard_bar.size, CHASE_SHARD_PACKS);
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, 1);
}

#[tokio::test]
async fn r1306000_common_cosmetics_come_from_a_cosmetic_pack() {
    let (app, _) = setup().await;
    // Nothing above Common is droppable: the first slot, whose floor is Epic, finds none and takes
    // the Common cosmetic that is left.
    let registry = vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        cosmetic(
            "table-fixture-common",
            CosmeticKind::Table,
            CosmeticRarity::Common,
        ),
    ];
    let packs = 5;
    grant_item(&app, InventoryItem::CosmeticPack, "", packs).await;
    let mut granted = Vec::new();
    for at in 0..packs {
        let pack = open_cosmetic_pack(&app, PROFILE, &registry, &format!("r1306000-common-{at}"))
            .await
            .expect("the opening");
        assert_eq!(pack.slots.len(), COSMETIC_PACK_SLOTS);
        granted.extend(pack.slots);
    }
    let mut cards = 0;
    let mut account = 0;
    for slot in &granted {
        let CosmeticSlotGrant::Cosmetic {
            cosmetic_id,
            cosmetic_rarity,
            card_id,
            ..
        } = &slot.grant
        else {
            continue;
        };
        assert_eq!(*cosmetic_rarity, CosmeticRarity::Common, "{slot:?}");
        assert!(
            owns_cosmetic(&app, card_id.as_deref(), cosmetic_id).await,
            "{slot:?}"
        );
        match card_id {
            Some(_) => cards += 1,
            None => account += 1,
        }
    }
    assert!(cards > 0, "no Common card cosmetic in {granted:?}");
    // The account cosmetic is one, owned once, and the pool has no card left for the rest.
    assert_eq!(account, 1, "{granted:?}");
}

#[tokio::test]
async fn r1306000_an_unowned_card_s_cosmetic_also_grants_the_card() {
    let (app, _) = setup().await;
    let cards = three_cards(&app);
    // `cards[1]` is owned already, `cards[0]` is not.
    own(&app, &cards[1..2]).await;
    let registry = vec![
        full_art("full-art-fixture-unowned", CosmeticRarity::Epic, &cards[0]),
        full_art("full-art-fixture-owned", CosmeticRarity::Epic, &cards[1]),
    ];
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    let pack = open_cosmetic_pack(&app, PROFILE, &registry, "r1306000-card")
        .await
        .expect("the opening");

    let mut seen = Vec::new();
    for slot in &pack.slots {
        if let CosmeticSlotGrant::Cosmetic {
            card_id: Some(card_id),
            card_granted,
            ..
        } = &slot.grant
        {
            assert_eq!(*card_granted, *card_id == cards[0], "{slot:?}");
            seen.push(card_id.clone());
        }
    }
    // Each full art drops once, on its own card, so both are granted when the pack has slots.
    assert_eq!(seen.len(), COSMETIC_PACK_SLOTS.min(2));
    assert!(seen.contains(&cards[0]), "{seen:?}");
    // The card is owned, and the collection's ledger says it came from a pack, once.
    let owned = owned_map(&app, PROFILE).await.expect("the collection");
    assert_eq!(owned[&cards[0]], 1);
    assert_eq!(owned[&cards[1]], 1);
    assert!(owns_cosmetic(&app, Some(&cards[0]), "full-art-fixture-unowned").await);
    let grants = fake(&app).lock().await.tables.grants.clone();
    let rows: Vec<_> = grants.iter().filter(|grant| grant.card_id == cards[0]).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].delta, rows[0].reason.as_str()), (1, "pack"));
    // The owned card's only row is the admin grant that gave it.
    let rows: Vec<_> = grants.iter().filter(|grant| grant.card_id == cards[1]).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reason, "admin");
}

#[tokio::test]
async fn r1306001_a_legendary_or_mythic_roll_falls_to_the_highest_tier_with_something_unowned() {
    let (app, _) = setup().await;
    // The shipped registry has no Legendary or Mythic cosmetic a pack can drop: its droppable
    // ones are six Epic full arts. A seed whose first slot rolls Legendary, and one whose first
    // slot rolls Mythic, each find the Epic one.
    let registry = shipped().expect("the shipped registry");
    let pool = pack_pool(&app.catalog.defs, PackKind::Global);
    for rolled in [CosmeticRarity::Legendary, CosmeticRarity::Mythic] {
        let seed = (0..)
            .map(|at| format!("r1306001-{rolled}-{at}"))
            .find(|seed| {
                roll_cosmetic_pack(&pool, &CosmeticHoldings::default(), &registry, seed).slots[0].rarity
                    == rolled
            })
            .expect("a seed that rolls the tier");
        let (app, _) = setup().await;
        grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
        let pack = open_cosmetic_pack(&app, PROFILE, &registry, &seed)
            .await
            .expect("the opening");
        assert_eq!(pack.slots[0].rarity, rolled);
        let CosmeticSlotGrant::Cosmetic {
            cosmetic_id,
            cosmetic_rarity,
            card_id,
            ..
        } = &pack.slots[0].grant
        else {
            panic!("an Epic full art was open: {:?}", pack.slots[0]);
        };
        assert_eq!(*cosmetic_rarity, CosmeticRarity::Epic);
        assert!(owns_cosmetic(&app, card_id.as_deref(), cosmetic_id).await);
    }
}

#[tokio::test]
async fn r1306001_nothing_unowned_pays_bar_points() {
    let (app, _) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    // An empty registry drops nothing: every slot is spent.
    let pack = open_cosmetic_pack(&app, PROFILE, &[], "r1306001")
        .await
        .expect("the opening");
    assert!(
        pack.slots
            .iter()
            .all(|slot| slot.grant == CosmeticSlotGrant::Bars)
    );
    let spent = COSMETIC_PACK_SLOTS as i64;
    assert_eq!(
        counter_of(&app, Counter::CardTokenBar).await,
        spent * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE
    );
    assert_eq!(
        counter_of(&app, Counter::CosmeticTokenBar).await,
        spent * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE
    );
    // The spent slots still count as one pack for the Chase Shard bar.
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, 1);
    // One row per bar, carrying the opening's log.
    let ledger = ledger(&app).await;
    for counter in [Counter::CardTokenBar, Counter::CosmeticTokenBar] {
        let rows: Vec<_> = ledger
            .counters
            .iter()
            .filter(|row| row.counter == counter)
            .collect();
        assert_eq!(rows.len(), 1, "{counter:?}");
        assert_eq!(rows[0].reason, EconomyReason::PackOpen);
        assert!(rows[0].reference.is_some());
    }
    assert!(holdings(&app).await.card_cosmetics.is_empty());
}

#[tokio::test]
async fn a_cosmetic_pack_s_spent_slots_answer_each_bar_before_and_after() {
    let (app, token) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    // The card bar one point short of full, the cosmetic bar left at its start; an empty registry
    // drops nothing, so every slot is spent and both bars gain.
    set_counter(&app, Counter::CardTokenBar, CARD_TOKEN_BAR_SIZE - 1).await;
    let pack = open_cosmetic_pack(&app, PROFILE, &[], "bars-spent")
        .await
        .expect("the opening");
    let spent = COSMETIC_PACK_SLOTS as i64;
    assert!(
        pack.slots
            .iter()
            .all(|slot| slot.grant == CosmeticSlotGrant::Bars)
    );
    let card_gain = spent * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE;
    let cosmetic_gain = spent * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE;
    assert_eq!(
        pack.bars,
        BarsSettled {
            card: BarSettle {
                before: CARD_TOKEN_BAR_SIZE - 1,
                after: CARD_TOKEN_BAR_SIZE - 1 + card_gain - CARD_TOKEN_BAR_SIZE,
                size: CARD_TOKEN_BAR_SIZE,
                pays: 1,
            },
            cosmetic: BarSettle {
                before: 0,
                after: cosmetic_gain,
                size: COSMETIC_TOKEN_BAR_SIZE,
                pays: 0,
            },
            cosmetic_paid: Vec::new(),
        }
    );
    // What it answers is what it left.
    assert_eq!(
        counter_of(&app, Counter::CardTokenBar).await,
        pack.bars.card.after
    );
    assert_eq!(
        counter_of(&app, Counter::CosmeticTokenBar).await,
        pack.bars.cosmetic.after
    );
    assert_eq!(held(&app, InventoryItem::CardToken, "").await, 1);

    // On the wire the answer is `bars` in camelCase, the cosmetic bar's paid list empty.
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let bars = &body["bars"];
    assert_eq!(bars["card"]["before"], json!(pack.bars.card.after));
    assert_eq!(bars["cosmetic"]["size"], json!(COSMETIC_TOKEN_BAR_SIZE));
    assert_eq!(bars["cosmeticPaid"], json!([]));
    assert_eq!(opened(&body).bars.cosmetic.before, pack.bars.cosmetic.after);
}

#[tokio::test]
async fn a_cosmetic_pack_answers_the_cycle_s_next_step_when_its_cosmetic_bar_pays() {
    let (app, _) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    let gain = COSMETIC_PACK_SLOTS as i64 * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE;
    // One step into the cycle, so the next is its second (an Epic token).
    set_counter(&app, Counter::CosmeticTokenCycle, 1).await;
    set_counter(&app, Counter::CosmeticTokenBar, COSMETIC_TOKEN_BAR_SIZE - gain).await;
    let pack = open_cosmetic_pack(&app, PROFILE, &[], "bars-cycle")
        .await
        .expect("the opening");
    assert_eq!(
        pack.bars.cosmetic,
        BarSettle {
            before: COSMETIC_TOKEN_BAR_SIZE - gain,
            after: 0,
            size: COSMETIC_TOKEN_BAR_SIZE,
            pays: 1,
        }
    );
    assert_eq!(pack.bars.cosmetic_paid, vec![CosmeticTokenStep::Epic]);
    assert_eq!(
        serde_json::to_value(&pack.bars).expect("the bars")["cosmeticPaid"],
        json!(["epic"])
    );
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Epic").await, 1);
    assert_eq!(counter_of(&app, Counter::CosmeticTokenCycle).await, 2);
    // The card bar only gained.
    assert_eq!(pack.bars.card.pays, 0);
}

#[tokio::test]
async fn a_cosmetic_pack_with_no_spent_slot_answers_both_bars_unmoved() {
    let (app, _) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    set_counter(&app, Counter::CardTokenBar, 7).await;
    set_counter(&app, Counter::CosmeticTokenBar, 11).await;
    // A foil at each tier fits any card, so no slot is left with nothing to own.
    let registry = vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-rare", CosmeticRarity::Rare),
        foil("foil-fixture-epic", CosmeticRarity::Epic),
    ];
    let pack = open_cosmetic_pack(&app, PROFILE, &registry, "bars-unspent")
        .await
        .expect("the opening");
    assert!(
        pack.slots
            .iter()
            .all(|slot| slot.grant != CosmeticSlotGrant::Bars),
        "{:?}",
        pack.slots
    );
    assert_eq!(
        pack.bars,
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
    assert_eq!(counter_of(&app, Counter::CardTokenBar).await, 7);
    assert_eq!(counter_of(&app, Counter::CosmeticTokenBar).await, 11);
}

#[tokio::test]
async fn r1333000_a_cosmetic_pack_leaves_the_card_pity_counters_alone() {
    let (app, _) = setup().await;
    // Card packs' pity, part-way to each threshold, as earlier card packs left it.
    let pity = [
        (Counter::PityEpic, 3),
        (Counter::PityLegendary, 7),
        (Counter::PityMythic, 11),
    ];
    for (counter, value) in pity {
        set_counter(&app, counter, value).await;
    }
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    let registry = shipped().expect("the shipped registry");
    let pack = open_cosmetic_pack(&app, PROFILE, &registry, "r1333000-pity")
        .await
        .expect("the opening");
    assert!(
        pack.slots
            .iter()
            .any(|slot| matches!(slot.grant, CosmeticSlotGrant::Cosmetic { .. })),
        "{pack:?}"
    );
    // No cosmetic roll moves a card pack's counters (§14.6).
    for (counter, value) in pity {
        assert_eq!(counter_of(&app, counter).await, value, "{counter:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// The Chase Shard bar (R1306002)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1306002_the_shard_bar_counts_one_per_pack_pays_when_full_and_empties() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 4).await;

    // Two openings read 2.
    for at in 0..2 {
        open_cosmetic_pack(&app, PROFILE, &[], &format!("r1306002-{at}"))
            .await
            .expect("the opening");
    }
    let (status, _, body) = call(&app, "GET", "/api/cosmetic-packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["shardBar"],
        json!({ "points": 2, "size": CHASE_SHARD_PACKS })
    );

    // One short of full, one opening fills it.
    nearly_full(&app).await;
    let pack = open_cosmetic_pack(&app, PROFILE, &[], "r1306002-full")
        .await
        .expect("the opening");
    assert_eq!(pack.shard_bar.points, CHASE_SHARD_PACKS);
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, CHASE_SHARD_PACKS);

    // A full bar buys a chosen Legendary cosmetic for a card the account owns, and empties.
    let registry = registry(&cards[0]);
    buy_chase(
        &app,
        PROFILE,
        &registry,
        "foil-fixture-legendary",
        Some(&cards[0]),
    )
    .await
    .expect("the purchase");
    assert!(owns_cosmetic(&app, Some(&cards[0]), "foil-fixture-legendary").await);
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, 0);
    let ledger = ledger(&app).await;
    let bought: Vec<_> = ledger
        .cosmetics
        .iter()
        .filter(|row| row.reason == EconomyReason::ChaseShard)
        .collect();
    assert_eq!(bought.len(), 1);
    assert_eq!(
        (
            bought[0].card_id.as_deref(),
            bought[0].cosmetic_id.as_str(),
            bought[0].delta
        ),
        (Some(cards[0].as_str()), "foil-fixture-legendary", 1)
    );
    let emptied: Vec<_> = ledger
        .counters
        .iter()
        .filter(|row| row.counter == Counter::ChaseShardBar && row.reason == EconomyReason::ChaseShard)
        .collect();
    assert_eq!(emptied.len(), 1);
    assert_eq!(emptied[0].delta, -CHASE_SHARD_PACKS);

    // The next opening starts the bar again at 1.
    let pack = open_cosmetic_pack(&app, PROFILE, &[], "r1306002-again")
        .await
        .expect("the opening");
    assert_eq!(pack.shard_bar.points, 1);

    // An account Mythic cosmetic, with no card, is bought the same way.
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    let before = holdings(&app).await;
    assert!(!before.account_cosmetics.contains("table-fixture-mythic"));
    buy_chase(&app, PROFILE, &registry, "table-fixture-mythic", None)
        .await
        .expect("the purchase");
    assert!(owns_cosmetic(&app, None, "table-fixture-mythic").await);
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, 0);
}

#[tokio::test]
async fn r1306002_a_chase_cosmetic_is_allowed_and_the_route_buys_the_shipped_registry_s() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..1]).await;
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    // A chase is no token's, but the bar's.
    buy_chase(
        &app,
        PROFILE,
        &registry(&cards[0]),
        "foil-fixture-chase",
        Some(&cards[0]),
    )
    .await
    .expect("the purchase");
    assert!(owns_cosmetic(&app, Some(&cards[0]), "foil-fixture-chase").await);

    // The shipped registry: its Legendaries are starters, owned by everyone, so the route
    // refuses them, and so it does a full art the account has no card for.
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    let before = everything(&app).await;
    for body in [
        json!({ "cosmeticId": "foil-cracked-ice", "cardId": cards[0] }),
        json!({ "cosmeticId": "finish-gilded-edge", "cardId": cards[0] }),
        json!({ "cosmeticId": "full-art-gary-the-gambler", "cardId": "core-004" }),
    ] {
        let (status, _, answer) = call(
            &app,
            "POST",
            "/api/cosmetic-packs/chase",
            Some(&token),
            body.clone(),
        )
        .await;
        assert_eq!(status, 409, "{body}: {answer}");
    }
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn r1306002_buying_refuses_an_owned_cosmetic_and_anything_below_legendary() {
    let (app, _) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..2]).await;
    let registry = registry(&cards[0]);
    give(&app, Some(&cards[0]), "foil-fixture-owned-legendary").await;
    nearly_full(&app).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    open_cosmetic_pack(&app, PROFILE, &[], "r1306002-refuse")
        .await
        .expect("the opening");
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, CHASE_SHARD_PACKS);

    let refusals: [(&str, Option<&str>); 9] = [
        // Below Legendary.
        ("foil-fixture-common", Some(&cards[0])),
        ("foil-fixture-rare", Some(&cards[0])),
        ("foil-fixture-epic", Some(&cards[0])),
        // Already owned: a ledger row, and a starter every account has.
        ("foil-fixture-owned-legendary", Some(&cards[0])),
        ("foil-fixture-starter-legendary", Some(&cards[0])),
        // A card the account does not own, and no card at all.
        ("foil-fixture-legendary", Some(&cards[2])),
        ("foil-fixture-legendary", None),
        // A full art on another card than its own, and an account cosmetic given one.
        ("full-art-fixture-legendary", Some(&cards[1])),
        ("table-fixture-mythic", Some(&cards[0])),
    ];
    let before = everything(&app).await;
    for (cosmetic_id, card_id) in refusals {
        let refused = buy_chase(&app, PROFILE, &registry, cosmetic_id, card_id).await;
        assert_eq!(refused.expect_err(cosmetic_id).status(), 409, "{cosmetic_id}");
    }
    assert_eq!(everything(&app).await, before);
    // An unknown cosmetic is 400.
    let refused = buy_chase(&app, PROFILE, &registry, "no-such-cosmetic", Some(&cards[0])).await;
    assert_eq!(refused.expect_err("unknown").status(), 400);
    assert_eq!(everything(&app).await, before);

    // A bar below full buys nothing, whatever the cosmetic.
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS - 1).await;
    let before = everything(&app).await;
    let refused = buy_chase(
        &app,
        PROFILE,
        &registry,
        "foil-fixture-legendary",
        Some(&cards[0]),
    )
    .await;
    assert_eq!(refused.expect_err("a bar not full").status(), 409);
    assert_eq!(everything(&app).await, before);

    // The same choice buys once the bar is full, and a full art on its own card too.
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    buy_chase(
        &app,
        PROFILE,
        &registry,
        "full-art-fixture-legendary",
        Some(&cards[0]),
    )
    .await
    .expect("a full art on its own card");
    assert!(owns_cosmetic(&app, Some(&cards[0]), "full-art-fixture-legendary").await);
}

#[tokio::test]
async fn r1306002_a_full_bar_holds_when_no_legendary_or_mythic_cosmetic_exists() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..1]).await;
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 2).await;

    // The shipped registry holds no Legendary or Mythic a player can own: its two Legendaries
    // are starters. The bar holds full.
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(opened(&body).shard_bar.points, CHASE_SHARD_PACKS);
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, CHASE_SHARD_PACKS);
    open_cosmetic_pack(&app, PROFILE, &[], "r1306002-holds")
        .await
        .expect("the opening");
    assert_eq!(counter_of(&app, Counter::ChaseShardBar).await, CHASE_SHARD_PACKS);
    // It wrote no row for a bar that did not move: the only one is the test's own setting.
    let rows: Vec<_> = ledger(&app)
        .await
        .counters
        .into_iter()
        .filter(|row| row.counter == Counter::ChaseShardBar)
        .collect();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].reference.is_none());

    // And nothing it could buy exists to buy: the shipped registry's Legendaries are starters.
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/chase",
        Some(&token),
        json!({ "cosmeticId": "foil-cracked-ice", "cardId": cards[0] }),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn r1306002_get_reads_the_shard_bar_and_unopened_packs() {
    let (app, token) = setup().await;
    // One roll's odds on the cosmetic table, a slot's and the floored slot's (R1339000).
    let odds = json!({
        "slot": cosmetic_odds(None),
        "flooredSlot": cosmetic_odds(Some(COSMETIC_PACK_FLOOR_RARITY)),
    });
    let (status, _, body) = call(&app, "GET", "/api/cosmetic-packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "shardBar": { "points": 0, "size": CHASE_SHARD_PACKS }, "unopened": 0, "odds": odds })
    );

    set_counter(&app, Counter::ChaseShardBar, 4).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 3).await;
    let (status, _, body) = call(&app, "GET", "/api/cosmetic-packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "shardBar": { "points": 4, "size": CHASE_SHARD_PACKS }, "unopened": 3, "odds": odds })
    );

    // An opening moves both.
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let (status, _, body) = call(&app, "GET", "/api/cosmetic-packs", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "shardBar": { "points": 5, "size": CHASE_SHARD_PACKS }, "unopened": 2, "odds": odds })
    );
}

// ---------------------------------------------------------------------------------------------
// The log and the lock (R1306003)
// ---------------------------------------------------------------------------------------------

/// Checks the opening whose rows start at `before` against its log: every economy row it wrote
/// carries the log, the log holds the slots the opening answered, and [`roll_cosmetic_pack`] over
/// the catalog's pool, `before_holdings`, the logged cosmetics and the logged seed rolls them
/// again.
async fn assert_replays(
    app: &App,
    before: &EconomyLedger,
    before_holdings: &CosmeticHoldings,
    answered: &OpenedCosmeticPack,
) {
    let after = ledger(app).await;
    let log = after.inventory[before.inventory.len()..]
        .iter()
        .find(|row| row.item == InventoryItem::CosmeticPack && row.reason == EconomyReason::PackOpen)
        .and_then(|row| row.reference.clone())
        .expect("the pack spent, with its log");
    let references = new_references(before, &after);
    assert!(references.len() >= 3, "{references:?}");
    assert!(
        references
            .iter()
            .all(|reference| reference.as_ref() == Some(&log)),
        "{references:?}"
    );

    let log: CosmeticOpeningLog = serde_json::from_str(&log).expect("the log");
    assert_eq!(log.catalog, app.catalog.version);
    assert_eq!(log.slots, answered.slots);
    let replayed = roll_cosmetic_pack(
        &pack_pool(&app.catalog.defs, PackKind::Global),
        before_holdings,
        &log.cosmetics,
        &log.seed,
    );
    assert_eq!(replayed.slots, log.slots);
    assert_eq!(replayed.draws, log.draws);
}

#[tokio::test]
async fn r1306003_an_opening_replays_from_its_stored_seed() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..1]).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 2).await;

    // Through the route, over the shipped registry and a fresh seed.
    let (before_ledger, before_holdings) = (ledger(&app).await, holdings(&app).await);
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_replays(&app, &before_ledger, &before_holdings, &opened(&body)).await;

    // Again over a fixture registry, whose Common cosmetics fall on cards and on the account.
    let registry = vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-epic", CosmeticRarity::Epic),
        cosmetic("table-fixture-rare", CosmeticKind::Table, CosmeticRarity::Rare),
        full_art("full-art-fixture", CosmeticRarity::Epic, &cards[1]),
    ];
    let (before_ledger, before_holdings) = (ledger(&app).await, holdings(&app).await);
    let pack = open_cosmetic_pack(&app, PROFILE, &registry, "r1306003")
        .await
        .expect("the opening");
    assert_replays(&app, &before_ledger, &before_holdings, &pack).await;
    // The seed is the one given, and a second opening of it over the same holdings is the same.
    let again = roll_cosmetic_pack(
        &pack_pool(&app.catalog.defs, PackKind::Global),
        &before_holdings,
        &registry,
        "r1306003",
    );
    assert_eq!(again.slots, pack.slots);
}

#[tokio::test]
async fn opening_without_a_cosmetic_pack_is_refused() {
    let (app, token) = setup().await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    let refused = open_cosmetic_pack(&app, PROFILE, &[], "none").await;
    assert_eq!(refused.expect_err("no pack held").status(), 409);
    assert_eq!(everything(&app).await, before);

    // A card pack is not a cosmetic pack.
    grant_item(&app, InventoryItem::GlobalPack, "", 1).await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/cosmetic-packs/open",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn the_routes_refuse_a_signed_out_caller() {
    let (app, _) = setup().await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 1).await;
    set_counter(&app, Counter::ChaseShardBar, CHASE_SHARD_PACKS).await;
    let before = everything(&app).await;
    for (method, path) in ROUTES {
        let (status, _, body) = call(
            &app,
            method,
            path,
            None,
            json!({ "cosmeticId": "foil-cracked-ice" }),
        )
        .await;
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
        let (status, _, body) = call(
            &app,
            method,
            path,
            Some(&pending),
            json!({ "cosmeticId": "foil-cracked-ice" }),
        )
        .await;
        assert_eq!(status, 403, "{method} {path}: {body}");
    }
    assert_eq!(everything(&app).await, before);
    let declared: Vec<(&str, &str, bool)> = app::ROUTES
        .iter()
        .filter(|route| route.1.starts_with("/api/cosmetic-packs"))
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
