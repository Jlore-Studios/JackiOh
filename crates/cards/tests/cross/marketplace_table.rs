//! Stinkyscape's Marketplace table, S21's data (`config::MARKETPLACE_TABLE`; R1853001, R1881006,
//! docs/stinkyscape-set.md ST3): every lot's pool resolves to cards the catalog holds, each the kind ST3
//! names, and an offering rolled from it holds every lot at its count and stock.

use jackioh_cards::{card_def, register_all};
use jackioh_engine::config::{
    MARKETPLACE_AI_CARDS, MARKETPLACE_CARD_STOCK_MAX, MARKETPLACE_CARD_STOCK_MIN, MARKETPLACE_DRONE_CORES,
    MARKETPLACE_LAUNCHERS, MARKETPLACE_PROBE_MK1, MARKETPLACE_PROBE_MK1_STOCK, MARKETPLACE_PROBE_MK2,
    MARKETPLACE_PROBE_MK2_STOCK, MARKETPLACE_RESOURCE_STOCK_MAX, MARKETPLACE_RESOURCE_STOCK_MIN,
    MARKETPLACE_TABLE, MARKETPLACE_WARP_DISRUPTORS, MARKETPLACE_WEAPONS, MarketPool,
};
use jackioh_engine::testkit::scenario;
use jackioh_engine::{
    DORMANT_STARSHIP_DEF_ID, KeywordKind, PlayerId, Rng, SetName, Tag, find_def, has_keyword, made_by_a_rule,
    registered_catalog, roll_offers,
};
use serde_json::json;

/// The ids a lot's pool holds in the registered catalog, as `marketplace::pool_ids` reads it: a set
/// lot's no token and no card only a rule makes (R1857001).
fn pool(pool: MarketPool) -> Vec<String> {
    match pool {
        MarketPool::Resources => registered_catalog()
            .values()
            .filter(|def| def.resource.is_some())
            .map(|def| def.id.clone())
            .collect(),
        MarketPool::Ids(ids) => ids.iter().map(|id| (*id).to_string()).collect(),
        MarketPool::Set(set) => registered_catalog()
            .values()
            .filter(|def| def.set == set && !def.token && !made_by_a_rule(def))
            .map(|def| def.id.clone())
            .collect(),
        MarketPool::SetKeyword(set, keyword) => registered_catalog()
            .values()
            .filter(|def| {
                def.set == set
                    && !def.token
                    && !made_by_a_rule(def)
                    && has_keyword(&def.base.keywords, keyword)
            })
            .map(|def| def.id.clone())
            .collect(),
    }
}

#[test]
fn r1881006_every_lots_pool_resolves_to_cards_the_catalog_holds_at_least_its_count() {
    register_all();
    assert_eq!(MARKETPLACE_TABLE.len(), 10, "ST3's ten lots");
    for lot in MARKETPLACE_TABLE {
        if let MarketPool::Ids(ids) = lot.pool {
            for id in ids {
                assert!(find_def(None, id).is_some(), "{id} is in the catalog");
            }
        }
        let ids = pool(lot.pool);
        assert!(
            ids.len() >= lot.count as usize,
            "{:?} holds {} cards, fewer than {}",
            lot.pool,
            ids.len(),
            lot.count
        );
        assert!(lot.stock_min >= 1 && lot.stock_min <= lot.stock_max, "{lot:?}");
    }
}

#[test]
fn r1881006_each_named_pool_is_the_kind_st3_names() {
    register_all();
    let named = |ids: &[&str], what: &str, is: &dyn Fn(&str) -> bool| {
        for id in ids {
            assert!(is(id), "{id} is {what}");
        }
    };
    named(MARKETPLACE_WEAPONS, "a v3–v5 Weapon", &|id| {
        let def = card_def(id);
        def.tags.contains(&Tag::Weapon) && ["v3", "v4", "v5"].iter().any(|v| def.name.ends_with(v))
    });
    assert_eq!(MARKETPLACE_WEAPONS.len(), 12);
    named(MARKETPLACE_WARP_DISRUPTORS, "a Warp Disruptor", &|id| {
        card_def(id).name.starts_with("Warp Disruptor")
    });
    assert_eq!(MARKETPLACE_WARP_DISRUPTORS.len(), 2);
    named(MARKETPLACE_PROBE_MK1, "Scanning Probe Mk1", &|id| {
        card_def(id).name == "Scanning Probe Mk1"
    });
    named(MARKETPLACE_PROBE_MK2, "Scanning Probe Mk2", &|id| {
        card_def(id).name == "Scanning Probe Mk2"
    });
    named(MARKETPLACE_DRONE_CORES, "a Drone Core", &|id| {
        card_def(id).tags.contains(&Tag::DroneCore)
    });
    assert_eq!(
        MARKETPLACE_DRONE_CORES.len(),
        registered_catalog()
            .values()
            .filter(|def| def.tags.contains(&Tag::DroneCore))
            .count(),
        "every Drone Core"
    );
    named(MARKETPLACE_LAUNCHERS, "a Launcher", &|id| {
        card_def(id).name.ends_with("Launcher")
    });
    named(MARKETPLACE_AI_CARDS, "an AI card of the set", &|id| {
        let def = card_def(id);
        def.set == SetName::Stinkyscape && def.tags.contains(&Tag::Ai)
    });
    assert_eq!(MARKETPLACE_AI_CARDS.len(), 10);
    for id in pool(MarketPool::SetKeyword(
        SetName::Stinkyscape,
        KeywordKind::StarshipPiece,
    )) {
        assert!(!card_def(&id).token, "{id}: a Starship Piece lot holds no token");
    }
}

#[test]
fn r1881006_an_offering_rolled_from_it_holds_each_lot_at_its_count_and_stock() {
    register_all();
    let s = scenario(json!({ "seed": "s21-table" }));
    let state = s.state();
    for seed in 0..20 {
        let mut rng = Rng::new(&format!("s21-table-{seed}"), 0);
        let offers = roll_offers(state, &mut rng, MARKETPLACE_TABLE);
        let counts: i32 = MARKETPLACE_TABLE.iter().map(|lot| lot.count).sum();
        assert_eq!(offers.len(), counts as usize, "every lot fills its count");
        let mut at = 0;
        for lot in MARKETPLACE_TABLE {
            let ids = pool(lot.pool);
            for offer in &offers[at..at + lot.count as usize] {
                assert!(ids.contains(&offer.def_id), "{} in {:?}", offer.def_id, lot.pool);
                assert!(
                    (lot.stock_min..=lot.stock_max).contains(&offer.stock),
                    "{offer:?}"
                );
                assert_eq!(offer.bought[PlayerId::P1], 0);
            }
            at += lot.count as usize;
        }
    }
}

#[test]
fn r1881006_the_stocks_are_st3s() {
    let stocks: Vec<(i32, i32)> = MARKETPLACE_TABLE
        .iter()
        .map(|lot| (lot.stock_min, lot.stock_max))
        .collect();
    let card = (MARKETPLACE_CARD_STOCK_MIN, MARKETPLACE_CARD_STOCK_MAX);
    assert_eq!(
        (MARKETPLACE_RESOURCE_STOCK_MIN, MARKETPLACE_RESOURCE_STOCK_MAX),
        (4, 8)
    );
    assert_eq!((MARKETPLACE_PROBE_MK1_STOCK, MARKETPLACE_PROBE_MK2_STOCK), (6, 3));
    assert_eq!(card, (1, 3));
    assert_eq!(
        stocks,
        vec![
            (MARKETPLACE_RESOURCE_STOCK_MIN, MARKETPLACE_RESOURCE_STOCK_MAX),
            card,
            card,
            (MARKETPLACE_PROBE_MK1_STOCK, MARKETPLACE_PROBE_MK1_STOCK),
            (MARKETPLACE_PROBE_MK2_STOCK, MARKETPLACE_PROBE_MK2_STOCK),
            card,
            card,
            card,
            card,
            card,
        ]
    );
}

#[test]
fn r1857001_the_marketplace_never_offers_the_dormant_starship() {
    register_all();
    // It is a Stinkyscape card and no token, so only R1857001's rule keeps it out of the set lots.
    let starship = card_def(DORMANT_STARSHIP_DEF_ID);
    assert_eq!(starship.set, SetName::Stinkyscape);
    assert!(!starship.token);
    for lot in MARKETPLACE_TABLE {
        assert!(
            !pool(lot.pool).iter().any(|id| id == DORMANT_STARSHIP_DEF_ID),
            "{:?} holds no Dormant Starship",
            lot.pool
        );
    }
    let s = scenario(json!({ "seed": "r1857001-market" }));
    for seed in 0..200 {
        let mut rng = Rng::new(&format!("r1857001-market-{seed}"), 0);
        let offers = roll_offers(s.state(), &mut rng, MARKETPLACE_TABLE);
        assert!(
            offers.iter().all(|offer| offer.def_id != DORMANT_STARSHIP_DEF_ID),
            "seed {seed} offers the Dormant Starship"
        );
    }
}
