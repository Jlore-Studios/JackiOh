//! A unit the first tick of its own Burn and Toxic destroys before it can attack (#4065, R4065000): a Burn
//! ticks at the end of the turn under way (R4154000) and a Toxic at the start of its controller's turn
//! (R3540001), each before that unit's next attacks, so `face_threat` counts
//! none of its attacks, and neither `wall_switches` nor `stands_down` offers a switch on its account. Each
//! board is then stepped through the engine, which destroys exactly the units the read leaves out. The cards:
//! Mr. Vanilla, a 4/4; Core #30 Archivist set to a 3/3; Core #25, a 4-mana 7/7 with Armor 7; and Classic+ #6
//! Wrong-House Attacker, whose Radiant face is a 2/2 with Rush, Lifesteal, Poisonous and Reborn.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, in_graveyard, on_field, register_cards};

const VANILLA: &str = "core-008";
const ARCHIVIST: &str = "core-030";
const BIG: &str = "core-025";
const ATTACKER: &str = "classicplus-006";
/// S+ #42 Force Projector, a Field Spell: the first 5 damage its side would take each turn is nullified.
const PROJECTOR: &str = "stinkyscapeplus-042";

/// A library for each side, so an end of turn draws a card and no fatigue moves a number.
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// AI_EVAL with the affliction read off: every unit that may swing counts, as before #4065.
const OFF: EvalWeights = EvalWeights {
    affliction_kills_before_swing: false,
    ..AI_EVAL
};

/// One side at `health` with `field` on it.
fn side(health: i32, field: Value) -> Value {
    json!({ "health": health, "field": field, "library": LIBRARY })
}

/// A scenario from `options`, on the scenario's turn 9 with the AI on turn unless `options` says otherwise.
fn at(mut options: Value) -> Scenario {
    register_cards();
    options["seed"] = json!("ai-affliction-threat");
    scenario(options)
}

/// A board of `p1` against `p2`, the AI on turn.
fn game(p1: Value, p2: Value) -> Scenario {
    at(json!({ "p1": p1, "p2": p2 }))
}

/// The AI's side of `lethal`: 7 life and the 7/7 in lane 3.
fn walled() -> Value {
    side(7, json!([{ "def": BIG, "lane": 3 }]))
}

/// The human's side of `lethal`: a 4/4 in lane 2 carrying `counters`, beside a 3/3 in lane 4.
fn burning(counters: Value) -> Value {
    side(
        30,
        json!([
            { "def": VANILLA, "lane": 2, "counters": counters },
            { "def": ARCHIVIST, "lane": 4, "statsOverride": { "attack": 3, "health": 3 } },
        ]),
    )
}

/// The AI at 7 life with the 7/7 in lane 3, against the 4/4 carrying `counters` and the 3/3: a threat of 7,
/// lethal, which the 7/7's Taunt in Defense Position walls, while the 4/4 still swings.
fn lethal(counters: Value) -> Scenario {
    game(walled(), burning(counters))
}

/// The human's unit in `lane`, by its zone: a def's name would find the AI's library copy first, since the
/// scenario looks through the player on turn's zones first.
fn theirs(s: &Scenario, lane: i32) -> CardInstance {
    s.unit_at(HUMAN, lane)
}

/// The 7/7's switch to Defense Position.
fn wall(state: &GameState) -> ActionBody {
    let big = active_units_of(state, AI)
        .into_iter()
        .find(|card| card.def_id == BIG)
        .expect("the 7/7");
    ActionBody::SwitchPosition {
        instance_id: big.id.clone(),
    }
}

/// The walls the AI is offered on `state`.
fn walls(state: &GameState) -> Vec<ActionBody> {
    wall_switches(state, AI, &candidate_actions(state, AI, Prices::NONE))
}

/// Burn 3 leaves the 4/4 at 1 health, so it still swings and the 7/7 still walls; Burn 5 destroys it at the
/// end of the AI's turn 9 (R4154000), so only the 3/3's 3 can come and there is no lethal to wall.
#[test]
fn r4065000_a_unit_its_own_burn_destroys_first_adds_nothing_to_the_face_threat() {
    let mut s = lethal(json!({ "burn": 3 }));
    assert_eq!(face_threat(s.state(), HUMAN), 7);
    assert_eq!(walls(s.state()), vec![wall(s.state())]);
    s.end_turn();
    assert_eq!(s.stats(theirs(&s, 2)).health, 1);

    let mut s = lethal(json!({ "burn": 5 }));
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 7);
    assert!(walls(s.state()).is_empty(), "{:?}", walls(s.state()));
    close(
        evaluate(s.state(), AI, NextSwing::Enemy, &AI_EVAL) - evaluate(s.state(), AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat - 3.0 * AI_EVAL.threat_per_damage,
    );
    s.end_turn();
    assert!(in_graveyard(s.state(), HUMAN, VANILLA));
    s.attack(ARCHIVIST, "hero").expect_health(AI, 4);
}

/// Burn's tick meets Armor: Burn 5 on a 4/4 with Armor 2 deals 3 and leaves it standing. Toxic's pierces it:
/// Toxic 4 deals 4 and destroys it.
#[test]
fn r4065000_armor_meets_burn_and_toxic_pierces_it() {
    let mut s = lethal(json!({ "burn": 5 }));
    s.card_mut(theirs(&s, 2))
        .granted_keywords
        .push(Keyword::Armor { n: 2 });
    assert_eq!(face_threat(s.state(), HUMAN), 7);
    s.end_turn();
    assert_eq!(s.stats(theirs(&s, 2)).health, 1);

    let mut s = lethal(json!({ "toxic": 4 }));
    s.card_mut(theirs(&s, 2))
        .granted_keywords
        .push(Keyword::Armor { n: 2 });
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 7);
    s.end_turn();
    assert!(in_graveyard(s.state(), HUMAN, VANILLA));
}

/// R4064000: the human's Force Projector, full when the Burn ticks at the end of the AI's turn, soaks the whole
/// Burn 5, so the 4/4 stands and still swings: read as `afflicted` reads the tick, it still counts, and with the pool
/// read off it is left out as on a board without the Projector. The engine leaves it at full health.
#[test]
fn r4065000_a_unit_its_damage_pool_keeps_standing_still_counts() {
    const NO_POOL: EvalWeights = EvalWeights {
        pool_and_conversion_read: false,
        ..AI_EVAL
    };
    let mut guarded = burning(json!({ "burn": 5 }));
    guarded["backrow"] = json!([{ "def": PROJECTOR, "faceUp": true }]);
    let mut s = game(walled(), guarded);
    assert_eq!(face_threat(s.state(), HUMAN), 7);
    assert_eq!(face_threat_with(s.state(), HUMAN, &NO_POOL), 3);
    s.end_turn();
    assert_eq!(s.stats(theirs(&s, 2)).health, 4);
}

/// An unspent Divine Shield takes the Burn's hit and Indestructible takes none, so the 4/4 still swings. With
/// Toxic 4 beside the Burn the shield takes only the first hit, and the Toxic's destroys it.
#[test]
fn r4065000_a_shielded_or_indestructible_unit_still_counts() {
    for keyword in [Keyword::DivineShield, Keyword::Indestructible] {
        let mut s = lethal(json!({ "burn": 5 }));
        s.card_mut(theirs(&s, 2)).granted_keywords.push(keyword.clone());
        assert_eq!(face_threat(s.state(), HUMAN), 7, "{keyword:?}");
        s.end_turn();
        assert!(on_field(s.state(), HUMAN, VANILLA), "{keyword:?}");
    }

    let mut s = lethal(json!({ "burn": 5, "toxic": 4 }));
    s.card_mut(theirs(&s, 2))
        .granted_keywords
        .push(Keyword::DivineShield);
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    s.end_turn();
    assert!(in_graveyard(s.state(), HUMAN, VANILLA));
}

/// Read with the side on turn swinging first, its attacks come this turn, before its next tick.
#[test]
fn r4065000_the_side_on_turn_swings_before_its_tick() {
    // (a) The AI's own Burn 5 4/4 against an empty board and a hero at 4: read at the end of the AI's turn it
    // burns first, so no lethal pressure; read with the AI swinging first it swings now, and it is lethal.
    let mut s = game(
        side(
            30,
            json!([{ "def": VANILLA, "lane": 3, "counters": { "burn": 5 } }]),
        ),
        side(4, json!([])),
    );
    assert_eq!(face_threat(s.state(), AI), 0);
    assert_eq!(face_threat_with(s.state(), AI, &OFF), 4);
    close(
        evaluate(s.state(), AI, NextSwing::Seat, &AI_EVAL),
        evaluate(s.state(), AI, NextSwing::Seat, &OFF),
    );
    close(
        evaluate(s.state(), AI, NextSwing::Enemy, &OFF) - evaluate(s.state(), AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.lethal_pressure,
    );
    s.attack(VANILLA, "hero");
    assert!(s.state().result.is_some());

    // (b) The human on turn 10: read with it swinging first, its 4/4 swings now, before its Burn ticks at the
    // turn's end (R4154000).
    let mut s =
        at(json!({ "turn": 10, "active": "p2", "p1": walled(), "p2": burning(json!({ "burn": 5 })) }));
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    close(
        evaluate(s.state(), AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(s.state(), AI, NextSwing::Enemy, &OFF),
    );
    s.attack(VANILLA, "hero").expect_health(AI, 3);
}

/// R83: the Reborn body of a 4/4 its Burn destroys comes back summoning sick, with no Rush or Charge, so it
/// cannot attack on that turn.
#[test]
fn r4065000_an_unspent_reborn_does_not_bring_the_swing_back() {
    let mut s = game(
        side(30, json!([])),
        side(
            30,
            json!([{ "def": VANILLA, "lane": 3, "counters": { "burn": 5 } }]),
        ),
    );
    s.card_mut(theirs(&s, 3)).granted_keywords.push(Keyword::Reborn);
    assert_eq!(face_threat(s.state(), HUMAN), 0);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 4);
    s.end_turn();
    assert_eq!(s.state().active, HUMAN);
    let body = theirs(&s, 3);
    assert_eq!((body.def_id.as_str(), s.stats(&body).health), (VANILLA, 1));
    assert!(s.attacks_of(&body).is_empty(), "{:?}", s.attacks_of(&body));
}

/// R83: a Reborn body with Rush may attack a Unit on the turn it comes back, never the hero: a Radiant
/// Wrong-House Attacker its Burn destroys adds nothing to the face threat, and its body attacks the AI's unit.
#[test]
fn r4065000_a_reborn_body_with_rush_attacks_only_a_unit() {
    let mut s = game(
        side(30, json!([{ "def": VANILLA, "lane": 3 }])),
        side(
            30,
            json!([{ "def": ATTACKER, "radiant": true, "lane": 2, "counters": { "burn": 5 } }]),
        ),
    );
    assert_eq!(face_threat(s.state(), HUMAN), 0);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 2);
    s.end_turn();
    assert_eq!(s.state().active, HUMAN);
    let vanilla = s.unit_at(AI, 3).id;
    let attacks = s.attacks_of(ATTACKER);
    let target = |id: &str| {
        attacks
            .iter()
            .any(|attack| matches!(attack, ActionBody::Attack { target_id, .. } if target_id == id))
    };
    assert!(target(&vanilla), "{attacks:?}");
    assert!(!target(&format!("hero-{}", AI.as_str())), "{attacks:?}");
}

/// The AI's 3/3 against a 4/4 in lane 3: the 4/4 would kill it and survive, so it stands down, still with
/// Burn 3 on the 4/4, but not with Burn 5, which destroys the 4/4 first.
#[test]
fn r4065000_a_burning_enemy_stands_nothing_down() {
    let duel = |counters: Value| {
        game(
            side(
                30,
                json!([{ "def": ARCHIVIST, "lane": 3, "statsOverride": { "attack": 3, "health": 3 } }]),
            ),
            side(30, json!([{ "def": VANILLA, "lane": 3, "counters": counters }])),
        )
    };
    let stand_down = |s: &Scenario| ActionBody::SwitchPosition {
        instance_id: s.unit_at(AI, 3).id,
    };
    for counters in [json!({}), json!({ "burn": 3 })] {
        let s = duel(counters.clone());
        assert!(stands_down(s.state(), &stand_down(&s)), "{counters}");
    }
    let mut s = duel(json!({ "burn": 5 }));
    assert!(!stands_down(s.state(), &stand_down(&s)));
    s.end_turn();
    assert!(in_graveyard(s.state(), HUMAN, VANILLA));
}

/// The greedy yardstick stays frozen: GREEDY_EVAL leaves the flag off and still counts the burning 4/4, and
/// its score is the same as on the board without the Burn (its affliction weights are 0). With no Burn and
/// no Toxic on the board, AI_EVAL reads as with the flag off.
#[test]
fn r4065000_greedy_eval_and_boards_without_counters_read_as_before() {
    const { assert!(AI_EVAL.affliction_kills_before_swing) };
    const { assert!(!GREEDY_EVAL.affliction_kills_before_swing) };
    let plain = lethal(json!({}));
    let burned = lethal(json!({ "burn": 5 }));
    assert_eq!(face_threat_with(burned.state(), HUMAN, &GREEDY_EVAL), 7);
    close(
        evaluate(plain.state(), AI, NextSwing::Enemy, &GREEDY_EVAL),
        evaluate(burned.state(), AI, NextSwing::Enemy, &GREEDY_EVAL),
    );

    assert_eq!(face_threat(plain.state(), HUMAN), 7);
    assert_eq!(face_threat_with(plain.state(), HUMAN, &OFF), 7);
    for next in [NextSwing::Enemy, NextSwing::Seat] {
        close(
            evaluate(plain.state(), AI, next, &AI_EVAL),
            evaluate(plain.state(), AI, next, &OFF),
        );
    }
}
