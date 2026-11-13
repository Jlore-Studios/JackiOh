//! Stinkyscape's Marketplace table, S21's data (`config::MARKETPLACE_TABLE`; R1853001, R1881006,
//! R2250002, R3047000, docs/stinkyscape-set.md ST3): every lot's pool resolves to cards the catalog
//! holds, each the kind ST3 names, each weighted by its ST3 count, and a player's offers rolled from it
//! are four different cards of its lots, one copy each, and no Resource.

use jackioh_cards::{card_def, register_all};
use jackioh_engine::config::{
    MARKETPLACE_AI_CARDS, MARKETPLACE_DRONE_CORES, MARKETPLACE_LAUNCHERS, MARKETPLACE_OFFER_STOCK,
    MARKETPLACE_OFFERS_PER_TURN, MARKETPLACE_PROBE_MK1, MARKETPLACE_PROBE_MK2, MARKETPLACE_TABLE,
    MARKETPLACE_WARP_DISRUPTORS, MARKETPLACE_WEAPONS, MarketPool,
};
use jackioh_engine::testkit::scenario;
use jackioh_engine::{
    DORMANT_STARSHIP_DEF_ID, KeywordKind, PlayerId, Rng, SetName, Tag, find_def, has_keyword, made_by_a_rule,
    market_card_price, registered_catalog, roll_offers, set_is_open, set_ships, tuned_price,
};
use serde_json::json;

/// The ids a lot's pool holds in the registered catalog, as `marketplace::pool_ids` reads it: a set
/// lot's no token and no card only a rule makes (R1857001), and an open-ids lot's only those whose set
/// is open (R3571001).
fn pool(pool: MarketPool) -> Vec<String> {
    match pool {
        MarketPool::Ids(ids) => ids.iter().map(|id| (*id).to_string()).collect(),
        MarketPool::OpenIds(ids) => ids
            .iter()
            .filter(|id| find_def(None, id).is_some_and(|def| set_is_open(def.set)))
            .map(|id| (*id).to_string())
            .collect(),
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
    assert_eq!(
        MARKETPLACE_TABLE.len(),
        9,
        "ST3's lots but its Resources (R2250002)"
    );
    for lot in MARKETPLACE_TABLE {
        if let MarketPool::Ids(ids) | MarketPool::OpenIds(ids) = lot.pool {
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
        assert!(
            ids.iter().all(|id| card_def(id).resource.is_none()),
            "{:?} holds no Resource",
            lot.pool
        );
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
    named(
        MARKETPLACE_AI_CARDS,
        "an AI card of Stinkyscape or Stinkyscape+",
        &|id| {
            let def = card_def(id);
            matches!(def.set, SetName::Stinkyscape | SetName::StinkyscapePlus) && def.tags.contains(&Tag::Ai)
        },
    );
    assert_eq!(MARKETPLACE_AI_CARDS.len(), 17);
    for id in pool(MarketPool::SetKeyword(
        SetName::Stinkyscape,
        KeywordKind::StarshipPiece,
    )) {
        assert!(!card_def(&id).token, "{id}: a Starship Piece lot holds no token");
    }
}

#[test]
fn r1881006_r2250002_r3047000_a_roll_from_it_offers_four_different_cards_of_its_lots() {
    register_all();
    let s = scenario(json!({ "seed": "s21-table" }));
    let state = s.state();
    let pools: Vec<Vec<String>> = MARKETPLACE_TABLE.iter().map(|lot| pool(lot.pool)).collect();
    for seed in 0..20 {
        let mut rng = Rng::new(&format!("s21-table-{seed}"), 0);
        let offers = roll_offers(state, &mut rng, MARKETPLACE_TABLE, PlayerId::P1);
        assert_eq!(offers.len() as i32, MARKETPLACE_OFFERS_PER_TURN);
        for (at, offer) in offers.iter().enumerate() {
            assert!(
                pools.iter().any(|ids| ids.contains(&offer.def_id)),
                "{} is in a lot",
                offer.def_id
            );
            assert!(
                offers[..at].iter().all(|before| before.def_id != offer.def_id),
                "the offers differ"
            );
            assert_eq!(offer.stock, MARKETPLACE_OFFER_STOCK, "{offer:?}");
            assert_eq!(offer.owner, Some(PlayerId::P1));
            assert_eq!(offer.bought[PlayerId::P1], 0);
            // R3047000, R3047002: its price is the designer's by its printed cost, Buffed or Nerfed, and
            // it is never both.
            assert!(offer.buffs.is_none() || offer.nerfs.is_none(), "{offer:?}");
            let designer = market_card_price(&card_def(&offer.def_id), offer.radiant == Some(true));
            assert_eq!(
                offer.base,
                tuned_price(designer, offer.buffs.unwrap_or(0), offer.nerfs.unwrap_or(0)),
                "{offer:?}"
            );
        }
    }
}

#[test]
fn r1881006_r2250002_the_lots_weigh_st3s_counts() {
    let counts: Vec<i32> = MARKETPLACE_TABLE.iter().map(|lot| lot.count).collect();
    assert_eq!(counts, vec![3, 2, 1, 1, 2, 1, 2, 2, 1]);
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
        let offers = roll_offers(s.state(), &mut rng, MARKETPLACE_TABLE, PlayerId::P1);
        assert!(
            offers.iter().all(|offer| offer.def_id != DORMANT_STARSHIP_DEF_ID),
            "seed {seed} offers the Dormant Starship"
        );
    }
}

#[test]
fn r3571001_the_ai_lot_offers_stinkyscape_plus_s_seven_only_once_the_set_ships() {
    register_all();
    let lot = *MARKETPLACE_TABLE.last().expect("the AI lot is the table's last");
    assert_eq!(lot.count, 1, "R2250002: its share is unchanged");
    let MarketPool::OpenIds(ids) = lot.pool else {
        panic!("the AI lot is an open-ids lot: {:?}", lot.pool);
    };
    // The ten Stinkyscape cards keep their place and order, then Stinkyscape+'s seven.
    let (stinkyscape, stinkyscape_plus) = ids.split_at(10);
    let ten: Vec<String> = (1..=10).map(|n| format!("stinkyscape-t-ai-{n:02}")).collect();
    let seven: Vec<String> = (1..=7).map(|n| format!("stinkyscapeplus-t-ai-{n:02}")).collect();
    assert_eq!(stinkyscape, ten.as_slice());
    assert_eq!(stinkyscape_plus, seven.as_slice());

    let s = scenario(json!({ "seed": "r3571001-ai-lot" }));
    // Every roll takes the lot's four cards alone, so its offers show which cards it draws from.
    let offered = |tag: &str| -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for seed in 0..200 {
            let mut rng = Rng::new(&format!("r3571001-{tag}-{seed}"), 0);
            for offer in roll_offers(s.state(), &mut rng, &[lot], PlayerId::P1) {
                if !seen.contains(&offer.def_id) {
                    seen.push(offer.def_id);
                }
            }
        }
        seen
    };

    // Stinkyscape+ ships (#3573's release), so the lot is the seventeen with no preview. The held set's
    // ten is the engine's fixture test (`rules/marketplace.rs`, `r3571001_an_open_ids_lot_…`).
    assert!(set_ships(SetName::StinkyscapePlus));
    assert_eq!(pool(lot.pool).len(), 17);
    let seen = offered("open");
    for id in &seven {
        assert!(seen.contains(id), "{id} is offered once the set is open");
    }
}
