//! ME-CRAFT's two Meditative+ modes (issue #1182), through the fixture Spells of
//! `fixtures/craft.rs`: the potion frame of M+ #16 Jazakus and M+ #16.1 Jazakus Prime
//! (`pm-potion-crafter`, `pm-huge-potion-crafter`; R1182000, R1182001) and Scratch Pad's
//! from-scratch canvas (`pm-scratch-crafter`, R1182002). The cards themselves are proved again in
//! `crates/cards`.

use jackioh_engine::subsystems::craft::{
    compile_crafted_def, compile_crafted_scripts, craft_id, craft_potion_presets, craft_presets,
    craft_preview, points_budget, recipe_points, validate_recipe,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::craft::register_craft_fixtures;
use crate::rules::fixtures::prompt_harness::{act, answer_keys, board, cast_now, must, open_as, round_trip};

/// An `Action` from its JSON, as the socket sends it.
fn action(literal: Value) -> Action {
    json_as(literal)
}

/// A Potion recipe in `frame` at `cost`: an `attack`/`health` Unit whose Activate runs `effects`.
fn potion(frame: &str, cost: i32, attack: i32, health: i32, effects: Value) -> Value {
    json!({
        "cost": cost, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": attack, "health": health, "keywords": [], "echo": 0,
        "hats": [{ "hat": "activate", "effects": effects }],
        "frame": frame,
    })
}

/// A 2/2 Potion whose Activate deals 4 to the enemy hero: 3 + 4 = 7 points, a Potion's whole budget.
fn two_two_dealing_four() -> Value {
    potion("potion", 1, 2, 2, json!([{ "verb": "damageEnemyHero", "n": 4 }]))
}

/// The fixture Spell `def_id` cast by p1 (Radiant when asked): its craft prompt open.
fn crafting(seed: &str, def_id: &str, radiant: bool) -> GameState {
    let mut state = board(seed);
    register_craft_fixtures();
    cast_now(&mut state, def_id, P1, radiant);
    state
}

/// The open craft prompt's presets, in order.
fn presets(state: &GameState) -> Vec<CraftRecipe> {
    must(state.pending.clone(), "a craft prompt")
        .options
        .iter()
        .map(|option| match &option.selection {
            Selection::Craft { recipe } => recipe.clone(),
            other => panic!("a preset is a recipe, not {other:?}"),
        })
        .collect()
}

/// The open craft prompt answered with `recipe` through `reduce`, as the editor sends it.
fn answered(state: &GameState, recipe: Value) -> ReduceResult {
    let pending = must(state.pending.clone(), "a craft prompt");
    reduce(
        state,
        &action(json!({
            "type": "answer", "playerId": "p1", "nonce": format!("craft-{}", pending.id),
            "choiceId": pending.id, "selection": [{ "pick": "craft", "recipe": recipe }],
        })),
    )
}

/// The crafted card in p1's hand.
fn crafted(state: &GameState) -> CardInstance {
    must(
        state.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the crafted card",
    )
    .clone()
}

/// Refused with `reason`, the state unchanged and the craft prompt still open.
fn refused(state: &GameState, recipe: Value, reason: &str) {
    let result = answered(state, recipe.clone());
    let error = must(result.error, &format!("a refusal of {recipe}"));
    assert!(error.contains(reason), "{error} (wanted: {reason})");
    assert_eq!(
        hash_state(&result.state),
        hash_state(state),
        "the state is unchanged"
    );
    assert_eq!(
        result.state.pending.as_ref().map(|pending| pending.kind),
        Some(PromptKind::Craft),
        "the prompt stays open"
    );
}

/// p1 plays the crafted Potion into lane 1 and activates it with `targets`.
fn played_and_activated(state: &GameState, targets: Value) -> GameState {
    let potion = crafted(state).id;
    let state = act(
        state,
        json!({ "type": "play", "playerId": "p1", "instanceId": potion, "zone": { "row": "units", "lane": 1 } }),
        None,
    );
    act(
        &state,
        json!({ "type": "activate", "playerId": "p1", "instanceId": potion, "targets": targets }),
        None,
    )
}

#[test]
fn r1182000_a_potion_crafter_opens_a_potion_prompt_at_cost_1_whose_presets_are_potions() {
    let state = crafting("potion-open", "pm-potion-crafter", false);
    let pending = open_as(&state, PromptKind::Craft, P1);
    assert_eq!(pending.budget, Some(CRAFT_POTION_COST));
    assert_eq!(pending.craft_mode, Some(CraftMode::Potion));
    let offered = presets(&state);
    assert_eq!(offered.len(), CRAFT_PRESETS);
    for recipe in &offered {
        assert_eq!(recipe.frame, Some(CraftFrame::Potion));
        assert_eq!(recipe.type_, CardType::Unit);
        assert_eq!(recipe.cost, 1);
        assert_eq!(
            recipe.hats.iter().map(|hat| hat.hat).collect::<Vec<_>>(),
            vec![CraftHatKind::Activate]
        );
        assert!(validate_recipe(recipe, 1).is_ok(), "{recipe:?}");
    }
    // The holder's view carries the mode; the opponent's shows only that a prompt is open.
    let own = view_for(&state, P1);
    match own.pending {
        Some(PendingView::ForYou(view)) => assert_eq!(view.craft_mode, Some(CraftMode::Potion)),
        other => panic!("p1's own prompt, not {other:?}"),
    }
    let theirs = serde_json::to_string(&view_for(&state, P2)).expect("the view serializes");
    assert!(!theirs.contains("craftMode"), "{theirs}");
}

#[test]
fn r1182000_a_crafted_potion_is_a_meditative_plus_unit_tagged_potion_with_one_self_tributing_activate() {
    let state = crafting("potion-def", "pm-potion-crafter", false);
    let result = answered(&state, two_two_dealing_four());
    assert_eq!(result.error, None);
    let card = crafted(&result.state);
    let def = must(result.state.transient_defs.get(&card.def_id), "its definition").clone();
    assert_eq!(def.set, SetName::MeditativePlus);
    assert_eq!(def.tags, vec![Tag::Potion]);
    assert_eq!(def.type_, CardType::Unit);
    assert_eq!(def.cost, CardCost::Fixed(1));
    assert_eq!(def.rarity, Rarity::Mythic);
    assert!(!def.token);
    assert_eq!(def.name, "Pure Closure Potion");
    assert_eq!(
        def.base.text,
        "Activate: Tribute this. Deal {b1} damage to the enemy hero."
    );
    assert_eq!([def.base.attack, def.base.health], [Some(2), Some(2)]);
    let recipe: CraftRecipe = json_as(two_two_dealing_four());
    assert_eq!(card.def_id, craft_id(&recipe));
    assert_eq!(def, compile_crafted_def(&recipe));
    // The frame, the Activate and its verb: 3 + 4 + 1.
    assert_eq!(def.loc, Some(8));
    let scripts = compile_crafted_scripts(&recipe);
    assert!(scripts.base.cry.is_none());
    assert!(scripts.base.targets.is_empty());
    assert_eq!(scripts.base.activations.len(), 1);
    let ability = &scripts.base.activations[0];
    assert_eq!(ability.uses, ActivationUses::Count(1));
    assert_eq!(ability.cost.and_then(|cost| cost.tribute_self), Some(true));
}

#[test]
fn r1182000_activating_a_crafted_potion_tributes_it_and_runs_its_effects() {
    let state = crafting("potion-activate", "pm-potion-crafter", false);
    let state = answered(&state, two_two_dealing_four()).state;
    let potion = crafted(&state).id;
    let after = played_and_activated(&state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 4);
    let gone = must(find_instance(&after, &potion), "the tributed Potion");
    assert_eq!(gone.zone.z(), ZoneName::Graveyard, "the Potion paid with itself");
}

#[test]
fn r1182000_a_radiant_potion_doubles_its_numbers_and_stats_and_still_tributes_itself() {
    let state = crafting("potion-radiant", "pm-potion-crafter", true);
    let state = answered(&state, two_two_dealing_four()).state;
    let card = crafted(&state);
    assert!(card.radiant, "the Radiant face crafts it Radiant");
    let def = must(state.transient_defs.get(&card.def_id), "its definition");
    assert_eq!([def.radiant.attack, def.radiant.health], [Some(4), Some(4)]);
    let after = played_and_activated(&state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 8);
    assert_eq!(
        must(find_instance(&after, &card.id), "the Potion").zone.z(),
        ZoneName::Graveyard
    );
}

#[test]
fn r1182000_a_potions_activate_declares_its_own_target_never_the_potion() {
    // A 2/2 whose Activate deals 3 to a target: 3 + 3 = 6 points.
    let aimed = potion("potion", 1, 2, 2, json!([{ "verb": "damageTarget", "n": 3 }]));
    let scripts = compile_crafted_scripts(&json_as(aimed.clone()));
    assert!(scripts.base.targets.is_empty(), "the play declares nothing");
    assert_eq!(scripts.base.activations[0].targets.len(), 1);
    let filter = serde_json::to_value(&scripts.base.activations[0].targets[0]).expect("a declaration");
    assert_eq!(filter["filter"]["excludeSelf"], json!(true), "{filter}");

    let state = crafting("potion-aimed", "pm-potion-crafter", false);
    let state = answered(&state, aimed).state;
    let after = played_and_activated(&state, json!([{ "pick": "hero", "player": "p2" }]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 3);
}

#[test]
fn r1182000_a_recipe_outside_the_potion_frame_is_refused_and_the_prompt_stays_open() {
    let state = crafting("potion-refused", "pm-potion-crafter", false);
    // Meditative #17's recipe, with no frame.
    let unframed = json!({
        "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 3, "health": 4, "keywords": [], "echo": 0, "hats": [],
    });
    refused(&state, unframed, "this prompt crafts a Potion");
    refused(
        &state,
        potion(
            "hugePotion",
            3,
            2,
            2,
            json!([{ "verb": "damageEnemyHero", "n": 4 }]),
        ),
        "this prompt crafts a Potion",
    );
    // A Cry is no Potion's hat; two hats, nor are they; no hat at all, nor is that.
    let mut cry = two_two_dealing_four();
    cry["hats"][0]["hat"] = json!("cry");
    refused(&state, cry, "exactly one hat, its Activate");
    let mut two = two_two_dealing_four();
    two["hats"] = json!([
        { "hat": "activate", "effects": [{ "verb": "gainArmor", "n": 1 }] },
        { "hat": "death", "effects": [{ "verb": "gainArmor", "n": 1 }] },
    ]);
    refused(&state, two, "exactly one hat, its Activate");
    let mut bare = two_two_dealing_four();
    bare["hats"] = json!([]);
    refused(&state, bare, "exactly one hat, its Activate");
    // The frame fixes the type and the cost.
    let mut spell = two_two_dealing_four();
    spell["type"] = json!("Spell");
    spell["attack"] = json!(0);
    spell["health"] = json!(1);
    refused(&state, spell, "a Potion is a Unit");
    let mut dearer = two_two_dealing_four();
    dearer["cost"] = json!(2);
    refused(&state, dearer, "a Potion costs (1)");
    // One point over the budget of 7.
    refused(
        &state,
        potion("potion", 1, 3, 2, json!([{ "verb": "damageEnemyHero", "n": 4 }])),
        "over the budget of 7",
    );
    // An Activate with nothing under it.
    refused(
        &state,
        potion("potion", 1, 2, 2, json!([])),
        "has no effect under it",
    );
}

#[test]
fn r1182000_the_free_crafter_takes_no_potion_and_no_unit_takes_an_activate_there() {
    let mut state = crafting("free-no-potion", "pm-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:1"]).error, None);
    assert_eq!(must(state.pending.clone(), "the craft prompt").craft_mode, None);
    refused(&state, two_two_dealing_four(), "this prompt crafts no Potion");
    let mut activate = two_two_dealing_four();
    activate.as_object_mut().expect("an object").remove("frame");
    refused(&state, activate, "a Unit takes no Activate");
}

#[test]
fn r1182000_the_random_policy_and_a_timeout_answer_a_potion_prompt_with_a_preset() {
    use jackioh_engine::subsystems::ai_policy::choose_action;

    let state = crafting("potion-policy", "pm-potion-crafter", false);
    let offered = presets(&state);
    let mut rng = Rng::new("potion-policy-pick", 0);
    match choose_action(&state, P1, &mut rng).expect("the policy answers") {
        ActionBody::Answer { selection, .. } => match &selection[..] {
            [Selection::Craft { recipe }] => assert!(offered.contains(recipe)),
            other => panic!("one preset, not {other:?}"),
        },
        other => panic!("an answer, not {other:?}"),
    }
    let timed = reduce(
        &state,
        &action(json!({ "type": "timeout", "playerId": "p1", "nonce": "potion-t1" })),
    );
    assert_eq!(timed.error, None);
    let card = crafted(&timed.state);
    let def = must(timed.state.transient_defs.get(&card.def_id), "its definition");
    assert!(offered.contains(must(def.craft.as_ref(), "its recipe")));
    assert_eq!(def.tags, vec![Tag::Potion]);
}

#[test]
fn r1182000_every_potion_preset_is_a_different_verb_valid_and_spends_the_whole_budget() {
    for frame in [CraftFrame::Potion, CraftFrame::HugePotion] {
        let cost = if frame == CraftFrame::Potion {
            CRAFT_POTION_COST
        } else {
            CRAFT_HUGE_POTION_COST
        };
        for seed in 0..50 {
            let mut rng = Rng::new(&format!("potion-preset-{seed}"), 0);
            let offered = craft_potion_presets(&mut rng, frame);
            assert_eq!(offered.len(), CRAFT_PRESETS, "{frame} seed {seed}");
            let verbs: IndexSet<CraftVerb> = offered
                .iter()
                .map(|recipe| recipe.hats[0].effects[0].verb)
                .collect();
            assert_eq!(verbs.len(), CRAFT_PRESETS, "no two alike: {offered:?}");
            for recipe in &offered {
                assert_eq!(recipe.frame, Some(frame));
                assert!(
                    validate_recipe(recipe, cost).is_ok(),
                    "{:?}",
                    validate_recipe(recipe, cost)
                );
                assert_eq!(
                    recipe_points(recipe),
                    points_budget(Some(frame), cost),
                    "{recipe:?}"
                );
            }
        }
    }
}

#[test]
fn r1182000_a_crafted_potion_survives_a_json_round_trip_and_activates_the_same() {
    let state = crafting("potion-trip", "pm-potion-crafter", false);
    let state = answered(&state, two_two_dealing_four()).state;
    let potion = crafted(&state).id;
    let state = act(
        &state,
        json!({ "type": "play", "playerId": "p1", "instanceId": potion, "zone": { "row": "units", "lane": 1 } }),
        None,
    );
    let activate =
        action(json!({ "type": "activate", "playerId": "p1", "instanceId": potion, "nonce": "trip-1" }));
    let live = reduce(&state, &activate);
    let tripped = reduce(&round_trip(&state), &activate);
    assert_eq!(live.error, None);
    assert_eq!(tripped.error, None);
    assert_eq!(
        tripped.state.players[P2].hero.health,
        HERO_HEALTH - 4,
        "the rebuilt Activate deals its 4"
    );
    assert_eq!(hash_state(&tripped.state), hash_state(&live.state));
}

#[test]
fn r1182000_an_unframed_recipe_and_a_free_prompt_serialize_as_before() {
    // D9: Meditative #17's recipes, digests and prompts carry no new field.
    let mut state = crafting("free-shape", "pm-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:2"]).error, None);
    let text = serde_json::to_string(&state.pending).expect("the prompt serializes");
    assert!(!text.contains("craftMode"), "{text}");
    assert!(!text.contains("\"frame\""), "{text}");
}

#[test]
fn r1182001_a_huge_potion_costs_3_with_five_times_a_potions_points() {
    let state = crafting("huge-open", "pm-huge-potion-crafter", false);
    let pending = open_as(&state, PromptKind::Craft, P1);
    assert_eq!(pending.budget, Some(CRAFT_HUGE_POTION_COST));
    assert_eq!(pending.craft_mode, Some(CraftMode::HugePotion));
    let budget = points_budget(Some(CraftFrame::HugePotion), 3);
    assert_eq!(
        budget,
        CRAFT_HUGE_POTION_BUDGET_MULTIPLE * points_budget(Some(CraftFrame::Potion), 1)
    );
    assert_eq!(budget, 35);
    for recipe in presets(&state) {
        assert_eq!(recipe.frame, Some(CraftFrame::HugePotion));
        assert_eq!(recipe.cost, 3);
    }
    // A 10/10 (19) whose Activate deals 10 (10) and gains 6 Armor (6): 35, the whole budget.
    let full = potion(
        "hugePotion",
        3,
        10,
        10,
        json!([{ "verb": "damageEnemyHero", "n": 10 }, { "verb": "gainArmor", "n": 6 }]),
    );
    let preview = craft_preview(&json_as(full.clone()), 3);
    assert!(preview.valid, "{:?}", preview.reasons);
    assert_eq!([preview.points, preview.points_budget], [35, 35]);
    let mut over = full.clone();
    over["hats"][0]["effects"][1]["n"] = json!(7);
    refused(&state, over, "over the budget of 35");
    refused(&state, two_two_dealing_four(), "this prompt crafts a HUGE Potion");

    let result = answered(&state, full);
    assert_eq!(result.error, None);
    let card = crafted(&result.state);
    let def = must(result.state.transient_defs.get(&card.def_id), "its definition");
    assert_eq!(def.name, "HUGE Pure Closure Potion");
    assert_eq!(def.cost, CardCost::Fixed(3));
    assert_eq!(def.tags, vec![Tag::Potion]);
    let after = played_and_activated(&result.state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 10);
    assert_eq!(after.players[P1].hero.armor, 6);
}

#[test]
fn r1182002_from_scratch_is_the_free_crafters_chain_with_its_flag() {
    let mut state = crafting("scratch-open", "pm-scratch-crafter", false);
    let cost = open_as(&state, PromptKind::Number, P1);
    let keys: Vec<&str> = cost.options.iter().map(|option| option.key.as_str()).collect();
    assert_eq!(keys, vec!["mode:0", "mode:1", "mode:2", "mode:3", "mode:4"]);
    let (seed, cursor) = (state.seed.clone(), state.rng_cursor);
    assert_eq!(answer_keys(&mut state, &["mode:2"]).error, None);
    let pending = open_as(&state, PromptKind::Craft, P1);
    assert_eq!(pending.budget, Some(2));
    assert_eq!(pending.craft_mode, Some(CraftMode::Scratch));
    // The engine still rolls Meditative #17's four presets, from the match rng (R881).
    let mut rng = Rng::new(&seed, cursor);
    assert_eq!(presets(&state), craft_presets(&mut rng, 2));
    match view_for(&state, P1).pending {
        Some(PendingView::ForYou(view)) => {
            assert_eq!(view.craft_mode, Some(CraftMode::Scratch));
            assert_eq!(view.options.len(), CRAFT_PRESETS);
        }
        other => panic!("p1's own prompt, not {other:?}"),
    }
    let theirs = serde_json::to_string(&view_for(&state, P2)).expect("the view serializes");
    assert!(!theirs.contains("craftMode"), "{theirs}");
}

#[test]
fn r1182002_a_card_from_scratch_is_the_card_the_free_crafter_makes_of_that_recipe() {
    let mut state = crafting("scratch-mint", "pm-scratch-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:2"]).error, None);
    let recipe = json!({
        "cost": 2, "type": "Unit", "adjective": "Lazy", "noun": "Iterator",
        "attack": 5, "health": 6, "keywords": [], "echo": 0, "hats": [],
    });
    refused(&state, two_two_dealing_four(), "this prompt crafts no Potion");
    let result = answered(&state, recipe.clone());
    assert_eq!(result.error, None);
    let card = crafted(&result.state);
    let parsed: CraftRecipe = json_as(recipe);
    assert_eq!(card.def_id, craft_id(&parsed));
    let def = must(result.state.transient_defs.get(&card.def_id), "its definition");
    assert_eq!(def, &compile_crafted_def(&parsed));
    assert_eq!(def.set, SetName::Meditative);
    assert!(def.tags.is_empty());
}

#[test]
fn r1182002_the_random_policy_and_a_timeout_answer_from_scratch_with_a_preset() {
    use jackioh_engine::subsystems::ai_policy::choose_action;

    let mut state = crafting("scratch-policy", "pm-scratch-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:3"]).error, None);
    let offered = presets(&state);
    let mut rng = Rng::new("scratch-policy-pick", 0);
    match choose_action(&state, P1, &mut rng).expect("the policy answers") {
        ActionBody::Answer { selection, .. } => match &selection[..] {
            [Selection::Craft { recipe }] => assert!(offered.contains(recipe)),
            other => panic!("one preset, not {other:?}"),
        },
        other => panic!("an answer, not {other:?}"),
    }
    let timed = reduce(
        &state,
        &action(json!({ "type": "timeout", "playerId": "p1", "nonce": "scratch-t1" })),
    );
    assert_eq!(timed.error, None);
    let card = crafted(&timed.state);
    let def = must(timed.state.transient_defs.get(&card.def_id), "its definition");
    assert!(offered.contains(must(def.craft.as_ref(), "its recipe")));
}
