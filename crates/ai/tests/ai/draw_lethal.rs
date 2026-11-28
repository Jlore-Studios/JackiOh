//! R4697000 (issue #4697): with `AI_SEARCH.replay_fill_steps` at 3, a finalist's line that draws a card
//! and then plays it, cut on a determinization whose shuffle dealt the line another card, plays that
//! world's turn on (`fill_turn`, R4447000) instead of ending it with its mana unspent, so R4608000's
//! check of a win on determinization 0 alone sees a win every draw gives.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, ai_options, det, ids, trace};

/// Stockpile (#5): (1) Draw 2, heal 2.
const STOCKPILE: &str = "core-005";
/// Mr. Vanilla (#8): (1) 4/4.
const VANILLA: &str = "core-008";
/// True Strike (#44): (1) Pierce, deal 4 damage, exile this.
const TRUE_STRIKE: &str = "core-044";
/// The True Strikes the AI's library holds, all of it: whatever Stockpile draws is lethal.
const STRIKES: usize = 20;
/// The seeds the scene test decides on.
const SEEDS: usize = 40;
/// The fill the replay test plays with: `AI_SEARCH.replay_fill_steps` as shipped.
const FILL: ReplayFill = ReplayFill {
    steps: 3,
    width: AI_BUDGET.branching,
};

/// The AI's turn 5 at 2 crystals, Stockpile and Mr. Vanilla in hand over 20 True Strikes, against an
/// empty board on 4 health. 25 instances, the most R73 reads as p1's deck (support.rs).
fn draw_lethal_turn() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "r4697000", "turn": 5, "active": "p1",
        "p1": { "hand": [STOCKPILE, VANILLA], "mana": 2, "library": vec![TRUE_STRIKE; STRIKES] },
        "p2": { "health": 4, "library": ["core-008", "core-011", "core-008"] },
    }))
    .state()
    .clone()
}

/// `line` replayed on `world` with `fill` on a fresh counter of `AI_BUDGET.nodes`, scored statically.
fn replayed(world: &GameState, line: &[ActionBody], fill: ReplayFill) -> LineReplay {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    replay_line(world, AI, line, &counter, None, Prices::NONE, fill)
        .unwrap_or_else(|| panic!("the counter ran out"))
}

/// R4697000: drawing first wins the turn on every draw, and the AI draws first on every seed.
#[test]
fn r4697000_the_ai_wins_a_turn_whose_lethal_it_reaches_by_drawing_first() {
    let state = draw_lethal_turn();
    assert!(state.players[AI].hand.iter().any(|card| card.def_id == VANILLA));
    let lost: Vec<String> = (0..SEEDS)
        .filter_map(|n| {
            let seed = format!("r4697000-{n}");
            let turn = play_ai_turn(&state, AI, &mut ai_options(&seed, AI_GATE_BUDGET));
            let won = turn.state.result.and_then(|result| result.winner.player()) == Some(AI);
            (!won).then(|| format!("{seed}: {}", trace(&turn)))
        })
        .collect();
    assert!(
        lost.is_empty(),
        "{} of {SEEDS} seeds not won:\n{}",
        lost.len(),
        lost.join("\n")
    );
}

/// Worlds A and B, drawn in turn from `cursor` of the scene's stream, and the line [Stockpile, a True Strike
/// it drew on A that B does not offer and that wins on A], when A drew one.
fn worlds_and_line(cursor: u32) -> Option<(GameState, [ActionBody; 2])> {
    let public = redact(&draw_lethal_turn(), AI);
    let mut rng = create_rng("r4697000-worlds", cursor);
    let a = det(&public, AI, &mut rng);
    let b = det(&public, AI, &mut rng);
    let stockpile_id = a.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == STOCKPILE)
        .map(|card| card.id.clone())
        .unwrap_or_else(|| panic!("Stockpile is not in hand"));
    let stockpile = candidate_actions(&a, AI, Prices::NONE)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == stockpile_id))
        .unwrap_or_else(|| panic!("Stockpile is not a candidate"));
    let before = ids(&a.players[AI].hand);
    let drawn = act(&a, AI, &stockpile);
    let offered_on_b = candidate_set(&act(&b, AI, &stockpile), AI);
    let strike = candidate_actions(&drawn, AI, Prices::NONE)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { instance_id, .. } if !before.contains(instance_id))
                && !offered_on_b.contains(action)
                && act(&drawn, AI, action)
                    .result
                    .and_then(|result| result.winner.player())
                    == Some(AI)
        })?;
    Some((b, [stockpile, strike]))
}

/// R4697000: [Stockpile, a True Strike it drew on world A at the opponent's hero] is cut on world B,
/// which drew other True Strikes. Off, B's replay ends its turn after the draw, a crystal unspent and
/// the game on; with the fill on it plays one of B's own True Strikes and wins. The worlds sample the
/// opponent's hidden cards from the pools, so the stream is searched (#848) for the first cursor whose
/// worlds show the cut as the premise reads it (a card a set adds to the pools, sampled into B, can change
/// what its replay closes on).
#[test]
fn r4697000_a_line_that_plays_the_card_it_drew_wins_on_a_world_that_drew_another_with_the_fill_on() {
    let premise = |cursor: u32| {
        worlds_and_line(cursor).is_some_and(|(b, line)| {
            let off = replayed(&b, &line, ReplayFill::OFF);
            off.cut
                && off.closed.result.is_none()
                && off.closed.players[AI].turn_log.unspent_at_end == Some(1)
        })
    };
    let cursor = cursor_where("r4697000-worlds", premise);
    let (b, line) = worlds_and_line(cursor).expect("the premise's worlds");
    let off = replayed(&b, &line, ReplayFill::OFF);
    assert!(off.cut);
    assert!(off.closed.result.is_none());
    assert_eq!(off.closed.players[AI].turn_log.unspent_at_end, Some(1));
    let on = replayed(&b, &line, FILL);
    assert!(on.cut);
    assert_eq!(
        on.closed.result.and_then(|result| result.winner.player()),
        Some(AI)
    );
    assert!(on.score > AI_EVAL.win * AI_WON_SCORE_SHARE, "{}", on.score);
}
