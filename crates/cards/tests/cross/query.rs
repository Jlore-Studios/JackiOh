//! BUILD M4-T2 acceptance for SPEC §5.1's one catalog query, and the reference every card agent
//! copies when its card needs a random pool or a Discover.
//!
//! The pools below are written as explicit id lists, so the test is the diff: if the filter or the
//! catalog drifts, the failure names the exact card that appeared or vanished. An index repeats across
//! sets since patch v0.2.0 ("43" is a Core, a Classic and a Classic+ card, B2.2), so pools are named by
//! id; and a pool that names no set draws from every set (R380, B2.6), in catalog order: Core, then
//! Classic, then Classic+, each by §5 index. Each pool test also states, in words and in code, the
//! argument object the card script must pass — that is the part the card files are going to copy.
//!
//! `query` reads the *registered* catalog (`crates/engine/src/catalog.rs`), which in a real game is
//! registered by `register_all()` in `crates/cards/src/lib.rs`; here we register the catalog data
//! directly (the testkit's thread-local override, SURFACE §8) so this file does not pull in the script
//! registry.
//!
//! Port of `packages/cards/test/query.test.ts`.

use jackioh_cards::query::{CardQuery, TRAP_TYPES, catalog, pool, query, query_cost};
use jackioh_cards::{CATALOG, card_def_by_index};
use jackioh_engine::testkit::{json_as, register_catalog};
use jackioh_engine::{CardCost, CardDef, CardDefs, CardType, GLITCH_DEF_ID, Rarity, SetName, Tag};
use serde_json::{Value, json};

/// A query's arguments as the TS object literal (`{ tags: ["KY"], excludeDefId: "core-057" }`).
fn q(literal: Value) -> CardQuery {
    json_as::<CardQuery>(literal)
}

/// Results are compared by catalog id: an index names a card only within its set (B2.2).
fn ids<'a>(defs: impl IntoIterator<Item = &'a CardDef>) -> Vec<String> {
    defs.into_iter().map(|def| def.id.clone()).collect()
}

/// Core's own pools, by §5 index, for the checks §8 writes by number.
fn core_indices<'a>(defs: impl IntoIterator<Item = &'a CardDef>) -> Vec<String> {
    defs.into_iter()
        .filter(|def| def.set == SetName::Core)
        .map(|def| def.index.clone())
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

/// Every token of every set (B2.1's census itself is catalog.rs's to prove).
/// Every token a pool may name — all of them but Glitch, which is in none (R674).
fn token_ids() -> Vec<String> {
    CATALOG
        .values()
        .filter(|def| def.token && def.id != GLITCH_DEF_ID)
        .map(|def| def.id.clone())
        .collect()
}

/// Every non-token def, in catalog order — the pool a plain query answers (R380).
fn non_token() -> Vec<&'static CardDef> {
    CATALOG.values().filter(|def| !def.token).collect()
}

/// TS `beforeAll(() => registerCatalog(CATALOG, CATALOG_VERSION))`: each Rust test runs on its own
/// thread, and the override is the thread's, so each test registers it first.
fn register() {
    register_catalog((*CATALOG).clone());
}

/// `Object.fromEntries(Object.entries(CATALOG).reverse())`.
fn reversed_catalog() -> CardDefs {
    CATALOG
        .iter()
        .rev()
        .map(|(id, def)| (id.clone(), def.clone()))
        .collect()
}

/// SURFACE §4.4.4: `Number.parseFloat(index)` on a catalog index.
fn parse_float(index: &str) -> f64 {
    index.parse::<f64>().unwrap_or(f64::INFINITY)
}

mod the_cards_layer_surface_5_1_one_query_function {
    use super::*;

    #[test]
    fn exposes_query_pool_cost_and_trap_types_on_catalog_the_object_card_scripts_call() {
        register();
        // lib.rs re-exports `query` (and with it `catalog` and the `CardQuery` type); everything a
        // card script needs therefore has to be reachable through `catalog`.
        let args = q(json!({ "tags": ["KY"] }));
        assert_eq!(ids(catalog::query(&args)), ids(query(&args)));
        assert_eq!(ids(catalog::pool("core-057", &args)), ids(pool("core-057", &args)));
        let def = &CATALOG["core-046"];
        assert_eq!(catalog::cost(def), query_cost(def));
        assert_eq!(catalog::TRAP_TYPES.to_vec(), TRAP_TYPES.to_vec());
    }

    #[test]
    fn r35_r61_field_trap_counts_as_trap_so_trap_types_names_both_types() {
        assert_eq!(TRAP_TYPES.to_vec(), vec![CardType::Trap, CardType::FieldTrap]);
    }
}

mod tokens_are_out_of_every_pool_unless_the_card_names_the_token_pool_5_1 {
    use super::*;

    #[test]
    fn r380_a_plain_query_is_every_set_s_non_token_cards_no_def_token_no_token_tag_no_token_rarity() {
        register();
        let all = query(&q(json!({})));

        // Every non-token def of every set, in catalog order — the census is catalog.rs's to prove.
        assert_eq!(ids(all.iter().copied()), ids(non_token()));
        assert_eq!(all.iter().filter(|def| def.token).count(), 0);
        assert_eq!(all.iter().filter(|def| def.tags.contains(&Tag::Token)).count(), 0);
        assert_eq!(all.iter().filter(|def| def.rarity == Rarity::Token).count(), 0);
        // And named outright, because "never a token" is the rule cards depend on (BUILD M4-T4 row 51.1
        // wants #51.1 "absent from every random pool"):
        assert!(!token_ids().is_empty());
        let all_ids = ids(all.iter().copied());
        for id in token_ids() {
            assert!(!all_ids.contains(&id), "{id}");
        }
    }

    #[test]
    fn s5_1_query_tags_token_does_return_them_the_card_named_the_pool_itself() {
        register();
        assert_eq!(sorted(ids(query(&q(json!({ "tags": ["Token"] }))))), sorted(token_ids()));
    }

    #[test]
    fn s5_1_token_true_and_a_pool_named_by_id_reach_tokens_too_token_false_forbids_them() {
        register();
        assert_eq!(sorted(ids(query(&q(json!({ "token": true }))))), sorted(token_ids()));
        // A card that names its token by id (Rush Token, Sheep Token, …) names the pool itself.
        assert_eq!(
            sorted(ids(query(&q(json!({ "defId": ["core-t-rush", "core-t-sheep"] }))))),
            strings(&["core-t-rush", "core-t-sheep"])
        );
        assert_eq!(ids(query(&q(json!({ "token": false, "tags": ["Token"] })))), Vec::<String>::new());
    }

    #[test]
    fn r380_a_pool_that_names_a_set_keeps_to_it_core_82_ky_s_trial_97_zephyrs() {
        register();
        assert_eq!(
            ids(query(&q(json!({ "set": "Core" })))),
            ids(non_token().into_iter().filter(|def| def.set == SetName::Core))
        );
        assert_eq!(
            ids(query(&q(json!({ "set": "Classic", "tags": ["Book"] })))),
            strings(&[
                "classic-003",
                "classic-012",
                "classic-016",
                "classic-024",
                "classic-029",
                "classic-055",
                "classic-070",
            ])
        );
    }
}

mod r387_exclude_def_id_a_random_pool_never_offers_the_card_that_generated_it_5_1 {
    use super::*;

    #[test]
    fn s5_1_exclude_def_id_removes_a_single_card() {
        register();
        // #57 Conjure KY carries the KY tag itself, so without excludeDefId it is in its own pool.
        assert_eq!(core_indices(query(&q(json!({ "tags": ["KY"] })))), strings(&["31", "51", "57", "82"]));
        assert_eq!(
            core_indices(query(&q(json!({ "tags": ["KY"], "excludeDefId": "core-057" })))),
            strings(&["31", "51", "82"])
        );
        // By id, never by index: Classic+ #57 Book of Stats is not Core #57, and stays in its pools.
        assert!(
            ids(query(&q(json!({ "tags": ["Book"], "excludeDefId": "core-057" }))))
                .contains(&"classicplus-057".to_string())
        );
    }

    #[test]
    fn s5_1_exclude_def_id_removes_every_card_in_a_list() {
        register();
        assert_eq!(
            core_indices(query(&q(json!({ "tags": ["KY"], "excludeDefId": ["core-057", "core-082"] })))),
            strings(&["31", "51"])
        );
    }

    #[test]
    fn s5_1_exclude_def_id_composes_with_the_type_rarity_and_cost_filters() {
        register();
        assert_eq!(
            core_indices(query(&q(json!({ "type": TRAP_TYPES, "excludeDefId": "core-018" })))),
            strings(&["41", "60", "71", "85", "96"])
        );
        assert!(
            !ids(query(&q(json!({ "rarity": "Mythic", "excludeDefId": "core-096" }))))
                .contains(&"core-096".to_string())
        );
        assert_eq!(
            ids(query(&q(json!({ "cost": 1, "tags": ["KY"], "excludeDefId": "core-031" })))),
            strings(&[
                "core-051",
                "core-082",
                "classicplus-041",
                "classicplus-042",
                "classicplus-062",
            ])
        );
    }

    #[test]
    fn s5_1_pool_own_id_args_adds_the_caller_s_id_to_exclude_def_id_instead_of_replacing_it() {
        register();
        assert_eq!(
            core_indices(pool("core-057", &q(json!({ "tags": ["KY"] })))),
            strings(&["31", "51", "82"])
        );
        assert_eq!(
            core_indices(pool("core-082", &q(json!({ "tags": ["KY"], "excludeDefId": "core-057" })))),
            strings(&["31", "51"])
        );
    }
}

mod the_ky_pool_57_conjure_ky_core_31_51_82_and_classic_plus_41_42_62_build_m4_t4_row_57_b2_6 {
    use super::*;

    // THE ARGUMENTS A CARD SCRIPT PASSES:
    //     catalog::query(&q(json!({ "tags": ["KY"], "excludeDefId": "core-057" })))
    //   or, equivalently and harder to get wrong,
    //     catalog::pool("core-057", &q(json!({ "tags": ["KY"] })))
    // Nothing else is needed: tokens are excluded by default (§5.1), which is what keeps #51.1 KY's
    // Empty Notebook and Classic+ #42.1 KY's Gift — tokens that carry the KY tag — out of the pool;
    // and a pool that names no set reaches every set (R380), so the Classic+ KY cards join it.
    const KY_POOL: &[&str] = &[
        "core-031",
        "core-051",
        "core-082",
        "classicplus-041",
        "classicplus-042",
        "classicplus-062",
    ];

    #[test]
    fn build_row_57_the_query_a_card_script_writes_returns_exactly_those_six_defs() {
        register();
        assert_eq!(
            ids(catalog::query(&q(json!({ "tags": ["KY"], "excludeDefId": "core-057" })))),
            strings(KY_POOL)
        );
        assert_eq!(ids(catalog::pool("core-057", &q(json!({ "tags": ["KY"] })))), strings(KY_POOL));
    }

    #[test]
    fn build_row_51_1_the_ky_pool_excludes_the_ky_tagged_tokens_51_1_and_classic_plus_42_1_and_the_generator_57() {
        register();
        let names: Vec<String> = catalog::pool("core-057", &q(json!({ "tags": ["KY"] })))
            .iter()
            .map(|def| def.name.clone())
            .collect();

        assert_eq!(
            names,
            strings(&[
                "KY's Math Equation",
                "KY's Private Tutor",
                "KY's Trial",
                "KY's Constant",
                "KY's Test",
                "KY's Papaya",
            ])
        );
        assert!(card_def_by_index(SetName::Core, "51.1").tags.contains(&Tag::Ky)); // it really is in the tag …
        assert!(card_def_by_index(SetName::Core, "51.1").token); // … and it really is a token
        assert!(card_def_by_index(SetName::ClassicPlus, "42.1").tags.contains(&Tag::Ky));
        let pool_ids = ids(catalog::pool("core-057", &q(json!({ "tags": ["KY"] }))));
        assert!(!pool_ids.contains(&"core-051-1".to_string()));
        assert!(!pool_ids.contains(&"classicplus-042-1".to_string()));
        assert!(!pool_ids.contains(&"core-057".to_string()));
    }
}

mod the_trap_pool_67_zoomerbin_oomen_s_radiant_face_every_set_s_traps_and_field_traps {
    use super::*;

    // THE ARGUMENTS A CARD SCRIPT PASSES:
    //     catalog::query(&q(json!({ "type": catalog::TRAP_TYPES })))   // TRAP_TYPES = ["Trap", "Field Trap"]
    //   or catalog::pool("core-067", &q(json!({ "type": TRAP_TYPES })))   // #67 is not itself a trap, but §5.1 anyway
    // Both types, because SPEC says "Field Trap counts as Trap" (§8 #51, #85, R35, R61) while the
    // filter matches `def.type` exactly.
    const TRAP_POOL: &[&str] = &[
        "core-018",
        "core-041",
        "core-060",
        "core-071",
        "core-085",
        "core-096",
        "classic-005",
        "classic-009",
        "classic-010",
        "classic-014",
        "classic-017",
        "classic-038",
        "classic-052",
        "classic-063",
        "classic-065",
        "classic-072",
        "classic-088",
        "classicplus-001",
        "classicplus-002",
        "classicplus-022",
        "classicplus-074",
    ];

    fn trap_query() -> Vec<&'static CardDef> {
        catalog::query(&q(json!({ "type": catalog::TRAP_TYPES })))
    }

    #[test]
    fn build_row_67_the_query_a_card_script_writes_returns_exactly_those_defs() {
        register();
        assert_eq!(ids(trap_query()), strings(TRAP_POOL));
        assert_eq!(ids(catalog::pool("core-067", &q(json!({ "type": TRAP_TYPES })))), strings(TRAP_POOL));
    }

    #[test]
    fn s8_67_base_asks_for_a_1_cost_trap_every_core_trap_but_85_classic_10_exile_and_classic_plus_22_blood_moon() {
        register();
        assert_eq!(
            ids(catalog::query(&q(json!({ "type": TRAP_TYPES, "cost": 1 })))),
            strings(&[
                "core-018",
                "core-041",
                "core-060",
                "core-071",
                "core-096",
                "classic-010",
                "classicplus-022",
            ])
        );
    }

    #[test]
    fn s5_1_the_pool_mixes_both_types_core_18_71_classic_5_38_88_and_classic_plus_74_are_field_traps() {
        register();
        let by_type = |kind: CardType| -> Vec<String> {
            ids(trap_query().into_iter().filter(|def| def.type_ == kind))
        };

        assert_eq!(
            by_type(CardType::FieldTrap),
            strings(&[
                "core-018",
                "core-071",
                "classic-005",
                "classic-038",
                "classic-088",
                "classicplus-074",
            ])
        );
        assert_eq!(
            core_indices(trap_query().into_iter().filter(|def| def.type_ == CardType::Trap)),
            strings(&["41", "60", "85", "96"])
        );
    }

    #[test]
    fn s5_1_asking_for_type_trap_alone_drops_the_field_traps_why_trap_types_exists() {
        register();
        let expected: Vec<String> = TRAP_POOL
            .iter()
            .filter(|id| CATALOG.get(**id).is_some_and(|def| def.type_ == CardType::Trap))
            .map(|id| (*id).to_string())
            .collect();
        assert_eq!(ids(catalog::query(&q(json!({ "type": "Trap" })))), expected);
        assert_eq!(
            core_indices(catalog::query(&q(json!({ "type": "Trap" })))),
            strings(&["41", "60", "85", "96"])
        );
    }
}

mod r35_the_transmogulate_pool_83_every_non_token_legendary_but_83_b2_6 {
    use super::*;

    // THE ARGUMENTS A CARD SCRIPT PASSES:
    //     catalog::query(&q(json!({ "rarity": "Legendary", "excludeDefId": "core-083" })))
    //   or catalog::pool("core-083", &q(json!({ "rarity": "Legendary" })))
    // R35: "Pool: the Legendary-rarity cards except #83", which since patch v0.2.0 reaches every set.
    // For replacing a card on the board, the script narrows the same pool by type and asks for
    // TRAP_TYPES when the board card is a trap ("Field Trap counts as Trap").
    const R35_POOL: &[&str] = &[
        "core-052",
        "core-085",
        "core-087",
        "core-092",
        "core-093",
        "core-095",
        "classic-004",
        "classic-007",
        "classic-009",
        "classic-028",
        "classic-033",
        "classic-044",
        "classic-045",
        "classic-056",
        "classic-080",
        "classic-085",
        "classicplus-012",
        "classicplus-013",
        "classicplus-019",
        "classicplus-035",
        "classicplus-037",
        "classicplus-042",
        "classicplus-043",
        "classicplus-046",
        "classicplus-047",
        "classicplus-048",
        "classicplus-073",
        "classicplus-075",
        "classicplus-078",
    ];

    #[test]
    fn r35_the_filter_a_card_script_writes_returns_exactly_that_list() {
        register();
        assert_eq!(
            ids(catalog::query(&q(json!({ "rarity": "Legendary", "excludeDefId": "core-083" })))),
            strings(R35_POOL)
        );
        assert_eq!(
            ids(catalog::pool("core-083", &q(json!({ "rarity": "Legendary" })))),
            strings(R35_POOL)
        );
    }

    #[test]
    fn r35_the_pool_is_the_legendary_rarity_set_minus_83_and_nothing_else() {
        register();
        assert_eq!(
            core_indices(catalog::query(&q(json!({ "rarity": "Legendary" })))),
            strings(&["52", "83", "85", "87", "92", "93", "95"])
        );
        let legendary = catalog::pool("core-083", &q(json!({ "rarity": "Legendary" })));
        assert!(!ids(legendary.iter().copied()).contains(&"core-083".to_string()));
        assert!(legendary.iter().all(|def| def.rarity == Rarity::Legendary));
        assert!(legendary.iter().all(|def| !def.token));
    }

    #[test]
    fn r35_narrowed_by_type_for_a_board_replacement_with_field_trap_counting_as_trap() {
        register();
        // Core #85 and Classic #9 (Legendary since patch v0.2.9, issue #44) are the Legendary traps,
        // so a board trap — Trap or Field Trap — is replaced by one of them.
        assert_eq!(
            ids(catalog::pool("core-083", &q(json!({ "rarity": "Legendary", "type": TRAP_TYPES })))),
            strings(&["core-085", "classic-009"])
        );
        assert_eq!(
            core_indices(catalog::pool("core-083", &q(json!({ "rarity": "Legendary", "type": "Unit" })))),
            strings(&["52", "92"])
        );
    }
}

mod r65_pools_and_filters_read_a_definition_s_cost_out_of_play {
    use super::*;

    #[test]
    fn r65_an_x_cost_card_s_query_cost_is_0_and_it_answers_a_cost_0_query() {
        register();
        // Core #24 Efficiency Dividend, #74 Adaptive UI, Classic #87 Plague Chalice, Classic+ #40
        // Appropriations and #69 Buff Billy are the catalog's X-cost cards (#98 Heroic Power costs (0)
        // since R752).
        let x_cards = strings(&["core-024", "core-074", "classic-087", "classicplus-040", "classicplus-069"]);
        assert_eq!(
            ids(query(&q(json!({}))).into_iter().filter(|def| def.cost == CardCost::X)),
            x_cards
        );
        let cost0 = ids(query(&q(json!({ "cost": 0 }))));
        for id in &x_cards {
            assert_eq!(query_cost(&CATALOG[id.as_str()]), 0, "{id}");
            assert!(cost0.contains(id), "{id}");
        }
    }

    #[test]
    fn r65_an_embiggen_card_s_query_cost_is_its_base_price_and_it_answers_that_cost_s_query() {
        register();
        // Core #46 Suppressive Aura, #59 Unbiased Immigration, #84 Going Long and Classic+ #36 Conjure
        // Bones are "2 embiggen 4".
        for id in ["core-046", "core-059", "core-084", "classicplus-036"] {
            let def = &CATALOG[id];
            assert_eq!(def.cost, CardCost::Embiggen { base: 2, embiggen: 4 }, "{id}");
            assert_eq!(query_cost(def), 2, "{id}");
            assert!(ids(query(&q(json!({ "cost": 2 })))).contains(&id.to_string()), "{id}");
            assert!(!ids(query(&q(json!({ "cost": 4 })))).contains(&id.to_string()), "{id}");
        }
    }

    #[test]
    fn r65_cost_range_reads_the_same_number_so_a_bracket_agrees_with_query_cost_7_51() {
        register();
        // #7 Jewelosco Scarab's "2-cost" Discover and #51's 0-1 / 2 / 3 / 4+ brackets share this read.
        let bracket0to1 = query(&q(json!({ "costRange": { "min": 0, "max": 1 } })));
        assert_eq!(
            ids(bracket0to1.iter().copied()),
            ids(query(&q(json!({}))).into_iter().filter(|def| query_cost(def) <= 1))
        );
        assert!(ids(bracket0to1.iter().copied()).contains(&"core-024".to_string())); // an X card reads as 0
        assert!(!ids(query(&q(json!({ "costRange": { "min": 4 } })))).contains(&"core-046".to_string())); // an embiggen card reads as 2
    }
}

mod s9_3_r60_pool_order_is_deterministic_so_a_seeded_pick_replays {
    use super::*;

    #[test]
    fn s9_3_two_identical_calls_return_the_same_defs_in_the_same_order() {
        register();
        let first = query(&q(json!({ "type": "Unit", "costRange": { "min": 2, "max": 3 } })));
        let second = query(&q(json!({ "type": "Unit", "costRange": { "min": 2, "max": 3 } })));

        assert_eq!(ids(first.iter().copied()), ids(second.iter().copied()));
        assert!(first.len() > 1);
    }

    #[test]
    fn r60_a_multi_result_pool_comes_back_in_catalog_order_set_core_classic_classic_plus_then_5_index_ascending() {
        register();
        let units = query(&q(json!({ "type": "Unit" })));
        let set_rank = |set: SetName| -> f64 {
            match set {
                SetName::Core => 0.0,
                SetName::Classic => 1.0,
                SetName::ClassicPlus => 2.0,
                _ => 3.0,
            }
        };
        let keys: Vec<(f64, f64)> = units
            .iter()
            .map(|def| (set_rank(def.set), parse_float(&def.index)))
            .collect();

        assert!(keys.len() > 1);
        let mut in_order = keys.clone();
        in_order.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        assert_eq!(keys, in_order);
        // Spot-checked against the catalog so "ascending" cannot be satisfied by an empty result:
        assert_eq!(
            ids(query(&q(json!({ "rarity": "Legendary" })))).first().map(String::as_str),
            Some("core-052")
        );
        let unit_ids = ids(units.iter().copied());
        let position = |id: &str| unit_ids.iter().position(|unit| unit == id);
        assert!(position("classic-002") > position("core-100"));
    }

    #[test]
    fn s9_3_the_order_does_not_depend_on_the_order_the_registry_handed_the_defs_over() {
        register();
        let forward = ids(query(&q(json!({}))));

        register_catalog(reversed_catalog());
        let reversed = ids(query(&q(json!({}))));
        let ky = core_indices(query(&q(json!({ "tags": ["KY"], "excludeDefId": "core-057" }))));
        register_catalog((*CATALOG).clone());

        assert_eq!(reversed, forward);
        assert_eq!(ky, strings(&["31", "51", "82"]));
    }

    // ENGINE DEFECT, reported to the lead and NOT worked around in src/query.rs.
    // §9.3, R60: the pool order is "a strict total order … never depends on the order the registry
    // happened to hand the defs over", which is what makes a seeded `rng.pick`/`shuffle` over a pool
    // replay identically. This was pinned as `it.fails` while the engine's comparator subtracted two
    // `indexRank`s: both are +Infinity for a non-numeric index, `Infinity - Infinity` is NaN, and
    // `NaN !== 0` returned NaN, which `sort` reads as 0 — so both tie-breaks were skipped and the
    // four shared tokens came back in insertion order. The comparator now compares instead of
    // subtracting, so this is a plain test; registering the catalog reversed is the regression guard.
    #[test]
    fn s9_3_the_token_pool_s_order_is_insertion_independent() {
        register();
        let forward = ids(query(&q(json!({ "tags": ["Token"] }))));

        register_catalog(reversed_catalog());
        let reversed = ids(query(&q(json!({ "tags": ["Token"] }))));
        register_catalog((*CATALOG).clone());

        assert_eq!(reversed, forward);
    }
}

mod r382_the_fruit_pool_holds_the_five_grapes_a_pool_that_takes_every_token_takes_them_all_b2_6 {
    use super::*;

    const GRAPES: &[&str] = &[
        "classicplus-065-1",
        "classicplus-065-2",
        "classicplus-065-3",
        "classicplus-065-4",
        "classicplus-065-5",
    ];

    #[test]
    fn r382_a_fruit_pool_is_every_non_token_fruit_card_and_the_five_grapes_the_generating_card_excluded() {
        register();
        // Core #47 Fig of Life, Classic+'s Fruit cards and the Grapes (each after the card that defines
        // it, in catalog order); Classic+ #58 Fruit Basket asks for its pool.
        let expected: Vec<String> = CATALOG
            .values()
            .filter(|def| def.tags.contains(&Tag::Fruit) && (!def.token || GRAPES.contains(&def.id.as_str())))
            .map(|def| def.id.clone())
            .filter(|id| id != "classicplus-058")
            .collect();
        assert_eq!(ids(pool("classicplus-058", &q(json!({ "tags": ["Fruit"] })))), expected);
        let tokens: Vec<String> = expected
            .iter()
            .filter(|id| CATALOG.get(id.as_str()).is_some_and(|def| def.token))
            .cloned()
            .collect();
        assert_eq!(tokens, strings(GRAPES));
        assert!(ids(query(&q(json!({ "tags": ["Fruit"] })))).contains(&"core-047".to_string()));
    }

    #[test]
    fn r382_no_other_pool_reaches_a_grape_not_a_plain_pool_not_a_spell_pool_not_a_rarity_pool() {
        register();
        for args in [
            json!({}),
            json!({ "type": "Spell" }),
            json!({ "rarity": "Common" }),
            json!({ "cost": 1 }),
            json!({ "tags": ["KY"] }),
        ] {
            let got = ids(query(&q(args.clone())));
            let reached: Vec<&str> = GRAPES.iter().copied().filter(|id| got.contains(&(*id).to_string())).collect();
            assert_eq!(reached, Vec::<&str>::new(), "{args}");
        }
    }

    #[test]
    fn r382_classic_plus_23_dropshipping_s_pool_takes_every_card_and_every_token_of_every_set_but_itself() {
        register();
        let every = ids(pool("classicplus-023", &q(json!({ "withTokens": true }))));
        // R674: Glitch is the one token no pool takes.
        assert_eq!(every.len(), CATALOG.len() - 2);
        assert!(!every.contains(&"classicplus-023".to_string()));
        assert!(!every.contains(&GLITCH_DEF_ID.to_string()));
        for id in GRAPES
            .iter()
            .copied()
            .chain(["classicplus-019-3", "classicplus-t-ai-01", "core-t-coin", "core-051-1"])
        {
            assert!(every.contains(&id.to_string()), "{id}");
        }
    }
}
