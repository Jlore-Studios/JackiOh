//! Audit: the engine's special-purpose state machines (Fuse, Activate, Starships, Dormant, Evolve,
//! Quests, the hero power, the crafter, ...). Each test asserts the ruled or printed behaviour and fails
//! on the bug it proves.

#![allow(unused_imports, dead_code)]

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const TRUE_CRAFT: &str = "meditative-017"; // True Craft a Card: asks a cost, then a `craft` prompt

/// A Unit recipe with every point in stats and no block the prompt's offer could refuse (R1563007).
fn unit_recipe(cost: i32, attack: i64, health: i64) -> Value {
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

/// R880 (`subsystems/craft.rs`, `validate_recipe`): "it costs {points} points, over the budget", and
/// §9.3: `reduce` refuses an illegal action itself, the state unchanged. A `craft` prompt's answer is a
/// recipe the client writes (`Selection::Craft`), and its Unit stats have no upper bound but the points
/// budget, which `recipe_points` computes as `attack + health - 1` in `i32`. Two huge stats overflow
/// that sum: this build panics inside `reduce`; the shipped builds wrap it to a negative number, which
/// is under every budget, so the recipe is accepted and a 2147483647/2147483647 Unit is minted for (0).
#[test]
fn r880_a_crafted_units_stats_past_the_budget_are_refused_however_large() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "audit-craft-overflow",
        "p1": { "hand": [TRUE_CRAFT], "mana": 10 },
        "p2": { "hand": ["core-053"], "library": ["core-001", "core-002"] },
    }));
    s.play(TRUE_CRAFT, json!({}));
    s.answer(json!(["0"]));
    assert_eq!(
        s.state().pending.as_ref().map(|pending| pending.kind),
        Some(PromptKind::Craft)
    );
    // The control: an ordinary over-budget Unit is refused in the validator's words.
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(0, 10, 10) }])),
        "over the budget",
    );
    // The same refusal is owed to any stats past the budget.
    let max = i64::from(i32::MAX);
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(0, max, max) }])),
        "over the budget",
    );
}

// ---------------------------------------------------------------------------------------------
// PROBES (temporary)
// ---------------------------------------------------------------------------------------------

const HEROIC_POWER: &str = "core-098";
const CORE_SYSTEM: &str = "stinkyscape-117"; // "Bounce all permanents. Overload."

fn heroic(s: &Scenario) -> Option<CardInstance> {
    let state = s.state();
    state.players[P1]
        .backrow
        .iter()
        .flatten()
        .chain(state.players[P1].hand.iter())
        .find(|card| card.def_id == HEROIC_POWER)
        .cloned()
}

/// R3551002: "End Times' cuts add up over its controller's ends of turn ... and are taken back when it
/// is used, so the next use costs (10) again." The cuts are written to the card's tuning (`set_number` on
/// `param:endTimes`), which R78's reset keeps, and the bank that takes them back to `memory`
/// (`END_TIMES_TAKEN_KEY`), which R78's reset clears. So a Heroic Power that leaves the field between a
/// cut and its use (here Core System's "Bounce all permanents") comes back with the cuts and no bank,
/// and no use ever takes them back.
#[test]
fn r3551002_a_used_end_times_costs_10_again_after_the_card_was_bounced_and_replayed() {
    jackioh_cards::register_all();
    let fillers = json!(vec!["core-005"; 12]);
    let price = |s: &Scenario| {
        let card = heroic(s).expect("the Heroic Power");
        jackioh_engine::subsystems::hero_power::power_price(s.state(), &card)
    };
    for n in 0..200 {
        let mut s = scenario(json!({
            "seed": format!("audit-end-times-{n}"),
            "p1": { "hand": [HEROIC_POWER, CORE_SYSTEM, "core-005"], "mana": 10, "library": fillers },
            "p2": { "hand": ["core-005", "core-005"], "library": fillers },
        }));
        // No automatic turn end (R345), so an end of turn is always this test's own action.
        s.act(ActionBody::SetAutoEndTurn { enabled: false }, P1, "p1 ends its own turns");
        s.act(ActionBody::SetAutoEndTurn { enabled: false }, P2, "p2 ends its own turns");
        s.play(HEROIC_POWER, json!({}));
        if !s.prompt_modes().contains(&"endTimes".to_string()) {
            continue;
        }
        s.answer(json!(["endTimes"]));
        assert_eq!(price(&s), Some(config::END_TIMES_COST));
        // Three of p1's ends of turn take 3 off, banked for the use to take back.
        for _ in 0..3 {
            s.end_turn();
            s.end_turn();
        }
        assert_eq!(price(&s), Some(config::END_TIMES_COST - 3));
        // The card leaves the field and is played again; its Cry offers End Times again on this seed.
        s.play(CORE_SYSTEM, json!({}));
        s.play(HEROIC_POWER, json!({}));
        if !s.prompt_modes().contains(&"endTimes".to_string()) {
            continue;
        }
        s.answer(json!(["endTimes"]));
        // Wait for a turn of p1's on which the price is within their mana, and use it.
        for _ in 0..40 {
            let affordable = price(&s).unwrap_or(i32::MAX) <= s.state().players[P1].mana.current;
            if s.state().active == P1 && affordable {
                break;
            }
            s.end_turn();
        }
        let card = heroic(&s).expect("the Heroic Power");
        let uses = s.activations_of(card.id.as_str());
        assert_eq!(uses.len(), 1, "End Times is usable: {uses:?}");
        s.act(uses[0].clone(), P1, "use End Times");
        let answers = s.offered(P1);
        s.act(answers[0].clone(), P1, "Discover a Terminus card");
        assert_eq!(s.state().active, P1, "no end of turn since the use");
        assert_eq!(
            price(&s),
            Some(config::END_TIMES_COST),
            "a used End Times costs (10) again (R3551002)"
        );
        return;
    }
    panic!("no seed offered End Times on both plays");
}

fn lcg(x: u64) -> u64 {
    x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

/// A random walk from `setup`: at each step one of both seats' legal actions, applied through `reduce`.
/// Returns what went wrong, if anything: a listed action refused, a panic, or an invariant finding.
fn walk(name: &str, setup: Value, salt: u64, steps: usize) -> Option<String> {
    let s = scenario(setup);
    let mut state = s.state().clone();
    let mut monitor = create_invariant_monitor(&state);
    let mut x = lcg(salt ^ 0x9e37_79b9_7f4a_7c15);
    let mut trail: Vec<String> = Vec::new();
    for step in 0..steps {
        if state.result.is_some() {
            break;
        }
        let mut all: Vec<(PlayerId, ActionBody)> = Vec::new();
        for player in [P1, P2] {
            let listed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| legal_actions(&state, player)));
            let Ok(listed) = listed else {
                return Some(format!("{name}/{salt} step {step}: legal_actions panicked; trail {trail:?}"));
            };
            for action in listed {
                if matches!(
                    action,
                    ActionBody::Concede
                        | ActionBody::OfferDraw
                        | ActionBody::AnswerDraw { .. }
                        | ActionBody::Emote { .. }
                        | ActionBody::Mulligan { .. }
                ) {
                    continue;
                }
                all.push((player, action));
            }
        }
        if all.is_empty() {
            break;
        }
        x = lcg(x);
        let others: Vec<&(PlayerId, ActionBody)> =
            all.iter().filter(|(_, action)| !matches!(action, ActionBody::EndTurn)).collect();
        let (player, body) = if !others.is_empty() && (x >> 40) % 7 != 0 {
            x = lcg(x);
            others[((x >> 33) as usize) % others.len()].clone()
        } else {
            x = lcg(x);
            all[((x >> 33) as usize) % all.len()].clone()
        };
        let text = serde_json::to_string(&body).unwrap_or_default();
        trail.push(format!("{}:{}", player.as_str(), text));
        if trail.len() > 12 {
            trail.remove(0);
        }
        let before: Vec<String> = monitor
            .before(&state, player, &body)
            .into_iter()
            .filter(|finding| !finding.starts_with("I3"))
            .collect();
        if !before.is_empty() {
            return Some(format!("{name}/{salt} step {step}: before {before:?}; trail {trail:?}"));
        }
        let action = Action::new(body, player, format!("w{step}"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&state, &action)));
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                let message = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
                    .unwrap_or_default();
                return Some(format!("{name}/{salt} step {step}: PANIC {message}; trail {trail:?}"));
            }
        };
        if let Some(error) = result.error {
            return Some(format!("{name}/{salt} step {step}: REFUSED {error}; trail {trail:?}"));
        }
        let after: Vec<String> = monitor
            .after(&result.events, &result.state)
            .into_iter()
            .filter(|finding| !finding.starts_with("I3"))
            .collect();
        if !after.is_empty() {
            return Some(format!("{name}/{salt} step {step}: after {after:?}; trail {trail:?}"));
        }
        state = result.state;
    }
    None
}

fn boards() -> Vec<(&'static str, Value)> {
    let ships = json!({
        "p1": {
            "field": ["stinkyscapeplus-157", "stinkyscape-049", "stinkyscape-101", "stinkyscape-040"],
            "backrow": ["core-098", "stinkyscape-125"],
            "hand": ["stinkyscape-137", "stinkyscape-176", "stinkyscapeplus-155", "stinkyscapeplus-080",
                     "stinkyscape-144", "core-016", "stinkyscape-151", "stinkyscape-133"],
            "library": ["stinkyscape-090", "stinkyscape-106", "stinkyscape-136", "stinkyscapeplus-154",
                        "stinkyscapeplus-156", "stinkyscape-117", "core-016", "stinkyscape-052",
                        "stinkyscape-050", "stinkyscape-062", "stinkyscape-134", "stinkyscape-017"],
            "mana": 10,
        },
        "p2": {
            "field": ["stinkyscape-052", "stinkyscape-050", "stinkyscape-062", "stinkyscape-017"],
            "backrow": ["stinkyscape-131"],
            "hand": ["core-016", "stinkyscape-151", "stinkyscape-176", "stinkyscape-117", "core-052",
                     "stinkyscape-019", "classicplus-035"],
            "library": ["stinkyscape-090", "stinkyscape-106", "stinkyscape-133", "stinkyscapeplus-157",
                        "stinkyscape-137", "core-016", "stinkyscape-049", "stinkyscape-101",
                        "stinkyscapeplus-155", "stinkyscape-144", "core-052", "stinkyscape-019"],
            "mana": 10,
        },
    });
    let quests = json!({
        "p1": {
            "field": ["meditative-047", "stinkyscapeplus-161", "classicplus-030"],
            "backrow": ["stinkyscape-112", "classic-090", "stinkyscape-007", "core-098"],
            "hand": ["classicplus-031", "meditative-075", "stinkyscape-006", "core-016", "stinkyscape-117",
                     "core-005", "core-099", "meditative-017"],
            "library": ["core-005", "core-016", "classicplus-074", "stinkyscape-114", "core-085",
                        "meditativeplus-026", "stinkyscapeplus-087", "stinkyscapeplus-075", "core-052",
                        "classicplus-035", "meditative-042", "stinkyscape-148"],
            "mana": 10,
        },
        "p2": {
            "field": ["meditative-047", "stinkyscapeplus-161", "stinkyscape-114"],
            "backrow": ["classicplus-074", "core-085", "meditative-075", "stinkyscape-112"],
            "hand": ["core-016", "stinkyscape-117", "core-052", "classicplus-035", "core-005", "core-098",
                     "classic-090", "meditative-042"],
            "library": ["core-005", "core-016", "classicplus-031", "stinkyscape-007", "core-099",
                        "meditative-017", "stinkyscapeplus-087", "stinkyscapeplus-075", "stinkyscape-006",
                        "stinkyscape-148", "meditativeplus-026", "classicplus-030"],
            "mana": 10,
        },
    });
    vec![("ships", ships), ("quests", quests)]
}

#[test]
fn probe_walks() {
    jackioh_cards::register_all();
    let mut found: Vec<String> = Vec::new();
    for (name, setup) in boards() {
        for salt in 0..10u64 {
            let mut setup = setup.clone();
            setup["seed"] = json!(format!("audit-walk-{name}-{salt}"));
            if let Some(finding) = walk(name, setup, salt, 220) {
                println!("FOUND {finding}\n");
                found.push(finding);
            }
        }
    }
    println!("{} findings", found.len());
}
