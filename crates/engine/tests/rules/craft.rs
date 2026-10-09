//! ME-CRAFT (Meditative #17 True Craft a Card, R880–R883) through `pm-crafter`, the fixture
//! Spell running the card's own chain (`fixtures/craft.rs`): a `number` prompt for the cost, then
//! a `craft` prompt, then the mint. The card itself is proved again in `crates/cards`.

use jackioh_engine::effects::degrade;
use jackioh_engine::subsystems::craft::{
    compile_crafted_def, craft_id, craft_presets, craft_preview, recipe_loc, recipe_points, validate_recipe,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::craft::register_craft_fixtures;
use crate::rules::fixtures::harness::{setup_catalog, sink_for};
use crate::rules::fixtures::prompt_harness::{
    AnswerResult, Replayable, act, answer_keys, board, cast_now, expect_replays, hand_card, must, open_as,
    round_trip,
};
use crate::rules::fixtures::prompts::register_prompt_fixtures;

/// An `Action` from its JSON, as the socket sends it.
fn action(literal: Value) -> Action {
    json_as(literal)
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

/// A minimal valid Unit recipe at the given cost, all the budget in stats.
fn unit_recipe(cost: i32, attack: i32, health: i32) -> Value {
    json!({
        "cost": cost,
        "type": "Unit",
        "adjective": "Pure",
        "noun": "Closure",
        "attack": attack,
        "health": health,
        "keywords": [],
        "echo": 0,
        "hats": [],
    })
}

/// `pm-crafter` cast with its cost prompt answered: the craft prompt at that budget.
fn crafting(seed: &str, cost_pick: &str) -> GameState {
    let mut state = board(seed);
    register_craft_fixtures();
    cast_now(&mut state, "pm-crafter", P1, false);
    let answered = answer_keys(&mut state, &[cost_pick]);
    assert_eq!(answered.error, None, "the cost answer");
    state
}

/// Answer the open craft prompt with a recipe, as the editor sends it (not an option key).
fn answer_recipe(state: &mut GameState, recipe: Value) -> AnswerResult {
    let pending = must(state.pending.clone(), "a craft prompt");
    let selection: Vec<Selection> = vec![json_as(json!({ "pick": "craft", "recipe": recipe }))];
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let error = {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        match answer_prompt(
            &mut sink,
            &AnswerInput {
                player_id: pending.player_id,
                choice_id: pending.id.clone(),
                selection,
            },
        ) {
            Ok(_) => {
                settle(&mut sink, SettleOptions::default());
                None
            }
            Err(error) => Some(error.message),
        }
    };
    state.rng_cursor = rng.cursor();
    AnswerResult { events, error }
}

/// The recipe types of the craft prompt's options, in order.
fn option_types(state: &GameState) -> Vec<String> {
    must(state.pending.clone(), "a craft prompt")
        .options
        .iter()
        .map(|option| match &option.selection {
            Selection::Craft { recipe } => recipe.type_.as_str().to_string(),
            other => panic!("a preset is a recipe, not {other:?}"),
        })
        .collect()
}

#[test]
fn r880_the_crafter_asks_a_cost_0_to_4_then_a_craft_prompt_at_that_cost() {
    let mut state = board("craft-cost");
    register_craft_fixtures();
    cast_now(&mut state, "pm-crafter", P1, false);
    let pending = open_as(&state, PromptKind::Number, P1);
    let keys: Vec<&str> = pending.options.iter().map(|option| option.key.as_str()).collect();
    assert_eq!(keys, vec!["mode:0", "mode:1", "mode:2", "mode:3", "mode:4"]);

    let answered = answer_keys(&mut state, &["mode:3"]);
    assert_eq!(answered.error, None);
    let pending = open_as(&state, PromptKind::Craft, P1);
    assert_eq!(pending.budget, Some(3));
    assert_eq!(option_types(&state), vec!["Unit", "Spell", "Field Spell", "Trap"]);
}

#[test]
fn r880_an_invalid_recipe_is_refused_and_the_prompt_stays_open() {
    // (case, recipe, what the refusal says)
    let over_points = unit_recipe(2, 10, 10);
    let over_lines = json!({
        "cost": 4, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [
            { "hat": "cry", "effects": [
                { "verb": "damageTarget", "n": 1 }, { "verb": "damageTarget", "n": 1 }, { "verb": "damageTarget", "n": 1 },
            ] },
            { "hat": "death", "effects": [
                { "verb": "damageTarget", "n": 1 }, { "verb": "damageTarget", "n": 1 }, { "verb": "damageTarget", "n": 1 },
            ] },
            { "hat": "startOfTurn", "effects": [
                { "verb": "damageTarget", "n": 1 }, { "verb": "damageTarget", "n": 1 },
            ] },
        ],
    });
    let too_many = json!({
        "cost": 4, "type": "Spell", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "whenCast", "effects": [
            { "verb": "gainArmor", "n": 1 }, { "verb": "gainArmor", "n": 1 },
            { "verb": "gainArmor", "n": 1 }, { "verb": "gainArmor", "n": 1 },
            { "verb": "gainArmor", "n": 1 }, { "verb": "gainArmor", "n": 1 },
            { "verb": "gainArmor", "n": 1 }, { "verb": "gainArmor", "n": 1 },
            { "verb": "gainArmor", "n": 1 },
        ] }],
    });
    let wrong_for_type = json!({
        "cost": 1, "type": "Trap", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "cry", "effects": [{ "verb": "damageEnemyHero", "n": 1 }] }],
    });
    for (case, recipe, reason) in [
        ("over the points", over_points, "over the budget"),
        ("over the lines", over_lines, "over the budget"),
        ("too many effects", too_many, "more than 8"),
        ("wrong for its type", wrong_for_type, "takes no"),
    ] {
        let mut state = crafting(&format!("craft-refused-{case}"), "mode:2");
        let before = hash_state(&state);
        let refused = answer_recipe(&mut state, recipe);
        let error = refused.error.expect("the answer is refused");
        assert!(error.contains("cannot be crafted"), "{case}: {error}");
        assert!(error.contains(reason), "{case}: {error}");
        assert_eq!(open_as(&state, PromptKind::Craft, P1).budget, Some(2));
        assert_eq!(hash_state(&state), before, "{case}: the state is unchanged");
    }
}

#[test]
fn r880_a_free_recipe_within_budget_is_crafted() {
    let mut state = crafting("craft-free", "mode:0");
    // A 1/1 Unit: 1 point, within the budget of 2.
    let refused = answer_recipe(&mut state, unit_recipe(0, 1, 1));
    assert_eq!(refused.error, None);
    assert_eq!(state.pending, None);
    let card = must(
        state.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the crafted card",
    );
    assert!(state.transient_defs.contains_key(&card.def_id));
}

#[test]
fn r880_the_preview_and_the_reducer_agree() {
    for (recipe, cost, valid) in [
        (unit_recipe(2, 5, 6), 2, true),
        (unit_recipe(0, 1, 1), 0, true),
        (unit_recipe(2, 10, 10), 2, false),
        (unit_recipe(1, 3, 4), 2, false),
    ] {
        let preview = craft_preview(&json_as(recipe.clone()), cost);
        assert_eq!(preview.valid, valid, "{recipe}");
        let mut state = crafting(&format!("craft-agree-{cost}-{valid}"), &format!("mode:{cost}"));
        let refused = answer_recipe(&mut state, recipe);
        assert_eq!(refused.error.is_none(), valid, "{preview:?}");
        assert_eq!(preview.reasons.is_empty(), valid);
    }
}

#[test]
fn r880_the_opponent_sees_only_an_open_prompt_then_a_hidden_card() {
    let mut state = crafting("craft-hidden", "mode:1");
    let recipe = unit_recipe(1, 3, 4);
    let refused = answer_recipe(&mut state, recipe);
    assert_eq!(refused.error, None);

    let view = view_for(&state, P2);
    assert!(matches!(view.pending, Some(PendingView::Elsewhere(_))));
    // p1's hand is a count to p2, and the crafted definition travels nowhere it is readable.
    let held = state.players.p1.hand.len() as i32;
    assert!(matches!(view.opponent.hand, HandView::Count { count } if count == held));
    assert!(
        view.defs
            .as_ref()
            .is_none_or(|defs| defs.keys().all(|id| !id.starts_with("craft:"))),
        "no crafted def leaks: {:?}",
        view.defs.as_ref().map(|defs| defs.keys().collect::<Vec<_>>())
    );
}

#[test]
fn r881_every_preset_is_valid_at_every_cost() {
    for cost in 0..=4 {
        for seed in 0..50 {
            let mut rng = Rng::new(&format!("craft-preset-{cost}-{seed}"), 0);
            let presets = craft_presets(&mut rng, cost);
            assert_eq!(
                presets.iter().map(|recipe| recipe.type_).collect::<Vec<_>>(),
                vec![
                    CardType::Unit,
                    CardType::Spell,
                    CardType::FieldSpell,
                    CardType::Trap
                ],
                "cost {cost} seed {seed}"
            );
            for recipe in &presets {
                assert_eq!(recipe.cost, cost);
                assert!(
                    validate_recipe(recipe, cost).is_ok(),
                    "cost {cost} seed {seed}: {:?}",
                    validate_recipe(recipe, cost)
                );
            }
        }
    }
}

#[test]
fn r881_a_timeout_and_the_random_policy_answer_with_a_preset() {
    use jackioh_engine::subsystems::ai_policy::choose_action;

    let mut state = crafting("craft-policy", "mode:2");
    let offered: Vec<CraftRecipe> = must(state.pending.clone(), "a craft prompt")
        .options
        .iter()
        .map(|option| match &option.selection {
            Selection::Craft { recipe } => recipe.clone(),
            other => panic!("a preset is a recipe, not {other:?}"),
        })
        .collect();

    let mut rng = Rng::new("craft-policy-pick", 0);
    let picked = choose_action(&state, P1, &mut rng).expect("the policy answers");
    match picked {
        ActionBody::Answer { selection, .. } => {
            assert_eq!(selection.len(), 1);
            match &selection[0] {
                Selection::Craft { recipe } => assert!(offered.contains(recipe)),
                other => panic!("a preset, not {other:?}"),
            }
        }
        other => panic!("an answer, not {other:?}"),
    }

    let timed = reduce(
        &state,
        &action(json!({ "type": "timeout", "playerId": "p1", "nonce": "craft-t1" })),
    );
    assert_eq!(timed.error, None);
    assert_eq!(timed.state.pending, None);
    let card = must(
        timed.state.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the timeout's crafted card",
    );
    let def = must(timed.state.transient_defs.get(&card.def_id), "its definition");
    assert!(offered.contains(must(def.craft.as_ref(), "its recipe")));
}

/// `replayable` with the craft fixtures registered before the decks are built: the opening
/// hand holds `pm-crafter`, and the fold rebuilds the very same state.
fn replayable_with_crafter(seed: &str) -> Replayable {
    setup_catalog();
    register_prompt_fixtures();
    register_craft_fixtures();
    let mut first = vanilla_deck(DECK_SIZE - 1, 1);
    first.extend(["pm-crafter".to_string()]);
    let second = vanilla_deck(DECK_SIZE, 21);
    let decks = (first, second);
    let mut log: Vec<Action> = Vec::new();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameOptions::default()
    }))
    .state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "playerId": "p1", "keep": keep }),
        Some(&mut log),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "playerId": "p2", "keep": keep }),
        Some(&mut log),
    );
    Replayable { state, log, decks }
}

#[test]
fn r882_a_crafted_card_survives_a_json_round_trip_and_replays_exactly() {
    let game = replayable_with_crafter("craft-replay");
    let decks = game.decks.clone();
    let mut log = game.log;
    let mut state = game.state;

    let crafter = hand_card(&state, P1, "pm-crafter").id.clone();
    state = act(
        &state,
        json!({ "type": "play", "playerId": "p1", "instanceId": crafter }),
        Some(&mut log),
    );
    let cost_id = must(state.pending.clone(), "the cost prompt").id.clone();
    state = act(
        &state,
        json!({ "type": "answer", "playerId": "p1", "choiceId": cost_id, "selection": [{ "pick": "mode", "option": "1" }] }),
        Some(&mut log),
    );
    // A 3/4 Unit with a Cry dealing 3: 3+4-1+3 = 9 points, over 7 — so a 2/3 dealing 2:
    // 2+3-1+2 = 6, within 7.
    let recipe = json!({
        "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 2, "health": 3, "keywords": [], "echo": 0,
        "hats": [{ "hat": "cry", "effects": [{ "verb": "damageEnemyHero", "n": 2 }] }],
    });
    let craft_id = must(state.pending.clone(), "the craft prompt").id.clone();
    state = act(
        &state,
        json!({ "type": "answer", "playerId": "p1", "choiceId": craft_id, "selection": [{ "pick": "craft", "recipe": recipe }] }),
        Some(&mut log),
    );

    // The card is played after the round trip, and the fold rebuilds the live state exactly.
    let tripped = round_trip(&state);
    let crafted = must(
        tripped.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the crafted card",
    )
    .clone();
    let live = act(
        &tripped,
        json!({ "type": "play", "playerId": "p1", "instanceId": crafted.id, "zone": { "row": "units", "lane": 1 } }),
        Some(&mut log),
    );
    expect_replays("craft-replay", &decks, &log, &live);
}

#[test]
fn r882_the_crafted_definition_is_meditative_mythic_not_a_token_with_counted_loc() {
    let recipe: CraftRecipe = json_as(json!({
        "cost": 1, "type": "Spell", "adjective": "Seeded", "noun": "Reducer",
        "attack": 0, "health": 1, "keywords": [], "echo": 2,
        "hats": [{ "hat": "whenCast", "effects": [
            { "verb": "draw", "n": 2 }, { "verb": "healYourHero", "n": 2 },
        ] }],
    }));
    let def = compile_crafted_def(&recipe);
    assert_eq!(def.id, craft_id(&recipe));
    assert_eq!(def.index, craft_id(&recipe));
    assert_eq!(def.name, "Seeded Reducer");
    assert_eq!(def.set, SetName::Meditative);
    assert_eq!(def.rarity, Rarity::Mythic);
    assert!(!def.token);
    assert_eq!(def.cost, CardCost::Fixed(1));
    assert_eq!(def.craft.as_ref(), Some(&recipe));
    // The frame, When cast, and the two verbs: 3 + 2 + 1 + 1.
    assert_eq!(recipe_loc(&recipe), 7);
    assert_eq!(def.loc, Some(7));
    assert_eq!(recipe_points(&recipe), 7);
    assert_eq!(def.params.as_ref().map(|params| params.len()), Some(2));
    assert_eq!(def.refs, None);
}

#[test]
fn r882_a_nerf_moves_a_crafted_param() {
    let mut state = crafting("craft-nerf", "mode:4");
    // A cost-4 spell dealing 10: the cost row cannot apply (never above 4), the spell has no
    // stats, keyword or X rows, so a Degrade moves b1, and only b1.
    let recipe = json!({
        "cost": 4, "type": "Spell", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "whenCast", "effects": [{ "verb": "damageEnemyHero", "n": 10 }] }],
    });
    let refused = answer_recipe(&mut state, recipe);
    assert_eq!(refused.error, None);
    let before = hand_card(&state, P1, &craft_def_id(&state)).def_id.clone();
    // A Degrade outside a card, the way `prompts_v020.rs`'s `opened` applies its effect.
    let mut sink = sink_for(&mut state);
    let mut ctx = make_context(&mut sink, None, by(P1));
    let nerf = degrade(json_as(json!({ "scope": { "zones": ["hand"] }, "random": 1 })));
    (nerf.apply)(&mut ctx);
    let card = must(
        state.players[P1].hand.iter().find(|card| card.def_id == before),
        "the crafted card",
    );
    let tuning = must(card.tuning.clone(), "a change");
    assert!(
        tuning.numbers.as_ref().is_some_and(|numbers| !numbers.is_empty()),
        "a Nerf moves the crafted param: {tuning:?}"
    );
}

/// The crafted definition's id, read off the hand.
fn craft_def_id(state: &GameState) -> String {
    must(
        state.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the crafted card",
    )
    .def_id
    .clone()
}

#[test]
fn r882_a_full_hand_burns_the_crafted_card() {
    let mut state = board("craft-burn");
    register_craft_fixtures();
    cast_now(&mut state, "pm-crafter", P1, false);
    // A full hand of ten: the crafted card has nowhere to go.
    while state.players[P1].hand.len() < 10 {
        let mut filler = new_instance(&mut state, "pm-crafter", P1, Zone::Hand { player: P1 });
        filler.id = format!("filler-{}", state.players[P1].hand.len());
        state.players[P1].hand.push(filler);
    }
    let answered = answer_keys(&mut state, &["mode:1"]);
    assert_eq!(answered.error, None);
    let refused = answer_recipe(&mut state, unit_recipe(1, 3, 4));
    assert_eq!(refused.error, None);
    assert_eq!(state.players[P1].hand.len(), 10);
    assert!(
        state.players[P1]
            .hand
            .iter()
            .all(|card| !card.def_id.starts_with("craft:")),
        "a full hand burns the crafted card"
    );
}

#[test]
fn r883_stockpiles_recipe_costs_seven_points_and_seven_lines() {
    // Stockpile's recipe: a cost-1 Spell drawing 2 and healing its hero 2.
    let recipe: CraftRecipe = json_as(json!({
        "cost": 1, "type": "Spell", "adjective": "Seeded", "noun": "Reducer",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "whenCast", "effects": [
            { "verb": "draw", "n": 2 }, { "verb": "healYourHero", "n": 2 },
        ] }],
    }));
    assert_eq!(recipe_points(&recipe), 3 * 2 + (2 + 1) / 2);
    assert_eq!(recipe_points(&recipe), 7);
    assert_eq!(recipe_loc(&recipe), 3 + 2 + 1 + 1);
    assert_eq!(recipe_loc(&recipe), 7);
    assert!(validate_recipe(&recipe, 1).is_ok());
    let def = compile_crafted_def(&recipe);
    assert_eq!(def.base.text, "Draw {b1|card|cards}. Heal your hero {b2}.");
}

#[test]
fn r883_a_targeted_block_needs_a_cry_or_when_cast_hat() {
    // Aimed from a Death: refused. The same block under a Cry: crafted.
    let aimed = json!({
        "cost": 2, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 4, "health": 5, "keywords": [], "echo": 0,
        "hats": [{ "hat": "death", "effects": [{ "verb": "damageTarget", "n": 3 }] }],
    });
    let errors = validate_recipe(&json_as(aimed.clone()), 2).expect_err("aimed from a Death");
    assert!(
        errors.iter().any(|reason| reason.contains("Cry or When cast")),
        "{errors:?}"
    );
    let mut state = crafting("craft-aimed", "mode:2");
    let refused = answer_recipe(&mut state, aimed);
    assert!(
        refused
            .error
            .is_some_and(|error| error.contains("Cry or When cast"))
    );

    let mut state = crafting("craft-cry", "mode:2");
    let under_cry = json!({
        "cost": 2, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 4, "health": 5, "keywords": [], "echo": 0,
        "hats": [{ "hat": "cry", "effects": [{ "verb": "damageTarget", "n": 3 }] }],
    });
    let refused = answer_recipe(&mut state, under_cry);
    assert_eq!(refused.error, None);
}

#[test]
fn r883_each_hat_compiles_to_its_hook_and_a_reveal_hat_fires_on_its_event() {
    use jackioh_engine::subsystems::craft::compile_crafted_scripts;

    let recipe: CraftRecipe = json_as(json!({
        "cost": 4, "type": "Unit", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [
            { "hat": "cry", "effects": [{ "verb": "damageEnemyHero", "n": 1 }] },
            { "hat": "death", "effects": [{ "verb": "damageEnemyHero", "n": 1 }] },
            { "hat": "startOfTurn", "effects": [{ "verb": "gainArmor", "n": 1 }] },
            { "hat": "endOfTurn", "effects": [{ "verb": "gainArmor", "n": 1 }] },
        ],
    }));
    let scripts = compile_crafted_scripts(&recipe);
    assert!(scripts.base.cry.is_some());
    assert!(scripts.base.death.is_some());
    assert!(scripts.base.start_of_turn.is_some());
    assert!(scripts.base.end_of_turn.is_some());
    assert!(scripts.base.triggers.is_empty());

    let trap: CraftRecipe = json_as(json!({
        "cost": 1, "type": "Trap", "adjective": "Pure", "noun": "Closure",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "opponentPlaysUnit",
            "effects": [{ "verb": "damageEnemyHero", "n": 2 }] }],
    }));
    let scripts = compile_crafted_scripts(&trap);
    assert!(scripts.base.cry.is_none());
    assert_eq!(scripts.base.triggers.len(), 1);
    // Core #41's shape: the trigger fires on the opponent's Unit play.
    assert!(scripts.base.triggers[0].on.contains(&GameEventType::CardPlayed));

    // And behaviorally: a crafted start-of-turn Field Spell pays out on the turn.
    let mut state = crafting("craft-hooks", "mode:1");
    let field = json!({
        "cost": 1, "type": "Field Spell", "adjective": "Seeded", "noun": "Trait",
        "attack": 0, "health": 1, "keywords": [], "echo": 0,
        "hats": [{ "hat": "startOfTurn", "effects": [{ "verb": "damageEnemyHero", "n": 2 }] }],
    });
    let refused = answer_recipe(&mut state, field);
    assert_eq!(refused.error, None);
    let crafted = must(
        state.players[P1]
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:")),
        "the crafted Field Spell",
    )
    .id
    .clone();
    // Played on p1's turn, it pays out at p1's next start of turn.
    state = act(
        &state,
        json!({ "type": "play", "playerId": "p1", "instanceId": crafted, "zone": { "row": "backrow", "lane": 1 } }),
        None,
    );
    state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }), None);
    let p2 = state.players[P2].hero.health;
    state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
    assert_eq!(
        state.players[P2].hero.health,
        p2 - 2,
        "the crafted start-of-turn hook fires"
    );
}
