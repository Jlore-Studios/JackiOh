//! Face-down plays through the AI's eyes (R1046, R783009, R4237000, R4327000).
//!
//! A set card redacts to a placeholder with no `setAs`, and `determinize` samples for a hidden backrow
//! card a Trap, a card that prints Cloaked whatever acts (R783009), or a card an aura lends Cloaked to
//! in that side's hand now: every Unit while Knowledge Breaker acts there (R1046). A Unit the AI reveals
//! is summoning sick that turn, so it plans an attack by it only with Rush or Charge (R4237000). A card a
//! summon set face-down by its Cloaked (S+ #76 Ayys LMAO) is offered its Reveal like a played one, and
//! which card it is changes no decision of its opponent's (R4327000).

use jackioh_ai::{
    AI_BUDGET, AiOptions, DeterminizeOptions, decide, determinize, lethal_hint, play_ai_turn, redact,
};
use jackioh_engine::testkit::{
    ActionBody, CardScripts, CardType, GameState, Keyword, RevealAt, Row, ZoneChoice, create_rng, find_def,
    json, legal_actions, register_catalog, register_scripts, registered_catalog, registered_scripts,
};

use super::support::{AI, HUMAN, ai_options, register_cards, run_puzzle, scenario, trace};

const BREAKER: &str = "meditative-045";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";

/// p1 with a face-up Breaker acting and a vanilla set by the Cloaked it lends in the backrow, seen by p2.
fn set_state(seed: &str) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": {
            "mana": 8,
            "hand": [VANILLA, VANILLA],
            "field": [{ "def": BREAKER, "lane": 1 }],
        },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let unit = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == VANILLA)
        .cloned()
        .expect("the vanilla");
    state = super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: unit.id,
            zone: Some(ZoneChoice {
                row: Row::Backrow,
                lane: 1,
            }),
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(RevealAt::Cloaked),
        },
    );
    state
}

fn backrow_def(state: &GameState) -> String {
    state.players[AI].backrow[0]
        .as_ref()
        .expect("a set card")
        .def_id
        .clone()
}

#[test]
fn r1046_redact_drops_set_as_and_determinize_samples_units_only_under_the_breaker() {
    let state = set_state("alt-play-redact");
    let set = state.players[AI].backrow[0].as_ref().expect("a set card");
    assert!(set.set_as.is_some());

    let public = redact(&state, HUMAN);
    let hidden = public.players[AI].backrow[0].as_ref().expect("the placeholder");
    assert_eq!(hidden.set_as, None);
    let js = serde_json::to_value(hidden).expect("a placeholder serializes");
    assert!(js.get("setAs").is_none());

    // Determinized worlds may hold a Unit while the Breaker acts, stamped `cloaked`...
    let options = DeterminizeOptions {
        match_shown_cost: Some(false),
    };
    let mut units = 0;
    for n in 0..50 {
        let mut rng = create_rng(&format!("alt-play-det-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = world.players[AI].backrow[0].clone().expect("a sampled card");
        let def = find_def(Some(&world), &sampled.def_id).expect("a sampled def");
        if def.type_ == CardType::Unit {
            units += 1;
            assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        } else {
            assert!(
                def.type_ == CardType::Trap
                    || def.type_ == CardType::FieldTrap
                    || prints_cloaked(&world, &sampled.def_id),
                "a Trap or a card that prints Cloaked, got {:?}",
                def.type_
            );
        }
    }
    assert!(units > 0, "no Unit sampled in 50 worlds under the Breaker");

    // ...and without it, only a Trap or a card that prints Cloaked.
    register_cards();
    let plain = scenario(json!({
        "seed": "alt-play-no-grant",
        "active": "p1",
        "p1": { "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let public = redact(&plain, HUMAN);
    for n in 0..50 {
        let mut rng = create_rng(&format!("alt-play-plain-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = backrow_def(&world);
        let def = find_def(Some(&world), &sampled).expect("a sampled def");
        assert!(
            def.type_ == CardType::Trap
                || def.type_ == CardType::FieldTrap
                || prints_cloaked(&world, &sampled),
            "a Unit that prints no Cloaked sampled with no Breaker: {sampled}",
        );
    }

    let offered: Vec<_> = legal_actions(&state, AI)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                ActionBody::Play {
                    face_down: Some(RevealAt::Cloaked),
                    ..
                }
            )
        })
        .collect();
    assert!(!offered.is_empty());
}

/// Whether the definition prints Cloaked on a face (Cloaked Toe Cracker does, R783009).
fn prints_cloaked(state: &GameState, def_id: &str) -> bool {
    let def = find_def(Some(state), def_id).expect("a def");
    [&def.base, &def.radiant]
        .into_iter()
        .any(|face| face.keywords.contains(&Keyword::Cloaked))
}

/// R783009: a vanilla that prints Cloaked, registered beside the real cards on this thread only.
const CLOAKED: &str = "ai-cloaked";

fn register_cloaked() {
    register_cards();
    let mut def = find_def(None, VANILLA).expect("the vanilla").clone();
    def.id = CLOAKED.to_string();
    def.index = "783".to_string();
    def.base.keywords = vec![Keyword::Cloaked];
    def.radiant.keywords = vec![Keyword::Cloaked];
    let mut defs = registered_catalog().clone();
    defs.insert(def.id.clone(), def);
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    scripts.insert(CLOAKED.to_string(), CardScripts::default());
    register_scripts(scripts);
}

/// R4237000: `CLOAKED` with Charge too.
const CLOAKED_CHARGE: &str = "ai-cloaked-charge";

fn register_cloaked_charge() {
    register_cloaked();
    let mut def = find_def(None, CLOAKED).expect("the Cloaked vanilla").clone();
    def.id = CLOAKED_CHARGE.to_string();
    def.index = "4237".to_string();
    def.base.keywords = vec![Keyword::Cloaked, Keyword::Charge];
    def.radiant.keywords = vec![Keyword::Cloaked, Keyword::Charge];
    let mut defs = registered_catalog().clone();
    defs.insert(def.id.clone(), def);
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    scripts.insert(CLOAKED_CHARGE.to_string(), CardScripts::default());
    register_scripts(scripts);
}

/// R4237000: p1, the AI, set `def_id` (a 4/4) face-down by its Cloaked on its turn 7, and both seats
/// ended their turns: p1's turn 9, p2's hero at 4 with no Unit. The state and the set card's id.
fn set_two_turns_ago(seed: &str, def_id: &str) -> (GameState, String) {
    let state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "turn": 7,
        "p1": { "mana": 8, "hand": [def_id], "library": [VANILLA, VANILLA, VANILLA] },
        "p2": { "health": 4, "library": [VANILLA, VANILLA, VANILLA] },
    }))
    .state()
    .clone();
    let card = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .cloned()
        .expect("the Cloaked card");
    let set = super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: card.id,
            zone: Some(ZoneChoice {
                row: Row::Backrow,
                lane: 1,
            }),
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(RevealAt::Cloaked),
        },
    );
    let set_id = set.players[AI].backrow[0].clone().expect("the set card").id;
    let handed = super::support::act(&set, AI, ActionBody::EndTurn);
    let back = super::support::act(&handed, HUMAN, ActionBody::EndTurn);
    assert_eq!((back.active, back.turn), (AI, 9));
    (back, set_id)
}

/// Whether the AI's turn declared, or planned in a line, an attack by `attacker`.
fn planned_attack_by(turn: &jackioh_ai::AiTurnResult, attacker: &str) -> bool {
    turn.decisions.iter().any(|decision| {
        std::iter::once(&decision.action)
            .chain(&decision.line)
            .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == attacker))
    })
}

/// R4237000: a Unit the AI reveals from Cloaked is summoning sick that turn, so with neither Rush nor
/// Charge its 4 Attack is no lethal on a hero at 4, and the AI plans no attack by it.
#[test]
fn r4237000_the_ai_plans_no_attack_by_a_unit_it_reveals_this_turn() {
    register_cloaked();
    let (start, set_id) = set_two_turns_ago("r4237000-ai-sick", CLOAKED);
    assert!(!lethal_hint(&start, AI, None));
    let turn = play_ai_turn(&start, AI, &mut ai_options("r4237000-ai-sick", AI_BUDGET));
    assert!(!planned_attack_by(&turn, &set_id), "{}", trace(&turn));
    assert!(turn.state.result.is_none(), "{}", trace(&turn));
    assert_eq!(turn.state.players[HUMAN].hero.health, 4, "{}", trace(&turn));
}

/// R4237000: with Charge the revealed Unit may attack the hero at once, and the AI reveals it for lethal.
#[test]
fn r4237000_the_ai_reveals_a_charge_unit_and_attacks_the_hero_for_lethal() {
    register_cloaked_charge();
    let (start, set_id) = set_two_turns_ago("r4237000-ai-charge", CLOAKED_CHARGE);
    assert!(lethal_hint(&start, AI, None));
    let turn = play_ai_turn(&start, AI, &mut ai_options("r4237000-ai-charge", AI_BUDGET));
    assert_eq!(
        turn.state.result.and_then(|result| result.winner.player()),
        Some(AI),
        "{}",
        trace(&turn)
    );
    assert!(
        turn.decisions.iter().any(|decision| matches!(&decision.action,
            ActionBody::Attack { attacker_id, .. } if *attacker_id == set_id)),
        "{}",
        trace(&turn)
    );
}

#[test]
fn r783009_determinize_samples_a_printed_cloaked_card_stamped_cloaked() {
    register_cloaked();
    let mut state = scenario(json!({
        "seed": "r783009-cloaked",
        "active": "p1",
        "p1": { "mana": 8, "hand": [CLOAKED, VANILLA] },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let card = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == CLOAKED)
        .cloned()
        .expect("the Cloaked card");
    // No permission acts: Cloaked alone offers the face-down play.
    assert!(legal_actions(&state, AI).iter().any(|action| matches!(
        action,
        ActionBody::Play { instance_id, face_down: Some(RevealAt::Cloaked), .. } if *instance_id == card.id
    )));
    state = super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: card.id,
            zone: Some(ZoneChoice {
                row: Row::Backrow,
                lane: 1,
            }),
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(RevealAt::Cloaked),
        },
    );
    let set = state.players[AI].backrow[0].clone().expect("the set card");
    assert_eq!(set.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
    assert!(legal_actions(&state, AI).iter().any(|action| matches!(
        action,
        ActionBody::Activate { instance_id, ability, .. } if *instance_id == set.id && ability.as_deref() == Some("reveal")
    )));

    // The opponent's worlds may hold it, stamped `cloaked`; every other sample is a Trap, unstamped.
    let public = redact(&state, HUMAN);
    let options = DeterminizeOptions {
        match_shown_cost: Some(false),
    };
    let mut cloaked = 0;
    for n in 0..300 {
        let mut rng = create_rng(&format!("r783009-det-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = world.players[AI].backrow[0].clone().expect("a sampled card");
        if sampled.def_id == CLOAKED {
            cloaked += 1;
            assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        } else {
            let def = find_def(Some(&world), &sampled.def_id).expect("a sampled def");
            if prints_cloaked(&world, &sampled.def_id) {
                // Another card that prints Cloaked (Cloaked Toe Cracker), stamped the same way.
                assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
            } else {
                assert!(
                    def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap,
                    "a non-Trap sampled with no aura: {}",
                    sampled.def_id
                );
                assert_eq!(sampled.set_as, None);
            }
        }
    }
    assert!(cloaked > 0, "the Cloaked card was never sampled in 300 worlds");
}

/// R3806000: under Paranoia a set Spell resolves as it is set and again as it reveals, and the AI counts
/// both: p2 at 6, Lunar Eclipse's 3 played face-up is no lethal, set and revealed it is.
#[test]
fn r3806000_the_ai_sets_a_spell_under_paranoia_and_reveals_it_for_lethal() {
    let run = run_puzzle(
        "paranoia-set-lethal",
        json!({
            "p1": {
                "mana": 1,
                "hand": ["core-035"],
                "field": [{ "def": "meditative-099", "lane": 1 }],
                "library": [VANILLA, VANILLA],
            },
            "p2": { "health": 6, "library": [VANILLA, VANILLA] },
        }),
    );
    assert_eq!(
        run.end.result.and_then(|result| result.winner.player()),
        Some(AI),
        "{}",
        trace(&run.turn)
    );
    let first = run.turn.decisions.first().map(|decision| decision.action.clone());
    assert!(
        matches!(
            first,
            Some(ActionBody::Play {
                face_down: Some(RevealAt::Cloaked),
                ..
            })
        ),
        "{}",
        trace(&run.turn)
    );
}

/// S+ #76 Ayys LMAO, its Ayy token and the Tiny Asteroid it may summon instead (R4327002).
const AYYS_LMAO: &str = "stinkyscapeplus-076";
const AYY: &str = "stinkyscapeplus-076-1";
const ASTEROID: &str = "stinkyscape-169";

/// p1, the AI, cast Ayys LMAO on its turn: its three summons set face-down in its backrow (R4327000).
fn ayys_cast(seed: &str) -> GameState {
    register_cards();
    let state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": { "mana": 8, "hand": [AYYS_LMAO], "library": [VANILLA, VANILLA, VANILLA] },
        "p2": { "mana": 8, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }))
    .state()
    .clone();
    let spell = state.players[AI].hand[0].id.clone();
    super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: spell,
            zone: None,
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: None,
        },
    )
}

#[test]
fn r4327000_the_ai_is_offered_each_reveal_and_its_opponent_sees_placeholders() {
    let state = ayys_cast("r4327000-ai-reveal");
    let set: Vec<String> = state.players[AI]
        .backrow
        .iter()
        .flatten()
        .filter(|card| card.set_as.map(|stamp| stamp.reveal) == Some(RevealAt::Cloaked))
        .map(|card| card.id.clone())
        .collect();
    assert_eq!(set.len(), 3, "three set summons");
    let offered = legal_actions(&state, AI);
    for id in &set {
        assert!(
            offered.iter().any(|action| matches!(action,
                ActionBody::Activate { instance_id, ability, .. }
                    if instance_id == id && ability.as_deref() == Some("reveal"))),
            "the AI may reveal {id}"
        );
    }

    let public = redact(&state, HUMAN);
    for id in &set {
        let hidden = public.players[AI]
            .backrow
            .iter()
            .flatten()
            .find(|card| card.id == *id)
            .expect("the placeholder");
        assert_eq!(hidden.set_as, None);
        assert!(
            hidden.def_id != AYY && hidden.def_id != ASTEROID,
            "{}",
            hidden.def_id
        );
        assert!(!hidden.radiant);
        assert!(
            hidden.granted_keywords.is_empty(),
            "the asteroid's Cloaked is hidden too"
        );
    }
}

#[test]
fn r4327000_which_card_each_set_summon_is_changes_no_decision_of_its_opponent() {
    let a = super::support::act(&ayys_cast("r4327000-ai-blind"), AI, ActionBody::EndTurn);
    assert_eq!(a.active, HUMAN);
    // The same backs, each the other card: an Ayy for an asteroid given Cloaked, and back.
    let mut b = a.clone();
    for card in b.players[AI].backrow.iter_mut().flatten() {
        let ayy = card.def_id == AYY;
        card.def_id = if ayy { ASTEROID } else { AYY }.to_string();
        card.radiant = false;
        card.granted_keywords = if ayy { vec![Keyword::Cloaked] } else { Vec::new() };
    }
    assert_ne!(a.players[AI].backrow, b.players[AI].backrow);

    let decided = |state: &GameState| {
        decide(state, HUMAN, &mut AiOptions::new(create_rng("r4327000-ai", 0)))
            .map(|decision| (decision.action, decision.line))
    };
    let (from_a, from_b) = (decided(&a), decided(&b));
    assert!(from_a.is_some());
    assert_eq!(from_a, from_b);
}
