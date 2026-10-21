//! ME-CRAFT's two Meditative+ modes (issue #1182), through the fixture Spells of
//! `fixtures/craft.rs`: the potion frame of M+ #16 Jazakus and M+ #16.1 Jazakus Prime
//! (`pm-potion-crafter`, `pm-huge-potion-crafter`; R1182000, R1182001; a Spell with no stats since
//! #1563, R1563008) and Scratch Pad's from-scratch canvas (`pm-scratch-crafter`, R1182002), and
//! #1563's offer (R1563007) and retired Armor verb (R1563009). The cards themselves are proved again
//! in `crates/cards`.

use jackioh_engine::subsystems::craft::{
    compile_crafted_def, compile_crafted_scripts, craft_id, craft_potion_presets, craft_presets,
    craft_preview, points_budget, recipe_points, roll_offer, validate_answer, validate_recipe, whole_palette,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::craft::register_craft_fixtures;
use crate::rules::fixtures::prompt_harness::{act, answer_keys, board, cast_now, must, open_as, round_trip};

/// An `Action` from its JSON, as the socket sends it.
fn action(literal: Value) -> Action {
    json_as(literal)
}

/// A Potion recipe in `frame` at `cost`: a Spell with no stats whose When cast runs `effects`.
fn potion(frame: &str, cost: i32, effects: Value) -> Value {
    json!({
        "cost": cost, "type": "Spell", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "whenCast", "effects": effects }],
        "frame": frame,
    })
}

/// A Potion that deals 4 to the enemy hero: 4 points, a Potion's whole budget.
fn dealing_four() -> Value {
    potion("potion", 1, json!([{ "verb": "damageEnemyHero", "n": 4 }]))
}

/// The fixture Spell `def_id` cast by p1 (Radiant when asked): its craft prompt open.
fn crafting(seed: &str, def_id: &str, radiant: bool) -> GameState {
    let mut state = board(seed);
    register_craft_fixtures();
    cast_now(&mut state, def_id, P1, radiant);
    state
}

/// R1563007: the open craft prompt's offer widened to every block, so a test about something else
/// may answer with any verb.
fn any_block(mut state: GameState) -> GameState {
    if let Some(pending) = state.pending.as_mut() {
        pending.craft_offer = Some(whole_palette());
    }
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

/// p1 casts the crafted Potion with `targets`.
fn cast_potion(state: &GameState, targets: Value) -> GameState {
    let potion = crafted(state).id;
    act(
        state,
        json!({ "type": "play", "playerId": "p1", "instanceId": potion, "targets": targets }),
        None,
    )
}

#[test]
fn r1182000_r1563007_r1563008_a_potion_crafter_opens_a_potion_prompt_at_cost_1_whose_presets_are_spells_of_its_offer() {
    for n in 0..20 {
        let state = crafting(&format!("potion-open-{n}"), "pm-potion-crafter", false);
        let pending = open_as(&state, PromptKind::Craft, P1);
        assert_eq!(pending.budget, Some(CRAFT_POTION_COST));
        assert_eq!(pending.craft_mode, Some(CraftMode::Potion));
        let offer = must(pending.craft_offer.clone(), "a potion prompt's offer");
        assert_eq!(offer.hats, vec![CraftHatKind::WhenCast], "a Potion's one hat");
        // Four of the whole table: a Potion takes only a Spell's, but a granted keyword may be any.
        assert_eq!(offer.keywords.len(), CRAFT_OFFER_KEYWORDS, "{offer:?}");
        assert!(!offer.verbs.is_empty() && offer.verbs.len() <= CRAFT_OFFER_VERBS);
        assert!(!offer.verbs.contains(&CraftVerb::GainArmor));
        let offered = presets(&state);
        assert!(!offered.is_empty() && offered.len() <= CRAFT_PRESETS);
        for recipe in &offered {
            assert_eq!(recipe.frame, Some(CraftFrame::Potion));
            assert_eq!(recipe.type_, CardType::Spell);
            assert_eq!(recipe.cost, 1);
            assert_eq!((recipe.attack, recipe.health), (0, 1), "no stats");
            assert_eq!(
                recipe.hats.iter().map(|hat| hat.hat).collect::<Vec<_>>(),
                vec![CraftHatKind::WhenCast]
            );
            assert!(
                validate_answer(recipe, 1, Some(CraftMode::Potion), Some(&offer)).is_ok(),
                "{recipe:?}"
            );
        }
        // The holder's view carries the mode and the offer; the opponent's shows only that a prompt
        // is open.
        match view_for(&state, P1).pending {
            Some(PendingView::ForYou(view)) => {
                assert_eq!(view.craft_mode, Some(CraftMode::Potion));
                assert_eq!(view.craft_offer, Some(offer));
            }
            other => panic!("p1's own prompt, not {other:?}"),
        }
        let theirs = serde_json::to_string(&view_for(&state, P2)).expect("the view serializes");
        assert!(!theirs.contains("craftMode") && !theirs.contains("craftOffer"), "{theirs}");
    }
}

#[test]
fn r1182000_r1563008_a_crafted_potion_is_a_meditative_plus_spell_tagged_potion_cast_for_its_effects() {
    let state = any_block(crafting("potion-def", "pm-potion-crafter", false));
    let result = answered(&state, dealing_four());
    assert_eq!(result.error, None);
    let card = crafted(&result.state);
    let def = must(result.state.transient_defs.get(&card.def_id), "its definition").clone();
    assert_eq!(def.set, SetName::MeditativePlus);
    assert_eq!(def.tags, vec![Tag::Potion]);
    assert_eq!(def.type_, CardType::Spell);
    assert_eq!(def.cost, CardCost::Fixed(1));
    assert_eq!(def.rarity, Rarity::Mythic);
    assert!(!def.token);
    assert_eq!(def.name, "Pure Closure Potion");
    assert_eq!(def.base.text, "Deal {b1} damage to the enemy hero.");
    assert_eq!([def.base.attack, def.base.health], [None, None], "no stats");
    let recipe: CraftRecipe = json_as(dealing_four());
    assert_eq!(card.def_id, craft_id(&recipe));
    assert_eq!(def, compile_crafted_def(&recipe));
    // The frame, the When cast and its verb: 3 + 2 + 1.
    assert_eq!(def.loc, Some(6));
    let scripts = compile_crafted_scripts(&recipe);
    assert!(scripts.base.cry.is_some(), "a Spell resolves its When cast");
    assert!(scripts.base.activations.is_empty());
}

#[test]
fn r1563008_casting_a_crafted_potion_runs_its_effects_and_it_goes_to_the_graveyard() {
    let state = any_block(crafting("potion-cast", "pm-potion-crafter", false));
    let state = answered(&state, dealing_four()).state;
    let potion = crafted(&state).id;
    let after = cast_potion(&state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 4);
    let gone = must(find_instance(&after, &potion), "the cast Potion");
    assert_eq!(gone.zone.z(), ZoneName::Graveyard);
}

#[test]
fn r1182000_r1563008_a_radiant_potion_doubles_its_numbers() {
    let state = any_block(crafting("potion-radiant", "pm-potion-crafter", true));
    let state = answered(&state, dealing_four()).state;
    let card = crafted(&state);
    assert!(card.radiant, "the Radiant face crafts it Radiant");
    let after = cast_potion(&state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 8);
}

#[test]
fn r1563008_an_aimed_potion_declares_its_target_with_the_cast() {
    // A Potion that deals 3 to a target: 3 points.
    let aimed = potion("potion", 1, json!([{ "verb": "damageTarget", "n": 3 }]));
    let scripts = compile_crafted_scripts(&json_as(aimed.clone()));
    assert_eq!(scripts.base.targets.len(), 1, "the cast declares it");
    let state = any_block(crafting("potion-aimed", "pm-potion-crafter", false));
    let state = answered(&state, aimed).state;
    let after = cast_potion(&state, json!([{ "pick": "hero", "player": "p2" }]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 3);
}

#[test]
fn r1182000_r1563008_a_recipe_outside_the_potion_frame_is_refused_and_the_prompt_stays_open() {
    let state = any_block(crafting("potion-refused", "pm-potion-crafter", false));
    // Meditative #17's recipe, with no frame.
    let unframed = json!({
        "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 3, "health": 4, "keywords": [], "echo": 0, "hats": [],
    });
    refused(&state, unframed, "this prompt crafts a Potion");
    refused(
        &state,
        potion("hugePotion", 3, json!([{ "verb": "damageEnemyHero", "n": 4 }])),
        "this prompt crafts a Potion",
    );
    // A Cry is no Potion's hat; two hats, nor are they; no hat at all, nor is that.
    let mut cry = dealing_four();
    cry["hats"][0]["hat"] = json!("cry");
    refused(&state, cry, "exactly one hat, its When cast");
    let mut two = dealing_four();
    two["hats"] = json!([
        { "hat": "whenCast", "effects": [{ "verb": "draw", "n": 1 }] },
        { "hat": "death", "effects": [{ "verb": "draw", "n": 1 }] },
    ]);
    refused(&state, two, "exactly one hat, its When cast");
    let mut bare = dealing_four();
    bare["hats"] = json!([]);
    refused(&state, bare, "exactly one hat, its When cast");
    // The frame fixes the type, its stats and the cost.
    let mut unit = dealing_four();
    unit["type"] = json!("Unit");
    unit["attack"] = json!(2);
    unit["health"] = json!(2);
    refused(&state, unit, "a Potion is a Spell");
    let mut stats = dealing_four();
    stats["attack"] = json!(2);
    refused(&state, stats, "only a Unit has stats");
    let mut dearer = dealing_four();
    dearer["cost"] = json!(2);
    refused(&state, dearer, "a Potion costs (1)");
    // One point over the budget of 4.
    refused(
        &state,
        potion("potion", 1, json!([{ "verb": "damageEnemyHero", "n": 5 }])),
        "over the budget of 4",
    );
    // A When cast with nothing under it.
    refused(&state, potion("potion", 1, json!([])), "has no effect under it");
    // The Activate it had until #1563.
    let mut activate = dealing_four();
    activate["hats"][0]["hat"] = json!("activate");
    refused(&state, activate, "exactly one hat, its When cast");
}

#[test]
fn r1563007_a_block_the_offer_lacks_is_refused() {
    let state = crafting("potion-offer", "pm-potion-crafter", false);
    let offer = must(open_as(&state, PromptKind::Craft, P1).craft_offer, "its offer");
    let missing = must(
        [CraftVerb::DamageEnemyHero, CraftVerb::Draw, CraftVerb::GainMana, CraftVerb::DamageRandomEnemy]
            .into_iter()
            .find(|verb| !offer.verbs.contains(verb)),
        "a verb outside three",
    );
    let recipe = potion("potion", 1, json!([{ "verb": missing, "n": 1 }]));
    refused(&state, recipe, "was not offered");
    let offered = offer.verbs[0];
    let mut fits = potion("potion", 1, json!([{ "verb": offered }]));
    if verb_takes_n(offered) {
        fits["hats"][0]["effects"][0]["n"] = json!(1);
    }
    let result = answered(&state, fits);
    assert_eq!(result.error, None, "an offered verb crafts");
}

/// Whether a verb takes a number, read off the price table.
fn verb_takes_n(verb: CraftVerb) -> bool {
    CRAFT_VERB_PRICES.iter().any(|row| row.verb == verb && row.takes_n)
}

#[test]
fn r1563009_giving_your_hero_armor_is_no_block() {
    let mut state = crafting("free-armor", "pm-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:2"]).error, None);
    let state = any_block(state);
    let armor = json!({
        "cost": 2, "type": "Spell", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "whenCast", "effects": [{ "verb": "gainArmor", "n": 2 }] }],
    });
    refused(&state, armor, "gaining Armor is no craft block");
    assert!(!whole_palette().verbs.contains(&CraftVerb::GainArmor));
}

#[test]
fn r1563007_every_free_prompt_offers_four_keywords_two_hats_and_three_verbs_and_its_presets_use_them() {
    for cost in 0..=4 {
        for n in 0..30 {
            let mut rng = Rng::new(&format!("offer-{cost}-{n}"), 0);
            let offer = must(roll_offer(&mut rng, None, cost), "a free prompt's offer");
            assert_eq!(offer.keywords.len(), CRAFT_OFFER_KEYWORDS);
            assert_eq!(offer.hats.len(), CRAFT_OFFER_HATS);
            assert_eq!(offer.verbs.len(), CRAFT_OFFER_VERBS);
            assert!(!offer.hats.contains(&CraftHatKind::Activate));
            assert!(!offer.verbs.contains(&CraftVerb::GainArmor));
            let presets = craft_presets(&mut rng, cost, &offer);
            assert_eq!(presets.first().map(|recipe| recipe.type_), Some(CardType::Unit));
            for recipe in &presets {
                assert!(
                    validate_answer(recipe, cost, None, Some(&offer)).is_ok(),
                    "cost {cost} seed {n}: {recipe:?} {:?}",
                    validate_answer(recipe, cost, None, Some(&offer))
                );
            }
        }
    }
    let mut rng = Rng::new("offer-scratch", 0);
    assert_eq!(roll_offer(&mut rng, Some(CraftMode::Scratch), 2), None);
}

#[test]
fn r1182000_the_free_crafter_takes_no_potion_and_no_type_takes_an_activate_there() {
    let mut state = crafting("free-no-potion", "pm-crafter", false);
    assert_eq!(answer_keys(&mut state, &["mode:1"]).error, None);
    assert_eq!(must(state.pending.clone(), "the craft prompt").craft_mode, None);
    let state = any_block(state);
    refused(&state, dealing_four(), "this prompt crafts no Potion");
    let mut activate = dealing_four();
    activate.as_object_mut().expect("an object").remove("frame");
    activate["hats"][0]["hat"] = json!("activate");
    refused(&state, activate, "a Spell takes no Activate");
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
fn r1182000_r1563007_every_potion_preset_is_a_different_offered_verb_valid_within_the_frames_points() {
    for (frame, mode) in [
        (CraftFrame::Potion, CraftMode::Potion),
        (CraftFrame::HugePotion, CraftMode::HugePotion),
    ] {
        let cost = if frame == CraftFrame::Potion {
            CRAFT_POTION_COST
        } else {
            CRAFT_HUGE_POTION_COST
        };
        for seed in 0..50 {
            let mut rng = Rng::new(&format!("potion-preset-{seed}"), 0);
            let offer = must(roll_offer(&mut rng, Some(mode), cost), "a potion offer");
            let offered = craft_potion_presets(&mut rng, frame, &offer);
            assert!(!offered.is_empty(), "{frame} seed {seed}");
            let verbs: IndexSet<CraftVerb> = offered
                .iter()
                .map(|recipe| recipe.hats[0].effects[0].verb)
                .collect();
            assert_eq!(verbs.len(), offered.len(), "no two alike: {offered:?}");
            for recipe in &offered {
                assert_eq!(recipe.frame, Some(frame));
                assert!(
                    validate_recipe(recipe, cost).is_ok(),
                    "{:?}",
                    validate_recipe(recipe, cost)
                );
                assert!(offer.verbs.contains(&recipe.hats[0].effects[0].verb));
                assert!(recipe_points(recipe) <= points_budget(Some(frame), cost), "{recipe:?}");
            }
        }
    }
}

#[test]
fn r1182000_r1563008_a_crafted_potion_survives_a_json_round_trip_and_casts_the_same() {
    let state = any_block(crafting("potion-trip", "pm-potion-crafter", false));
    let state = answered(&state, dealing_four()).state;
    let potion = crafted(&state).id;
    let cast = action(json!({ "type": "play", "playerId": "p1", "instanceId": potion, "nonce": "trip-1" }));
    let live = reduce(&state, &cast);
    let tripped = reduce(&round_trip(&state), &cast);
    assert_eq!(live.error, None);
    assert_eq!(tripped.error, None);
    assert_eq!(
        tripped.state.players[P2].hero.health,
        HERO_HEALTH - 4,
        "the rebuilt cast deals its 4"
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
fn r1182001_r1563008_a_huge_potion_costs_3_with_five_times_a_potions_points() {
    let state = crafting("huge-open", "pm-huge-potion-crafter", false);
    let pending = open_as(&state, PromptKind::Craft, P1);
    assert_eq!(pending.budget, Some(CRAFT_HUGE_POTION_COST));
    assert_eq!(pending.craft_mode, Some(CraftMode::HugePotion));
    let budget = points_budget(Some(CraftFrame::HugePotion), 3);
    assert_eq!(
        budget,
        CRAFT_HUGE_POTION_BUDGET_MULTIPLE * points_budget(Some(CraftFrame::Potion), 1)
    );
    assert_eq!(budget, 20);
    for recipe in presets(&state) {
        assert_eq!(recipe.frame, Some(CraftFrame::HugePotion));
        assert_eq!(recipe.cost, 3);
    }
    let state = any_block(state);
    // A Spell that deals 10 (10) and draws 3 (9) and heals 1 (1): 20, the whole budget.
    let full = potion(
        "hugePotion",
        3,
        json!([
            { "verb": "damageEnemyHero", "n": 10 },
            { "verb": "draw", "n": 3 },
            { "verb": "healYourHero", "n": 1 },
        ]),
    );
    let preview = craft_preview(&json_as(full.clone()), 3);
    assert!(preview.valid, "{:?}", preview.reasons);
    assert_eq!([preview.points, preview.points_budget], [20, 20]);
    let mut over = full.clone();
    over["hats"][0]["effects"][0]["n"] = json!(10);
    over["hats"][0]["effects"][2]["n"] = json!(3);
    refused(&state, over, "over the budget of 20");
    refused(&state, dealing_four(), "this prompt crafts a HUGE Potion");

    let result = answered(&state, full);
    assert_eq!(result.error, None);
    let card = crafted(&result.state);
    let def = must(result.state.transient_defs.get(&card.def_id), "its definition");
    assert_eq!(def.name, "HUGE Pure Closure Potion");
    assert_eq!(def.cost, CardCost::Fixed(3));
    assert_eq!(def.tags, vec![Tag::Potion]);
    assert_eq!(def.type_, CardType::Spell);
    let after = cast_potion(&result.state, json!([]));
    assert_eq!(after.players[P2].hero.health, HERO_HEALTH - 10);
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
    // R1563007: from scratch draws no offer, and codes from the whole palette.
    assert_eq!(pending.craft_offer, None);
    // The engine still rolls Meditative #17's four presets, from the match rng (R881).
    let mut rng = Rng::new(&seed, cursor);
    assert_eq!(presets(&state), craft_presets(&mut rng, 2, &whole_palette()));
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
    refused(&state, dealing_four(), "this prompt crafts no Potion");
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

#[test]
fn r1563008_r1563009_a_potion_crafted_before_1563_still_compiles_as_it_did() {
    use jackioh_engine::subsystems::craft::{compile_crafted_def, compile_crafted_scripts};

    // A Unit Potion whose Activate gains Armor: no prompt takes it now, but its recipe, kept in an
    // older game's state, still builds the card it built.
    let old: CraftRecipe = json_as(json!({
        "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 2, "health": 2, "keywords": [], "echo": 0,
        "hats": [{ "hat": "activate", "effects": [{ "verb": "gainArmor", "n": 3 }] }],
        "frame": "potion",
    }));
    let def = compile_crafted_def(&old);
    assert_eq!(def.type_, CardType::Unit);
    assert_eq!(def.tags, vec![Tag::Potion]);
    assert!(def.base.text.starts_with("Activate: Tribute this."), "{}", def.base.text);
    let scripts = compile_crafted_scripts(&old);
    assert_eq!(scripts.base.activations.len(), 1);
    assert!(scripts.base.cry.is_none());
}
