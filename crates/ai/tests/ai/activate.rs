//! Activate abilities in search (docs/classic-sets.md B3.2, R384). `candidate_actions` puts `activate`
//! in the same tier as plays and Heroic Power (R43), round-robin by source instance and ordered by mana.

use jackioh_ai::{
    AI_BUDGET, AI_DECISION_PRICE_NODES, AI_EVAL, AI_PROMPT_ACTIVATION_PROBE_NODES, ActivationPrice,
    AiOptions, DecisionReason, EvalWeights, GREEDY_EVAL, NextSwing, NodeCounter, Prices, action_key,
    activation_prices, candidate_actions, create_node_counter, decide, decision_prices_at, det_ground,
    evaluate, evaluate_priced, probe_world, prompt_activation_prices, redact,
};
use jackioh_engine::testkit::{
    ActionBody, ActionType, ActivationCost, ActivationDecl, ActivationUses, CardDef, CardInstance,
    CardScripts, GameState, PromptKind, Scenario, Script, create_rng, effects, find_instance,
    find_instance_mut, hook, json, json_as, pick, register_catalog_as, register_scripts, seed_where,
    subsystems,
};

use super::support::{
    AI, HUMAN, act, in_graveyard, is_legal, on_field, register_cards, run_puzzle, scenario, trace,
};

const PUNISH: &str = "classic-020"; // Field Spell; Activate: deal 2 damage, or a discard, or a delayed destroy
const TURTINATOR: &str = "classic-021"; // Unit 5/4; Activate (once a turn): Tribute a Unit, deal its Attack to any target
const HEROIC: &str = "core-098";

/// A test-only Field Spell token whose one ability costs (3): the price `sourceCost` orders by.
const PRICEY: &str = "ai-test-pricey";

fn pricey_def() -> CardDef {
    json_as(json!({
        "id": PRICEY,
        "index": "ai-test-1",
        "name": "Pricey Ability (AI test)",
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Token",
        "token": true,
        "cost": 1,
        "base": { "keywords": [], "text": "Activate: Spend (3): deal 1 damage to the enemy hero." },
        "radiant": { "keywords": [], "text": "Activate: Spend (3): deal 1 damage to the enemy hero." },
    }))
}

fn pricey() -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "zap".to_string(),
            label: "Deal 1 damage to the enemy hero".to_string(),
            uses: ActivationUses::Count(1),
            cost: Some(ActivationCost {
                mana: Some(3),
                ..ActivationCost::default()
            }),
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                ))]
            }),
        }],
        ..Script::default()
    }
}

/// C #21 Turtinator with an Activate ♾️ in place of its once-a-turn one (#831), which no shipped Tribute
/// card has. A token, like `PRICEY`.
const TURTINATOR_INFINITE: &str = "ai-test-turtinator-infinite";

fn turtinator_infinite_def() -> CardDef {
    let mut def = serde_json::to_value(&jackioh_cards::CATALOG[TURTINATOR]).expect("a definition is JSON");
    def["id"] = json!(TURTINATOR_INFINITE);
    def["index"] = json!("ai-test-2");
    def["name"] = json!("Turtinator, Activate ♾️ (AI test)");
    def["rarity"] = json!("Token");
    def["token"] = json!(true);
    for face in ["base", "radiant"] {
        let text = def[face]["text"]
            .as_str()
            .unwrap_or_default()
            .replace("Activate:", "Activate ♾️:");
        def[face]["text"] = json!(text);
    }
    json_as(def)
}

fn turtinator_infinite() -> CardScripts {
    let mut scripts = jackioh_cards::scripts_of()[TURTINATOR].clone();
    for face in [&mut scripts.base, &mut scripts.radiant] {
        for decl in &mut face.activations {
            decl.uses = ActivationUses::Unlimited;
        }
    }
    scripts
}

/// Tokens, so no deck, pool or determinization ever deals them; registered for this file only.
fn install_pricey() {
    register_cards();
    let mut catalog = (*jackioh_cards::CATALOG).clone();
    catalog.insert(PRICEY.to_string(), pricey_def());
    catalog.insert(TURTINATOR_INFINITE.to_string(), turtinator_infinite_def());
    register_catalog_as(catalog, jackioh_cards::catalog_version());
    let mut scripts = jackioh_cards::scripts_of();
    scripts.insert(
        PRICEY.to_string(),
        CardScripts {
            base: pricey(),
            radiant: pricey(),
        },
    );
    scripts.insert(TURTINATOR_INFINITE.to_string(), turtinator_infinite());
    register_scripts(scripts);
}

/// The first index the test holds at, or -1.
fn index_where(candidates: &[ActionBody], test: impl Fn(&ActionBody) -> bool) -> i64 {
    candidates.iter().position(test).map_or(-1, |at| at as i64)
}

fn source_of(action: Option<&ActionBody>) -> Option<String> {
    match action? {
        ActionBody::Play { instance_id, .. }
        | ActionBody::Activate { instance_id, .. }
        | ActionBody::ActivatePower { instance_id, .. } => Some(instance_id.clone()),
        _ => None,
    }
}

mod b3_2_activations_are_first_class_candidates {
    use super::*;

    #[test]
    fn b3_2_candidate_actions_lists_activations_with_the_plays_round_robin_by_source_before_other_attacks_and_switches()
     {
        install_pricey();
        // p1: Mr. Vanilla (1) in hand, Tempo Timmy on the field, The Power to Punish (free ability) in the
        // backrow. p2's Midrange Menace (9/9 Taunt) makes Timmy's only attack a losing one (the late tier).
        let s = scenario(json!({
            "seed": "activate-order",
            "p1": { "hand": ["core-008"], "field": ["core-011"], "backrow": [PUNISH] },
            "p2": { "field": ["core-019"], "hand": ["core-005"] },
        }));
        let state = s.state();
        let candidates = candidate_actions(state, AI, Prices::NONE);
        for action in &candidates {
            assert!(
                is_legal(state, AI, action),
                "{}",
                serde_json::to_string(action).unwrap()
            );
        }

        let first_activate = index_where(&candidates, |action| action.action_type() == ActionType::Activate);
        let first_attack = index_where(&candidates, |action| action.action_type() == ActionType::Attack);
        let first_switch = index_where(&candidates, |action| {
            action.action_type() == ActionType::SwitchPosition
        });
        assert!(first_activate >= 0);
        assert!(first_attack > first_activate);
        assert!(first_switch > first_activate);

        // Round-robin: Mr. Vanilla's first lane, Punish's first choice, Mr. Vanilla's second lane, …
        let types: Vec<ActionType> = candidates.iter().take(3).map(ActionBody::action_type).collect();
        assert_eq!(
            types,
            vec![ActionType::Play, ActionType::Activate, ActionType::Play]
        );
        // Every activation sits in the plays' tier: none after the first attack or switch.
        let last_activate = candidates
            .iter()
            .rposition(|action| action.action_type() == ActionType::Activate)
            .map_or(-1, |at| at as i64);
        assert!(last_activate < first_attack);
        assert!(last_activate < first_switch);
    }

    #[test]
    fn b3_2_a_sources_place_in_each_round_is_its_price_an_abilitys_being_its_mana_cost() {
        install_pricey();
        // Pricey's ability costs (3), Mr. Vanilla (1), Punish's ability nothing: that order, highest first.
        let s = scenario(json!({
            "seed": "activate-price",
            "p1": { "hand": ["core-008"], "backrow": [PRICEY, PUNISH] },
            "p2": { "hand": ["core-005"] },
        }));
        let pricey = s.backrow(AI, 1).expect("setup");
        let punish = s.backrow(AI, 2).expect("setup");
        let vanilla = s.hand(AI).first().cloned().expect("setup");
        let price = subsystems::abilities_of(s.state(), &pricey)
            .first()
            .and_then(|ability| ability.cost)
            .and_then(|cost| cost.mana);
        assert_eq!(price, Some(3));

        let candidates = candidate_actions(s.state(), AI, Prices::NONE);
        let sources: Vec<Option<String>> = (0..3).map(|at| source_of(candidates.get(at))).collect();
        assert_eq!(
            sources,
            vec![
                Some(pricey.id.clone()),
                Some(vanilla.id.clone()),
                Some(punish.id.clone())
            ]
        );
    }

    #[test]
    fn b3_2_the_ai_spends_a_free_ability_on_the_kill_it_is_clearly_best_for() {
        install_pricey();
        // Punish's 2 damage kills p2's Pointmaster (7/1, First Strike), which nothing else of p1's can.
        let run = run_puzzle(
            "activate-punish",
            json!({
                "p1": { "hand": ["core-008"], "backrow": [PUNISH] },
                "p2": { "field": ["core-020"], "hand": ["core-005"] },
            }),
        );
        let activations = run
            .turn
            .actions
            .iter()
            .filter(|action| action.action_type() == ActionType::Activate)
            .count();
        assert_eq!(activations, 1, "{}", trace(&run.turn));
        assert!(in_graveyard(&run.end, HUMAN, "core-020"), "{}", trace(&run.turn));
        assert!(
            run.turn
                .decisions
                .iter()
                .all(|decision| decision.reason != DecisionReason::Fallback)
        );
    }

    #[test]
    fn b3_2_the_lethal_solver_finds_a_lethal_that_only_an_activation_reaches() {
        install_pricey();
        // p2 stands at 5 behind Midrange Menace (9/9 Taunt), so no attack reaches the face; a Turtinator
        // tributes the other Turtinator (never itself, R683) and deals its 5 Attack to p2's hero.
        let s = scenario(json!({
            "seed": "activate-lethal",
            "p1": { "field": [TURTINATOR, TURTINATOR] },
            "p2": { "field": ["core-019"], "health": 5, "hand": ["core-005"] },
        }));
        let decision = decide(
            s.state(),
            AI,
            &mut AiOptions::new(create_rng("activate-lethal", 0)),
        );
        assert_eq!(decision.as_ref().map(|d| d.reason), Some(DecisionReason::Lethal));
        assert_eq!(
            decision.as_ref().map(|d| d.action.action_type()),
            Some(ActionType::Activate)
        );
        assert_eq!(
            decision.as_ref().map(|d| d
                .line
                .iter()
                .any(|action| action.action_type() == ActionType::Attack)),
            Some(false)
        );

        let run = run_puzzle(
            "activate-lethal",
            json!({
                "p1": { "field": [TURTINATOR, TURTINATOR] },
                "p2": { "field": ["core-019"], "health": 5, "hand": ["core-005"] },
            }),
        );
        assert_eq!(
            run.end.result.and_then(|result| result.winner.player()),
            Some(AI),
            "{}",
            trace(&run.turn)
        );
    }

    #[test]
    fn b3_2_an_activate_infinite_ability_is_used_again_in_the_same_turn_while_each_use_is_good_and_the_turn_still_ends()
     {
        install_pricey();
        // Two Pointmasters (7/1, First Strike) face an Activate ♾️ Turtinator, Gary the Gambler and Jewelosco
        // Scarab (both 1/1). A 1/1 attacking a Pointmaster dies to First Strike first; tributing it to
        // Turtinator pings one dead. Two uses, then the turn ends. Both libraries hold cards, so no
        // fatigue race (R82's auto-ended turns of an empty side) makes throwing Turtinator away a win.
        let run = run_puzzle(
            "activate-infinite",
            json!({
                "p1": { "field": [TURTINATOR_INFINITE, "core-004", "core-007"], "library": ["core-053", "core-030", "core-037"] },
                "p2": { "field": ["core-020", "core-020"], "hand": ["core-005"], "library": ["core-053", "core-030", "core-037"] },
            }),
        );
        let turtinator = run.start.players.p1.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned()
            .expect("setup");
        let uses = run
            .turn
            .actions
            .iter()
            .filter(|action| {
                matches!(&action.body, ActionBody::Activate { instance_id, .. } if *instance_id == turtinator.id)
            })
            .count();
        assert!(uses >= 2, "{}", trace(&run.turn));
        let pointmasters = run
            .end
            .players
            .p2
            .graveyard
            .iter()
            .filter(|card| card.def_id == "core-020")
            .count();
        assert_eq!(pointmasters, 2, "{}", trace(&run.turn));
        // The turn ended: p2's turn has begun (or the game is over), well inside play_ai_turn's ceiling.
        assert!(
            run.end.result.is_some() || run.end.turn > run.start.turn,
            "{}",
            trace(&run.turn)
        );
        assert!(
            on_field(&run.end, AI, TURTINATOR_INFINITE),
            "{}",
            trace(&run.turn)
        );
    }
}

mod b3_2_rule_10_heroic_powers_power_is_an_activate_the_ai_decides_like_any_other_r752 {
    use super::*;

    fn heroic_board(seed: &str, p2_health: i32) -> GameState {
        let s = scenario(json!({
            "seed": seed,
            "p1": { "backrow": [HEROIC], "hand": ["core-053"] },
            "p2": { "health": p2_health, "hand": ["core-005"] },
        }));
        let power = s.backrow(AI, 1).expect("setup");
        let mut state = s.state().clone();
        // R754: the rolled power lives on the instance; "burn" is Steady Shot, 2 to the enemy hero for (1).
        find_instance_mut(&mut state, &power.id)
            .expect("setup")
            .memory
            .insert(subsystems::POWER_KEY.to_string(), json!("burn"));
        state
    }

    #[test]
    fn r752_the_power_is_listed_in_the_plays_tier_as_its_activate_and_is_the_lethal_when_it_is_one() {
        install_pricey();
        let state = heroic_board("activate-heroic", 2);
        let power = state.players.p1.backrow[0].clone().expect("setup");
        let reno = state.players.p1.hand.first().cloned().expect("setup");
        let shot = ActionBody::Activate {
            instance_id: power.id.clone(),
            ability: Some("burn".to_string()),
            targets: None,
            modes: None,
            tributes: None,
        };
        // Round-robin as ever: Reno's first lane (3), the power (its X, 1), Reno's second lane.
        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let sources: Vec<Option<String>> = (0..3).map(|at| source_of(candidates.get(at))).collect();
        assert_eq!(
            sources,
            vec![
                Some(reno.id.clone()),
                Some(power.id.clone()),
                Some(reno.id.clone())
            ]
        );
        assert_eq!(action_key(&candidates[1]), action_key(&shot));
        // R752: the alias is not listed; the power is the card's Activate ability.
        assert!(
            !candidates
                .iter()
                .any(|action| action.action_type() == ActionType::ActivatePower)
        );

        // The solver's first lethal may play Reno first; the power is in it either way, and legal now.
        let decision = decide(&state, AI, &mut AiOptions::new(create_rng("activate-heroic", 0)));
        assert_eq!(decision.as_ref().map(|d| d.reason), Some(DecisionReason::Lethal));
        let keys: Vec<String> = decision
            .as_ref()
            .map(|d| d.line.iter().map(action_key).collect())
            .unwrap_or_default();
        assert!(keys.contains(&action_key(&shot)));
        assert!(is_legal(&state, AI, &shot));
    }
}

mod r4197000_heroic_powers_pick_is_priced_by_what_each_power_keeps_giving {
    use super::*;

    /// Mr. Vanilla, so neither side runs out of cards to draw through the probe's two turn ends.
    const LIBRARY: &[&str] = &["core-008"; 4];
    /// The powers whose use opens a Discover (Witness Value, Stitching, Terminus Tricks, End Times), which
    /// a probed use prices at nothing (R2564000): the offer's third power, which the AI cannot afford
    /// after the play either.
    const PROMPTING: &[&str] = &["discover", "stitching", "tricks", "endTimes"];
    /// The enemy hero's health: Die Insect's 8 leaves it at 2, where the hero curve weighs most, and
    /// above 8, so the use does not win the game, which `use_price` prices at nothing.
    const ENEMY_HEALTH: i32 = 10;

    /// The AI plays a Heroic Power with its last crystal under `seed`; its Cry's Discover of a power is
    /// open. The state and the powers offered.
    fn heroic_played(seed: &str) -> (GameState, Vec<String>) {
        register_cards();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [HEROIC], "mana": 1, "library": LIBRARY },
            "p2": { "hand": ["core-008"], "health": ENEMY_HEALTH, "library": LIBRARY },
        }));
        s.play(HEROIC, json!({ "zone": 1 }));
        (s.state().clone(), s.prompt_modes())
    }

    /// The first seed whose offer holds Armor Up, Die Insect and one power whose use opens a Discover.
    fn offering_seed() -> String {
        seed_where("ai-r4197000", |seed| {
            let offered = heroic_played(seed).1;
            ["armor", "insect"]
                .iter()
                .all(|name| offered.iter().any(|power| power == name))
                && offered.iter().any(|power| PROMPTING.contains(&power.as_str()))
        })
    }

    /// The Heroic Power `state` holds in the AI's backrow.
    fn heroic_of(state: &GameState) -> CardInstance {
        state.players[AI]
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == HEROIC)
            .cloned()
            .expect("the Heroic Power stands in the backrow")
    }

    #[test]
    fn r4197000_the_ai_keeps_the_power_worth_more_on_every_turn_after() {
        let seed = offering_seed();
        let (state, offered) = heroic_played(&seed);
        let heroic = heroic_of(&state);
        let pending = state.pending.as_ref().expect("the Cry's Discover is open");
        assert_eq!(pending.player_id, AI);
        assert_eq!(pending.resume.instance_id.as_deref(), Some(heroic.id.as_str()));
        assert_eq!(
            state.players[AI].mana.current, 0,
            "no crystal is left to spend this turn"
        );

        let decision = decide(&state, AI, &mut AiOptions::new(create_rng(&seed, 0))).expect("a decision");
        assert!(
            matches!(decision.action, ActionBody::Answer { .. }),
            "{:?}",
            decision.action
        );
        assert!(decision.stats.nodes <= AI_BUDGET.nodes);
        let after = act(&state, AI, &decision.action);
        let kept = find_instance(&after, &heroic.id).expect("the Heroic Power is still in play");
        assert_eq!(
            kept.memory.get(subsystems::POWER_KEY),
            Some(&json!("insect")),
            "offered {offered:?}, the line {:?}",
            decision.line.iter().map(action_key).collect::<Vec<_>>()
        );
    }

    /// The price `prices` holds for the AI's Heroic Power with `power`.
    fn heroic_price(prices: &[ActivationPrice], power: &str) -> Option<f64> {
        prices
            .iter()
            .find(|entry| {
                entry.def_id == HEROIC && entry.controller == AI && entry.power.as_deref() == Some(power)
            })
            .map(|entry| entry.price)
    }

    #[test]
    fn r4197000_each_offered_power_is_priced_by_one_use_and_the_probe_keeps_its_cap() {
        let seed = offering_seed();
        let (state, offered) = heroic_played(&seed);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let priced = prompt_activation_prices(&state, AI, &counter);
        assert_eq!(priced.len(), 2, "{priced:?}");
        assert!(
            priced
                .iter()
                .all(|entry| entry.def_id == HEROIC && entry.controller == AI)
        );
        let insect = heroic_price(&priced, "insect").expect("Die Insect is priced");
        let armor = heroic_price(&priced, "armor").expect("Armor Up is priced");
        assert!(insect > armor && armor > 0.0, "insect {insect}, armor {armor}");
        // The third power's use opens a Discover, which a probed use prices at nothing (R2564000).
        let third = offered
            .iter()
            .find(|power| PROMPTING.contains(&power.as_str()))
            .expect("a power whose use opens a Discover");
        assert_eq!(heroic_price(&priced, third), None);
        assert!(
            (1..=AI_PROMPT_ACTIVATION_PROBE_NODES).contains(&counter.used()),
            "{} nodes",
            counter.used()
        );

        // An answer the nodes do not reach prices nothing.
        let short = create_node_counter(1, None);
        assert_eq!(prompt_activation_prices(&state, AI, &short), vec![]);
        assert!(short.used() <= 1);

        // The prompt is the AI's, so the other seat's decision prices nothing and spends nothing.
        let human = create_node_counter(AI_BUDGET.nodes, None);
        assert_eq!(prompt_activation_prices(&state, HUMAN, &human), vec![]);
        assert_eq!(human.used(), 0);
    }

    /// The AI's Heroic Power holding `power` beside a Punish, its turn, no prompt open (R754: the power
    /// lives on the instance, as the r752 module sets it).
    fn heroic_scenario(power: &str) -> Scenario {
        register_cards();
        let mut s = scenario(json!({
            "p1": { "backrow": [HEROIC, PUNISH], "library": LIBRARY },
            "p2": { "hand": ["core-008"], "health": ENEMY_HEALTH, "library": LIBRARY },
        }));
        let heroic = heroic_of(s.state());
        find_instance_mut(s.state_mut(), &heroic.id)
            .expect("setup")
            .memory
            .insert(subsystems::POWER_KEY.to_string(), json!(power));
        s
    }

    fn heroic_holding(power: &str) -> GameState {
        heroic_scenario(power).state().clone()
    }

    #[test]
    fn r4197000_a_price_follows_the_power_and_a_powerless_card_keys_as_before() {
        let insect_board = heroic_holding("insect");
        let armor_board = heroic_holding("armor");
        let insect_prices = activation_prices(&insect_board, AI, &create_node_counter(AI_BUDGET.nodes, None));
        let armor_prices = activation_prices(&armor_board, AI, &create_node_counter(AI_BUDGET.nodes, None));
        let insect = heroic_price(&insect_prices, "insect").expect("Die Insect is priced");
        let armor = heroic_price(&armor_prices, "armor").expect("Armor Up is priced");
        assert!(insect > armor && armor > 0.0, "insect {insect}, armor {armor}");
        assert_eq!(heroic_price(&insect_prices, "armor"), None);
        assert_ne!(insect_prices, armor_prices);
        // A card with no power is keyed as before: its definition, its face and its controller.
        let punish = insect_prices
            .iter()
            .find(|entry| entry.def_id == PUNISH)
            .expect("Punish is priced");
        assert_eq!(punish.power, None);
        assert_eq!(punish.controller, AI);
        assert!(punish.price > 0.0);

        // Each board reads the price of the power its card holds, never the other's.
        let prices = Prices {
            activations: insect_prices.clone(),
            ..Prices::default()
        };
        let read = |board: &GameState, w: &EvalWeights| {
            evaluate_priced(board, AI, NextSwing::Enemy, w, &prices)
                - evaluate(board, AI, NextSwing::Enemy, w)
        };
        assert!((read(&armor_board, &AI_EVAL) - AI_EVAL.activation * punish.price).abs() < 1e-9);
        assert!((read(&insect_board, &AI_EVAL) - AI_EVAL.activation * (insect + punish.price)).abs() < 1e-9);
        // The greedy baseline reads no price.
        assert_eq!(GREEDY_EVAL.activation, 0.0);
        assert_eq!(read(&insect_board, &GREEDY_EVAL), 0.0);
    }

    /// The prompt probe prices nothing on `state` and spends no node; the activations a decision there
    /// passes are `activation_prices`' alone, as before R4197000.
    fn prices_as_before(state: &GameState) {
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        assert_eq!(prompt_activation_prices(state, AI, &counter), vec![]);
        assert_eq!(counter.used(), 0);
        let public = redact(state, AI);
        let probe = probe_world(&public, AI, &det_ground(&public, AI));
        let alone = activation_prices(&probe, AI, &create_node_counter(AI_DECISION_PRICE_NODES, None));
        assert_eq!(decision_prices_at(state, AI).activations, alone);
    }

    #[test]
    fn r4197000_other_prompts_and_no_prompt_pass_the_prices_they_passed() {
        // No prompt open: the root's prices, a Heroic Power's keyed by the power it holds.
        let open = heroic_holding("burn");
        assert!(open.pending.is_none());
        prices_as_before(&open);
        assert!(heroic_price(&decision_prices_at(&open, AI).activations, "burn").is_some());

        // A Discover of a card to hand: #72 Reminisce's, a Spell's.
        let mut s = scenario(json!({
            "seed": "decide-not-forced",
            "p1": { "hand": ["core-072"], "graveyard": ["core-008", "core-044", "core-005"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"], "field": ["core-008"] },
        }));
        s.play("core-072", json!({}));
        let discover = s.state().clone();
        assert_eq!(
            discover.pending.as_ref().map(|pending| pending.player_id),
            Some(AI)
        );
        prices_as_before(&discover);
        assert_eq!(decision_prices_at(&discover, AI).activations, vec![]);

        // A target choice with an Activate card on the field: #31 KY's Math Equation's Echo repeat, asked
        // afresh after #79's Twinspell (R3044000).
        let mut s = scenario(json!({
            "seed": "ky-math-echo",
            "p1": { "hand": ["core-031"], "field": ["core-015"], "backrow": ["core-079"], "library": ["core-015", "core-015"], "mana": 10 },
            "p2": { "field": ["core-015"], "library": ["core-015", "core-015"] },
        }));
        s.activate("core-079", json!({}));
        s.play("core-031", pick::targets(pick::hero("p2")));
        let target = s.state().clone();
        let pending = target.pending.as_ref().expect("the repeat's target prompt");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Target, AI));
        prices_as_before(&target);
        assert_eq!(decision_prices_at(&target, AI).activations, vec![]);

        // A Discover of a Unit by a Heroic Power's own use (Witness Value): its card can take a power, so
        // each answer is tried, but none sets one, so nothing is priced, within the cap.
        let mut s = heroic_scenario("discover");
        s.activate(HEROIC, json!({}));
        let witness = s.state().clone();
        assert_eq!(
            witness.pending.as_ref().map(|pending| pending.player_id),
            Some(AI)
        );
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        assert_eq!(prompt_activation_prices(&witness, AI, &counter), vec![]);
        assert!(
            (1..=AI_PROMPT_ACTIVATION_PROBE_NODES).contains(&counter.used()),
            "{} nodes",
            counter.used()
        );
        assert_eq!(decision_prices_at(&witness, AI).activations, vec![]);
    }
}
