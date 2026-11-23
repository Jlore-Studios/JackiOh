//! The golden recorder's steered policy (R3935001, amending R1365000): a recorded game whose first
//! forced card is played the way its change shows, so stage 2 of `classify` can see a change that
//! only another card brings out.
//!
//! The random policy (§10.7) seldom plays an Embiggened Wall and then a card that sets or Buffs it, so
//! 40 forced games never showed a change to how the Walls take a Set stats or a Buff. A steer is a list
//! of card ids, the **subject** first and the cards that act on it after. A seat holding the subject
//! plays it, at its embiggen price when it has one, and ends its turn while it cannot pay for it, up
//! to `STEER_HOLD_TURNS`, so nothing goes ahead of it. Once the subject stands on its field the seat
//! plays the cards that act on it, in the order given, each aimed at the subject when its plays
//! name targets. Nothing else changes: every other action is the policy's, the
//! steer takes no draw of its own, and the recorded file replays on any build like any other
//! (`golden check --file`).

use jackioh_engine::catalog::def_of;
use jackioh_engine::subsystems::{PolicyOptions, policy_actions};
use jackioh_engine::{ActionBody, CardCost, GameState, PlayerId, Selection, active_units_of, find_instance};

/// R3935001: the turn after which a seat stops holding its cards back for an earlier steered card it
/// cannot play, so a card nothing lets it play never stalls the game (the cap is `STEP_CAP` anyway).
pub(crate) const STEER_HOLD_TURNS: i32 = 12;

/// Whether `action` is a play naming, among its `targets`, an instance of the card `subject`.
fn aims_at(action: &ActionBody, state: &GameState, subject: &str) -> bool {
    let ActionBody::Play {
        targets: Some(targets),
        ..
    } = action
    else {
        return false;
    };
    targets.iter().any(|selection| {
        matches!(selection, Selection::Instance { instance_id }
            if find_instance(state, instance_id).is_some_and(|card| card.def_id == subject))
    })
}

/// The action `player` takes by `steer` (the subject first, then the cards that act on it), or `None`
/// when the policy chooses: no steered card is held, a prompt is open, or the hold is over.
pub(crate) fn steered_action(state: &GameState, player: PlayerId, steer: &[String]) -> Option<ActionBody> {
    let subject = steer.first()?;
    let hand = &state.players[player].hand;
    let legal = policy_actions(state, player, PolicyOptions::default());
    let standing = active_units_of(state, player)
        .iter()
        .any(|card| &card.def_id == subject);
    for id in steer {
        let held: Vec<&str> = hand
            .iter()
            .filter(|card| &card.def_id == id)
            .map(|card| card.id.as_str())
            .collect();
        // The cards that act on the subject wait for it to stand, so none is spent before it.
        if held.is_empty() || (id != subject && !standing) {
            continue;
        }
        let embiggens = matches!(def_of(Some(state), id).cost, CardCost::Embiggen { .. });
        let plays = |action: &&ActionBody| matches!(action, ActionBody::Play { instance_id, .. } if held.contains(&instance_id.as_str()));
        let mut candidates: Vec<&ActionBody> = legal.iter().filter(plays).collect();
        if id == subject {
            if embiggens {
                candidates.retain(|action| {
                    matches!(
                        action,
                        ActionBody::Play {
                            embiggen: Some(true),
                            ..
                        }
                    )
                });
            }
        } else {
            // A play aimed at the subject, else any.
            if candidates.iter().any(|action| aims_at(action, state, subject)) {
                candidates.retain(|action| aims_at(action, state, subject));
            }
        }
        if let Some(play) = candidates.first() {
            return Some((*play).clone());
        }
        // A card that acts on the subject and has no play now, such as a Spell with no target it may
        // take, holds nothing back.
        if id != subject {
            continue;
        }
        // The subject is held and not yet playable: nothing later goes first, so the seat waits for
        // its mana.
        if state.turn <= STEER_HOLD_TURNS {
            return legal
                .into_iter()
                .find(|action| matches!(action, ActionBody::EndTurn));
        }
        return None;
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use jackioh_engine::{Action, CreateGameArgs, begin_game, create_game, reduce};
    use serde_json::Value;

    use super::*;
    use crate::golden::record_seeds;

    const WALL: &str = "stinkyscapeplus-065";
    const SETTER: &str = "meditativeplus-021";
    const BUFFER: &str = "classicplus-071";

    /// What the steered games of seeds 1 to 40 played, in order: each play of a steered card with its
    /// card and whether it was embiggened, read by folding the recorded actions on this build.
    fn steered_plays() -> Vec<Vec<(String, bool)>> {
        jackioh_cards::register_all();
        let ids: Vec<String> = [WALL, SETTER, BUFFER].map(String::from).to_vec();
        let out = std::env::temp_dir().join(format!("jackioh-steer-test-{}.jsonl", std::process::id()));
        let seeds: Vec<u32> = (1..=40).collect();
        record_seeds(&seeds, &ids, &ids, &out).expect("recorded");
        let text = fs::read_to_string(&out).expect("the traces");
        let _ = fs::remove_file(&out);
        text.lines()
            .map(|line| {
                let game: Value = serde_json::from_str(line).expect("a game");
                let args: CreateGameArgs = serde_json::from_value(game["args"].clone()).expect("its args");
                let mut state = begin_game(&create_game(&args)).state;
                let mut plays = Vec::new();
                for step in game["steps"].as_array().expect("steps") {
                    let action: Action = serde_json::from_value(step["a"].clone()).expect("an action");
                    if let ActionBody::Play {
                        instance_id,
                        embiggen,
                        ..
                    } = &action.body
                        && let Some(card) = find_instance(&state, instance_id)
                        && ids.contains(&card.def_id)
                    {
                        plays.push((card.def_id.clone(), *embiggen == Some(true)));
                    }
                    state = reduce(&state, &action).state;
                }
                plays
            })
            .collect()
    }

    #[test]
    fn r3935001_a_steered_game_plays_the_wall_embiggened_and_then_the_cards_that_act_on_it() {
        let games = steered_plays();
        // The seats play the Wall at its embiggen price and the cards that act on it after it.
        let embiggened = games
            .iter()
            .flatten()
            .filter(|(id, big)| id == WALL && *big)
            .count();
        let then_acted = games
            .iter()
            .filter(|plays| {
                plays
                    .iter()
                    .skip_while(|(id, big)| !(id == WALL && *big))
                    .any(|(id, _)| id != WALL)
            })
            .count();
        assert!(embiggened >= 10, "{embiggened} embiggened Wall plays in 40 games");
        assert!(then_acted >= 5, "{then_acted} games set or Buffed after the Wall");
    }
}
