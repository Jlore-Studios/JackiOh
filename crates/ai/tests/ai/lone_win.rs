//! R4608000 (issue #4608): `decide` ranks a finalist whose line won on determinization 0, the world the
//! beam searched, and on none of the other worlds it was scored on by its mean on those others
//! (`finalist_means`), so a win only world 0 shows counts once another world confirms it.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, ai_options, close, det};

/// A won game's score, as `evaluate_priced` gives it on turn 5.
const W: f64 = AI_EVAL.win - 5.0;
/// TG #18 Knife Juggler: (1) 3/2, after you summon a Unit, 1 damage to a random enemy.
const JUGGLER: &str = "traininggrounds-018";
/// Right-house defender (#3): (1) 1/1, the Unit whose summon throws the knife.
const DEFENDER: &str = "core-003";
/// C+ #5 Guy Att: (2) 6/8, the board play.
const GUY_ATT: &str = "classicplus-005";
/// The seeds each scene test decides on.
const SEEDS: usize = 100;

/// The finalist an exact style plays given these means (no draw at a temperature of 0).
fn picked(means: &[f64]) -> usize {
    pick_finalist(means, 0.0, &mut create_rng("r4608000", 0))
}

#[test]
fn r4608000_a_win_on_world_zero_alone_ranks_by_the_other_worlds() {
    let rows = vec![vec![W, 10.0, 20.0], vec![0.0, 50.0, 60.0]];
    let means = finalist_means(&rows, true);
    close(means[0], 15.0);
    close(means[1], 110.0 / 3.0);
    assert_eq!(picked(&means), 1, "{means:?}");
    // Today's means: the lone win's third of a won game beats every position.
    assert_eq!(picked(&finalist_means(&rows, false)), 0);
}

#[test]
fn r4608000_a_win_on_two_worlds_keeps_the_plain_mean() {
    for won in [vec![W, W - 1.0, 10.0], vec![W, 10.0, W - 1.0]] {
        let plain = (won[0] + won[1] + won[2]) / 3.0;
        let rows = vec![won.clone(), vec![0.0, 50.0, 60.0]];
        let means = finalist_means(&rows, true);
        close(means[0], plain);
        assert_eq!(picked(&means), 0, "{won:?}: {means:?}");
    }
}

#[test]
fn r4608000_a_decision_that_scored_one_world_keeps_today_s_order() {
    // The counter or the clock stopped every other world: there is nothing to confirm the win with.
    let means = finalist_means(&[vec![W], vec![5.0]], true);
    assert_eq!(means, vec![W, 5.0]);
    assert_eq!(picked(&means), 0);
}

#[test]
fn r4608000_a_row_not_won_on_world_zero_keeps_the_plain_mean() {
    let means = finalist_means(&[vec![10.0, W, W], vec![-W, 3.0, 4.0]], true);
    close(means[0], (10.0 + W + W) / 3.0);
    close(means[1], (-W + 3.0 + 4.0) / 3.0);
}

#[test]
fn r4608000_a_lone_win_beside_a_loss_is_ranked_by_the_others() {
    // A loss on another world is no win there: the line is ranked by the loss and the position.
    let means = finalist_means(&[vec![W, -W + 5.0, 10.0]], true);
    close(means[0], (-W + 15.0) / 2.0);
}

#[test]
fn r4608000_with_the_switch_off_the_means_are_today_s_exactly() {
    let rows = vec![vec![W, 10.0, 20.0], vec![0.1, 0.2, 0.3], vec![W, W - 1.0, 0.7]];
    let means = finalist_means(&rows, false);
    for (row, mean) in rows.iter().zip(&means) {
        // Today's sum: the world-0 score, each other world's added in order, over the worlds scored.
        let today = row.iter().skip(1).fold(row[0], |total, score| total + score) / f64::from(3);
        assert_eq!(mean.to_bits(), today.to_bits(), "{row:?}: {mean} vs {today}");
    }
}

/// The AI's turn 5 with 2 crystals: the Juggler then the defender throws one knife at a random enemy,
/// the opponent's hero on 1 health or its Big D-fender (Core #1, 0/7); Guy Att spends the same crystals.
fn knife_turn() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "r4608000", "turn": 5, "active": "p1",
        "p1": { "hand": [JUGGLER, DEFENDER, GUY_ATT], "mana": 2,
                "library": ["core-008", "core-011", "core-008", "core-011", "core-008"] },
        "p2": { "health": 1, "field": ["core-001"], "library": ["core-008", "core-011", "core-008"] },
    }))
    .state()
    .clone()
}

fn hand_id(state: &GameState, def: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def)
        .map(|card| card.id.clone())
        .unwrap_or_else(|| panic!("{def} is not in hand"))
}

/// Whether `action` plays the card of `def` from the AI's hand.
fn plays(state: &GameState, action: &ActionBody, def: &str) -> bool {
    let id = hand_id(state, def);
    matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id)
}

fn play_of(state: &GameState, def: &str) -> ActionBody {
    candidate_actions(state, AI, Prices::NONE)
        .into_iter()
        .find(|action| plays(state, action, def))
        .unwrap_or_else(|| panic!("{def} is not a candidate"))
}

fn won_floor() -> f64 {
    AI_EVAL.win * AI_WON_SCORE_SHARE
}

/// One seed's decision beside both lines' scores, after the opponent's reply, on each of decide's worlds.
struct Run {
    seed: String,
    gamble: Vec<f64>,
    board: Vec<f64>,
    decision: Decision,
}

impl Run {
    /// The worlds on which the knife is lethal.
    fn won(&self) -> Vec<bool> {
        self.gamble.iter().map(|score| *score > won_floor()).collect()
    }

    /// Whether the board play wins on no world: a reply that ends the game for the AI whatever it
    /// does leaves nothing to compare.
    fn clean(&self) -> bool {
        self.board.iter().all(|score| *score <= won_floor())
    }
}

/// Every seed's run. Decide's worlds are rebuilt from its stream: at `AI_STYLE_EXACT` its only draws
/// are the determinizations'.
fn runs() -> Vec<Run> {
    let state = knife_turn();
    let public = redact(&state, AI);
    (0..SEEDS)
        .map(|n| {
            let seed = format!("r4608000-{n}");
            with_pool_shift(0, || {
                let mut rng = create_rng(&seed, 0);
                let worlds: Vec<GameState> = (0..AI_BUDGET.determinizations)
                    .map(|_| det(&public, AI, &mut rng))
                    .collect();
                let first = worlds.first().expect("a world");
                let juggler = play_of(first, JUGGLER);
                let gamble = vec![juggler.clone(), play_of(&act(first, AI, &juggler), DEFENDER)];
                let board = vec![play_of(first, GUY_ATT)];
                let score = |line: &[ActionBody], world: &GameState| {
                    let counter = create_node_counter(AI_BUDGET.nodes, None);
                    let hidden = hidden_card_ids(world, AI);
                    replay_line(
                        world,
                        AI,
                        line,
                        &counter,
                        Some(&hidden),
                        Prices::NONE,
                        ReplayFill::OFF,
                    )
                    .unwrap_or_else(|| panic!("{seed}: the counter ran out"))
                    .score
                };
                let decision = decide(&state, AI, &mut ai_options(&seed, AI_BUDGET))
                    .unwrap_or_else(|| panic!("{seed}: no decision"));
                Run {
                    gamble: worlds.iter().map(|world| score(&gamble, world)).collect(),
                    board: worlds.iter().map(|world| score(&board, world)).collect(),
                    seed: seed.clone(),
                    decision,
                }
            })
        })
        .collect()
}

#[test]
fn r4608000_the_ai_makes_the_board_play_when_only_its_searched_world_rolls_lethal() {
    let state = knife_turn();
    let (mut lone, mut none) = (0, 0);
    for run in runs().iter().filter(|run| run.clean()) {
        let seed = &run.seed;
        let won = run.won();
        if won == [true, false, false] {
            lone += 1;
            for world in 1..run.gamble.len() {
                assert!(
                    run.board[world] > run.gamble[world],
                    "{seed}: the board play scores better on world {world}: {:?} vs {:?}",
                    run.board,
                    run.gamble
                );
            }
            assert_eq!(
                run.decision.stats.worlds_scored, AI_BUDGET.determinizations,
                "{seed}: every world was scored"
            );
            assert!(
                plays(&state, &run.decision.action, GUY_ATT),
                "{seed}: a knife lethal only on the searched world: {:?} ({:?})",
                run.decision.action,
                run.decision.reason
            );
        } else if won.iter().all(|won| !won) {
            none += 1;
            assert!(
                plays(&state, &run.decision.action, GUY_ATT),
                "{seed}: no world rolls lethal: {:?} ({:?})",
                run.decision.action,
                run.decision.reason
            );
        }
    }
    assert!(
        lone >= 5 && none >= 5,
        "too few seeds of a kind: {lone} lone, {none} none"
    );
}

#[test]
fn r4608000_the_ai_still_goes_for_a_lethal_two_of_three_worlds_roll() {
    let state = knife_turn();
    let (mut two, mut three) = (0, 0);
    for run in runs().iter().filter(|run| run.clean()) {
        let seed = &run.seed;
        let won = run.won();
        let others = won.iter().skip(1).filter(|won| **won).count();
        if !won.first().copied().unwrap_or(false) || others == 0 {
            continue;
        }
        assert!(
            plays(&state, &run.decision.action, JUGGLER),
            "{seed}: the knife is lethal on {won:?}: {:?} ({:?})",
            run.decision.action,
            run.decision.reason
        );
        if others == 1 {
            two += 1;
            assert_eq!(run.decision.reason, DecisionReason::Search, "{seed}: {won:?}");
            assert!(
                run.decision.stats.score > won_floor(),
                "{seed}: {}",
                run.decision.stats.score
            );
        } else {
            three += 1;
            assert_eq!(run.decision.reason, DecisionReason::Lethal, "{seed}: {won:?}");
        }
    }
    assert!(
        two >= 5 && three >= 3,
        "too few seeds of a kind: {two} on two worlds, {three} on three"
    );
}
