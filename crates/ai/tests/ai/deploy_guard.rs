//! The deploy guard (candidates.rs `deploy_guard`, config.rs `AI_DEPLOY_GUARD`): a Play the lane's
//! records show cast into its own funeral is refused by the deciding seat's beam where its printed
//! downside is armed, and kept where it is not.

use jackioh_ai::deploy_guard;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario};

/// A plain Play of `def_id` out of the AI's hand, as `legal_actions` would carry it.
fn play_of(state: &GameState, def_id: &str) -> ActionBody {
    let card = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .unwrap_or_else(|| panic!("{def_id} not in {AI}'s hand"));
    ActionBody::Play {
        instance_id: card.id.clone(),
        zone: None,
        x: None,
        embiggen: None,
        equip: None,
        tributes: None,
        targets: None,
        modes: None,
        plague: None,
        face_down: None,
    }
}

#[test]
fn a_plain_play_passes() {
    register_cards();
    let s = scenario(json!({
        "turn": 6,
        "p1": { "hand": ["core-008"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = s.state();
    assert!(deploy_guard(state, AI, &play_of(state, "core-008")));
    assert!(deploy_guard(state, AI, &ActionBody::EndTurn));
}

#[test]
fn aluneth_casts_at_any_hand_since_the_draws_came_priced() {
    register_cards();
    // #1194 priced Aluneth's draws ("They cost (1) less"), and the lane's records swung to the cast:
    // no hand is too full for the guard to keep it out.
    let full = scenario(json!({
        "turn": 6,
        "p1": { "hand": ["meditative-080", "core-008", "core-011", "core-013"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = full.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-080")));

    let starving = scenario(json!({
        "turn": 6,
        "p1": { "hand": ["meditative-080", "core-008"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = starving.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-080")));

    let radiant = scenario(json!({
        "turn": 6,
        "p1": { "hand": [{ "def": "meditative-080", "radiant": true }, "core-008", "core-011", "core-013"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = radiant.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-080")));
}

#[test]
fn doom_is_kept_only_when_its_gift_has_no_zone() {
    register_cards();
    // One open enemy unit zone is enough for the Ready… I'm to land.
    let open = scenario(json!({
        "turn": 6,
        "p1": { "hand": ["meditative-070"] },
        "p2": { "field": ["core-008", "core-011", "core-013", "core-014"] },
    }));
    let state = open.state();
    assert!(!deploy_guard(state, AI, &play_of(state, "meditative-070")));

    // All five enemy unit zones filled: the summon fizzles and the 5/8 is free.
    let full = scenario(json!({
        "turn": 6,
        "p1": { "hand": ["meditative-070"] },
        "p2": { "field": ["core-008", "core-011", "core-013", "core-015", "core-019"] },
    }));
    let state = full.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-070")));
}

#[test]
fn jlockwork_is_kept_inside_the_window_its_body_can_close() {
    register_cards();
    let early = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["meditative-003"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = early.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-003")));

    let late = scenario(json!({
        "turn": 13,
        "p1": { "hand": ["meditative-003"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = late.state();
    assert!(!deploy_guard(state, AI, &play_of(state, "meditative-003")));
}

#[test]
fn catnip_is_refused_before_a_spell_could_wake_it() {
    register_cards();
    let early = scenario(json!({
        "turn": 3,
        "p1": { "hand": ["meditative-068"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = early.state();
    assert!(!deploy_guard(state, AI, &play_of(state, "meditative-068")));

    let later = scenario(json!({
        "turn": 5,
        "p1": { "hand": ["meditative-068"] },
        "p2": { "field": ["core-008"] },
    }));
    let state = later.state();
    assert!(deploy_guard(state, AI, &play_of(state, "meditative-068")));
}

#[test]
fn suppressive_aura_is_kept_only_where_the_debuff_costs_the_enemy_more() {
    register_cards();
    // Only our board stands to be debuffed.
    let ours = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["core-046"], "field": ["core-008", "core-011"] },
        "p2": {},
    }));
    let state = ours.state();
    assert!(!deploy_guard(state, AI, &play_of(state, "core-046")));

    // An enemy board with more to lose than ours.
    let theirs = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["core-046"], "field": ["core-008"] },
        "p2": { "field": ["core-011", "core-013", "core-014"] },
    }));
    let state = theirs.state();
    assert!(deploy_guard(state, AI, &play_of(state, "core-046")));

    // The Radiant face debuffs the enemy alone: kept however the board stands.
    let radiant = scenario(json!({
        "turn": 8,
        "p1": { "hand": [{ "def": "core-046", "radiant": true }], "field": ["core-008", "core-011"] },
        "p2": {},
    }));
    let state = radiant.state();
    assert!(deploy_guard(state, AI, &play_of(state, "core-046")));
}

#[test]
fn back_breaker_is_kept_while_its_death_cannot_take_our_backrow() {
    register_cards();
    let armed = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["classic-051"], "backrow": ["core-041"] },
        "p2": {},
    }));
    let state = armed.state();
    assert!(!deploy_guard(state, AI, &play_of(state, "classic-051")));

    let clean = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["classic-051"] },
        "p2": {},
    }));
    let state = clean.state();
    assert!(deploy_guard(state, AI, &play_of(state, "classic-051")));

    // Theirs outnumbers ours: the sweeper's Death is worth it.
    let sweeper = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["classic-051"], "backrow": ["core-041"] },
        "p2": { "backrow": ["core-041", "core-073"] },
    }));
    let state = sweeper.state();
    assert!(deploy_guard(state, AI, &play_of(state, "classic-051")));

    let radiant = scenario(json!({
        "turn": 8,
        "p1": { "hand": [{ "def": "classic-051", "radiant": true }], "backrow": ["core-041"] },
        "p2": {},
    }));
    let state = radiant.state();
    assert!(deploy_guard(state, AI, &play_of(state, "classic-051")));
}

#[test]
fn the_ivory_tower_casts_but_every_play_onto_it_is_refused() {
    register_cards();
    let s = scenario(json!({
        "turn": 8,
        "p1": { "hand": ["classicplus-033", "core-008"], "backrow": [{ "def": "classicplus-033", "lane": 2 }] },
        "p2": { "field": ["core-008"] },
    }));
    let state = s.state();
    // R1194000's aura ("Your Units have Flying and Stack") made the cast a real card: unguarded.
    assert!(deploy_guard(state, AI, &play_of(state, "classicplus-033")));

    // A unit play that names the tower's backrow zone fuses into it (R446, R653).
    let mut fused = play_of(state, "core-008");
    if let ActionBody::Play { zone, .. } = &mut fused {
        *zone = Some(ZoneChoice {
            row: Row::Backrow,
            lane: 2,
        });
    }
    assert!(!deploy_guard(state, AI, &fused));

    // A play beside it, into a unit zone of its own, is not the tower's business.
    let mut beside = play_of(state, "core-008");
    if let ActionBody::Play { zone, .. } = &mut beside {
        *zone = Some(ZoneChoice {
            row: Row::Units,
            lane: 2,
        });
    }
    assert!(deploy_guard(state, AI, &beside));
}
