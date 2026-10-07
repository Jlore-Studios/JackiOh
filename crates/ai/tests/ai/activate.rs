//! Activate abilities in the AI's search (docs/classic-sets.md B3.2, R384). `candidate_actions` puts an
//! `activate` in the same tier as a play and Heroic Power's `activatePower` (R43), round-robin by source
//! instance and ordered by the ability's mana price, so the beam and the lethal solver try abilities as
//! early as plays. Activate N and ♾️ need nothing of their own: the AI re-plans after every action, so a
//! second use is simply the next decision's candidate. Every board below is built from the real Classic
//! cards (`crates/cards/src/scripts/classic/`); one test-only card carries a mana price, which no
//! Classic ability has yet.
//!
//! Port of `packages/ai/test/activate.test.ts`. TS's per-test `{ timeout: PUZZLE_TIMEOUT }` (120 s)
//! has no `cargo test` twin and is dropped. TS registered the test-only card for the whole file at
//! import; the registries' override is per thread here, so every test installs it first
//! (`install_pricey`).

use jackioh_ai::{AiOptions, DecisionReason, action_key, candidate_actions, decide};
use jackioh_engine::testkit::{
    ActionBody, ActionType, ActivationCost, ActivationDecl, ActivationUses, CardDef, CardScripts, GameState,
    Script, create_rng, effects, find_instance_mut, hook, json, json_as, register_catalog_as,
    register_scripts, subsystems,
};

use super::support::{
    AI, HUMAN, in_graveyard, is_legal, on_field, register_cards, run_puzzle, scenario, trace,
};

const PUNISH: &str = "classic-020"; // Field Spell; Activate: deal 2 damage, or a discard, or a delayed destroy
const TURTINATOR: &str = "classic-021"; // Unit 5/4; Activate ♾️: Tribute a Unit, deal its Attack to any target
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

/// A token, so no deck, pool or determinization ever deals it; registered for this file only.
fn install_pricey() {
    register_cards();
    let mut catalog = (*jackioh_cards::CATALOG).clone();
    catalog.insert(PRICEY.to_string(), pricey_def());
    register_catalog_as(catalog, jackioh_cards::catalog_version());
    let mut scripts = jackioh_cards::scripts_of();
    scripts.insert(
        PRICEY.to_string(),
        CardScripts {
            base: pricey(),
            radiant: pricey(),
        },
    );
    register_scripts(scripts);
}

/// TS `findIndex`: the first index the test holds at, or -1.
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
        let candidates = candidate_actions(state, AI);
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

        let candidates = candidate_actions(s.state(), AI);
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
        // Two Pointmasters (7/1, First Strike) face Turtinator, Gary the Gambler and Jewelosco Scarab
        // (both 1/1). A 1/1 attacking a Pointmaster dies to First Strike first; tributing it to
        // Turtinator pings one dead. Two uses, then the turn ends. Both libraries hold cards, so no
        // fatigue race (R82's auto-ended turns of an empty side) makes throwing Turtinator away a win.
        let run = run_puzzle(
            "activate-infinite",
            json!({
                "p1": { "field": [TURTINATOR, "core-004", "core-007"], "library": ["core-053", "core-030", "core-037"] },
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
        assert!(on_field(&run.end, AI, TURTINATOR), "{}", trace(&run.turn));
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
        let candidates = candidate_actions(&state, AI);
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
        // R752: the alias is no longer listed; the power is the card's Activate ability.
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
