//! Stand-in cards (#3486, `testkit/stand_in.rs`): a card a `stand-in …` id spells out plays from every
//! zone with no registration step, and no catalog query, random pool or seeded draw ever sees one.

use jackioh_engine::catalog::query;
use jackioh_engine::effects::{SummonArgs, summon};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{run_as_p1, setup_catalog};

const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
const BIG: &str = "stand-in (3) 9/9";
const QUICK: &str = "stand-in (1) 3/3 Rush, First Strike";
const VANILLA: &str = "stand-in (1) 4/4 R12/12";
const TARGET: &str = "stand-in (1) 3/3";
const SPELL: &str = "stand-in (0) Spell";
const FIELD_SPELL: &str = "stand-in (2) Field Spell";
const TRAP: &str = "stand-in (1) Trap";

const STAND_INS: [&str; 8] = [ARMORED, BIG, QUICK, VANILLA, TARGET, SPELL, FIELD_SPELL, TRAP];

fn every_card() -> CatalogQueryArgs {
    json_as(json!({ "everySet": true, "withTokens": true }))
}

fn units() -> CatalogQueryArgs {
    json_as(json!({ "type": "Unit" }))
}

fn pool_ids(pool: &[&CardDef]) -> Vec<String> {
    pool.iter().map(|def| def.id.clone()).collect()
}

#[test]
fn stand_ins_play_attack_and_summon_from_every_zone() {
    setup_catalog();
    let mut s = scenario(json!({
        "p1": {
            "hand": [QUICK, SPELL],
            "field": [ARMORED, { "def": BIG, "radiant": true }],
            "backrow": [TRAP],
            "library": [SPELL, FIELD_SPELL],
        },
        "p2": { "field": [TARGET] },
    }));
    s.expect_stats(BIG, json!({ "attack": 18, "health": 18 }));
    assert_eq!(s.card(BIG).def_id, BIG);

    let target = s.card(TARGET).clone();
    s.attack(ARMORED, &target);
    s.expect_in_zone(&target, "graveyard");
    s.expect_stats(ARMORED, json!({ "attack": 7, "health": 7 }));

    s.play(QUICK, json!({}));
    let quick = s.unit("p1", 3).expect("QUICK took the leftmost free lane");
    assert_eq!(quick.def_id, QUICK);
    let keywords = s.stats(&quick).keywords;
    assert!(has_keyword(&keywords, KeywordKind::Rush), "{keywords:?}");
    assert!(has_keyword(&keywords, KeywordKind::FirstStrike), "{keywords:?}");

    let spell = s
        .hand("p1")
        .into_iter()
        .find(|card| card.def_id == SPELL)
        .expect("SPELL in hand");
    s.play(&spell, json!({}));
    s.expect_in_zone(&spell, "graveyard");

    run_as_p1(
        s.state_mut(),
        summon(SummonArgs {
            def_id: Some(VANILLA.to_string()),
            ..Default::default()
        }),
    );
    let vanilla = s.unit("p1", 4).expect("the summon took lane 4");
    assert_eq!(vanilla.def_id, VANILLA);
    s.expect_stats(&vanilla, json!({ "attack": 4, "health": 4 }));

    assert_eq!(s.backrow("p1", 1).map(|card| card.def_id), Some(TRAP.to_string()));
    let library: Vec<String> = s
        .pile("p1", "library")
        .into_iter()
        .map(|card| card.def_id)
        .collect();
    assert_eq!(library, vec![SPELL.to_string(), FIELD_SPELL.to_string()]);
}

#[test]
fn no_catalog_query_or_pool_reaches_a_stand_in() {
    setup_catalog();
    let every_before = pool_ids(&query(&every_card()));
    let units_before = pool_ids(&query(&units()));
    let registered_before = registered_catalog().len();

    for id in STAND_INS {
        assert_eq!(find_def(None, id).map(|def| def.id.as_str()), Some(id));
    }
    scenario(json!({ "p1": { "hand": [SPELL], "field": [BIG], "backrow": [TRAP] } }));

    assert_eq!(pool_ids(&query(&every_card())), every_before);
    assert_eq!(pool_ids(&query(&units())), units_before);
    for id in STAND_INS {
        assert!(!every_before.iter().any(|known| known == id), "{id} is in a pool");
        let named = query(&json_as(
            json!({ "defId": id, "everySet": true, "withTokens": true }),
        ));
        assert!(named.is_empty(), "a pool naming {id} found it");
        assert!(registered_catalog().get(id).is_none(), "{id} is registered");
        assert!(def_by_index(SetName::Core, id).is_none(), "{id} has an index");
    }
    assert_eq!(registered_catalog().len(), registered_before);
}

#[test]
fn a_seeded_pool_draw_is_the_same_with_stand_ins_in_play() {
    setup_catalog();
    let draw = |hand: &str, field: &str, library: [&str; 2]| {
        let s = scenario(json!({
            "seed": "stand-in-pool",
            "p1": { "hand": [hand], "field": [field], "library": library },
        }));
        let state = s.state();
        let pool = query(&units());
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let drawn = rng.pick(&pool).map(|def| def.id.clone());
        (state.rng_cursor, pool_ids(&pool), drawn)
    };
    let fixtures = draw("fx-1", "fx-2", ["fx-3", "fx-4"]);
    let stand_ins = draw(VANILLA, BIG, [QUICK, TARGET]);
    assert!(fixtures.2.is_some());
    assert_eq!(stand_ins, fixtures);
}
