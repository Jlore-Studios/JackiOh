//! R5228000, #5228: the AI's Taunt walk (`damage_past_taunts_with`, so `face_threat`, the threat term,
//! `wall_switches` and `ready_gap`) wears the defending hero's own Armor by 1 for each hit it meets, as the
//! engine does since R5183000 (§4.4 step 2): Trample excesses as they land past the Taunts, then the free
//! hits, smallest first, through one worn count per walk. A Pierce hit neither meets nor wears it, an aura's
//! Armor (Going Long) never wears, and a hero immune to damage wears nothing. Each board is played out through
//! `reduce` in the order the walk reads it, and the hero damage the engine deals is the read. `GREEDY_EVAL`
//! reads every board as before.
//!
//! The units are the testkit's `stand-in` bodies, so a balance patch cannot move them, Mr. Vanilla (Core #8)
//! to draw, and Going Long (Core #84), face up in the AI's backrow: an aura of Armor 2 on the AI's hero.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close, register_cards, unit};

/// The walk as it read before #5228.
const OFF: EvalWeights = EvalWeights {
    armor_wear_read: false,
    ..AI_EVAL
};

/// Mr. Vanilla, 4/4: the cards each side draws.
const VANILLA: &str = "core-008";
/// Going Long: the AI's hero has Armor 2 while it stands face up in the backrow.
const GOING_LONG: &str = "core-084";
/// The issue's board: four 3/3s, each a card of its own so each is found by its id.
const ATTACKERS: [&str; 4] = [
    "stand-in (1) 3/3",
    "stand-in (2) 3/3",
    "stand-in (3) 3/3",
    "stand-in (4) 3/3",
];

/// p1 (the AI, on turn at turn 9) as `p1` says, against p2 (the human) as `p2` says, each with plain cards to
/// draw. Neither turn ends by itself (R82), so a play-out stays on the human's turn.
fn board(seed: &str, p1: Value, p2: Value) -> GameState {
    register_cards();
    let (mut p1, mut p2) = (p1, p2);
    p1["library"] = json!([VANILLA, VANILLA]);
    p2["library"] = json!([VANILLA, VANILLA]);
    let mut state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "turn": 9,
        "p1": p1,
        "p2": p2,
    }))
    .state()
    .clone();
    for player in [AI, HUMAN] {
        state.players[player].auto_end_turn = Some(false);
    }
    state
}

/// A field of `defs`, in lanes 1, 2, … in order.
fn field(defs: &[&str]) -> Value {
    Value::Array(
        defs.iter()
            .zip(1..)
            .map(|(def, lane)| json!({ "def": def, "lane": lane }))
            .collect(),
    )
}

/// The AI passes, then each of the human's `attackers`, in turn, attacks the AI's hero: the state after the
/// pass, and after the attacks.
fn at_ai(state: &GameState, attackers: &[&str]) -> (GameState, GameState) {
    let passed = act(state, AI, ActionBody::EndTurn);
    let after = at_hero(&passed, attackers);
    (passed, after)
}

/// Each of the human's `attackers`, in turn, attacks the AI's hero, on the human's turn: the state after.
fn at_hero(state: &GameState, attackers: &[&str]) -> GameState {
    let mut after = state.clone();
    for def in attackers {
        let attacker_id = unit(&after, HUMAN, def).id.clone();
        after = act(
            &after,
            HUMAN,
            ActionBody::Attack {
                attacker_id,
                target_id: format!("hero-{}", AI.as_str()),
            },
        );
    }
    after
}

/// What the play-out dealt the AI's hero.
fn dealt(passed: &GameState, after: &GameState) -> i32 {
    passed.players[AI].hero.health - after.players[AI].hero.health
}

/// The issue's board: the AI at 6 with Armor 3 of its own, against four ready 3/3s and no Taunt.
fn issue_board() -> GameState {
    board(
        "ai-armor-wear-issue",
        json!({ "health": 6, "armor": 3 }),
        json!({ "field": field(&ATTACKERS) }),
    )
}

/// R5228000: the four hits meet Armor 3, 2, 1 and 0, so they deal 0 + 1 + 2 + 3 = 6, the AI's whole life,
/// where the walk set each against the whole Armor and read 0. The threat term turns lethal with it.
#[test]
fn r5228000_face_threat_wears_the_heros_armor_hit_by_hit() {
    let s = issue_board();
    assert_eq!(face_threat(&s, HUMAN), 6);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0);
    assert_eq!(face_threat_with(&s, HUMAN, &GREEDY_EVAL), 0);
    close(
        evaluate(&s, AI, NextSwing::Enemy, &OFF) - evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.lethal_threat,
    );

    let (passed, after) = at_ai(&s, &ATTACKERS);
    assert_eq!(dealt(&passed, &after), 6);
    assert_eq!(
        after.result.as_ref().and_then(|result| result.winner.player()),
        Some(HUMAN),
        "the four attacks are lethal in the engine"
    );
}

/// R5228000: with no Armor there is nothing to wear, and every weight reads 4 × 3 = 12, as the engine deals.
#[test]
fn r5228000_a_hero_without_armor_reads_as_before() {
    let s = board(
        "ai-armor-wear-bare",
        json!({ "health": 20 }),
        json!({ "field": field(&ATTACKERS) }),
    );
    for w in [&AI_EVAL, &OFF, &GREEDY_EVAL] {
        assert_eq!(face_threat_with(&s, HUMAN, w), 12);
    }
    let (passed, after) = at_ai(&s, &ATTACKERS);
    assert_eq!(dealt(&passed, &after), 12);
}

/// R5228000: the same board from the attacker's side. The walk reads the four 3/3s' 6 against the human at 6
/// with Armor 3, so `ready_gap` reads 0, and the lethal solver proves the lethal the engine plays.
#[test]
fn r5228000_ready_gap_and_find_lethal_read_the_wear() {
    let s = board(
        "ai-armor-wear-attacker",
        json!({ "field": field(&ATTACKERS) }),
        json!({ "health": 6, "armor": 3 }),
    );
    assert!(ready_gap(&s, AI) <= 0.0, "{}", ready_gap(&s, AI));
    let attacks: Vec<(&CardInstance, i32, bool)> = active_units_of(&s, AI)
        .into_iter()
        .map(|u| (u, 3, true))
        .collect();
    assert_eq!(attacks.len(), 4);
    assert_eq!(damage_past_taunts_with(&s, HUMAN, &attacks, &AI_EVAL), 6);
    assert_eq!(damage_past_taunts_with(&s, HUMAN, &attacks, &OFF), 0);
    assert_eq!(damage_past_taunts_with(&s, HUMAN, &attacks, &GREEDY_EVAL), 0);

    let line = find_lethal(
        std::slice::from_ref(&s),
        AI,
        &create_node_counter(AI_BUDGET.nodes, None),
        AI_BUDGET.lethal_nodes,
        Prices::NONE,
    );
    let Some(line) = line else {
        panic!("find_lethal proves no lethal")
    };
    let end = line
        .iter()
        .fold(s.clone(), |state, action| act(&state, AI, action));
    assert_eq!(
        end.result.as_ref().and_then(|result| result.winner.player()),
        Some(AI),
        "the line wins in the engine"
    );
}

/// The AI's 2/2 on the wall board.
const GUARD: &str = "stand-in (1) 2/2";

/// R5228000: the AI at 6 with Armor 3 and a 2/2 in Attack Position. The four 3/3s threaten 6, so the switch
/// to Defense Position is offered: its Taunt falls to the first hit, and the other three deal 0 + 1 + 2 = 3.
/// Without the wear the threat read 0, short of the AI's life, and no switch was offered.
#[test]
fn r5228000_the_wall_switches_are_offered_once_the_wear_reaches_the_life() {
    let s = board(
        "ai-armor-wear-wall",
        json!({ "health": 6, "armor": 3, "field": field(&[GUARD]) }),
        json!({ "field": field(&ATTACKERS) }),
    );
    assert_eq!(
        wall_switches(&s, AI, &candidate_actions(&s, AI, Prices::NONE)),
        vec![ActionBody::SwitchPosition {
            instance_id: unit(&s, AI, GUARD).id.clone(),
        }]
    );
    assert!(face_threat_with(&s, HUMAN, &OFF) < effective_life(&s, AI));

    let mut walled = s.clone();
    let guard = unit(&s, AI, GUARD).id.clone();
    walled = act(&walled, AI, ActionBody::SwitchPosition { instance_id: guard });
    assert_eq!(face_threat(&walled, HUMAN), 3);
}

/// The AI's 0/2 Taunt on the Trample board.
const TAUNT: &str = "stand-in (1) 0/2 Taunt";
/// The human's Trample 4/4 on the Trample board.
const TRAMPLER: &str = "stand-in (1) 4/4 Trample";

/// R5228000: the 4/4 Trample's hit on the 0/2 Taunt tramples 2 onto the AI's hero, which its Armor 3 takes
/// whole, wearing it to 2; the two 5/5s then meet Armor 2 and 1: 0 + 3 + 4 = 7, where the walk read
/// 0 + 2 + 2 = 4.
#[test]
fn r5228000_a_trample_excess_past_a_taunt_meets_the_armor_already_worn() {
    let fives = ["stand-in (1) 5/5", "stand-in (2) 5/5"];
    let s = board(
        "ai-armor-wear-trample",
        json!({ "health": 20, "armor": 3, "field": field(&[TAUNT]) }),
        json!({ "field": field(&[TRAMPLER, fives[0], fives[1]]) }),
    );
    assert_eq!(face_threat(&s, HUMAN), 7);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 4);
    assert_eq!(face_threat_with(&s, HUMAN, &GREEDY_EVAL), 4);

    let passed = act(&s, AI, ActionBody::EndTurn);
    let trampled = act(
        &passed,
        HUMAN,
        ActionBody::Attack {
            attacker_id: unit(&passed, HUMAN, TRAMPLER).id.clone(),
            target_id: unit(&passed, AI, TAUNT).id.clone(),
        },
    );
    assert_eq!(dealt(&passed, &trampled), 0, "the Armor takes the excess whole");
    assert_eq!(trampled.players[AI].hero.armor, 2, "and wears");
    let after = at_hero(&trampled, &fives);
    assert_eq!(dealt(&passed, &after), 7);
    assert_eq!(after.players[AI].hero.armor, 0);
}

/// R5228000: Going Long's Armor 2 is an aura's and never wears, so each 3/3 deals 1: 4, with the flag or
/// without. With 1 Armor of the hero's own beside it, the first hit meets 3 and wears the 1; the other three
/// meet Going Long's 2: 0 + 1 + 1 + 1 = 3, where the walk read 0.
#[test]
fn r5228000_armor_from_going_long_alone_wears_nothing() {
    let backrow = json!([{ "def": GOING_LONG, "faceUp": true }]);
    let s = board(
        "ai-armor-wear-going-long",
        json!({ "health": 20, "backrow": backrow }),
        json!({ "field": field(&ATTACKERS) }),
    );
    assert_eq!(hero_armor_of(&s, AI), 2);
    assert_eq!(face_threat(&s, HUMAN), 4);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 4);
    let (passed, after) = at_ai(&s, &ATTACKERS);
    assert_eq!(dealt(&passed, &after), 4);

    let s = board(
        "ai-armor-wear-going-long-and-own",
        json!({ "health": 20, "armor": 1, "backrow": backrow }),
        json!({ "field": field(&ATTACKERS) }),
    );
    assert_eq!(hero_armor_of(&s, AI), 3);
    assert_eq!(face_threat(&s, HUMAN), 3);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0);
    let (passed, after) = at_ai(&s, &ATTACKERS);
    assert_eq!(dealt(&passed, &after), 3);
    assert_eq!(after.players[AI].hero.armor, 0);
}

/// R5228000: the Pierce 3/3's hit skips the Armor whole and wears none of it (R346), so the other three still
/// meet 3, 2 and 1: 3 + 0 + 1 + 2 = 6, where the walk read 3.
#[test]
fn r5228000_a_pierce_hit_neither_meets_nor_wears_the_armor() {
    let attackers = [
        "stand-in (1) 3/3 Pierce",
        ATTACKERS[1],
        ATTACKERS[2],
        ATTACKERS[3],
    ];
    let s = board(
        "ai-armor-wear-pierce",
        json!({ "health": 20, "armor": 3 }),
        json!({ "field": field(&attackers) }),
    );
    assert_eq!(face_threat(&s, HUMAN), 6);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 3);
    let (passed, after) = at_ai(&s, &attackers);
    assert_eq!(dealt(&passed, &after), 6);
}

/// R5228000: a hero immune to damage takes nothing from any hit, and its Armor wears none: 0, as the engine
/// deals, its Armor still 3.
#[test]
fn r5228000_a_hero_immune_to_damage_wears_nothing() {
    let mut s = board(
        "ai-armor-wear-immune",
        json!({ "health": 20, "armor": 3 }),
        json!({ "field": field(&ATTACKERS) }),
    );
    s.players[AI].mods.push(PlayerModifier {
        id: "r5228000-immune".into(),
        expiry: ModifierExpiry::Never,
        kind: ModifierKind::HeroImmune,
    });
    assert_eq!(face_threat(&s, HUMAN), 0);
    let (passed, after) = at_ai(&s, &ATTACKERS);
    assert_eq!(dealt(&passed, &after), 0);
    assert_eq!(after.players[AI].hero.armor, 3);
}

#[test]
fn r5228000_the_flag_is_on_in_ai_eval_and_off_in_greedy_eval() {
    const { assert!(AI_EVAL.armor_wear_read) };
    const { assert!(!GREEDY_EVAL.armor_wear_read) };
}
