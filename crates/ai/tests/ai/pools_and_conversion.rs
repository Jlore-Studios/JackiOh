//! The AI's static reads of a hit through §4.4 steps 3a and 3b (#4064). Each side's damage pools (S+ #42
//! Force Projector, S+ #123 Oct) soak what they have left, once across the hits the walk counts (R4064000),
//! and a hit from a source whose damage Burns or is Toxic (S+ #151 Volcanic Rounds, S+ #153 Noxious Rounds)
//! deals nothing and is its target's counter (R4064001). Each board is also played through the engine,
//! which deals what the static read says. The stand-ins: Mr. Vanilla, a 4/4, and the `stand-in (1)` bodies.

use jackioh_ai::*;
use jackioh_engine::testkit::*;
use jackioh_engine::{hero_burn_of, hero_toxic_of};

use super::support::{AI, HUMAN, close, register_cards};

const PROJECTOR: &str = "stinkyscapeplus-042";
const OCT: &str = "stinkyscapeplus-123";
const VOLCANIC: &str = "stinkyscapeplus-151";
const NOXIOUS: &str = "stinkyscapeplus-153";
const VANILLA: &str = "core-008";
const THREE_THREE: &str = "stand-in (1) 3/3";

/// AI_EVAL with the pools and the conversions read off: every hit is damage and no pool soaks, as before
/// #4064.
const OFF: EvalWeights = EvalWeights {
    pool_and_conversion_read: false,
    ..AI_EVAL
};

/// The AI's life on the issue's board, and what its Force Projector soaks of the 7 coming at it.
const LIFE: i32 = 6;

/// p1 (the AI, on turn in its main phase) and p2 as given, each with plain cards to draw and no turn that
/// ends by itself, so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_cards();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        side["library"] = json!([VANILLA; 6].to_vec());
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [AI, HUMAN] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// The issue's board: the AI at 6 behind `guard` (its backrow and field entries), a 4/4 and a 3/3 of
/// the enemy's ready to swing, and the enemy's 5/20 in Defense Position across from the AI's Mr. Vanilla.
fn guarded(seed: &str, backrow: Value, field: Value) -> Scenario {
    let mut ai_field = json!([{ "def": VANILLA, "lane": 1 }]);
    if let (Some(units), Some(more)) = (ai_field.as_array_mut(), field.as_array()) {
        units.extend(more.iter().cloned());
    }
    board(
        seed,
        json!({ "health": LIFE, "field": ai_field, "backrow": backrow }),
        json!({ "field": [
            { "def": "stand-in (1) 5/20", "lane": 1, "position": "DEF" },
            { "def": VANILLA, "lane": 2 },
            { "def": THREE_THREE, "lane": 3 },
        ] }),
    )
}

fn projector(radiant: bool) -> Value {
    json!([{ "def": PROJECTOR, "faceUp": true, "radiant": radiant }])
}

fn health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// What the enemy's 4/4 and 3/3 deal the AI on the enemy's turn, from its health once that turn starts
/// (Oct's end-of-turn heal comes before it).
fn swung(mut s: Scenario) -> i32 {
    s.end_turn();
    let before = health(&s, AI);
    for lane in [2, 3] {
        let attacker = s.unit_at(HUMAN, lane).id;
        s.attack(attacker.as_str(), "hero");
    }
    before - health(&s, AI)
}

/// R4064000: behind its unspent Force Projector, the AI at 6 faces 2 of the 7 the enemy's board swings,
/// not lethal: no wall is offered and the evaluation pays `threat_per_damage` for 2, not `lethal_threat`.
/// The engine lands exactly 2. Once the AI's own attack has spent the pool on the 5/20's strike back,
/// the 7 is lethal again. The old reading and the greedy yardstick count 7 throughout.
#[test]
fn r4064000_an_unspent_pool_leaves_two_of_seven_and_no_lethal() {
    let s = guarded("ai-pool-unspent", projector(false), json!([]));
    let state = s.state();
    assert_eq!(face_threat(state, HUMAN), 2);
    assert_eq!(face_threat_with(state, HUMAN, &OFF), 7);
    assert_eq!(face_threat_with(state, HUMAN, &GREEDY_EVAL), 7);
    assert!(wall_switches(state, AI, &candidate_actions(state, AI, Prices::NONE)).is_empty());
    close(
        evaluate(state, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(state, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat - AI_EVAL.threat_per_damage * 2.0,
    );
    assert_eq!(swung(s), 2);

    let mut spent = guarded("ai-pool-unspent", projector(false), json!([]));
    let vanilla = spent.unit_at(AI, 1).id;
    let wall = spent.unit_at(HUMAN, 1).id;
    spent.attack(vanilla.as_str(), wall.as_str());
    assert_eq!(
        unit_view(spent.state(), spent.card(vanilla.as_str())).health,
        4,
        "the pool took the strike back whole"
    );
    assert_eq!(face_threat(spent.state(), HUMAN), 7);
    assert_eq!(swung(spent), 7);
}

/// R4064000: Oct's pool is read as the Force Projector's is, and a Radiant Force Projector's 10 soaks the
/// whole 7; the engine lands what each read says.
#[test]
fn r4064000_oct_and_a_radiant_projector_read_alike() {
    let s = guarded("ai-pool-oct", json!([]), json!([{ "def": OCT, "lane": 4 }]));
    assert_eq!(face_threat(s.state(), HUMAN), 2);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 7);
    assert_eq!(swung(s), 2);

    let s = guarded("ai-pool-radiant", projector(true), json!([]));
    assert_eq!(face_threat(s.state(), HUMAN), 0);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 7);
    assert_eq!(swung(s), 0);
}

/// The AI at `ARMED_LIFE` behind its Force Projector and `walls` (Defense Position Taunts), facing the
/// enemy's `attackers`.
fn walled(seed: &str, walls: Value, attackers: Value) -> Scenario {
    board(
        seed,
        json!({ "health": ARMED_LIFE, "field": walls, "backrow": projector(false) }),
        json!({ "field": attackers }),
    )
}

/// The AI's life on the walled boards, more than any of their reads.
const ARMED_LIFE: i32 = 20;

/// R4064000: the walk spends one pool across the hits on a Taunt: a 1/3 Taunt in Defense Position (Armor 1)
/// takes 2 and then 3 of the pool's 5 and stands, so nothing reaches the hero, where the old reading breaks
/// it with the 4 and sends the 3 face. The engine deals nothing in either order.
#[test]
fn r4064000_the_walk_spends_one_pool_on_a_taunts_hits() {
    let walls = json!([{ "def": "stand-in (1) 1/3", "lane": 1, "position": "DEF" }]);
    let attackers = json!([{ "def": VANILLA, "lane": 2 }, { "def": THREE_THREE, "lane": 3 }]);
    let s = walled("ai-pool-taunt", walls.clone(), attackers.clone());
    assert_eq!(face_threat(s.state(), HUMAN), 0);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 3);

    for order in [[2, 3], [3, 2]] {
        let mut s = walled("ai-pool-taunt", walls.clone(), attackers.clone());
        s.end_turn();
        let wall = s.unit_at(AI, 1).id;
        for lane in order {
            let attacker = s.unit_at(HUMAN, lane).id;
            s.attack(attacker.as_str(), wall.as_str());
        }
        assert_eq!(health(&s, AI), ARMED_LIFE, "{order:?}");
        assert_eq!(unit_view(s.state(), s.card(wall.as_str())).health, 3, "{order:?}");
    }
}

/// R4064000: a Cleave and Trample 8 into a 1/2 Taunt beside a 1/4 Taunt (each Armor 1 in Defense Position)
/// lands its blow first (§4.4 step 10): the pool's 5 soaks the blow's 7, leaving 2 that destroys the 1/2
/// with nothing past it, and the splash's 7 on the 1/4 tramples 3 onto the hero. So 3 lands, where the old
/// reading counts 8; a splash read before the blow would have read 5. The engine lands 3.
#[test]
fn r4064000_a_blow_meets_the_pool_before_its_cleave_splash() {
    let walls = json!([
        { "def": "stand-in (1) 1/4", "lane": 1, "position": "DEF" },
        { "def": "stand-in (1) 1/2", "lane": 2, "position": "DEF" },
    ]);
    let attackers = json!([{ "def": VANILLA, "lane": 2, "statsOverride": { "attack": 8, "health": 8 } }]);
    let cleaving = || {
        let mut s = walled("ai-pool-cleave", walls.clone(), attackers.clone());
        let cleaver = s.unit_at(HUMAN, 2).id;
        s.card_mut(cleaver.as_str())
            .granted_keywords
            .extend([Keyword::Cleave, Keyword::Trample]);
        (s, cleaver)
    };
    let (s, _) = cleaving();
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 8);

    let (mut s, cleaver) = cleaving();
    s.end_turn();
    let small = s.unit_at(AI, 2).id;
    s.attack(cleaver.as_str(), small.as_str());
    assert_eq!(health(&s, AI), ARMED_LIFE - 3);
}

/// R4064000: each Burn and Toxic tick meets its side's pools full: a Toxic at its turn start, which refills
/// them first, and a Burn at the end of the turn (R4154000), here the AI's own turn, whose pool nothing spent:
/// the AI's hero with Toxic 3 and its Mr. Vanilla with Burn 3, each behind a Force Projector (two
/// boards, since the engine ticks the hero first and the Vanilla would meet only what that tick left),
/// lose nothing to their ticks, where the old reading counts them. The engine plays the ticks out alike.
#[test]
fn r4064000_ticks_meet_a_refilled_pool() {
    const TOXIC: i32 = 3;
    const BURN: i32 = 3;

    let mut s = board(
        "ai-pool-hero-ticks",
        json!({ "backrow": projector(false) }),
        json!({}),
    );
    let clean = s.state().clone();
    s.state_mut().players[AI].hero_toxic = Some(TOXIC);
    let toxic = s.state().clone();
    close(
        evaluate(&toxic, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&clean, AI, NextSwing::Enemy, &AI_EVAL),
    );
    assert!(evaluate(&toxic, AI, NextSwing::Enemy, &OFF) < evaluate(&clean, AI, NextSwing::Enemy, &OFF));
    let life = health(&s, AI);
    for _ in 0..AI_AFFLICTION_TICKS {
        s.end_turn();
        s.end_turn();
    }
    assert_eq!(hero_toxic_of(s.state(), AI), TOXIC, "Toxic never fades");
    assert_eq!(health(&s, AI), life);

    let mut s = board(
        "ai-pool-unit-ticks",
        json!({
            "backrow": projector(false),
            "field": [{ "def": VANILLA, "counters": { "burn": BURN } }],
        }),
        json!({}),
    );
    let vanilla = s.unit_at(AI, 1);
    let mut plain = vanilla.clone();
    plain.counters.burn = None;
    close(
        unit_worth(s.state(), &vanilla, &AI_EVAL, Prices::NONE),
        unit_worth(s.state(), &plain, &AI_EVAL, Prices::NONE),
    );
    assert!(
        unit_worth(s.state(), &vanilla, &OFF, Prices::NONE)
            < unit_worth(s.state(), &plain, &OFF, Prices::NONE)
    );
    let before = unit_view(s.state(), &vanilla).health;
    for _ in 0..AI_AFFLICTION_TICKS {
        s.end_turn();
        s.end_turn();
    }
    assert_eq!(unit_view(s.state(), s.card(vanilla.id.as_str())).health, before);
}

/// R4064001: a unit equipped with Volcanic Rounds (or Noxious Rounds) Burns (or Toxics) a hero for its
/// attack instead of hurting it, so against a hero at exactly that much life the lethal solver's gap and
/// the Taunt walk read no damage, where the old reading reads lethal. The engine's attack deals nothing.
#[test]
fn r4064001_an_equipped_rounds_unit_has_no_lethal() {
    for (rounds, host) in [(VOLCANIC, THREE_THREE), (NOXIOUS, "stand-in (1) 5/5")] {
        let mut s = board(
            "ai-rounds-lethal",
            json!({ "hand": [rounds], "field": [host], "mana": 10 }),
            json!({}),
        );
        let host = s.unit_at(AI, 1).id;
        s.play(rounds, json!({ "zone": 1, "equip": true }));
        let attack = s.stats(host.as_str()).attack;
        assert!(attack > 0, "{rounds}");
        s.state_mut().players[HUMAN].hero.health = attack;

        let state = s.state();
        assert_eq!(ready_gap(state, AI), f64::from(attack), "{rounds}");
        let unit = s.card(host.as_str());
        assert_eq!(
            damage_past_taunts_with(state, HUMAN, &[(unit, attack, true)], &OFF),
            attack,
            "{rounds}"
        );
        assert_eq!(
            damage_past_taunts_with(state, HUMAN, &[(unit, attack, true)], &AI_EVAL),
            0,
            "{rounds}"
        );

        s.attack(host.as_str(), "hero");
        assert_eq!(health(&s, HUMAN), attack, "{rounds}");
        let counter = if rounds == VOLCANIC {
            hero_burn_of(s.state(), HUMAN)
        } else {
            hero_toxic_of(s.state(), HUMAN)
        };
        assert_eq!(counter, attack, "{rounds}");
    }
}
