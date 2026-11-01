//! BUILD M4-T2 acceptance: the registry test. `REGISTRY` is keyed by catalog id, every def is the
//! catalog's own, `register_all()` hands the engine the whole catalog and the scripts, and a card with
//! no script file yet degrades to `empty_script()` instead of crashing. `crates/cards/build.rs`
//! (SURFACE §7.4) generates the registry and refuses a bad file (no `ID`, an `ID` the catalog lacks,
//! two files for one card, a file outside its set's folder), so those are proved here as properties.

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, CATALOG_IDS, CATALOG_VERSION, REGISTRY, card_def, register_all, scripts_of};
use jackioh_engine::{CardDefs, CardScripts, Script, empty_script};
use serde_json::{Value, json};

/// The whole catalog, however many entries it holds. `catalog.rs` proves the census (SPEC §8 + §7
/// and B2.1's per-set sizes).
fn catalog_size() -> usize {
    CATALOG_IDS.len()
}

const EXPECT_ALL_SCRIPTS: bool = false;

/// Classic and Classic+ scripts land in their own card waves (B10.2, M9-T6 … M9-T8).
fn ids_allowed_without_script() -> Vec<String> {
    if EXPECT_ALL_SCRIPTS {
        Vec::new()
    } else {
        CATALOG_IDS
            .iter()
            .filter(|id| !id.starts_with("core-"))
            .cloned()
            .collect()
    }
}

fn scripted_ids() -> Vec<String> {
    REGISTRY.iter().map(|(id, _)| (*id).to_string()).collect()
}

fn ids_without_script() -> Vec<String> {
    let scripted: IndexSet<String> = scripted_ids().into_iter().collect();
    CATALOG_IDS
        .iter()
        .filter(|id| !scripted.contains(id.as_str()))
        .cloned()
        .collect()
}

/// Any real card, for the error-path tests. #1 Big D-fender is index 1 (SPEC §8).
fn some_id() -> String {
    CATALOG_IDS
        .first()
        .cloned()
        .unwrap_or_else(|| "core-001".to_string())
}

fn module_for(id: &str) -> CardScripts {
    match REGISTRY.iter().find(|(entry, _)| *entry == id) {
        Some((_, script)) => script(),
        None => panic!("no script registered for \"{id}\""),
    }
}

fn fingerprint(script: &Script) -> Value {
    let trigger_ids = |triggers: &[jackioh_engine::TriggerDef]| -> Vec<String> {
        triggers.iter().map(|trigger| trigger.id.clone()).collect()
    };
    json!({
        "hooks": {
            "cost": script.cost.is_some(),
            "cry": script.cry.is_some(),
            "death": script.death.is_some(),
            "startOfGame": script.start_of_game.is_some(),
            "entersHand": script.enters_hand.is_some(),
            "delayed": script.delayed.is_some(),
            "setStat": script.set_stat.is_some(),
            "startOfTurn": script.start_of_turn.is_some(),
            "endOfTurn": script.end_of_turn.is_some(),
            "aura": script.aura.is_some(),
            "onPlayHook": script.on_play_hook.is_some(),
            "staticFlags": script.static_flags.is_some(),
            "conditionMet": script.condition_met.is_some(),
            "preview": script.preview.is_some(),
            "costAura": script.cost_aura.is_some(),
            "graveyardPlay": script.graveyard_play.is_some(),
            "targetingDiscards": script.targeting_discards.is_some(),
            "recordsPlayAs": script.records_play_as.is_some(),
            "drawLimit": script.draw_limit.is_some(),
            "discardGuard": script.discard_guard.is_some(),
            "heroGuard": script.hero_guard.is_some(),
            "conditionalKeywords": script.conditional_keywords.is_some(),
            "whenAttacks": script.when_attacks.is_some(),
            "afterAttack": script.after_attack.is_some(),
            "afterAttacked": script.after_attacked.is_some(),
            "plagueMultiplier": script.plague_multiplier.is_some(),
            "echoX": script.echo_x.is_some(),
            "quests": script.quests.is_some(),
            "tributeWhen": script.tribute_when.is_some(),
            "wouldCounter": script.would_counter.is_some(),
            "startOfOpponentTurn": script.start_of_opponent_turn.is_some(),
            "attackMods": script.attack_mods.is_some(),
        },
        "resume": script.resume.keys().map(|key| (*key).to_string()).collect::<Vec<_>>(),
        "triggers": trigger_ids(&script.triggers),
        "handTriggers": trigger_ids(&script.hand_triggers),
        "deckTriggers": trigger_ids(&script.deck_triggers),
        "graveyardTriggers": trigger_ids(&script.graveyard_triggers),
        "targets": serde_json::to_value(&script.targets).expect("target declarations serialize"),
        "modes": serde_json::to_value(&script.modes).expect("mode declarations serialize"),
        "activations": script.activations.len(),
        "targetChecks": script.target_checks.keys().map(|key| (*key).to_string()).collect::<Vec<_>>(),
        "replacements": script.replacements.len(),
    })
}

mod registry_build_m4_t2 {
    use super::*;

    #[test]
    fn keys_cards_by_catalog_id_and_takes_every_def_from_the_catalog() {
        for id in scripted_ids() {
            assert!(CATALOG_IDS.contains(&id), "\"{id}\" is not a catalog id");
            // (SURFACE §7.4), so its def is the one the engine and the client see: a hand-written def
            assert_eq!(card_def(&id).id, id, "{id}: def must come from the catalog");
        }
    }

    #[test]
    fn has_no_duplicate_def_id() {
        let ids = scripted_ids();
        assert_eq!(scripts_of().len(), REGISTRY.len());
        assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), REGISTRY.len());
    }

    #[test]
    fn two_scripts_never_claim_one_card_build_rs_refuses_it() {
        // (SURFACE §7.4): what reaches the registry has one file per id, sorted by id.
        let ids = scripted_ids();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "REGISTRY is sorted by ID");
        let mut claimed: IndexMap<String, usize> = IndexMap::new();
        for id in &ids {
            *claimed.entry(id.clone()).or_insert(0) += 1;
        }
        let twice: Vec<String> = claimed
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(id, _)| format!("two card scripts claim \"{id}\""))
            .collect();
        assert_eq!(twice, Vec::<String>::new());
        assert!(CATALOG.contains_key(some_id().as_str()));
    }

    #[test]
    fn a_script_s_def_is_always_a_catalog_card_build_rs_refuses_one_that_is_not() {
        let strangers: Vec<String> = scripted_ids()
            .into_iter()
            .filter(|id| !CATALOG.contains_key(id.as_str()))
            .map(|id| format!("\"{id}\" is not in crates/cards/catalog.json"))
            .collect();
        assert_eq!(strangers, Vec::<String>::new());
        assert!(!CATALOG.contains_key("core-999"));
    }

    #[test]
    fn register_all_registers_every_def_and_the_catalog_version() {
        register_all();
        let registered = jackioh_engine::catalog::registered_catalog();
        assert_eq!(registered.len(), catalog_size());
        for id in CATALOG_IDS.iter() {
            assert_eq!(
                registered.get(id.as_str()),
                Some(&card_def(id)),
                "\"{id}\" reached the engine"
            );
        }
        assert_eq!(jackioh_engine::catalog::catalog_version(), CATALOG_VERSION);
    }

    #[test]
    fn register_all_is_idempotent_and_cheap_to_repeat() {
        register_all();
        let catalog_after_first: &'static CardDefs = jackioh_engine::catalog::registered_catalog();
        let scripts_after_first: Vec<String> = scripts_of().keys().cloned().collect();
        register_all();
        register_all();
        assert!(std::ptr::eq(
            jackioh_engine::catalog::registered_catalog(),
            catalog_after_first
        ));
        assert_eq!(jackioh_engine::catalog::registered_catalog(), &*CATALOG);
        assert_eq!(
            scripts_of().keys().cloned().collect::<Vec<_>>(),
            scripts_after_first
        );
        assert_eq!(jackioh_engine::catalog::catalog_version(), CATALOG_VERSION);
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // std::thread::scope: the testkit's override must be shown to stay on its own thread (SURFACE §8); never the rules.
    fn register_all_puts_the_real_registries_back_after_a_fixture_catalog() {
        register_all();
        // An engine test registers its own fixtures (BUILD §0). In Rust a fixture registration is the
        // testkit's thread-local override (SURFACE §8), so it is in force on its own thread and never
        std::thread::scope(|scope| {
            scope.spawn(|| {
                jackioh_engine::testkit::register_catalog(CardDefs::new());
                jackioh_engine::testkit::register_scripts(IndexMap::new());
                assert!(jackioh_engine::catalog::registered_catalog().is_empty());
            });
        });
        register_all();
        assert_eq!(jackioh_engine::catalog::registered_catalog(), &*CATALOG);
        assert_eq!(jackioh_engine::catalog::catalog_version(), CATALOG_VERSION);
    }

    #[test]
    fn gives_the_engine_each_module_s_own_base_and_radiant_script() {
        register_all();
        let game = jackioh_engine::testkit::scenario(json!({}));
        for id in scripted_ids() {
            let module = module_for(&id);
            let entry = jackioh_engine::scripts::scripts_for(game.state(), &id);
            assert_eq!(
                fingerprint(&entry.base),
                fingerprint(&module.base),
                "{id}: base script"
            );
            assert_eq!(
                fingerprint(&entry.radiant),
                fingerprint(&module.radiant),
                "{id}: radiant script"
            );
        }
    }

    #[test]
    fn falls_back_to_empty_script_for_a_catalog_id_with_no_script_so_nothing_crashes() {
        register_all();
        let unscripted = ids_without_script()
            .first()
            .cloned()
            .unwrap_or_else(|| "core-not-a-card".to_string());
        let game = jackioh_engine::testkit::scenario(json!({}));
        let entry = jackioh_engine::scripts::scripts_for(game.state(), &unscripted);
        assert_eq!(fingerprint(&entry.base), fingerprint(&empty_script()));
        assert_eq!(fingerprint(&entry.radiant), fingerprint(&empty_script()));
    }

    #[test]
    fn every_catalog_id_has_a_script_and_every_script_has_a_catalog_entry() {
        let strangers: Vec<String> = scripted_ids()
            .into_iter()
            .filter(|id| !CATALOG_IDS.contains(id))
            .collect();
        assert_eq!(strangers, Vec::<String>::new());
        let allowed = ids_allowed_without_script();
        let missing: Vec<String> = ids_without_script()
            .into_iter()
            .filter(|id| !allowed.contains(id))
            .collect();
        assert_eq!(missing, Vec::<String>::new());
        if EXPECT_ALL_SCRIPTS {
            assert_eq!(scripted_ids().len(), catalog_size());
        }
    }
}
