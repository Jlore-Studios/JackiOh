//! The run layer's invariant monitor (§15): every accepted step of a run keeps these bounds,
//! and every refused action leaves the state untouched. The whole-run walks (`walk_tests`) check
//! the state side on theirs; the map fuzz test (`map_fuzz_tests`) checks both on random-policy
//! walks with illegal-action probes.

use super::*;
use crate::config::{RUN_ARTIFACT_LEVEL_MAX, RUN_DECK_MAX};

/// Every count a run holds in bounds (§15.2: nothing deals a card the deck holds; §15.4: no
/// artifact twice, one copy of each relic; §15.7, §15.8; §15.10: the score terms; R2701000: the
/// deck cap; R4067000: distinct curses). Each failure names its field, and each has a hand-built
/// failing state below (the old clauses under `walk_*_fails`, the new ones here).
pub(crate) fn check_run_invariants(state: &RunState, content: &RunContent) -> Result<(), String> {
    if matches!(state.phase, RunPhase::Dead) {
        if state.hp != 0 {
            return Err(format!("walk: hp {} while dead", state.hp));
        }
    } else {
        let max = run_max_hp(state, content);
        if state.hp < 1 || state.hp > max {
            return Err(format!("walk: hp {} while alive, max {max}", state.hp));
        }
    }
    if state.coins < 0 {
        return Err(format!("walk: coins {}", state.coins));
    }
    if state.tickets < 0 {
        return Err(format!("walk: tickets {}", state.tickets));
    }
    if state.deck.len() > RUN_DECK_MAX {
        return Err(format!(
            "walk: deck holds {} past {RUN_DECK_MAX}",
            state.deck.len()
        ));
    }
    for (i, card) in state.deck.iter().enumerate() {
        if state.deck[..i].iter().any(|other| other.id == card.id) {
            return Err(format!("walk: deck holds {} twice", card.id));
        }
    }
    for (i, artifact) in state.artifacts.iter().enumerate() {
        if state.artifacts[..i].iter().any(|other| other.id == artifact.id) {
            return Err(format!("walk: artifact {} twice", artifact.id));
        }
        if artifact.level < 1 || artifact.level > RUN_ARTIFACT_LEVEL_MAX {
            return Err(format!(
                "walk: artifact {} at level {}",
                artifact.id, artifact.level
            ));
        }
    }
    for (i, blessing) in state.blessings.iter().enumerate() {
        if state.blessings[..i].contains(blessing) {
            return Err(format!("walk: blessing {blessing} twice"));
        }
    }
    for (i, id) in state.curses.iter().enumerate() {
        if state.curses[..i].contains(id) {
            return Err(format!("walk: curse {id} twice"));
        }
    }
    let terms = run_score_terms(state);
    for (name, value) in [
        ("rows", terms.rows),
        ("wins", terms.wins),
        ("damage", terms.damage),
        ("healing", terms.healing),
        ("drawn", terms.drawn),
    ] {
        if value < 0 {
            return Err(format!("walk: score term {name} {value}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RUN_STARTER_SIZE;

    /// A started placeholder run to break one field of, as `walk_tests`' `started`.
    fn started() -> (RunState, RunContent) {
        let content = run_placeholder_content();
        let setup = RunSetup {
            seed: run_daily_seed("2026-11-01", 1),
            starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
            artifacts: vec!["temp-greedy".into()],
            curses: Vec::new(),
        };
        let state = run_create(&setup, &RunUnlocks::default(), &content).unwrap();
        (state, content)
    }

    #[test]
    fn invariants_a_curse_twice_fails() {
        let (mut state, content) = started();
        let id = content
            .curses
            .first()
            .expect("the placeholder names curses")
            .id
            .clone();
        state.curses.push(id.clone());
        state.curses.push(id.clone());
        let err = check_run_invariants(&state, &content).unwrap_err();
        assert!(err.contains(&id), "{err}");
    }

    #[test]
    fn invariants_a_negative_score_term_fails() {
        let (mut state, content) = started();
        state.stats.damage_dealt = -1;
        let err = check_run_invariants(&state, &content).unwrap_err();
        assert!(err.contains("damage"), "{err}");
    }
}
