// TEMPORARY reconcile measurement; deleted before delivery.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]
use jackioh_engine::script::HookArgs;
use jackioh_engine::testkit::{CardDefs, PlayerId, json_as, register_catalog, scenario};
use jackioh_luau::{compile, load_card};
use serde_json::json;

fn unit_def(id: &str) -> jackioh_engine::CardDef {
    json_as(json!({
        "id": id, "index": "lx-000", "name": id, "set": "Core", "type": "Unit", "tags": [],
        "rarity": "Common", "token": false, "cost": 0,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": "vanilla" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "vanilla" },
    }))
}

#[test]
fn zz_bench_aura_calls() {
    let mut defs = CardDefs::new();
    defs.insert("lx-fx-001".into(), unit_def("lx-fx-001"));
    defs.insert("lx-fx-002".into(), unit_def("lx-fx-002"));
    for n in 0..30 {
        let id = format!("lx-filler-{n:02}");
        defs.insert(id.clone(), unit_def(&id));
    }
    register_catalog(defs);
    let src = include_str!("fixtures/gap_aura.luau");
    let bc: &'static [u8] = Vec::leak(compile("gap_aura.luau", src).unwrap());
    let scripts = load_card("lx-gap8", bc).unwrap();
    let state =
        scenario(json!({ "seed": "b", "p1": { "field": ["lx-fx-001"] }, "p2": { "field": ["lx-fx-002"] } }))
            .state()
            .clone();
    let me = jackioh_engine::active_units_of(&state, PlayerId::P1)
        .into_iter()
        .next()
        .unwrap();
    let aura = scripts.base.aura.clone().unwrap();
    let n = 20000;
    for _ in 0..1000 {
        let _ = aura(HookArgs {
            state: &state,
            self_: me,
            radiant: false,
        });
    }
    let t = std::time::Instant::now();
    for _ in 0..n {
        let e = aura(HookArgs {
            state: &state,
            self_: me,
            radiant: false,
        });
        assert_eq!(e.len(), 2);
    }
    let el = t.elapsed();
    println!("AURA per call: {:.2} us", el.as_secs_f64() * 1e6 / n as f64);
    let src2 = "--!strict\nlocal J = require(\"@jackioh\")\nreturn { base = { aura = function(_c: J.HookArgs) return {} end }, radiant = \"base\" }\n";
    let bc2: &'static [u8] = Vec::leak(compile("empty.luau", src2).unwrap());
    let s2 = load_card("lx-empty", bc2).unwrap();
    let aura2 = s2.base.aura.clone().unwrap();
    let t = std::time::Instant::now();
    for _ in 0..n {
        let e = aura2(HookArgs {
            state: &state,
            self_: me,
            radiant: false,
        });
        assert_eq!(e.len(), 0);
    }
    println!(
        "EMPTY AURA per call: {:.2} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let src3 = "--!strict\nlocal J = require(\"@jackioh\")\nreturn { base = { aura = function(c: J.HookArgs) local _ = c:controller() return {} end }, radiant = \"base\" }\n";
    let bc3: &'static [u8] = Vec::leak(compile("ctl.luau", src3).unwrap());
    let s3 = load_card("lx-ctl", bc3).unwrap();
    let aura3 = s3.base.aura.clone().unwrap();
    let t = std::time::Instant::now();
    for _ in 0..n {
        let _ = aura3(HookArgs {
            state: &state,
            self_: me,
            radiant: false,
        });
    }
    println!(
        "ONE-READ AURA per call: {:.2} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
}
