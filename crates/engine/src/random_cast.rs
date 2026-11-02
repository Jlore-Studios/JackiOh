//! Random casts and casts that target enemies (docs/classic-sets.md B5 E12, E39, R452).
//!
//! A cast (R70) is §10.5's pipeline entered at step 3. A random cast (Classic+ #47 Jogg's Box, #38.1
//! Solarius Prime) makes every choice its caster would make at random, every prompt opened for the
//! caster while it resolves included, and so does a cast made while it resolves (the chain is capped,
//! RANDOM_CAST_CHAIN_CAP); the other player's prompts are asked as usual.
//! A cast that targets enemies when it can (`targetEnemies`, Classic+ #40 Appropriations, E39) aims
//! each target pick by its declaration (R656).
//!
//! While a cast's steps run its mode sits on `state.castsResolving`. A step that pauses takes it off,
//! and the owed step puts it back when driven again (the run record keeps `random` and `targetEnemies`).

use indexmap::IndexSet;

use crate::config::RANDOM_CAST_CHAIN_CAP;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{CastMode, GameState, Resume, find_instance};
use crate::wire::{PlayerId, Selection, Zone, opponent_of};

/// The casts whose modes are in force now, outermost first.
pub fn cast_modes_of(state: &GameState) -> &[CastMode] {
    state.casts_resolving.as_deref().unwrap_or(&[])
}

/// Run `body` with a cast's mode in force, and take it off again whatever `body` does (a pause, the end
/// of the game), so the stack is empty whenever the state is at rest.
pub fn with_cast_mode<'a, T>(
    sink: &mut EngineSink<'a>,
    mode: Option<CastMode>,
    body: impl FnOnce(&mut EngineSink<'a>) -> T,
) -> T {
    let Some(mode) = mode else {
        return body(sink);
    };
    let instance_id = mode.instance_id.clone();
    let mut stack = cast_modes_of(sink.state).to_vec();
    stack.push(mode);
    sink.state.casts_resolving = Some(stack);
    let out = body(sink);
    // By value, not by reference: an AI playout inside the cast (R44) adopts a cloned state.
    let mut stack = cast_modes_of(sink.state).to_vec();
    if let Some(at) = stack.iter().rposition(|entry| entry.instance_id == instance_id) {
        stack.remove(at);
    }
    sink.state.casts_resolving = if stack.is_empty() { None } else { Some(stack) };
    out
}

/// R1855001: where a run whose caster's choices are all random outside any cast (a replaced board's laid
/// Cries) names that caster: the card's data every continuation carries, so the mode a pause drops comes
/// back (`random_choices_of`). A delayed effect, a run of its own, drops it (`effects::delay`).
pub const RANDOM_CHOICES_KEY: &str = "__randomChoices";

/// R1855001: the random mode a continuation runs under (R452's, for the player `RANDOM_CHOICES_KEY`
/// names, as a cast that makes no cast of its own), or null.
pub fn random_choices_of(resume: &Resume) -> Option<CastMode> {
    let player: PlayerId = serde_json::from_value(resume.data.get(RANDOM_CHOICES_KEY)?.clone()).ok()?;
    Some(CastMode {
        instance_id: resume
            .instance_id
            .clone()
            .unwrap_or_else(|| resume.def_id.clone()),
        player,
        random: true,
        target_enemies: false,
        casts: 0,
        friends_only: None,
    })
}

/// R452: the innermost random cast of this player's being driven now, or null.
pub fn random_cast_of(state: &GameState, player: PlayerId) -> Option<&CastMode> {
    cast_modes_of(state)
        .iter()
        .rev()
        .find(|entry| entry.player == player && entry.random)
}

/// R452: the outermost random cast of this player's — the one whose chain every inner cast counts in.
fn root_random_cast(state: &GameState, player: PlayerId) -> Option<usize> {
    cast_modes_of(state)
        .iter()
        .position(|entry| entry.player == player && entry.random)
}

/// R452: whether one more cast may begin under this player's random cast chain (capped). A cast verb asks
/// this before it makes or moves the card, and a cast it refuses resolves into nothing (R28's shape).
pub fn may_cast_now(state: &GameState, player: PlayerId) -> bool {
    match root_random_cast(state, player) {
        None => true,
        Some(at) => cast_modes_of(state)[at].casts < RANDOM_CAST_CHAIN_CAP,
    }
}

/// R452: count one more cast in this player's random cast chain, when one is running.
pub fn count_chain_cast(state: &mut GameState, player: PlayerId) {
    let Some(at) = root_random_cast(state, player) else {
        return;
    };
    if let Some(root) = state.casts_resolving.as_mut().and_then(|stack| stack.get_mut(at)) {
        root.casts += 1;
    }
}

/// How a prompt is to be answered while casts are being driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastPromptMode {
    pub random: bool,
    pub target_enemies: bool,
    /// R1850004: its target options are the caster's own side alone (`friends_only`).
    pub friends_only: bool,
}

/// R452: how a prompt opened for `player` is answered: at random under a random cast of theirs, narrowed
/// when it is the own text of a cast of theirs that targets enemies (`resumeInstanceId`), else null.
pub fn cast_mode_for_prompt(
    state: &GameState,
    player: PlayerId,
    resume_instance_id: Option<&str>,
) -> Option<CastPromptMode> {
    if let Some(random) = random_cast_of(state, player) {
        return Some(CastPromptMode {
            random: true,
            target_enemies: random.target_enemies,
            friends_only: random.friends_only == Some(true),
        });
    }
    let own = cast_modes_of(state).iter().find(|entry| {
        entry.player == player
            && (entry.target_enemies || entry.friends_only == Some(true))
            && Some(entry.instance_id.as_str()) == resume_instance_id
    });
    own.map(|entry| CastPromptMode {
        random: false,
        target_enemies: entry.target_enemies,
        friends_only: entry.friends_only == Some(true),
    })
}

// Enemies (R452)

/// A pick that names a card or a hero, which "target enemies" can narrow; a mode or a zone it cannot.
pub fn is_target_pick(selection: &Selection) -> bool {
    matches!(selection, Selection::Instance { .. } | Selection::Hero { .. })
}

fn zone_player(zone: &Zone) -> PlayerId {
    zone.player()
}

/// R452: whether a pick names one of `chooser`'s enemies (on the field by control, elsewhere by pile).
pub fn is_enemy_pick(state: &GameState, chooser: PlayerId, selection: &Selection) -> bool {
    let enemy = opponent_of(chooser);
    match selection {
        Selection::Hero { player } => *player == enemy,
        Selection::Instance { instance_id } => {
            let Some(card) = find_instance(state, instance_id) else {
                return false;
            };
            if matches!(card.zone, Zone::Field { .. }) {
                card.controller == enemy
            } else {
                zone_player(&card.zone) == enemy
            }
        }
        _ => false,
    }
}

/// R452: "target enemies when possible": the target picks narrowed to the enemies, when there are enough
/// for the `required` picks; otherwise every option, since a friendly target is then the legal one.
pub fn prefer_enemies<T: Clone>(
    state: &GameState,
    chooser: PlayerId,
    options: &[T],
    selection_of: impl Fn(&T) -> Selection,
    required: i32,
) -> Vec<T> {
    if !options
        .iter()
        .any(|option| is_enemy_pick(state, chooser, &selection_of(option)))
    {
        return options.to_vec();
    }
    let narrowed: Vec<T> = options
        .iter()
        .filter(|option| {
            let selection = selection_of(option);
            !is_target_pick(&selection) || is_enemy_pick(state, chooser, &selection)
        })
        .cloned()
        .collect();
    if narrowed.len() as i32 >= required {
        narrowed
    } else {
        options.to_vec()
    }
}

/// R656: whether a pick names one of `chooser`'s friendly targets, the mirror of `is_enemy_pick`.
pub fn is_friendly_pick(state: &GameState, chooser: PlayerId, selection: &Selection) -> bool {
    match selection {
        Selection::Hero { player } => *player == chooser,
        Selection::Instance { instance_id } => {
            let Some(card) = find_instance(state, instance_id) else {
                return false;
            };
            if matches!(card.zone, Zone::Field { .. }) {
                card.controller == chooser
            } else {
                zone_player(&card.zone) == chooser
            }
        }
        _ => false,
    }
}

/// R656: "target allies when beneficial", the mirror of `prefer_enemies`, for a declaration with
/// `aim: "help"` under a random cast that targets enemies.
pub fn prefer_friends<T: Clone>(
    state: &GameState,
    chooser: PlayerId,
    options: &[T],
    selection_of: impl Fn(&T) -> Selection,
    required: i32,
) -> Vec<T> {
    if !options
        .iter()
        .any(|option| is_friendly_pick(state, chooser, &selection_of(option)))
    {
        return options.to_vec();
    }
    let narrowed: Vec<T> = options
        .iter()
        .filter(|option| {
            let selection = selection_of(option);
            !is_target_pick(&selection) || is_friendly_pick(state, chooser, &selection)
        })
        .cloned()
        .collect();
    if narrowed.len() as i32 >= required {
        narrowed
    } else {
        options.to_vec()
    }
}

/// R1850004: "cast it for yourself": the target picks narrowed strictly to the chooser's hero and the
/// cards they control on the field, even when none is left (a mode or a zone option is kept).
pub fn friends_only<T: Clone>(
    state: &GameState,
    chooser: PlayerId,
    options: &[T],
    selection_of: impl Fn(&T) -> Selection,
) -> Vec<T> {
    options
        .iter()
        .filter(|option| match selection_of(option) {
            Selection::Hero { player } => player == chooser,
            Selection::Instance { instance_id } => find_instance(state, &instance_id)
                .is_some_and(|card| matches!(card.zone, Zone::Field { .. }) && card.controller == chooser),
            _ => true,
        })
        .cloned()
        .collect()
}

/// R452: a uniformly random set of `low` to `high` of `items` (the size first, then the set), kept in
/// offered order: a declaration's picks are a set (R221).
pub fn random_picks<T: Clone>(rng: &mut Rng, items: &[T], low: i32, high: i32) -> Vec<T> {
    let top = 0.max(high.min(items.len() as i32));
    let bottom = 0.max(low.min(top));
    let size = bottom + rng.int(top - bottom + 1);
    let indexes: Vec<usize> = (0..items.len()).collect();
    let chosen: IndexSet<usize> = rng
        .shuffle(&indexes)
        .into_iter()
        .take(size.max(0) as usize)
        .collect();
    items
        .iter()
        .enumerate()
        .filter(|(index, _)| chosen.contains(index))
        .map(|(_, item)| item.clone())
        .collect()
}
