//! The cross-card tests (SURFACE §1): those that are not a card's own (each card's are the `mod tests`
//! of its script).
//! The helpers the files share live here, beside `scenario`: `register_fixture_script`, `fixture`,
//! `in_hand`, `targets_of`, `tributes_of` (#1826; crates/tools/tests/acceptance.rs refuses a copy)
//! and `sidecar`.

use indexmap::IndexMap;
use jackioh_engine::testkit::{
    ActionBody, AttackHealth, CardDef, CardInstance, CardScripts, CardType, PlayerId, Scenario, Script,
    Selection, Zone, json, json_as, new_instance, register_scripts, registered_scripts,
};
use serde_json::{Map, Value};

/// A sidecar `crates/tools/tests/export_card_text.rs` writes from `crates/cards/card-text.yaml`
/// (`flavour.json`, `chinese.json`; #2498): its entries by catalog id, each kept as raw JSON so an
/// unknown field or a value that is not a string is seen, not refused by a typed parse. The file's
/// `"//"` entry, which says where it comes from, is checked and dropped.
pub fn sidecar(json: &str, file: &str) -> IndexMap<String, Map<String, Value>> {
    let mut entries: IndexMap<String, Value> =
        serde_json::from_str(json).unwrap_or_else(|error| panic!("{file} is an object: {error}"));
    let header = entries.shift_remove("//");
    assert!(
        header.as_ref().is_some_and(Value::is_string),
        "{file} starts with its \"//\" header, as the export writes it: {header:?}"
    );
    entries
        .into_iter()
        .map(|(id, entry)| match entry {
            Value::Object(entry) => (id, entry),
            other => panic!("{file}: {id} is {other}, not an object"),
        })
        .collect()
}

pub fn scenario(setup: serde_json::Value) -> jackioh_engine::testkit::Scenario {
    jackioh_cards::register_all();
    jackioh_engine::testkit::scenario(setup)
}

/// §8 #99 Craft a Card's result, made the way the card makes it (R77) without its two Discovers: the
/// definitions go into `subsystems.fuse` as ingredients that were never cards, and the result is a fresh,
/// non-Radiant hand card costing 0. A test about the fused card, not about which two offers the Discovers
/// make, builds it here, so a card or keyword joining the pools (R380) moves nothing it depends on
/// (issue #848). `re_entry.rs`'s `craft` is the same body.
pub fn craft_fused(
    g: &mut jackioh_engine::testkit::Scenario,
    player: jackioh_engine::PlayerId,
    def_ids: &[&str],
) -> jackioh_engine::CardInstance {
    use jackioh_engine::subsystems::{self, FuseArgs};
    use jackioh_engine::testkit::{EngineSink, GameEvent, Rng, Zone, new_instance};

    let mut events: Vec<GameEvent> = vec![];
    let mut rng = Rng::new(&g.state().seed, g.state().rng_cursor);
    let made = {
        let state = g.state_mut();
        let ingredients: Vec<jackioh_engine::CardInstance> = def_ids
            .iter()
            .map(|def_id| new_instance(&mut *state, def_id, player, Zone::Gone { player }))
            .collect();
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        subsystems::fuse(
            &mut sink,
            FuseArgs {
                ingredients,
                to_hand: Some(player),
                ..FuseArgs::default()
            },
        )
    };
    g.state_mut().rng_cursor = rng.cursor();
    made.expect("the crafted card")
}

pub fn register_fixture_script(id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

pub fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script, stats: Option<AttackHealth>) {
    let stats = stats.unwrap_or(AttackHealth { attack: 2, health: 2 });
    let face = if type_ == CardType::Unit {
        json!({ "attack": stats.attack, "health": stats.health, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
}

pub fn in_hand(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    s.state_mut().players[player].hand.push(card.clone());
    card
}

pub fn targets_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Play { targets, .. } | ActionBody::Activate { targets, .. } => {
            targets.clone().unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub fn tributes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Play { tributes, .. } | ActionBody::Activate { tributes, .. } => {
            tributes.clone().unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub mod activate_listed;
pub mod after_resolution;
pub mod aura_table;
pub mod balance_2521_a;
pub mod balance_2521_b;
pub mod buff_priorities;
pub mod card_metrics;
pub mod card_text;
pub mod catalog;
pub mod chinese;
pub mod code_blocks;
pub mod combat_windows;
pub mod condition_active;
pub mod control_change;
pub mod control_change_carry;
pub mod costs_and_mana;
pub mod deaths_and_reborn;
pub mod duos;
pub mod echo_and_exile;
pub mod embiggen_choices;
pub mod flavour;
pub mod forced_attacks;
pub mod fuse_registry;
pub mod fused_hooks;
pub mod fused_nested_resume;
pub mod fused_target_checks;
pub mod game_over;
pub mod game_summary;
pub mod hand_returns;
pub mod hidden_information;
pub mod invariants;
pub mod lasting_effects;
pub mod lesson_pools;
pub mod marketplace_table;
pub mod my_pawn;
pub mod params;
pub mod paused_sequences;
pub mod play_choices;
pub mod plays_and_casts;
pub mod pools_and_randomness;
pub mod preview;
pub mod query;
pub mod radiant_standard;
pub mod re_entry;
pub mod references;
pub mod registry;
pub mod resolving_face;
pub mod self_generation;
pub mod setup_and_mulligan;
pub mod stacks_and_reborn;
pub mod tributes;
pub mod trigger_multipliers;
pub mod trigger_stays;
pub mod turn_clock_and_legality;
pub mod turn_stages;
pub mod vanilla_and_positions;
