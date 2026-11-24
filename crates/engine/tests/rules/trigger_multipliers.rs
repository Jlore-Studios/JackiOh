//! ME-TRIG (docs/meditative-set.md M5; R820–R824): the trigger multipliers and "trigger your End of
//! turn effects", proved on fixture cards (`fixtures::trigger_multipliers`).

use jackioh_engine::multipliers::{Multiplied, extra_runs};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use super::fixtures::trigger_multipliers::{
    AIMER, ASKER, BOLT, DEATHS, DIER, DOUBLE, DOUBLE_TWO, EXTRA_UNIT_ONE, EXTRA_UNIT_TWO, FEAR, GIVER, JOINT,
    JOINT_TWO, LEAVER, LIVING, NUKE, PAWN, REBORNER, REBORNS, SPAWNER, TICKER, TRIGGERER, register,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// A board for p1, with a vanilla unit and a stocked library on each side so no turn auto-ends
/// (§2.5) and no draw fatigues; `p1` and `p2` add their own entries.
fn game(p1: Value, p2: Value) -> Scenario {
    register();
    let mut sides = json!({
        "p1": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN] },
        "p2": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN] },
    });
    for (seat, extra) in [("p1", p1), ("p2", p2)] {
        if let Value::Object(entries) = extra {
            for (key, value) in entries {
                let side = sides[seat].as_object_mut().expect("a side is an object");
                match (side.get_mut(&key), value) {
                    (Some(Value::Array(list)), Value::Array(more)) => list.extend(more),
                    (_, value) => {
                        side.insert(key, value);
                    }
                }
            }
        }
    }
    scenario(sides)
}

fn health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn hero(player: PlayerId) -> Value {
    json!({ "pick": "hero", "player": player.as_str() })
}

fn instance(card: &CardInstance) -> Value {
    json!({ "pick": "instance", "instanceId": card.id })
}

fn damage_events_on(s: &Scenario, target_id: &str) -> usize {
    s.events()
        .iter()
        .filter(|event| {
            let event = serde_json::to_value(event).expect("an event serializes");
            event["type"] == "damage" && event["targetId"] == target_id
        })
        .count()
}

fn on_field(s: &Scenario, player: PlayerId, def_id: &str) -> usize {
    jackioh_engine::zones::active_units_of(s.state(), player)
        .into_iter()
        .filter(|card| card.def_id == def_id)
        .count()
}

/// R1107: give `unit` the fixture Death `tm-giver#hit` (1 to the enemy hero), as a grant effect would.
fn grant_hit(s: &mut Scenario, unit: &CardInstance) {
    s.card_mut(unit).grants = Some(vec![Grant {
        grant: format!("{GIVER}#hit"),
        radiant: false,
        params: IndexMap::new(),
    }]);
}

/// p1's Unit in lane 1 (`unit`), under the multiplier `multiplier`, given `tm-giver#hit` and then
/// destroyed by a Bolt: the enemy hero's health after its Deaths ran.
fn granted_death_under(multiplier: &str, unit: &str) -> i32 {
    let mut s = game(
        json!({ "backrow": [multiplier], "field": [unit], "hand": [BOLT] }),
        json!({}),
    );
    let unit = s.unit(P1, 1).expect("p1's unit in lane 1");
    grant_hit(&mut s, &unit);
    s.play(BOLT, json!({ "targets": [instance(&unit)] }));
    s.expect_in_zone(unit, "graveyard");
    health(&s, P2)
}

// R820: the highest multiplier holds

#[test]
fn r820_the_highest_multiplier_holds_and_two_never_add() {
    let mut s = game(
        json!({ "backrow": [JOINT, JOINT_TWO], "field": [TICKER] }),
        json!({}),
    );
    assert_eq!(extra_runs(s.state(), P1, Multiplied::TurnHooks), 2);
    assert_eq!(extra_runs(s.state(), P2, Multiplied::TurnHooks), 0);
    s.end_turn();
    assert_eq!(
        health(&s, P2),
        27,
        "three runs of the Ticker's end of turn, not four"
    );
}

#[test]
fn r820_a_fused_card_gives_its_ingredients_highest_extra() {
    let mut s = game(
        json!({ "field": [EXTRA_UNIT_ONE], "hand": [EXTRA_UNIT_TWO] }),
        json!({}),
    );
    let kept = s.unit(P1, 1).expect("the first unit");
    let added = s.hand(P1)[0].clone();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed.clone(), s.state().rng_cursor);
    {
        let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
        fuse(
            &mut sink,
            FuseArgs {
                ingredients: vec![added],
                target: Some(kept.clone()),
                ..FuseArgs::default()
            },
        )
        .expect("the fusion keeps the unit");
    }
    let fused = s.card(kept.id.as_str()).clone();
    assert_ne!(
        fused.def_id, EXTRA_UNIT_ONE,
        "the kept unit wears the fused definition"
    );
    assert_eq!(extra_runs(s.state(), P1, Multiplied::TurnHooks), 2);
}

// R821: start- and end-of-turn hooks, copy by copy

#[test]
fn r821_end_of_turn_hooks_are_queued_twice() {
    let mut s = game(json!({ "backrow": [JOINT], "field": [TICKER] }), json!({}));
    s.end_turn();
    assert_eq!(health(&s, P2), 28);
}

#[test]
fn r821_start_of_turn_hooks_too() {
    let mut s = game(json!({ "backrow": [JOINT], "field": [TICKER] }), json!({}));
    s.start_turn();
    assert_eq!(health(&s, P2), 28);
}

#[test]
fn r821_a_copy_whose_card_left_fizzles() {
    let mut s = game(json!({ "backrow": [JOINT], "field": [LEAVER] }), json!({}));
    s.end_turn();
    assert_eq!(
        health(&s, P2),
        29,
        "the copy finds the Leaver in hand and fizzles"
    );
    assert!(s.hand(P1).iter().any(|card| card.def_id == LEAVER));
}

#[test]
fn r821_the_opponents_hooks_run_once() {
    let mut s = game(
        json!({ "backrow": [JOINT, LIVING] }),
        json!({ "field": [TICKER] }),
    );
    s.end_turn();
    assert_eq!(s.state().active, P2);
    assert_eq!(health(&s, P1), 29, "p2's Ticker runs once: p2 has no multiplier");
    assert_eq!(
        health(&s, P2),
        29,
        "p1's start-of-opponent-turn hook is not p1's own turn hook"
    );
}

// R822: a Spell's resolution is no Cry

#[test]
fn r822_a_spells_resolution_does_not_repeat() {
    let mut s = game(json!({ "backrow": [DOUBLE], "hand": [AIMER, BOLT] }), json!({}));
    s.play(AIMER, json!({ "targets": [hero(P2)] }));
    assert_eq!(health(&s, P2), 28, "the Unit's Cry runs twice");
    s.play(BOLT, json!({ "targets": [hero(P2)] }));
    assert_eq!(health(&s, P2), 27, "the Spell resolves once");
}

// R823: what an extra Cry and an extra Death reuse

#[test]
fn r823_a_target_gone_since_fizzles() {
    let mut s = game(
        json!({ "backrow": [DOUBLE], "hand": [AIMER] }),
        json!({ "field": [PAWN] }),
    );
    let target = s.unit(P2, 1).expect("p2's first pawn");
    s.play(AIMER, json!({ "targets": [instance(&target)] }));
    assert_eq!(
        damage_events_on(&s, &target.id),
        1,
        "the extra Cry finds its target gone"
    );
    s.expect_in_zone(target, "graveyard");
}

#[test]
fn r823_a_cry_that_asks_survives_a_json_round_trip() {
    let mut s = game(json!({ "backrow": [DOUBLE], "hand": [ASKER] }), json!({}));
    s.play(ASKER, json!({}));
    s.answer(json!("left"));
    let saved = serde_json::to_string(s.state()).expect("a paused state serializes");
    *s.state_mut() = serde_json::from_str(&saved).expect("and reads back");
    s.answer(json!("right"));
    assert!(s.state().pending.is_none());
    assert_eq!(
        health(&s, P2),
        30 - 2 * (1 + 4 + 2),
        "each run deals 1, then 4, then 2"
    );
}

#[test]
fn r823_a_triggered_cry_reuses_its_answers() {
    let mut s = game(
        json!({ "backrow": [DOUBLE], "field": [AIMER], "hand": [TRIGGERER] }),
        json!({}),
    );
    let aimer = s.unit(P1, 1).expect("the aimer");
    s.play(TRIGGERER, json!({ "targets": [instance(&aimer)] }));
    s.answer(json!([hero(P2)]));
    assert!(s.state().pending.is_none(), "the extra run asks nothing again");
    assert_eq!(health(&s, P2), 28);
}

#[test]
fn r823_a_death_runs_again_for_its_controller() {
    let mut s = game(
        json!({ "backrow": [DOUBLE], "field": [DIER], "hand": [BOLT, BOLT] }),
        json!({ "field": [DIER] }),
    );
    let mine = s.unit(P1, 1).expect("p1's dier");
    let theirs = s.unit(P2, 1).expect("p2's dier");
    s.play(BOLT, json!({ "targets": [instance(&mine)] }));
    assert_eq!(health(&s, P2), 28, "p1's Dier's Death runs twice");
    s.play(BOLT, json!({ "targets": [instance(&theirs)] }));
    assert_eq!(
        health(&s, P1),
        29,
        "p2's Dier's Death runs once: p2 has no multiplier"
    );
}

#[test]
fn r823_r1107_a_granted_death_runs_again_under_a_multiplier_of_one() {
    // The Pawn prints no Death: its only one is granted, and it runs twice for its controller.
    assert_eq!(granted_death_under(DOUBLE, PAWN), 28);
}

#[test]
fn r823_r1107_a_granted_death_runs_twice_more_under_a_multiplier_of_two() {
    assert_eq!(granted_death_under(DOUBLE_TWO, PAWN), 27);
}

#[test]
fn r823_r1107_a_printed_and_a_granted_death_run_again_as_one_list() {
    // The Dier's own Death (1) and the granted one (1) are one list, and the list runs twice.
    assert_eq!(granted_death_under(DOUBLE, DIER), 26);
}

#[test]
fn r823_dying_together_doubles_nothing() {
    let mut s = game(
        json!({ "backrow": [DOUBLE], "field": [DIER], "hand": [NUKE] }),
        json!({}),
    );
    s.play(NUKE, json!({}));
    assert_eq!(health(&s, P2), 29, "Double Counting left the field with the Dier");
}

// R824: trigger your End of turn effects

#[test]
fn r824_each_round_follows_the_last_and_the_turn_goes_on() {
    let mut s = game(json!({ "field": [SPAWNER], "hand": [FEAR] }), json!({}));
    s.play(FEAR, json!({}));
    assert_eq!(
        on_field(&s, P1, TICKER),
        2,
        "each round's Spawner summons a Ticker"
    );
    assert_eq!(
        health(&s, P2),
        29,
        "the Ticker the first round summoned answers the second"
    );
    assert_eq!(s.state().active, P1);
    assert_eq!(s.state().phase, Phase::Main);
}

#[test]
fn r824_rounds_are_multiplied() {
    let mut s = game(
        json!({ "backrow": [JOINT], "field": [TICKER], "hand": [FEAR] }),
        json!({}),
    );
    s.play(FEAR, json!({}));
    assert_eq!(health(&s, P2), 26, "two rounds, each run twice");
}

// R4589003: The Recursionist's Death-only and Reborn multipliers

#[test]
fn r4589003_a_death_multiplier_reruns_deaths_not_cries() {
    let mut s = game(
        json!({ "backrow": [DEATHS], "field": [DIER], "hand": [AIMER, BOLT] }),
        json!({}),
    );
    s.play(AIMER, json!({ "targets": [hero(P2)] }));
    assert_eq!(health(&s, P2), 29, "the Cry runs once");
    let dier = s.unit(P1, 1).expect("p1's dier");
    s.play(BOLT, json!({ "targets": [instance(&dier)] }));
    assert_eq!(health(&s, P2), 27, "the Death runs twice");
}

#[test]
fn r4589003_the_highest_multiplier_holds() {
    let mut s = game(
        json!({ "backrow": [DOUBLE, DEATHS], "field": [DIER], "hand": [BOLT] }),
        json!({}),
    );
    let dier = s.unit(P1, 1).expect("p1's dier");
    s.play(BOLT, json!({ "targets": [instance(&dier)] }));
    assert_eq!(health(&s, P2), 28, "the Death runs twice, not three times");
}

#[test]
fn r4589003_a_doubled_reborn_returns_twice_then_stays_dead() {
    let mut s = game(
        json!({ "backrow": [REBORNS], "field": [REBORNER], "hand": [BOLT, BOLT, BOLT] }),
        json!({}),
    );
    for round in 1..=3 {
        let body = s.unit(P1, 1).expect("the reborner");
        s.play(BOLT, json!({ "targets": [instance(&body)] }));
        let back = s.unit(P1, 1).filter(|unit| unit.def_id == REBORNER);
        if round < 3 {
            assert!(back.is_some(), "round {round}: the doubled Reborn returns it");
        } else {
            assert!(back.is_none(), "round 3: its Reborn is spent, so it stays dead");
        }
    }
    assert!(
        s.pile(P1, "graveyard").iter().any(|card| card.def_id == REBORNER),
        "the body rests in the graveyard"
    );
}

#[test]
fn r4589003_one_dying_in_the_pass_doubles_nothing() {
    let mut s = game(
        json!({ "backrow": [REBORNS], "field": [REBORNER], "hand": [NUKE, BOLT, BOLT] }),
        json!({}),
    );
    s.play(NUKE, json!({}));
    let body = s.unit(P1, 1).expect("the reborner returns once");
    assert_eq!(body.def_id, REBORNER);
    assert!(
        !body
            .memory
            .contains_key(jackioh_engine::state_check::REBORN_KEPT_KEY),
        "a Recursionist dying in the same pass doubles nothing (R823)"
    );
    // Its ordinary Reborn spent itself on that one return: the next death leaves it dead.
    s.play(BOLT, json!({ "targets": [instance(&body)] }));
    assert!(
        s.unit(P1, 1).filter(|unit| unit.def_id == REBORNER).is_none(),
        "no Reborn is left to spend"
    );
}
