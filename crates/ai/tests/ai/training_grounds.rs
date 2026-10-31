//! The Training Grounds set (issue #2193) meets the AI (issue #2203). The set's prompts and choices need
//! nothing new in the search: `legal_actions` lists every play they allow (a declared target, a lane, a
//! face-down set), and the search tries each through `reduce`, so a card whose Cry reads its lane is
//! placed by the evaluation of what each lane's play leaves. These tests hold the AI to answering each
//! one legally, the random policy and the timeout (R79) too, and to the placements and targets a card
//! asks judgment of.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, is_legal, register_cards, scenario};

const PERSONAL_TRAINER: &str = "traininggrounds-002";
const THWOMP: &str = "traininggrounds-011";
const FORTIFY: &str = "traininggrounds-012";
const MIDLANE_MENACE: &str = "traininggrounds-005";
const CELESTIAL_CALLER: &str = "traininggrounds-016";
const FLAME_TONGUE_TOTEM: &str = "traininggrounds-021";
const RIGHT_HAND_FELINOR: &str = "traininggrounds-022";
const GOLIATH_SITUATION: &str = "traininggrounds-024";
const OCCIDENTAL: &str = "traininggrounds-028";
const PATH_OF_EXILE: &str = "traininggrounds-026";
const HOPEFUL_JELLY_BEAN: &str = "traininggrounds-027";
const FLOODGATES: &str = "traininggrounds-030";
const DINO_SIZE: &str = "traininggrounds-034";
const SEANCE: &str = "traininggrounds-037";
/// TG #38 Tutorial Master: a (4) 4/16 Human.
const TUTORIAL_MASTER: &str = "traininggrounds-038";
const ORIENTAL: &str = "traininggrounds-029";
const CALL_TO_CHAOS: &str = "traininggrounds-039";
/// TG #8 Clobberer: a (3) 14/7 Human, the hand card Goliath Situation makes cheap.
const CLOBBERER: &str = "traininggrounds-008";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text, the hand, library and field filler.
const VANILLA: &str = "core-008";
/// Core #16 Hit Job: (3) Spell, "Destroy a Unit."
const HIT_JOB: &str = "core-016";
/// Core #43 Big Felinor: a (4) Unit, dearer than Clobberer.
const BIG_FELINOR: &str = "core-043";

fn library() -> Value {
    json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA])
}

fn decide_at(state: &GameState, seed: &str) -> Decision {
    decide(state, AI, &mut AiOptions::new(create_rng(seed, 0))).expect("the AI acts")
}

/// The instance id of the AI's hand card of `def_id`.
fn in_hand(state: &GameState, def_id: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in the AI's hand")
}

/// The lane the AI's `decision` plays `card` into, when it is a play of it into the unit row.
fn played_lane(decision: &Decision, card: &str) -> Option<i32> {
    match &decision.action {
        ActionBody::Play {
            instance_id,
            zone: Some(zone),
            ..
        } if instance_id == card && zone.row == Row::Units => Some(zone.lane),
        _ => None,
    }
}

/// Every lane the search's root tries `card` in: one per distinct placement (#1635).
fn candidate_lanes(state: &GameState, card: &str) -> Vec<i32> {
    let mut lanes: Vec<i32> = root_candidate_set(state, AI)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                zone: Some(zone),
                ..
            } if instance_id == card && zone.row == Row::Units => Some(zone.lane),
            _ => None,
        })
        .collect();
    lanes.sort_unstable();
    lanes.dedup();
    lanes
}

/// The AI holds `hand` with `mana` and its `field`; the human's field is `enemy_field`.
fn board(seed: &str, hand: Value, mana: i32, field: Value, enemy_field: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": hand, "mana": mana, "field": field, "library": library() },
        "p2": { "hand": [VANILLA], "field": enemy_field, "library": library() },
    }))
    .state()
    .clone()
}

/// The random policy and the turn's timeout (R79) answer `state` too: each picks a legal action,
/// and the timeout's step is accepted.
fn random_and_timeout_answer(state: &GameState, seed: &str) {
    let mut rng = create_rng(seed, 0);
    let random = random_action(state, AI, &mut rng).expect("the random policy acts");
    assert!(is_legal(state, AI, &random), "{random:?}");
    let result = reduce(state, &Action::new(ActionBody::Timeout, AI, "timeout"));
    assert!(result.error.is_none(), "{:?}", result.error);
}

// TG #5 Midlane Menace, #22 Right-Hand Felinor, #16 Celestial Caller: a Cry that reads the lane

#[test]
fn r2203001_the_ai_plays_midlane_menace_into_the_midlane() {
    let state = board(
        "ai-tg-midlane",
        json!([MIDLANE_MENACE]),
        2,
        json!([]),
        json!([{ "def": VANILLA, "lane": 1 }]),
    );
    let menace = in_hand(&state, MIDLANE_MENACE);
    // Lanes 2, 3 and 4 are one placement, and the midlane is the one tried (R685).
    assert_eq!(candidate_lanes(&state, &menace), [1, 3, 5]);
    let decision = decide_at(&state, "ai-tg-midlane");
    assert!(is_legal(&state, AI, &decision.action));
    assert_eq!(
        played_lane(&decision, &menace),
        Some(3),
        "the midlane's +5/+5 and Rush: {:?}",
        decision.action
    );
}

#[test]
fn r2203001_the_ai_plays_right_hand_felinor_to_the_right_of_its_unit() {
    let state = board(
        "ai-tg-right-hand",
        json!([RIGHT_HAND_FELINOR]),
        2,
        json!([{ "def": VANILLA, "lane": 2 }]),
        json!([]),
    );
    let felinor = in_hand(&state, RIGHT_HAND_FELINOR);
    let decision = decide_at(&state, "ai-tg-right-hand");
    assert!(is_legal(&state, AI, &decision.action));
    assert_eq!(
        played_lane(&decision, &felinor),
        Some(3),
        "lane 3 is to the right of the Vanilla in lane 2: {:?}",
        decision.action
    );
}

#[test]
fn r2203001_the_ai_plays_celestial_caller_where_both_its_summons_land() {
    let state = board("ai-tg-caller", json!([CELESTIAL_CALLER]), 4, json!([]), json!([]));
    let caller = in_hand(&state, CELESTIAL_CALLER);
    let decision = decide_at(&state, "ai-tg-caller");
    assert!(is_legal(&state, AI, &decision.action));
    let lane = played_lane(&decision, &caller)
        .unwrap_or_else(|| panic!("a play of the Caller: {:?}", decision.action));
    assert!(
        (2..=4).contains(&lane),
        "lane {lane} leaves one of its two summons with nowhere to land"
    );
    let after = act(&state, AI, &decision.action);
    let units = after.players[AI].units.iter().flatten().count();
    assert_eq!(units, 3, "the Caller and both its summons");
}

// TG #28 Occidental, #29 Oriental: a Cry that sweeps one lane, itself too if it stands there

#[test]
fn r2203001_the_ai_plays_occidental_outside_the_lane_it_sweeps() {
    for (card, swept) in [(OCCIDENTAL, 5), (ORIENTAL, 1)] {
        let seed = format!("ai-tg-sweep-{swept}");
        let state = board(
            &seed,
            json!([card]),
            3,
            json!([]),
            json!([{ "def": VANILLA, "lane": swept }]),
        );
        let unit = in_hand(&state, card);
        assert!(candidate_lanes(&state, &unit).contains(&swept));
        let decision = decide_at(&state, &seed);
        assert!(is_legal(&state, AI, &decision.action));
        let lane = played_lane(&decision, &unit)
            .unwrap_or_else(|| panic!("{card} is played: {:?}", decision.action));
        assert_ne!(lane, swept, "{card} in lane {swept} destroys itself");
        let after = act(&state, AI, &decision.action);
        assert!(
            after.players[HUMAN].units.iter().flatten().next().is_none(),
            "{card} swept the enemy's lane {swept}"
        );
    }
}

// TG #2 Personal Trainer: a declared target, one of your other Humans

#[test]
fn r2203001_the_ai_aims_personal_trainer_at_its_own_human() {
    let state = board(
        "ai-tg-trainer",
        json!([PERSONAL_TRAINER]),
        1,
        json!([{ "def": MIDLANE_MENACE, "lane": 2 }]),
        json!([{ "def": MIDLANE_MENACE, "lane": 4 }]),
    );
    let trainer = in_hand(&state, PERSONAL_TRAINER);
    let own_human = state.players[AI]
        .units
        .iter()
        .flatten()
        .flatten()
        .next()
        .expect("the Human")
        .id
        .clone();
    let mine = [Selection::Instance {
        instance_id: own_human,
    }];
    let aims: Vec<Vec<Selection>> = candidate_actions(&state, AI, Prices::NONE)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id, targets, ..
            } if instance_id == trainer => Some(targets.unwrap_or_default()),
            _ => None,
        })
        .collect();
    assert!(!aims.is_empty());
    assert!(
        aims.iter().all(|aim| *aim == mine),
        "only the AI's own Human: {aims:?}"
    );
    let decision = decide_at(&state, "ai-tg-trainer");
    assert!(is_legal(&state, AI, &decision.action));
    assert!(
        matches!(
            &decision.action,
            ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == trainer && *targets == mine
        ),
        "{:?}",
        decision.action
    );
}

// TG #24 Goliath Situation: a Field Trap the AI sets, and the cheap hand it leaves

#[test]
fn r2203001_the_ai_sets_goliath_situation_and_plays_the_hand_it_makes_cheap() {
    register_cards();
    let state = board(
        "ai-tg-goliath-set",
        json!([GOLIATH_SITUATION]),
        1,
        json!([]),
        json!([]),
    );
    let goliath = in_hand(&state, GOLIATH_SITUATION);
    assert!(
        candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == goliath)),
        "setting it is a candidate"
    );
    let decision = decide_at(&state, "ai-tg-goliath-set");
    assert!(is_legal(&state, AI, &decision.action));
    random_and_timeout_answer(&state, "ai-tg-goliath-set");

    // The human plays a (4) Unit into it: the AI's (3) Clobberer costs (0), and the AI plays it with no mana.
    let mut s = scenario(json!({
        "seed": "ai-tg-goliath",
        "active": "p2",
        "p1": {
            "hand": [CLOBBERER],
            "mana": 0,
            "backrow": [{ "def": GOLIATH_SITUATION, "faceUp": false, "lane": 1 }],
            "library": library(),
        },
        "p2": { "hand": [BIG_FELINOR, VANILLA], "mana": 10, "library": library() },
    }));
    s.play(BIG_FELINOR, json!({ "zone": 1 }));
    s.end_turn();
    let state = s.state().clone();
    let clobberer = in_hand(&state, CLOBBERER);
    assert_eq!(state.active, AI);
    let decision = decide_at(&state, "ai-tg-goliath");
    assert!(is_legal(&state, AI, &decision.action));
    assert!(
        matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == clobberer),
        "the (0) Clobberer: {:?}",
        decision.action
    );
}

// TG #39 Call to Chaos (Training-Grounds Edition): each of its ten entries, then the AI plays on

#[test]
fn r2203001_after_each_call_to_chaos_entry_the_ai_finishes_its_turn_cleanly() {
    register_cards();
    for entry in subsystems::CHAOS_TG_EFFECTS {
        let seed = format!("ai-tg-chaos-{}", entry.name);
        let cursor = cursor_where(&seed, |cursor| {
            let mut rng = Rng::new(&seed, cursor);
            subsystems::roll_chaos_effects(&mut rng, false, Some(subsystems::CHAOS_TG_EFFECTS))
                .first()
                .is_some_and(|rolled| rolled.name == entry.name)
        });
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [CALL_TO_CHAOS, VANILLA], "mana": 10, "library": library() },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": library() },
        }));
        s.state_mut().rng_cursor = cursor;
        s.play(CALL_TO_CHAOS, json!({}));
        let state = s.state().clone();
        if state.result.is_some() {
            continue;
        }
        let mut options = AiOptions::new(create_rng(&seed, 0));
        let turn = play_ai_turn(&state, AI, &mut options);
        for decision in &turn.decisions {
            assert_ne!(
                decision.reason,
                DecisionReason::Fallback,
                "{}: {:?}",
                entry.name,
                decision.action
            );
            assert_eq!(decision.first_sim_panic, None, "{}", entry.name);
        }
        assert!(
            turn.state.result.is_some() || turn.state.active == HUMAN,
            "{}: the AI's turn ends",
            entry.name
        );
    }
}

// TG #21 Flame Tongue Totem: an aura on the Units beside it

#[test]
fn r2203001_the_ai_plays_flame_tongue_totem_beside_its_unit() {
    let state = board(
        "ai-tg-totem",
        json!([FLAME_TONGUE_TOTEM]),
        1,
        json!([{ "def": VANILLA, "lane": 4 }]),
        json!([]),
    );
    let totem = in_hand(&state, FLAME_TONGUE_TOTEM);
    let decision = decide_at(&state, "ai-tg-totem");
    assert!(is_legal(&state, AI, &decision.action));
    let lane = played_lane(&decision, &totem)
        .unwrap_or_else(|| panic!("a play of the Totem: {:?}", decision.action));
    assert!(
        lane == 3 || lane == 5,
        "lane {lane} is not beside the Vanilla in lane 4"
    );
}

// TG #30 Floodgates: its Death destroys every Unit of its side

#[test]
fn r2203001_the_ai_kills_an_enemy_floodgates_so_its_death_destroys_their_units() {
    let state = board(
        "ai-tg-floodgates",
        json!([HIT_JOB]),
        3,
        json!([]),
        json!([
            { "def": VANILLA, "lane": 1 },
            { "def": FLOODGATES, "lane": 3 },
            { "def": VANILLA, "lane": 5 },
        ]),
    );
    let floodgates = state.players[HUMAN]
        .units
        .iter()
        .flatten()
        .flatten()
        .find(|card| card.def_id == FLOODGATES)
        .expect("the Floodgates")
        .id
        .clone();
    let hit_job = in_hand(&state, HIT_JOB);
    let decision = decide_at(&state, "ai-tg-floodgates");
    assert!(is_legal(&state, AI, &decision.action));
    assert!(
        matches!(
            &decision.action,
            ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == hit_job
                    && *targets == [Selection::Instance { instance_id: floodgates.clone() }]
        ),
        "the Hit Job at the Floodgates: {:?}",
        decision.action
    );
    let after = act(&state, AI, &decision.action);
    assert!(
        after.players[HUMAN].units.iter().flatten().next().is_none(),
        "its Death destroyed the enemy's other Units"
    );
}

// TG #11 Thwomp, #26 Path of Exile, #34 Dino-size, #12 Fortify, #27 Hopeful Jelly Bean, #37 Seance:
// one declared target, which the AI picks by what the play leaves

/// The id of `player`'s Unit in `lane`.
fn unit_in(state: &GameState, player: PlayerId, lane: i32) -> String {
    card_at(
        state,
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        },
    )
    .map(|card| card.id.clone())
    .unwrap_or_else(|| panic!("no Unit in {player:?}'s lane {lane}"))
}

/// The AI's decision plays `card` at `target`, legally; the random policy and a timeout answer too.
fn aims(state: &GameState, seed: &str, card: &str, target: &str) {
    let spell = in_hand(state, card);
    let decision = decide_at(state, seed);
    assert!(is_legal(state, AI, &decision.action));
    assert!(
        matches!(
            &decision.action,
            ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == spell
                    && *targets == [Selection::Instance { instance_id: target.to_string() }]
        ),
        "{card} at {target}: {:?}",
        decision.action
    );
    random_and_timeout_answer(state, seed);
}

#[test]
fn r2203001_the_ai_spends_thwomp_on_the_unit_it_most_needs_gone() {
    let state = board(
        "ai-tg-thwomp",
        json!([THWOMP]),
        2,
        json!([]),
        json!([{ "def": VANILLA, "lane": 1 }, { "def": CLOBBERER, "lane": 3 }]),
    );
    aims(&state, "ai-tg-thwomp", THWOMP, &unit_in(&state, HUMAN, 3));
}

#[test]
fn r2203001_the_ai_exiles_the_big_unit_with_path_of_exile() {
    let state = board(
        "ai-tg-path",
        json!([PATH_OF_EXILE]),
        4,
        json!([]),
        json!([{ "def": VANILLA, "lane": 1 }, { "def": CLOBBERER, "lane": 3 }]),
    );
    aims(&state, "ai-tg-path", PATH_OF_EXILE, &unit_in(&state, HUMAN, 3));
}

#[test]
fn r2203001_the_ai_dino_sizes_its_own_small_unit_not_the_enemys() {
    let state = board(
        "ai-tg-dino",
        json!([DINO_SIZE]),
        3,
        json!([{ "def": VANILLA, "lane": 2 }]),
        json!([{ "def": VANILLA, "lane": 4 }]),
    );
    aims(&state, "ai-tg-dino", DINO_SIZE, &unit_in(&state, AI, 2));
}

#[test]
fn r2203001_the_ai_fortifies_its_own_unit() {
    let state = board(
        "ai-tg-fortify",
        json!([FORTIFY]),
        1,
        json!([{ "def": VANILLA, "lane": 2 }]),
        json!([{ "def": VANILLA, "lane": 4 }]),
    );
    aims(&state, "ai-tg-fortify", FORTIFY, &unit_in(&state, AI, 2));
}

#[test]
fn r2203001_the_ai_makes_its_own_unit_radiant_with_hopeful_jelly_bean() {
    let state = board(
        "ai-tg-jelly-bean",
        json!([HOPEFUL_JELLY_BEAN]),
        4,
        json!([{ "def": TUTORIAL_MASTER, "lane": 2 }]),
        json!([{ "def": TUTORIAL_MASTER, "lane": 4 }]),
    );
    aims(
        &state,
        "ai-tg-jelly-bean",
        HOPEFUL_JELLY_BEAN,
        &unit_in(&state, AI, 2),
    );
}

#[test]
fn r2203001_the_ai_copies_a_permanent_with_seance() {
    let state = board(
        "ai-tg-seance",
        json!([SEANCE, VANILLA]),
        6,
        json!([]),
        json!([{ "def": CLOBBERER, "lane": 3 }]),
    );
    let seance = in_hand(&state, SEANCE);
    let decision = decide_at(&state, "ai-tg-seance");
    assert!(is_legal(&state, AI, &decision.action));
    random_and_timeout_answer(&state, "ai-tg-seance");
    // Played, the copy is the AI's: a Clobberer in its hand.
    let clobberer = unit_in(&state, HUMAN, 3);
    let play = candidate_actions(&state, AI, Prices::NONE)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == seance))
        .expect("Seance is a candidate");
    let ActionBody::Play {
        targets: Some(targets),
        ..
    } = &play
    else {
        panic!("Seance declares its permanent: {play:?}");
    };
    assert_eq!(
        targets.as_slice(),
        [Selection::Instance {
            instance_id: clobberer
        }]
    );
    let after = act(&state, AI, &play);
    assert!(after.players[AI].hand.iter().any(|card| card.def_id == CLOBBERER));
}
