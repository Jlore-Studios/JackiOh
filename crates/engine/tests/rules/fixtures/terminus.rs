//! Test-only cards for the verbs Stinkyscape's Terminus cards are written with (issue #1870): a win a
//! card carries (R1870000), an exile that takes a Stack pile whole (R1870001), the Echo a resolving
//! card gives itself (R1870002), "your Units have <keyword>" through the next turn (R1870004) and a
//! keyword a card keeps for good (R1870007). Each does it through the engine's own verbs, as a card
//! script would, so the engine never imports `crates/cards`; the cards come behind the set's gate.
//!
//! Ids are prefixed `tm-` and indexed from 18700, so they collide with no other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// What the exam deals the enemy hero with each resolution.
pub const EXAM_DAMAGE: i32 = 1;

/// A Spell with no text.
pub const PLAIN: &str = "tm-plain";
/// A Spell: "Add a Plain to your hand. It wins you the game when you play it."
pub const CROWN: &str = "tm-crown";
/// A Spell: "Exile all permanents", each Stack pile whole.
pub const SWEEP: &str = "tm-sweep";
/// A Spell: "Deal 1 damage to the enemy hero, and this gains Echo +1."
pub const EXAM: &str = "tm-exam";
/// A Spell: "Your Units have Charge during your next turn."
pub const RALLY: &str = "tm-rally";
/// A Spell: "This gains Lucky +1 for good."
pub const CHARM: &str = "tm-charm";
/// A plain 3/3.
pub const GRUNT: &str = "tm-grunt";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": [], "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": [], "text": id }),
        ),
        None => (
            json!({ "keywords": [], "text": id }),
            json!({ "keywords": [], "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (Terminus fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    }))
}

fn cry(effects: fn() -> Vec<Effect>) -> CardScripts {
    both(Script {
        cry: Some(hook(move |_ctx| effects())),
        ..Script::default()
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    IndexMap::from([
        (
            CROWN.to_string(),
            cry(|| {
                vec![effects::add_to_hand(json_as(json!({
                    "defId": PLAIN,
                    "memory": { (effects::WIN_ON_PLAY_KEY): true },
                })))]
            }),
        ),
        (
            SWEEP.to_string(),
            cry(|| {
                vec![effects::exile_all_piles(json_as(
                    json!({ "side": "any", "rows": ["units", "backrow"] }),
                ))]
            }),
        ),
        (
            EXAM.to_string(),
            cry(|| {
                vec![
                    effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": EXAM_DAMAGE }),
                    )),
                    effects::gain_echo(json_as(json!({ "amount": 1 }))),
                ]
            }),
        ),
        (
            RALLY.to_string(),
            cry(|| {
                vec![effects::give_units_keyword_next_turn(json_as(
                    json!({ "keyword": { "kind": "Charge" } }),
                ))]
            }),
        ),
        (
            CHARM.to_string(),
            cry(|| {
                vec![effects::gain_keyword_for_good(json_as(
                    json!({ "keyword": { "kind": "Lucky", "n": 1 } }),
                ))]
            }),
        ),
    ])
}

fn defs() -> Vec<CardDef> {
    vec![
        card(18700, PLAIN, "Spell", None),
        card(18701, CROWN, "Spell", None),
        card(18702, SWEEP, "Spell", None),
        card(18703, EXAM, "Spell", None),
        card(18704, RALLY, "Spell", None),
        card(18705, CHARM, "Spell", None),
        card(18706, GRUNT, "Unit", Some((3, 3))),
    ]
}

/// The harness's catalog and scripts plus this file's cards.
pub fn register_terminus() {
    super::harness::setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}
