//! A fused card's scripts are its own, whatever else the process has fused (R77, R179, §9.3).
//!
//! The script registry is the process's. A server runs many matches in one process, and the practice
//! worker runs the AI's simulated worlds beside the real game, so two states can each fuse into the
//! same `t-<n>` slot from different cards. R179's id names the ingredients, so those two fusions get
//! two ids and two registry entries; and because the id names them, the engine rebuilds any of a
//! state's fused scripts the registry lacks — a state that came through JSON into a process that never
//! ran its Fuse, or a registry replaced wholesale — whenever it is entered through `reduce`,
//! `legalActions` or `viewFor`.
//!
//! In Rust a fused script is never registered at all: `scripts::script_of(state, def_id)` composes it
//! from the state's fused definition (SURFACE §6.6), and the state keeps what it composed
//! (`GameState::fused_scripts`, SURFACE §17), so `syncFusedScripts` is gone and "the registry lacks
//! it" is the permanent condition the TS tests set up by hand. Each test keeps its steps: what TS
//! read off the registry after a rebuild is read here off `script_of`, and where TS showed the
//! registry empty before the rebuild, this shows the registry holds no entry for the id.
//!
//! Port of `packages/engine/test/fuse-registry.test.ts`.

use std::cell::Cell;
use std::sync::Arc;

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::reduce::{legal_actions, reduce};
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::scripts::{ScriptRef, face_ref, script_of};
use jackioh_engine::subsystems::fuse::fused_ingredients;
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::P1;

use crate::rules::fixtures::harness::new_game;

/// TS's `let nextIndex = 1700`, incremented once per `unit`: the four cards are 1701–1704.
fn unit(name: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("fr-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (fuse registry)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": format!("{name} radiant") },
    }))
}

fn cards() -> [CardDef; 4] {
    [
        unit("alpha", 1701),
        unit("beta", 1702),
        unit("gamma", 1703),
        unit("delta", 1704),
    ]
}

thread_local! {
    /// The markers the last Cry run applied, in order (TS's module-level `seen`). Each `#[test]` runs on
    /// its own thread, so the list is the test's own.
    static SEEN: Cell<Vec<String>> = const { Cell::new(Vec::new()) };
}

/// A Cry whose one effect records its card, so the concatenated hook shows its ingredients.
fn marked(name: &str) -> Script {
    let name = name.to_string();
    Script {
        cry: Some(hook(move |_ctx| {
            let name = name.clone();
            vec![Effect::new("marker", move |_ctx| {
                SEEN.with(|seen| {
                    let mut list = seen.take();
                    list.push(format!("marker:{name}"));
                    seen.set(list);
                });
            })]
        })),
        ..Script::default()
    }
}

fn setup() {
    let mut catalog = registered_catalog().clone();
    for def in cards() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    for def in cards() {
        scripts.insert(
            def.id.clone(),
            CardScripts {
                base: marked(&def.id),
                radiant: marked(&def.id),
            },
        );
    }
    register_scripts(scripts);
}

/// p1 crafts `def_ids` into hand, in order: each call is the state's next fused definition.
fn craft(state: &mut GameState, def_ids: &[String]) {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        apply_effects(
            &[fuse_cards(json_as(
                json!({ "defIds": def_ids, "toHand": "self" }),
            ))],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
}

/// Fresh matches on p1's turn 3. `newGame` registers the fixture catalog and scripts afresh, which
/// drops every fused entry, so every match a test needs is made before any of them fuses.
fn games(seeds: &[&str]) -> Vec<GameState> {
    let states = seeds
        .iter()
        .map(|seed| {
            let mut state = new_game(seed, None);
            state.turn = 3;
            state.active = P1;
            state
        })
        .collect();
    setup();
    states
}

/// A fresh match in which p1 crafts `first` + `second` into hand: the match's first fusion.
fn crafted_game(seed: &str, first: &CardDef, second: &CardDef) -> GameState {
    let mut state = games(&[seed]).into_iter().next().expect("one match");
    craft(&mut state, &[first.id.clone(), second.id.clone()]);
    state
}

/// The state's only fused definition's id.
fn only_fused(state: &GameState) -> String {
    let ids: Vec<String> = state.transient_defs.keys().cloned().collect();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

/// What the scripts for `def_id` do right now: its Cry, run with no play behind it on a copy of
/// `state` and applied, as the markers its ingredients' Cries record. TS read the registry directly,
/// so no engine entry point got to rebuild the entry first; Rust has no registry entry for a fused id,
/// and `script_of` is the one place its scripts come from.
fn markers(state: &GameState, def_id: &str) -> Vec<String> {
    let Some(cry) = script_of(state, def_id).base.cry.clone() else {
        return vec![];
    };
    SEEN.with(|seen| seen.set(Vec::new()));
    let mut copy: GameState =
        serde_json::from_value(serde_json::to_value(state).expect("a state serialises"))
            .expect("a state survives JSON");
    let mut events = Vec::new();
    let mut rng = Rng::new(&copy.seed, copy.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut copy, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        let effects = cry(&mut ctx);
        apply_effects(&effects, &mut ctx);
    }
    SEEN.with(|seen| seen.take())
}

/// The registry without these entries, as a package re-registering its catalog scripts leaves it.
fn drop_scripts(def_ids: &[&str]) {
    let mut rest = registered_scripts().clone();
    for def_id in def_ids {
        rest.shift_remove(*def_id);
    }
    register_scripts(rest);
}

/// The registry holds nothing for `def_id`: TS's `markers` reading `[]` off it before a rebuild.
fn registry_lacks(def_id: &str) -> bool {
    registered_scripts().get(def_id).is_none()
}

fn marker_list(defs: &[&CardDef]) -> Vec<String> {
    defs.iter().map(|def| format!("marker:{}", def.id)).collect()
}

mod fused_scripts_belong_to_the_state_that_fused_them {
    use super::*;

    #[test]
    fn r179_two_matches_in_one_process_that_fuse_different_pairs_into_the_same_slot_keep_two_ids_and_their_own_cry()
     {
        let [alpha, beta, gamma, delta] = cards();
        let mut matches = games(&["fuse-registry-a", "fuse-registry-b"]);
        let mut second = matches.pop().expect("the second match");
        let mut first = matches.pop().expect("the first match");
        craft(&mut first, &[alpha.id.clone(), beta.id.clone()]);
        craft(&mut second, &[gamma.id.clone(), delta.id.clone()]);
        let first_id = only_fused(&first);
        let second_id = only_fused(&second);
        assert_eq!(first_id, format!("t-1:{}+{}", alpha.id, beta.id));
        assert_eq!(second_id, format!("t-1:{}+{}", gamma.id, delta.id));

        // The second fusion wrote beside the first, not over it.
        assert_eq!(markers(&first, &first_id), marker_list(&[&alpha, &beta]));
        assert_eq!(markers(&second, &second_id), marker_list(&[&gamma, &delta]));

        // Entering the engine with either match, through every entry point, leaves both as they were.
        let _ = legal_actions(&first, P1);
        let _ = view_for(&second, P1);
        let _ = reduce(&first, &Action::new(ActionBody::EndTurn, P1, "fuse-registry"));
        assert_eq!(markers(&first, &first_id), marker_list(&[&alpha, &beta]));
        assert_eq!(markers(&second, &second_id), marker_list(&[&gamma, &delta]));
    }

    #[test]
    fn r179_the_id_alone_rebuilds_the_scripts_so_a_state_that_came_through_json_runs_its_own_fusion() {
        let [alpha, beta, _, _] = cards();
        let first = crafted_game("fuse-registry-json", &alpha, &beta);
        let mut copy: GameState =
            serde_json::from_value(serde_json::to_value(&first).expect("a state serialises")).expect("JSON");
        let id = only_fused(&copy);
        // The def carries a card's fields and, since R468, the ingredient list its id spells out. A
        // readable id is still enough on its own: with the list taken off the def, it rebuilds the same.
        let mut keys: Vec<String> = serde_json::to_value(&copy.transient_defs[&id])
            .expect("a def serialises")
            .as_object()
            .map(|def| def.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort();
        let mut expected: Vec<String> = [
            "base",
            "cost",
            "id",
            "index",
            "ingredients",
            "name",
            "radiant",
            "rarity",
            "set",
            "tags",
            "token",
            "type",
        ]
        .iter()
        .map(|key| key.to_string())
        .collect();
        expected.sort();
        assert_eq!(keys, expected);
        if let Some(def) = copy.transient_defs.get_mut(&id) {
            def.ingredients = None;
        }
        assert_eq!(
            fused_ingredients(&copy, &id),
            Some(vec![alpha.id.clone(), beta.id.clone()])
        );

        // A process that never ran this Fuse.
        drop_scripts(&[&id]);
        assert!(registry_lacks(&id));
        assert_eq!(markers(&copy, &id), marker_list(&[&alpha, &beta]));
    }

    #[test]
    fn r179_a_registry_replaced_wholesale_register_scripts_is_repaired_on_the_next_entry() {
        let [alpha, beta, _, _] = cards();
        let first = crafted_game("fuse-registry-reset", &alpha, &beta);
        let id = only_fused(&first);
        drop_scripts(&[&id]);
        assert!(registry_lacks(&id));

        let _ = legal_actions(&first, P1);
        assert_eq!(markers(&first, &id), marker_list(&[&alpha, &beta]));
    }

    #[test]
    fn r179_a_fusion_of_a_fusion_writes_its_older_ingredient_in_parentheses_and_rebuilds_the_whole_chain() {
        let [alpha, beta, gamma, _] = cards();
        let mut state = crafted_game("fuse-registry-chain", &alpha, &beta);
        let inner = only_fused(&state);
        craft(&mut state, &[inner.clone(), gamma.id.clone()]);
        let outer = state
            .transient_defs
            .keys()
            .find(|id| **id != inner)
            .cloned()
            .expect("the outer fusion");
        assert_eq!(outer, format!("t-2:({inner})+{}", gamma.id));
        assert_eq!(
            fused_ingredients(&state, &outer),
            Some(vec![inner.clone(), gamma.id.clone()])
        );
        assert!(script_of(&state, outer.as_str()).base.cry.is_some());

        drop_scripts(&[&inner, &outer]);
        let _ = view_for(&state, P1);
        assert_eq!(markers(&state, &inner), marker_list(&[&alpha, &beta]));
        assert_eq!(markers(&state, &outer), marker_list(&[&alpha, &beta, &gamma]));
    }

    #[test]
    fn r179_reads_a_fused_id_back_one_way_the_parentheses_keep_a_nested_ingredient_whole() {
        // `fused_ingredients` reads a digest id's list off the state's definition; a readable id needs
        // none, so any state answers for these.
        let state = new_game("fuse-registry-ids", None);
        let parts = |ids: &[&str]| Some(ids.iter().map(|id| id.to_string()).collect::<Vec<_>>());
        // `t-1:a+b` fused with c and d, and `t-1:a+b+c` fused with d, are two different cards.
        assert_eq!(
            fused_ingredients(&state, "t-2:(t-1:a+b)+c+d"),
            parts(&["t-1:a+b", "c", "d"])
        );
        assert_eq!(
            fused_ingredients(&state, "t-2:(t-1:a+b+c)+d"),
            parts(&["t-1:a+b+c", "d"])
        );
        assert_eq!(
            fused_ingredients(&state, "t-3:(t-2:(t-1:a+b)+c)+(t-1:d+e)"),
            parts(&["t-2:(t-1:a+b)+c", "t-1:d+e"])
        );
        // Nothing a Fuse did not mint: a catalog id, a bare count, a single ingredient.
        assert_eq!(fused_ingredients(&state, "core-085"), None);
        assert_eq!(fused_ingredients(&state, "t-1"), None);
        assert_eq!(fused_ingredients(&state, "t-1:a"), None);
    }

    #[test]
    fn r179_a_state_keeps_the_scripts_a_fuse_composed_and_reduce_composes_them_for_a_state_from_json() {
        let [alpha, beta, _, _] = cards();
        let first = crafted_game("fuse-registry-kept", &alpha, &beta);
        let id = only_fused(&first);
        let composed = |state: &GameState| match face_ref(state, &id, false) {
            ScriptRef::Composed(script) => script,
            ScriptRef::Static(_) => panic!("a fused id's script is composed"),
        };

        // The Fuse composed the scripts as it minted the id, so every lookup reads that one script, which
        // runs its ingredients' texts.
        assert!(Arc::ptr_eq(&composed(&first), &composed(&first)));
        assert_eq!(markers(&first, &id), marker_list(&[&alpha, &beta]));

        // A state that came through JSON holds none: each lookup composes its own, which runs the same.
        let copy: GameState =
            serde_json::from_value(serde_json::to_value(&first).expect("a state serialises")).expect("JSON");
        assert!(!Arc::ptr_eq(&composed(&copy), &composed(&copy)));
        assert_eq!(markers(&copy, &id), marker_list(&[&alpha, &beta]));

        // Entering `reduce` (any action: here a concession) composes them, and its state keeps them.
        let next = reduce(&copy, &Action::new(ActionBody::Concede, P1, "fuse-registry-kept"));
        assert_eq!(next.error, None);
        assert!(Arc::ptr_eq(&composed(&next.state), &composed(&next.state)));

        // A registry replaced wholesale is not the one they were composed from: the lookup composes anew.
        drop_scripts(&[&id]);
        assert!(!Arc::ptr_eq(&composed(&next.state), &composed(&next.state)));
    }
}
