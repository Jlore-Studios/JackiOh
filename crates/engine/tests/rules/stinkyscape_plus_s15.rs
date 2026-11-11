// The verbs Stinkyscape+ S15 (issue #3566) added, each through a fixture script before a card uses it: the
// Cast-on-draw catalog query (R3566007), a stat line for a card with none (R3566004) and a memory key
// written onto another card (R3566006). The cards' own tests play them in the real scripts; these pin
// the verbs themselves, so the engine never imports `crates/cards`. Ids are `s15-`.

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, setup_catalog};

/// A Spell that casts on draw.
const COD: &str = "s15-cast-on-draw";
const PLAIN: &str = "s15-plain";
/// A Field Spell: no stat line of its own.
const FIELD: &str = "s15-field";
const UNIT: &str = "s15-unit";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>) -> CardDef {
    let face = match stats {
        Some((attack, health)) => json!({ "attack": attack, "health": health, "keywords": [], "text": id }),
        None => json!({ "keywords": [], "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (S15 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(35660, COD, "Spell", None),
        card(35661, PLAIN, "Spell", None),
        card(35662, FIELD, "Field Spell", None),
        card(35663, UNIT, "Unit", Some((2, 2))),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    let on_draw = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    scripts.insert(
        COD.to_string(),
        CardScripts {
            base: on_draw.clone(),
            radiant: on_draw,
        },
    );
    scripts.insert(PLAIN.to_string(), CardScripts::default());
    register_scripts(scripts);
}

fn board(p1: Value) -> Scenario {
    register();
    scenario(json!({ "seed": "s15", "p1": p1, "p2": {} }))
}

fn at(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

mod r3566007_the_cast_on_draw_query {
    use super::*;

    fn pool(args: Value) -> Vec<String> {
        let args: CatalogQueryArgs = json_as(args);
        query(&args).iter().map(|def| def.id.clone()).collect()
    }

    #[test]
    fn r3566007_the_cast_on_draw_query_reads_the_scripts_flag() {
        register();
        assert_eq!(
            pool(json!({ "defId": [COD, PLAIN], "castOnDraw": true })),
            vec![COD]
        );
        assert_eq!(
            pool(json!({ "defId": [COD, PLAIN], "castOnDraw": false })),
            vec![PLAIN]
        );
        assert_eq!(
            pool(json!({ "defId": [COD, PLAIN] })).len(),
            2,
            "absent, it filters nothing"
        );
    }
}

mod r3566004_a_stat_line_for_a_card_with_none {
    use super::*;

    #[test]
    fn r3566004_a_statless_graveyard_card_takes_a_stat_line() {
        let mut s = board(json!({ "graveyard": [FIELD] }));
        let id = s.pile(P1, "graveyard")[0].id.clone();
        assert!(!jackioh_engine::layers::has_stat_line(s.state(), s.card(&id)));
        apply_now(
            s.state_mut(),
            effects::give_stat_line(json_as(json!({ "target": at(&id), "attack": 0, "health": 0 }))),
        );
        assert_eq!(
            s.card(&id).stats_override,
            Some(AttackHealth { attack: 0, health: 0 })
        );
        assert!(jackioh_engine::layers::has_stat_line(s.state(), s.card(&id)));
    }

    #[test]
    fn r3566004_a_card_with_a_stat_line_keeps_its_own() {
        let mut s = board(json!({ "graveyard": [UNIT] }));
        let id = s.pile(P1, "graveyard")[0].id.clone();
        apply_now(
            s.state_mut(),
            effects::give_stat_line(json_as(json!({ "target": at(&id), "attack": 0, "health": 0 }))),
        );
        assert_eq!(s.card(&id).stats_override, None);
    }
}

mod r3566006_a_memory_key_onto_another_card {
    use super::*;

    #[test]
    fn r3566006_remember_onto_writes_a_plain_key_on_another_card() {
        let mut s = board(json!({ "field": [UNIT] }));
        let id = s.unit_at(P1, 1).id;
        for _ in 0..2 {
            let n = s.card(&id).memory.get("k").and_then(Value::as_i64).unwrap_or(0) + 1;
            apply_now(
                s.state_mut(),
                effects::remember_onto(json_as(json!({ "target": at(&id), "key": "k", "value": n }))),
            );
        }
        assert_eq!(s.card(&id).memory["k"], json!(2));
    }
}
