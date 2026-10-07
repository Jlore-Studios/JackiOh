//! The prompt kinds patch v0.2.0 added (docs/classic-sets.md B5 E18): `number`, `answer`, `cell`,
//! `reward`, a budgeted `pick`, and a mode prompt held by the player who did not play the card. The AI
//! needs nothing new for them: `legalActions` lists every answer (`prompts.promptAnswers`), the beam
//! scores each one through `reduce`, and `simulate` answers a prompt the other seat holds mid-line. Each
//! test opens a kind for the AI's seat with a test-only continuation (the pattern of answer-key.test.ts,
//! which proves R465's key-stripping and is not repeated here) and checks that `decide` answers within
//! its budget, with reason "prompt", and picks the clearly better option where one is.
//!
//! Port of `packages/ai/test/prompts-v020.test.ts`. TS registered the fixture card for the whole file
//! at import; the testkit's override is per thread (SURFACE §8), so every test calls `install()`
//! first. TS's per-test `{ timeout }` has no `cargo test` twin.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act_with_nonce, clone, in_graveyard, is_legal};

/// Spell (1): the opponent chooses, three times (E18's mode held by the other player).
const PICKLE: &str = "classic-008";

/// The test-only card whose continuations the prompts below resume into: a Field Spell token, so a
/// verb that asks whether its source is a Spell (E35) finds a definition, and no pool ever deals it.
const E18: &str = "ai-test-e18";

fn e18_def() -> CardDef {
    json_as(json!({
        "id": E18,
        "index": "ai-test-3",
        "name": "Prompt Fixture (AI test)",
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Token",
        "token": true,
        "cost": 0,
        "base": { "keywords": [], "text": "Asks a question." },
        "radiant": { "keywords": [], "text": "Asks a question." },
    }))
}

fn damage_enemy(amount: i32) -> Vec<Effect> {
    if amount > 0 {
        vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
    } else {
        Vec::new()
    }
}

fn continuations() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    // The number picked is damage to the enemy hero.
    resume.insert("number", hook(|ctx| damage_enemy(effects::chosen_number(ctx).unwrap_or(0))));
    // The picked cards' costs, summed, are damage to the enemy hero.
    resume.insert(
        "pick",
        hook(|ctx| {
            let state: &GameState = &*ctx.state;
            let total = ctx.targets.iter().fold(0, |sum, selection| match selection {
                Selection::Instance { instance_id } => {
                    sum + find_instance(state, instance_id)
                        .map_or(0, |card| effective_cost(state, card, Default::default()))
                }
                _ => sum,
            });
            damage_enemy(total)
        }),
    );
    // The unit in the picked cell is destroyed, whoever's it is.
    resume.insert(
        "cell",
        hook(|ctx| {
            let cells = effects::chosen_cells(ctx);
            let state: &GameState = &*ctx.state;
            let unit = cells.first().and_then(|cell| card_at(state, cell)).map(|card| card.id.clone());
            match unit {
                None => Vec::new(),
                Some(id) => vec![effects::destroy(json_as(json!({ "target": { "of": "instance", "instanceId": id } })))],
            }
        }),
    );
    // Three rewards: 5 damage to your own hero, nothing, 5 damage to the enemy hero.
    resume.insert(
        "reward",
        hook(|ctx| {
            let picked = effects::chosen_options(ctx).first().cloned();
            match picked.as_deref() {
                Some("hurt-enemy") => damage_enemy(5),
                Some("hurt-self") => vec![effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 5 })))],
                _ => Vec::new(),
            }
        }),
    );
    resume.insert("answer", hook(|_ctx| Vec::new()));
    // Run as the card's controller (the owner), whoever answered: "owner-hurt" hits the owner's hero.
    resume.insert(
        "their-mode",
        hook(|ctx| {
            let picked = effects::chosen_options(ctx).first().cloned();
            match picked.as_deref() {
                Some("owner-hurt") => vec![effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 5 })))],
                Some("chooser-hurt") => damage_enemy(5),
                _ => Vec::new(),
            }
        }),
    );
    Script { resume, ..Script::default() }
}

/// TS's module-level `registerCatalog(…)` and `registerScripts(…)`: the real catalog and scripts plus
/// the fixture card, installed through the testkit's override for this thread.
fn install() {
    jackioh_cards::register_all();
    let mut catalog = registered_catalog().clone();
    catalog.insert(E18.to_string(), e18_def());
    register_catalog_as(catalog, &catalog_version());
    let mut scripts = registered_scripts().clone();
    let script = continuations();
    scripts.insert(E18.to_string(), CardScripts { base: script.clone(), radiant: script });
    register_scripts(scripts);
}

/// TS's `{ rng: createRng(seed) }`: AI_BUDGET, no clock.
fn ai_options(seed: &str) -> AiOptions<'static> {
    AiOptions { rng: create_rng(seed, 0), budget: AI_BUDGET, should_stop: None }
}

/// TS's `{ ...base, ...over }` on two JSON objects (an absent `over` changes nothing).
fn spread(base: Value, over: Option<&Value>) -> Value {
    let mut out = base;
    if let (Some(target), Some(Value::Object(extra))) = (out.as_object_mut(), over) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    out
}

/// `state` with `effect` applied for a card of `controller`'s, as a resolving card would apply it.
fn opened(mut state: GameState, controller: PlayerId, effect: Effect) -> GameState {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = create_rng(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions { controller: Some(controller), ..Default::default() });
        ctx.def_id = Some(E18.to_string());
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    state
}

/// A scenario with libraries on both sides (no fatigue in any line) and `effect` opened for p1's card.
fn asked(seed: &str, effect: Effect, setup: Value, controller: PlayerId) -> GameState {
    install();
    let mut options = spread(json!({ "seed": seed }), Some(&setup));
    options["p1"] = spread(json!({ "library": ["core-053", "core-030"] }), setup.get("p1"));
    options["p2"] = spread(json!({ "hand": ["core-005"], "library": ["core-053", "core-030"] }), setup.get("p2"));
    let state = scenario(options).state().clone();
    opened(state, controller, effect)
}

/// decide on a prompt the AI holds: a legal answer, reason "prompt", within the budget.
fn answered(state: &GameState, seed: &str) -> ActionBody {
    assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));
    let decision = decide(&clone(state), AI, &mut ai_options(seed));
    assert!(decision.is_some());
    let decision = decision.expect("a decision");
    assert_eq!(decision.reason, DecisionReason::Prompt);
    assert!(decision.stats.nodes <= AI_BUDGET.nodes);
    assert_eq!(decision.stats.sim_errors, 0);
    assert!(matches!(decision.action, ActionBody::Answer { .. }));
    assert!(is_legal(state, AI, &decision.action));
    decision.action
}

/// An `answer` action's selection (TS `answered(…).selection`).
fn selection_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Answer { selection, .. } => selection.clone(),
        _ => Vec::new(),
    }
}

fn mode(option: &str) -> Selection {
    Selection::Mode { option: option.to_string() }
}

mod e18_the_ai_answers_the_new_prompt_kinds {
    use super::*;

    /// E18: a `number` prompt whose number is damage to the enemy hero is answered with the largest
    #[test]
    fn e18_a_number_prompt_whose_number_is_damage_to_the_enemy_hero_is_answered_with_the_largest() {
        install();
        let effect = effects::choose_number(json_as(json!({ "step": "number", "from": 0, "to": 5 })));
        let state = asked("e18-number", effect, json!({}), AI);
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Number));
        assert_eq!(selection_of(&answered(&state, "e18-number")), vec![mode("5")]);
    }

    /// E18: a budgeted `pick` takes the most the budget affords
    #[test]
    fn e18_a_budgeted_pick_takes_the_most_the_budget_affords() {
        // Midrange Menace (3), Pointmaster (2), Archivist (2), Mr. Vanilla (1), two picks within (4): the
        // best sets spend all four.
        install();
        let effect = effects::choose_pick(json_as(json!({
            "step": "pick",
            "from": [{ "zone": "graveyard" }],
            "max": 2,
            "budget": 4,
        })));
        let state = asked(
            "e18-pick",
            effect,
            json!({ "p1": { "graveyard": ["core-019", "core-020", "core-030", "core-008"] } }),
            AI,
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Pick));
        assert_eq!(state.pending.as_ref().and_then(|pending| pending.budget), Some(4));
        let picked: Vec<i32> = selection_of(&answered(&state, "e18-pick"))
            .iter()
            .map(|selection| match selection {
                Selection::Instance { instance_id } => {
                    let card = find_instance(&state, instance_id).unwrap_or_else(|| panic!("no such card"));
                    effective_cost(&state, card, Default::default())
                }
                _ => panic!("a pick answers with cards"),
            })
            .collect();
        assert_eq!(picked.iter().sum::<i32>(), 4);
    }

    /// E18: a `cell` prompt picks the one cell where the destroy hits an enemy
    #[test]
    fn e18_a_cell_prompt_picks_the_one_cell_where_the_destroy_hits_an_enemy() {
        // p1's Mr. Vanilla in lane 1 and p2's Pointmaster in lane 3; every other cell is empty.
        install();
        let effect = effects::choose_cell(json_as(json!({ "step": "cell", "cells": { "rows": ["units"] } })));
        let state = asked(
            "e18-cell",
            effect,
            json!({ "p1": { "field": ["core-008"] }, "p2": { "field": [{ "def": "core-020", "lane": 3 }] } }),
            AI,
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Cell));
        assert_eq!(
            selection_of(&answered(&state, "e18-cell")),
            vec![Selection::Zone { player: HUMAN, row: Row::Units, lane: 3 }]
        );
    }

    /// E18: a `reward` prompt takes the reward that hurts the enemy
    #[test]
    fn e18_a_reward_prompt_takes_the_reward_that_hurts_the_enemy() {
        install();
        let rewards = json!([
            { "id": "hurt-self", "label": "Take 5" },
            { "id": "nothing", "label": "Nothing" },
            { "id": "hurt-enemy", "label": "Deal 5" },
        ]);
        let effect = effects::choose_reward(json_as(json!({ "step": "reward", "rewards": rewards })));
        let state = asked("e18-reward", effect, json!({}), AI);
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Reward));
        assert_eq!(selection_of(&answered(&state, "e18-reward")), vec![mode("hurt-enemy")]);
    }

    /// E18: an `answer` prompt gets one of its options
    #[test]
    fn e18_an_answer_prompt_gets_one_of_its_options() {
        install();
        let effect = effects::choose_answer(json_as(json!({
            "step": "answer",
            "statement": "2 + 2 = ?",
            "options": ["3", "4", "5"],
            "correct": 1,
            "shuffle": false,
        })));
        let state = asked("e18-answer", effect, json!({}), AI);
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Answer));
        let options: Vec<Selection> = state
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.selection.clone()).collect())
            .unwrap_or_default();
        let picked = selection_of(&answered(&state, "e18-answer"));
        assert!(picked.first().is_some_and(|first| options.contains(first)));
    }

    /// E18: a mode prompt the opponent's card hands the AI, on the opponent's turn, is answered against the card's owner
    #[test]
    fn e18_a_mode_prompt_the_opponents_card_hands_the_ai_on_the_opponents_turn_is_answered_against_the_cards_owner() {
        // p2 plays a card whose question p1 answers (`by: "enemy"`); the answer runs as p2's.
        install();
        let effect = effects::choose_mode(json_as(json!({
            "step": "their-mode",
            "options": ["chooser-hurt", "owner-hurt"],
            "by": "enemy",
        })));
        let state = asked(
            "e18-their-mode",
            effect,
            json!({ "active": "p2", "turn": 10, "p1": { "hand": ["core-053"] } }),
            HUMAN,
        );
        assert_eq!(state.active, HUMAN);
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        assert_eq!(selection_of(&answered(&state, "e18-their-mode")), vec![mode("owner-hurt")]);
    }

    /// E18: the AI's own play opens mode prompts the opponent holds; the search answers them in simulation and the turn completes
    #[test]
    fn e18_the_ais_own_play_opens_mode_prompts_the_opponent_holds_the_search_answers_them_in_simulation_and_the_turn_completes() {
        // Pickle is the AI's only card: the opponent chooses three times, discarding from their own hand.
        install();
        let mut state = scenario(json!({
            "seed": "e18-pickle",
            "p1": { "hand": [PICKLE], "library": ["core-053", "core-030", "core-037"] },
            "p2": { "hand": ["core-005", "core-011", "core-035"], "library": ["core-053", "core-030", "core-037"] },
        }))
        .state()
        .clone();
        let start = state.turn;

        let first = decide(&clone(&state), AI, &mut ai_options("e18-pickle"));
        assert_eq!(first.as_ref().map(|decision| decision.action.action_type()), Some(ActionType::Play));
        assert_eq!(first.as_ref().map(|decision| decision.stats.sim_errors), Some(0));

        let mut decisions: Vec<Decision> = Vec::new();
        let mut human_answers = 0;
        let mut step = 0;
        while step < 40 && state.result.is_none() && state.turn == start {
            let human_kind = state.pending.as_ref().filter(|pending| pending.player_id == HUMAN).map(|pending| pending.kind);
            if let Some(kind) = human_kind {
                assert!(kind == PromptKind::Mode || kind == PromptKind::Hand);
                let answer = legal_actions(&state, HUMAN)
                    .into_iter()
                    .find(|action| matches!(action, ActionBody::Answer { .. }))
                    .unwrap_or_else(|| panic!("the human's prompt has no answer"));
                state = act_with_nonce(&state, HUMAN, &answer, &format!("pickle-human-{step}"));
                human_answers += 1;
                step += 1;
                continue;
            }
            let turn = play_ai_turn(&state, AI, &mut ai_options(&format!("e18-pickle-{step}")));
            let none_played = turn.actions.is_empty();
            decisions.extend(turn.decisions);
            if none_played {
                break;
            }
            state = turn.state;
            step += 1;
        }

        assert!(human_answers >= 3);
        assert!(in_graveyard(&state, AI, PICKLE));
        assert!(state.result.is_some() || state.turn > start);
        assert!(decisions.iter().all(|decision| decision.reason != DecisionReason::Fallback && decision.stats.sim_errors == 0));
    }
}

mod e18_a_whole_game_with_the_new_prompts {
    use super::*;

    /// E18: playMatch finishes a short AI-vs-greedy game in which E18 prompts open, with no refusal, throw or fallback
    #[test]
    fn e18_play_match_finishes_a_short_ai_vs_greedy_game_in_which_e18_prompts_open_with_no_refusal_throw_or_fallback() {
        // Pickle (mode prompts the opponent holds), Back from the GY (a budgeted pick), Mind Melt (a pick
        // of the opponent's hand) and Ancient Acquisition (no prompt since R684: its returns are random),
        // beside cheap Units: eight cards and a hero of 20, so that the game is short and the prompt cards
        // are drawn. On this seed greedy's Pickle
        // hands the AI three mode prompts on greedy's turn, and the AI's Back from the GY asks a pick.
        install();
        let deck: Vec<String> =
            [PICKLE, "classic-011", "classic-034", "classic-044", "core-008", "core-020", "core-030", "core-011"]
                .iter()
                .map(|id| id.to_string())
                .collect();
        let handicap = Handicap { deck_size: deck.len() as i32, hero_health: AI_TUTORIAL.hero_health, ..HUMAN_HANDICAP };
        let config = MatchConfig {
            seed: "e18-match".to_string(),
            decks: (deck.clone(), deck),
            handicaps: Some(PerPlayerOpt { p1: Some(handicap), p2: Some(handicap) }),
            controllers: PerPlayer { p1: SeatController::Ai { budget: None }, p2: SeatController::Greedy },
            max_actions: None,
        };
        let mut opened: Vec<String> = Vec::new();
        let record = {
            let mut hooks = MatchHooks {
                after_action: Some(Box::new(
                    |_before: &GameState, after: &GameState, _seat: PlayerId, _action: &ActionBody| {
                        let Some(pending) = after.pending.as_ref() else { return };
                        let theirs = pending.kind == PromptKind::Mode && pending.player_id != after.active;
                        let new_kind = [PromptKind::Number, PromptKind::Answer, PromptKind::Cell, PromptKind::Reward, PromptKind::Pick]
                            .contains(&pending.kind);
                        if theirs || new_kind {
                            opened.push(pending.kind.as_str().to_string());
                        }
                    },
                )),
                ..Default::default()
            };
            play_match(&config, &mut hooks)
        };
        assert!(record.result.is_some());
        assert!(record.thrown.is_empty());
        assert!(record.rejected.is_empty());
        assert_eq!(record.fallbacks, 0);
        assert!(opened.iter().any(|kind| kind == "mode"), "{}", opened.join(" "));
        assert!(opened.iter().any(|kind| kind == "pick"), "{}", opened.join(" "));
    }
}
