//! The token bars and their tokens (`src/economy/tokens.rs`, SPEC §14.5; #1305): a bar paying
//! when full with the overflow kept, the cosmetic cycle's order, what each token buys and refuses,
//! and the read-back.
//!
//! Every request goes through the whole app over an empty fake store (`deps::empty_test_app`), as
//! the active account `p1`. Openings the test must steer (an empty registry, so every duplicate
//! is spent) call `open_pack` with that registry, as `packs.rs` does; cosmetic redemptions call
//! `redeem_cosmetic_token` with a fixture registry for the same reason, while card redemptions
//! and the read-back go through the routes over the shipped registry.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::SetName;
use jackioh_server::api::collection::{GrantInput, grant_cards, grant_entire_catalog, owned_map};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    CARD_TOKEN_BAR_POINTS_PER_DUPLICATE, CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
    COSMETIC_TOKEN_BAR_SIZE, PACK_PRICE_JLORE, PACK_SLOTS,
};
use jackioh_server::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db};
use jackioh_server::economy::packs::open_pack;
use jackioh_server::economy::roll::PackKind;
use jackioh_server::economy::store::{
    Counter, CounterGrant, EconomyLedger, EconomyReason, InventoryGrant, InventoryItem,
};
use jackioh_server::economy::tokens::redeem_cosmetic_token;

use crate::support::deps::{add_user, call, empty_test_app};

const PROFILE: &str = "p1";
const USER: &str = "u1";

/// The three routes, as `(method, path)`.
const ROUTES: [(&str, &str); 3] = [
    ("GET", "/api/tokens"),
    ("POST", "/api/tokens/redeem-card"),
    ("POST", "/api/tokens/redeem-cosmetic"),
];

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        Db::Pg(_) => panic!("the test app runs on the fake store"),
    }
}

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

/// `jlore` more Jlore in `p1`'s wallet, as a game pays it (§14.8).
async fn fund(app: &App, jlore: i64) {
    use jackioh_server::economy::store::{JloreGrant, JloreReason};
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

/// One Core pack bought through the route.
async fn buy(app: &Arc<App>, token: &str) {
    fund(app, price()).await;
    let (status, _, body) = call(
        app,
        "POST",
        "/api/packs/buy",
        Some(token),
        json!({ "pack": "Core" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
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

/// The first three non-token cards of the catalog: the first two owned, the third not.
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

// ---------------------------------------------------------------------------------------------
// A fixture registry (per-card cosmetics at every tier, and one of each refusal)
// ---------------------------------------------------------------------------------------------

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

fn registry(owned: &[String]) -> Vec<Cosmetic> {
    vec![
        foil("foil-fixture-common", CosmeticRarity::Common),
        foil("foil-fixture-rare", CosmeticRarity::Rare),
        foil("foil-fixture-epic", CosmeticRarity::Epic),
        foil("foil-fixture-legendary", CosmeticRarity::Legendary),
        foil("foil-fixture-mythic", CosmeticRarity::Mythic),
        // A chase is Legendary or Mythic (R1333000).
        Cosmetic {
            chase: true,
            ..foil("foil-fixture-chase", CosmeticRarity::Legendary)
        },
        Cosmetic {
            default: true,
            ..foil("foil-fixture-default", CosmeticRarity::Rare)
        },
        Cosmetic {
            starter: true,
            ..foil("foil-fixture-starter", CosmeticRarity::Rare)
        },
        Cosmetic {
            kind: CosmeticKind::Table,
            ..foil("table-fixture-rare", CosmeticRarity::Rare)
        },
        Cosmetic {
            kind: CosmeticKind::FullArt,
            card: Some(owned[0].clone()),
            ..foil("full-art-fixture", CosmeticRarity::Epic)
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// The bars paying (§14.5)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1305000_a_bar_pays_when_full_and_keeps_the_overflow() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    // One point short of full; the opening below spends every slot (an empty registry drops no
    // cosmetic), so it gains PACK_SLOTS points and pays once with the overflow kept.
    set_counter(&app, Counter::CardTokenBar, CARD_TOKEN_BAR_SIZE - 1).await;
    buy(&app, &token).await;
    let gained = PACK_SLOTS as i64 * CARD_TOKEN_BAR_POINTS_PER_DUPLICATE;
    open_pack(&app, PROFILE, PackKind::Set(SetName::Core), &[], "r1305000")
        .await
        .expect("the opening");

    assert_eq!(held(&app, InventoryItem::CardToken, "").await, 1);
    assert_eq!(
        counter_of(&app, Counter::CardTokenBar).await,
        CARD_TOKEN_BAR_SIZE - 1 + gained - CARD_TOKEN_BAR_SIZE
    );
    // The cosmetic bar accrues as before: no payout, one PackOpen row.
    assert_eq!(
        counter_of(&app, Counter::CosmeticTokenBar).await,
        PACK_SLOTS as i64 * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE
    );

    let ledger = ledger(&app).await;
    // Still one row per bar per opening: the paying write replaces the accrual row.
    let card_rows: Vec<_> = ledger
        .counters
        .iter()
        .filter(|row| row.counter == Counter::CardTokenBar && row.reference.is_some())
        .collect();
    assert_eq!(card_rows.len(), 1);
    assert_eq!(card_rows[0].reason, EconomyReason::TokenBar);
    assert_eq!(card_rows[0].delta, gained - CARD_TOKEN_BAR_SIZE);
    let token_rows: Vec<_> = ledger
        .inventory
        .iter()
        .filter(|row| row.reason == EconomyReason::TokenBar)
        .collect();
    assert_eq!(token_rows.len(), 1);
    assert_eq!(
        (
            token_rows[0].item,
            token_rows[0].key.as_str(),
            token_rows[0].delta
        ),
        (InventoryItem::CardToken, "", 1)
    );
    // Every row of the opening carries its log: the pack spent, the counters and the
    // tokens. (The bought pack's own row carries the pack's name instead.)
    let log = ledger
        .inventory
        .iter()
        .find(|row| row.reason == EconomyReason::PackOpen)
        .expect("the pack spent")
        .reference
        .clone();
    let refs: Vec<_> = ledger
        .inventory
        .iter()
        .map(|row| row.reference.clone())
        .chain(ledger.counters.iter().map(|row| row.reference.clone()))
        .filter(|reference| reference.is_some() && reference != &Some("Core".to_string()))
        .collect();
    assert!(!refs.is_empty());
    assert!(refs.iter().all(|reference| reference == &log));
}

#[tokio::test]
async fn r1305001_the_cycle_pays_rare_epic_rare_epic_rare_pack_then_restarts() {
    let (app, token) = setup().await;
    grant_entire_catalog(&app, PROFILE, None)
        .await
        .expect("R111's grant");
    for _ in 0..7 {
        buy(&app, &token).await;
    }
    // Each opening spends every slot, so prefilling to SIZE - PACK_SLOTS pays exactly one step.
    let fill = COSMETIC_TOKEN_BAR_SIZE - PACK_SLOTS as i64 * COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE;
    for at in 0..6 {
        set_counter(&app, Counter::CosmeticTokenBar, fill).await;
        open_pack(
            &app,
            PROFILE,
            PackKind::Set(SetName::Core),
            &[],
            &format!("r1305001-{at}"),
        )
        .await
        .expect("the opening");
    }
    assert_eq!(counter_of(&app, Counter::CosmeticTokenCycle).await, 6);
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Rare").await, 3);
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Epic").await, 2);
    assert_eq!(held(&app, InventoryItem::CosmeticPack, "").await, 1);
    let steps: Vec<(String, String)> = ledger(&app)
        .await
        .inventory
        .iter()
        .filter(|row| {
            row.reason == EconomyReason::TokenBar
                && matches!(
                    row.item,
                    InventoryItem::CosmeticToken | InventoryItem::CosmeticPack
                )
        })
        .map(|row| (format!("{:?}", row.item), row.key.clone()))
        .collect();
    assert_eq!(
        steps,
        vec![
            ("CosmeticToken".to_string(), "Rare".to_string()),
            ("CosmeticToken".to_string(), "Epic".to_string()),
            ("CosmeticToken".to_string(), "Rare".to_string()),
            ("CosmeticToken".to_string(), "Epic".to_string()),
            ("CosmeticToken".to_string(), "Rare".to_string()),
            ("CosmeticPack".to_string(), String::new()),
        ]
    );

    // The seventh fill restarts at Rare: the Cosmetic Pack step added an unopened pack.
    set_counter(&app, Counter::CosmeticTokenBar, fill).await;
    open_pack(
        &app,
        PROFILE,
        PackKind::Set(SetName::Core),
        &[],
        "r1305001-restart",
    )
    .await
    .expect("the opening");
    assert_eq!(counter_of(&app, Counter::CosmeticTokenCycle).await, 7);
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Rare").await, 4);
}

// ---------------------------------------------------------------------------------------------
// Redeeming a card token (R1305002)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1305002_a_card_token_unlocks_a_chosen_card_once() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    let token_id = app
        .catalog
        .card_ids
        .iter()
        .find(|id| app.catalog.is_token(id))
        .expect("a token")
        .clone();
    grant_item(&app, InventoryItem::CardToken, "", 2).await;

    // Neither an unknown card nor a token spends the token.
    for card_id in ["no-such-card", token_id.as_str()] {
        let before = everything(&app).await;
        let (status, _, body) = call(
            &app,
            "POST",
            "/api/tokens/redeem-card",
            Some(&token),
            json!({ "cardId": card_id }),
        )
        .await;
        assert_eq!(
            (status, body["error"]["code"].clone()),
            (409, json!("conflict")),
            "{body}"
        );
        assert_eq!(everything(&app).await, before);
    }

    // A chosen unowned card unlocks, spending the token.
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/tokens/redeem-card",
        Some(&token),
        json!({ "cardId": cards[0] }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        owned_map(&app, PROFILE).await.expect("the collection")[&cards[0]],
        1
    );
    assert_eq!(held(&app, InventoryItem::CardToken, "").await, 1);
    let grants = fake(&app).lock().await.tables.grants.clone();
    let rows: Vec<_> = grants.iter().filter(|grant| grant.card_id == cards[0]).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].delta, 1);
    assert_eq!(rows[0].reason, "craft");

    // Exactly once: the second redemption is refused and writes no row.
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/tokens/redeem-card",
        Some(&token),
        json!({ "cardId": cards[0] }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);
}

// ---------------------------------------------------------------------------------------------
// Redeeming a cosmetic token (R1305003)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1305003_a_rare_token_buys_common_and_rare_but_refuses_epic() {
    let (app, _) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..2]).await;
    let registry = registry(&cards);

    grant_item(&app, InventoryItem::CosmeticToken, "Rare", 1).await;
    redeem_cosmetic_token(
        &app,
        PROFILE,
        &registry,
        CosmeticRarity::Rare,
        "foil-fixture-common",
        Some(&cards[0]),
    )
    .await
    .expect("a Rare token buys a Common cosmetic");
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Rare").await, 0);

    grant_item(&app, InventoryItem::CosmeticToken, "Rare", 1).await;
    redeem_cosmetic_token(
        &app,
        PROFILE,
        &registry,
        CosmeticRarity::Rare,
        "foil-fixture-rare",
        Some(&cards[0]),
    )
    .await
    .expect("a Rare token buys a Rare cosmetic");

    // An Epic cosmetic, a full art (Epic whatever its card's rarity, R1313000) among them, and a
    // Legendary or Mythic one are above a Rare token's tier: refused, and the token kept.
    grant_item(&app, InventoryItem::CosmeticToken, "Rare", 1).await;
    let before = everything(&app).await;
    for cosmetic_id in [
        "foil-fixture-epic",
        "full-art-fixture",
        "foil-fixture-legendary",
        "foil-fixture-mythic",
    ] {
        let refused = redeem_cosmetic_token(
            &app,
            PROFILE,
            &registry,
            CosmeticRarity::Rare,
            cosmetic_id,
            Some(&cards[0]),
        )
        .await;
        assert_eq!(refused.unwrap_err().status(), 409, "{cosmetic_id}");
    }
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn r1305003_an_epic_token_buys_common_rare_and_epic_but_not_legendary() {
    let (app, _) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..2]).await;
    let registry = registry(&cards);

    // Common, Rare and Epic per-card cosmetics, a full art on its own card, and an account
    // cosmetic with no card.
    grant_item(&app, InventoryItem::CosmeticToken, "Epic", 5).await;
    for (cosmetic_id, card_id) in [
        ("foil-fixture-common", Some(cards[0].as_str())),
        ("foil-fixture-rare", Some(cards[0].as_str())),
        ("foil-fixture-epic", Some(cards[1].as_str())),
        ("full-art-fixture", Some(cards[0].as_str())),
        ("table-fixture-rare", None),
    ] {
        redeem_cosmetic_token(
            &app,
            PROFILE,
            &registry,
            CosmeticRarity::Epic,
            cosmetic_id,
            card_id,
        )
        .await
        .unwrap_or_else(|error| panic!("an Epic token buys {cosmetic_id}: {error}"));
    }
    assert_eq!(held(&app, InventoryItem::CosmeticToken, "Epic").await, 0);

    // Legendary and Mythic sit above every token's tier: no token reaches them.
    grant_item(&app, InventoryItem::CosmeticToken, "Epic", 2).await;
    let before = everything(&app).await;
    for cosmetic_id in ["foil-fixture-legendary", "foil-fixture-mythic"] {
        let refused = redeem_cosmetic_token(
            &app,
            PROFILE,
            &registry,
            CosmeticRarity::Epic,
            cosmetic_id,
            Some(&cards[0]),
        )
        .await;
        assert_eq!(refused.unwrap_err().status(), 409, "{cosmetic_id}");
    }
    // A full art names its own card alone (R1313000), and an account cosmetic takes no card.
    for (cosmetic_id, card_id) in [
        ("full-art-fixture", Some(cards[1].as_str())),
        ("table-fixture-rare", Some(cards[0].as_str())),
    ] {
        let refused = redeem_cosmetic_token(
            &app,
            PROFILE,
            &registry,
            CosmeticRarity::Epic,
            cosmetic_id,
            card_id,
        )
        .await;
        assert_eq!(refused.unwrap_err().status(), 409, "{cosmetic_id}");
    }
    assert_eq!(everything(&app).await, before);
}

#[tokio::test]
async fn r1305003_an_owned_card_and_an_owned_cosmetic_are_refused_without_spend() {
    let (app, token) = setup().await;
    let cards = three_cards(&app);
    own(&app, &cards[..2]).await;
    let registry = registry(&cards);

    // A card token refuses a card already owned, spending nothing.
    grant_item(&app, InventoryItem::CardToken, "", 1).await;
    let before = everything(&app).await;
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/tokens/redeem-card",
        Some(&token),
        json!({ "cardId": cards[0] }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (409, json!("conflict")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);

    // One success to own, then every refusal below leaves the account as it was.
    grant_item(&app, InventoryItem::CosmeticToken, "Rare", 4).await;
    grant_item(&app, InventoryItem::CosmeticToken, "Epic", 3).await;
    redeem_cosmetic_token(
        &app,
        PROFILE,
        &registry,
        CosmeticRarity::Rare,
        "foil-fixture-common",
        Some(&cards[0]),
    )
    .await
    .expect("the first buy");
    let before = everything(&app).await;
    let refusals = [
        // Already owned on that card.
        (
            CosmeticRarity::Rare,
            "foil-fixture-common",
            Some(cards[0].as_str()),
        ),
        // A per-card cosmetic for a card not owned, and with no card at all.
        (CosmeticRarity::Epic, "foil-fixture-epic", Some(cards[2].as_str())),
        (CosmeticRarity::Epic, "foil-fixture-epic", None),
        // A chase, a default and a starter are owned or unbuyable by every account.
        (
            CosmeticRarity::Rare,
            "foil-fixture-chase",
            Some(cards[0].as_str()),
        ),
        (
            CosmeticRarity::Rare,
            "foil-fixture-default",
            Some(cards[0].as_str()),
        ),
        (
            CosmeticRarity::Rare,
            "foil-fixture-starter",
            Some(cards[0].as_str()),
        ),
    ];
    for (token, cosmetic_id, card_id) in refusals {
        let refused = redeem_cosmetic_token(&app, PROFILE, &registry, token, cosmetic_id, card_id).await;
        assert_eq!(refused.unwrap_err().status(), 409, "{cosmetic_id}");
    }
    // A token outside Rare and Epic, and an unknown cosmetic, are bad requests.
    for (token, cosmetic_id) in [
        (CosmeticRarity::Common, "foil-fixture-common"),
        (CosmeticRarity::Legendary, "foil-fixture-legendary"),
        (CosmeticRarity::Mythic, "foil-fixture-mythic"),
    ] {
        let refused =
            redeem_cosmetic_token(&app, PROFILE, &registry, token, cosmetic_id, Some(&cards[0])).await;
        assert_eq!(refused.unwrap_err().status(), 400, "{cosmetic_id}");
    }
    let refused = redeem_cosmetic_token(
        &app,
        PROFILE,
        &registry,
        CosmeticRarity::Rare,
        "no-such-cosmetic",
        Some(&cards[0]),
    )
    .await;
    assert_eq!(refused.unwrap_err().status(), 400);
    // The route names a token outside its words the same way.
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/tokens/redeem-cosmetic",
        Some(&token),
        json!({ "token": "banana", "cosmeticId": "foil-fixture-common", "cardId": cards[0] }),
    )
    .await;
    assert_eq!(
        (status, body["error"]["code"].clone()),
        (400, json!("bad_request")),
        "{body}"
    );
    assert_eq!(everything(&app).await, before);
}

// ---------------------------------------------------------------------------------------------
// Reading back (R1305004)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1305004_get_tokens_reads_bars_cycle_and_tokens() {
    let (app, token) = setup().await;
    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({
            "cardBar": { "points": 0, "size": CARD_TOKEN_BAR_SIZE },
            "cosmeticBar": { "points": 0, "size": COSMETIC_TOKEN_BAR_SIZE },
            "nextStep": "rare",
            "tokens": { "card": 0, "cosmeticRare": 0, "cosmeticEpic": 0, "cosmeticPack": 0 },
        })
    );

    set_counter(&app, Counter::CardTokenBar, 10).await;
    set_counter(&app, Counter::CosmeticTokenBar, 20).await;
    set_counter(&app, Counter::CosmeticTokenCycle, 1).await;
    grant_item(&app, InventoryItem::CardToken, "", 2).await;
    grant_item(&app, InventoryItem::CosmeticToken, "Rare", 1).await;
    grant_item(&app, InventoryItem::CosmeticPack, "", 3).await;
    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({
            "cardBar": { "points": 10, "size": CARD_TOKEN_BAR_SIZE },
            "cosmeticBar": { "points": 20, "size": COSMETIC_TOKEN_BAR_SIZE },
            "nextStep": "epic",
            "tokens": { "card": 2, "cosmeticRare": 1, "cosmeticEpic": 0, "cosmeticPack": 3 },
        })
    );

    // The cycle's last step is a cosmetic pack, then it starts over.
    set_counter(&app, Counter::CosmeticTokenCycle, 5).await;
    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["nextStep"], json!("cosmeticPack"));
    set_counter(&app, Counter::CosmeticTokenCycle, 6).await;
    let (status, _, body) = call(&app, "GET", "/api/tokens", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["nextStep"], json!("rare"));
}

#[tokio::test]
async fn the_routes_refuse_a_signed_out_caller() {
    let (app, _) = setup().await;
    grant_item(&app, InventoryItem::CardToken, "", 1).await;
    let cards = three_cards(&app);
    let before = everything(&app).await;
    for (method, path) in ROUTES {
        let (status, _, body) = call(&app, method, path, None, json!({ "cardId": cards[0] })).await;
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
        let (status, _, body) = call(&app, method, path, Some(&pending), json!({ "cardId": cards[0] })).await;
        assert_eq!(status, 403, "{method} {path}: {body}");
    }
    assert_eq!(everything(&app).await, before);
    let declared: Vec<(&str, &str, bool)> = app::ROUTES
        .iter()
        .filter(|route| route.1.starts_with("/api/tokens"))
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
