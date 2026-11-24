//! Audit: the engine's special-purpose state machines (Fuse, Activate, Starships, Dormant, Evolve,
//! Quests, the hero power, the crafter, ...). Each test asserts the ruled or printed behaviour and fails
//! on the bug it proves.

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

const SCRATCH_PAD: &str = "meditativeplus-026"; // Scratch Pad: True Craft's chain with no offer (R1182002)

/// R880, R883 (`subsystems/craft.rs`): the block table is "R21's keywords plus Armor and Echo, the
/// hooks, the verbs, priced in points", and "a `grantKeyword` costs what its keyword costs". A keyword
/// the table does not price is no block: the validator refuses it on the card's own keyword line
/// ("{kind} is not a craft block"). The "Give a unit a keyword" verb is not held to the table: any
/// keyword kind but Armor passes, and `effect_points` prices an unpriced one at 0. Scratch Pad's prompt
/// has no offer to narrow it (R1182002), so a client's recipe grants Indestructible for no points.
#[test]
fn r883_a_crafted_grant_of_a_keyword_the_block_table_does_not_price_is_refused() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "audit-craft-unpriced-grant",
        "p1": { "hand": [SCRATCH_PAD], "mana": 10 },
        "p2": { "hand": ["core-053"], "library": ["core-001", "core-002"] },
    }));
    s.play(SCRATCH_PAD, json!({}));
    s.answer(json!(["0"]));
    assert_eq!(
        s.state().pending.as_ref().map(|pending| pending.kind),
        Some(PromptKind::Craft)
    );
    // The control: the same keyword on the card's own keyword line is refused as no craft block.
    let mut printed = unit_recipe(0, 1, 1);
    printed["keywords"] = json!([{ "kind": "Indestructible" }]);
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": printed }])),
        "Indestructible is not a craft block",
    );
    // A 1/2 Unit spends the whole budget at cost 0 (2 points); its Cry grants Indestructible on top.
    let mut granting = unit_recipe(0, 1, 2);
    granting["hats"] = json!([{ "hat": "cry", "effects": [{ "verb": "grantKeyword", "keyword": "Indestructible" }] }]);
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": granting }])),
        "cannot be crafted",
    );
}

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
