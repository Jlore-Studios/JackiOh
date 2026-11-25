//! R4447000 (issue #4447): a finalist's line replayed on another determinization meets a step that world
//! does not offer when the line's own earlier step drew a card that is another card there (each world
//! shuffles the seat's own library) or rolled otherwise (each world has a seed of its own). `replay_line`
//! reports that cut and `decide` counts it in `SearchStats.replaysCut`; the line ends its turn at the cut.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, ai_options, det, ids, js};

/// Stockpile (#5): (1) Draw 2, heal 2.
const STOCKPILE: &str = "core-005";
/// Mr. Vanilla (#8): (1) 4/4.
const VANILLA: &str = "core-008";

/// The AI's turn 5 at 3 crystals, Stockpile and `extra` in hand over a library of 1-drops and a few
/// dearer cards, against one 4/4.
fn draw_turn(extra: &[&str]) -> GameState {
    jackioh_cards::register_all();
    let mut hand = vec![STOCKPILE];
    hand.extend_from_slice(extra);
    scenario(json!({
        "seed": "replay-fill",
        "active": "p1",
        "turn": 5,
        "p1": {
            "hand": hand,
            "mana": 3,
            "library": ["core-011", "core-003", "core-015", "core-001", "core-008", "core-025"],
        },
        "p2": { "field": ["core-008"], "library": ["core-008", "core-011"], "health": 20 },
    }))
    .state()
    .clone()
}

/// The candidate that plays the AI's card `id`.
fn play_instance(state: &GameState, id: &str) -> ActionBody {
    candidate_actions(state, AI, Prices::NONE)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
        .unwrap_or_else(|| panic!("{id} is not a candidate"))
}

/// `draw_turn(extra)` determinized twice for the AI, worlds A and B, each with its own order of the
/// AI's library, and the line [Stockpile, the first card Stockpile drew on A that B did not draw] found
/// on A, which B does not offer.
fn cut_scene(extra: &[&str]) -> (Vec<ActionBody>, GameState, GameState) {
    let public = redact(&draw_turn(extra), AI);
    let mut rng = create_rng("replay-fill-worlds-3", 0);
    let a = det(&public, AI, &mut rng);
    let b = det(&public, AI, &mut rng);
    let stockpile_id = a.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == STOCKPILE)
        .map(|card| card.id.clone())
        .unwrap_or_else(|| panic!("Stockpile is not in hand"));
    let stockpile = play_instance(&a, &stockpile_id);
    let before = ids(&a.players[AI].hand);
    let drawn = act(&a, AI, &stockpile);
    let candidates = candidate_actions(&drawn, AI, Prices::NONE);
    let offered_on_b = candidate_set(&act(&b, AI, &stockpile), AI);
    let play_drawn = ids(&drawn.players[AI].hand)
        .into_iter()
        .filter(|id| !before.contains(id))
        .find_map(|id| {
            candidates
                .iter()
                .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
                .filter(|action| !offered_on_b.contains(action))
                .cloned()
        })
        .unwrap_or_else(|| panic!("B offers every card Stockpile drew on A"));
    (vec![stockpile, play_drawn], a, b)
}

/// `line` replayed on `world` on a fresh counter of `AI_BUDGET.nodes`, scored statically, and the nodes
/// it spent.
fn replayed(world: &GameState, line: &[ActionBody]) -> (LineReplay, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let replay = replay_line(world, AI, line, &counter, None, Prices::NONE)
        .unwrap_or_else(|| panic!("the counter ran out"));
    (replay, counter.used())
}

/// R4447000: a decision is node for node what it was before its replays' cuts were counted: its
/// action, its line and its nodes are the base's.
#[test]
fn r4447000_a_pinned_decision_keeps_its_action_line_and_nodes() {
    // A recorded decision, so it reads the pools as they are, not as the pool probe (#3117) rotates them:
    // each determinization samples the hidden cards from them, and a rotation deals another search.
    let decision = with_pool_shift(0, || {
        decide(
            &draw_turn(&[VANILLA]),
            AI,
            &mut ai_options("replay-fill-decide", AI_BUDGET),
        )
    })
    .unwrap_or_else(|| panic!("the AI has no decision"));
    let pinned = (
        action_key(&decision.action),
        decision.line.iter().map(action_key).collect::<Vec<_>>(),
        decision.stats.nodes,
    );
    // Recorded on the base before R4447000 (3af8296300): the 4/4 first, then Stockpile, then a card
    // Stockpile drew on determinization 0.
    let vanilla = r#"{"instanceId":"c2","type":"play","zone":{"lane":1,"row":"units"}}"#;
    let line = [
        vanilla,
        r#"{"instanceId":"c1","type":"play"}"#,
        r#"{"instanceId":"c2","type":"switchPosition"}"#,
        r#"{"instanceId":"c5","type":"play","zone":{"lane":2,"row":"units"}}"#,
    ];
    assert_eq!(
        pinned,
        (
            vanilla.to_string(),
            line.iter().map(|key| key.to_string()).collect(),
            137
        )
    );
}

/// R4447000: the same decision counts the finalist replays its other worlds cut: a finalist that plays a
/// card Stockpile drew on determinization 0 meets a step a world that drew other cards does not offer.
#[test]
fn r4447000_a_decision_counts_the_finalist_replays_its_worlds_cut() {
    let decision = decide(
        &draw_turn(&[VANILLA]),
        AI,
        &mut ai_options("replay-fill-decide", AI_BUDGET),
    )
    .unwrap_or_else(|| panic!("the AI has no decision"));
    let stats = &decision.stats;
    assert_eq!(stats.worlds_scored, AI_BUDGET.determinizations);
    assert!(stats.replays_cut >= 1, "{stats:?}");
    assert!(
        stats.replays_cut <= AI_BUDGET.finalists * (AI_BUDGET.determinizations - 1),
        "{stats:?}"
    );
    assert_eq!(stats.replays_cut, 4);
    assert_eq!(js(stats)["replaysCut"], json!(stats.replays_cut));
}

/// R4447000: a line that plays a card it drew is cut on a world that drew other cards, and not on the
/// world it was found on; `score_line` is `replay_line`'s score. A counter that runs out before the
/// replay's closing endTurn ends it with no score.
#[test]
fn r4447000_a_line_that_plays_a_card_it_drew_is_cut_on_a_world_that_drew_another() {
    let (line, a, b) = cut_scene(&[VANILLA]);
    let (cut, used) = replayed(&b, &line);
    assert!(cut.cut);
    assert!(!replayed(&a, &line).0.cut);
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    assert_eq!(
        score_line(&b, AI, &line, &counter, false, None, Prices::NONE),
        Some(cut.score)
    );
    let short = create_node_counter(used - 1, None);
    assert_eq!(replay_line(&b, AI, &line, &short, None, Prices::NONE), None);
}
