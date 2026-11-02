//! The Starship in the AI's reads (R1852000, R1852003): the launch is the engine's Activate, so it comes
//! to the AI through `legal_actions` like any ability, and the prompts its fused Cries open are ordinary
//! prompts, which `decide` and the random policy answer; the dense features leave the two new keyword
//! kinds out, so the compiled network's inputs keep their layout.
//!
//! The file installs two test-only cards on top of the real catalog (Stinkyscape's shipped pieces are
//! priced in `deaths.rs`, R2566000): the Dormant Starship under the id the engine summons, and a 1/1
//! piece whose Cry deals 1 damage to a Unit. The AI's piece dies attacking the human's Mr. Vanilla
//! (4/4), which builds the AI a Dormant Starship.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

/// The 1/1 piece.
const PIECE: &str = "ai-test-starship-piece";

fn dormant_starship_def() -> CardDef {
    json_as(json!({
        "id": DORMANT_STARSHIP_DEF_ID,
        "index": "ai-test-starship-1",
        "name": "Dormant Starship (AI test)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Legendary",
        "token": false,
        "cost": 3,
        "base": { "attack": 0, "health": 0, "keywords": [], "text": "Dormant. Activate: pay (3) to launch." },
        "radiant": { "attack": 0, "health": 0, "keywords": [], "text": "Dormant. Activate: pay (3) to launch." },
    }))
}

fn piece_def() -> CardDef {
    json_as(json!({
        "id": PIECE,
        "index": "ai-test-starship-2",
        "name": "Gunner Piece (AI test)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Token",
        "token": true,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Starship Piece" }], "text": "Starship Piece. Cry: deal 1 damage to a Unit." },
        "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Starship Piece" }], "text": "Starship Piece. Cry: deal 2 damage to a Unit." },
    }))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// The two cards, registered for this thread on top of the real catalog.
fn install() {
    register_cards();
    let mut catalog = (*jackioh_cards::CATALOG).clone();
    catalog.insert(DORMANT_STARSHIP_DEF_ID.to_string(), dormant_starship_def());
    catalog.insert(PIECE.to_string(), piece_def());
    register_catalog_as(catalog, jackioh_cards::catalog_version());
    let mut scripts = jackioh_cards::scripts_of();
    scripts.insert(
        DORMANT_STARSHIP_DEF_ID.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                slumber: Some(SlumberText::default()),
                starship_hull: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        PIECE.to_string(),
        both(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

/// The AI's turn 9, its piece dead against the human's Mr. Vanilla: the AI holds a Dormant Starship
/// and nothing in hand, so its actions are the launch, an end of turn, and little else.
fn built() -> (GameState, String) {
    install();
    let library = json!(["core-008", "core-011", "core-008", "core-011"]);
    let mut s = scenario(json!({
        "seed": "ai-starship",
        "p1": { "hand": [], "field": [{ "def": PIECE, "lane": 1 }], "library": library },
        "p2": { "hand": ["core-011"], "field": ["core-008"], "library": library },
    }));
    let piece = s.unit(AI, 1).expect("the piece").id;
    let vanilla = s.unit(HUMAN, 1).expect("Mr. Vanilla").id;
    s.attack(piece.as_str(), vanilla.as_str());
    let ship = dormant_starship_of(s.state(), AI)
        .expect("the AI's Dormant Starship")
        .id
        .clone();
    (s.state().clone(), ship)
}

fn is_launch(action: &ActionBody, ship: &str) -> bool {
    matches!(action, ActionBody::Activate { instance_id, ability: Some(ability), .. }
        if instance_id == ship && ability == STARSHIP_LAUNCH_ABILITY)
}

fn apply(state: &GameState, body: ActionBody, player: PlayerId, nonce: &str) -> GameState {
    let result = reduce(
        state,
        &ActionInput {
            body,
            player_id: player,
        }
        .with_nonce(nonce),
    );
    assert_eq!(result.error, None, "{nonce}");
    result.state
}

#[test]
fn r1852003_the_launch_is_a_candidate_the_random_policy_takes_and_the_ai_answers_its_cry() {
    let (state, ship) = built();
    assert!(
        candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .any(|action| is_launch(action, &ship)),
        "the launch is among the AI's candidates"
    );
    let launched_by_policy = (0..32).any(|cursor| {
        let mut rng = create_rng("ai-starship-policy", cursor);
        subsystems::ai_policy::choose_action(&state, AI, &mut rng)
            .is_some_and(|action| is_launch(&action, &ship))
    });
    assert!(launched_by_policy, "the random policy draws the launch");
    // Launched, the piece's fused Cry asks the AI for its target; `decide` answers it.
    let paused = apply(&state, launch(&ship), AI, "launch");
    let pending = paused.pending.clone().expect("the Cry's target prompt");
    assert_eq!(pending.player_id, AI);
    let decision =
        decide(&paused, AI, &mut AiOptions::new(create_rng("ai-starship", 0))).expect("a decision");
    assert!(
        matches!(decision.action, ActionBody::Answer { .. }),
        "{:?}",
        decision.action
    );
    let answered = apply(&paused, decision.action, AI, "answer");
    assert!(answered.pending.is_none());
    assert!(is_launched_starship(
        find_instance(&answered, &ship).expect("the Starship")
    ));
}

fn launch(ship: &str) -> ActionBody {
    ActionBody::Activate {
        instance_id: ship.to_string(),
        ability: Some(STARSHIP_LAUNCH_ABILITY.to_string()),
        targets: None,
        modes: None,
        tributes: None,
    }
}

#[test]
fn r1852000_the_dense_features_keep_their_layout_with_a_starship_on_the_board() {
    let (state, ship) = built();
    // The Dormant Starship is no Unit (`active_units_of`), and the launched one carries the Starship
    // keyword, which no dense feature counts: the layout is the compiled network's either way.
    assert!(active_units_of(&state, AI).iter().all(|unit| unit.id != ship));
    assert_eq!(features_of(&state, AI).dense.len(), dense_len());
    let paused = apply(&state, launch(&ship), AI, "launch");
    assert!(
        unit_view(&paused, find_instance(&paused, &ship).expect("the Starship"))
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Starship)
    );
    assert_eq!(features_of(&paused, AI).dense.len(), dense_len());
    assert!(
        !dense_names().iter().any(|name| name.contains("Starship")),
        "no dense feature names a Starship keyword"
    );
    // And the search tells a Dormant Starship's position apart from the board without it, standing in
    // the middle of the backrow (R2249000).
    let lane = STARSHIP_LANE_ORDER[0] as usize - 1;
    let mut without = state.clone();
    assert_eq!(
        without.players[AI].backrow[lane]
            .as_ref()
            .map(|card| card.id.clone()),
        Some(ship.clone())
    );
    without.players[AI].backrow[lane] = None;
    assert_ne!(search_signature(&state, AI), search_signature(&without, AI));
}
