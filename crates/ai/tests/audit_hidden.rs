//! Audit: hidden information (R185) — `redact`, `determinize` and the entry points that read the true
//! state only through them. Each test asserts the documented behaviour and fails on the bug it proves.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serializable")
}

/// Every path at which two JSON values differ.
fn diff_paths(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => diff_paths(p, q, &format!("{path}.{key}"), out),
                    _ => out.push(format!("{path}.{key} (one side only)")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}[] (len {} vs {})", x.len(), y.len()));
                return;
            }
            for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
                diff_paths(p, q, &format!("{path}[{i}]"), out);
            }
        }
        _ => {
            if a != b {
                out.push(format!("{path}: {a} vs {b}"));
            }
        }
    }
}

/// R602 (`redact`'s last step before the prompts): "every unit keeps the stats it shows: the difference
/// its placeholder made goes on the unit's buffs". With nothing hidden touching a unit, the placeholder
/// made no difference, so a card the seat reads in full must come through `redact` as it is.
/// `observe.rs` subtracts the raw `unclamped_attack` of the redacted copy from the *floored*
/// `unit_view(..).attack` of the true state, so a Unit whose Attack is below 0 before the floor (§10.4:
/// "Attack floors at 0") is handed the shortfall as a permanent buff.
#[test]
fn redact_leaves_a_public_unit_with_attack_below_zero_as_it_is() {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "audit-hidden-negative-attack",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
        "p2": { "field": ["core-019"], "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    let printed = unit_view(&state, find_instance(&state, &id).unwrap()).attack;
    // A −(printed + 2) Attack buff, as Core #50 Adaptive Growth's "give all Units −2/−2" or C+ #19.3's
    // "−3/−3" leave on a small Unit: raw Attack −2, shown 0.
    find_instance_mut(&mut state, &id).unwrap().buffs.attack = -(printed + 2);
    let truth = find_instance(&state, &id).unwrap().clone();
    assert_eq!(unclamped_attack(&state, &truth), -2);
    assert_eq!(unit_view(&state, &truth).attack, 0);

    let seen = redact(&state, AI);
    let seen_unit = find_instance(&seen, &id).unwrap().clone();
    assert_eq!(
        seen_unit.buffs, truth.buffs,
        "redact changed the buffs of a Unit no hidden card touches"
    );
    assert_eq!(unclamped_attack(&seen, &seen_unit), -2);
}

/// The same bug, as a wrong answer of `lethal_hint` (R2102000: "checks if `find_lethal` finds a win across
/// all worlds"). The AI's Tempo Timmy stands at −2 Attack before the floor (shown 0), its hand holds
/// Core #14 Jlockeed's Weapons ("Your Units have +1 Attack, Rush and First Strike"), and the enemy hero
/// has 1 health and no Units. On the true state the Weapons leave Timmy at −1, shown 0, so there is no
/// lethal; on the redacted copy Timmy was lifted to 0, the Weapons make it 1, and the hint says lethal.
#[test]
fn lethal_hint_counts_no_damage_from_a_unit_below_zero_attack() {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "audit-hidden-negative-attack-hint",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-011"], "hand": ["core-014"], "mana": 4, "library": ["core-020", "core-053"] },
        "p2": { "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    find_instance_mut(&mut state, &id).unwrap().buffs.attack = -5;
    state.players[HUMAN].hero.health = 1;
    assert_eq!(unit_view(&state, find_instance(&state, &id).unwrap()).attack, 0);

    // The truth: with the Weapons down, Timmy still shows 0 Attack, and nothing else deals damage.
    let armed = act(&state, AI, play_of(&state, AI, "core-014"), "weapons");
    assert_eq!(unit_view(&armed, find_instance(&armed, &id).unwrap()).attack, 0);
    assert!(armed.result.is_none());

    assert!(
        !lethal_hint(&state, AI, None),
        "the hint reports a lethal the true state does not hold"
    );
}

/// One action through `reduce`, panicking with the engine's refusal.
fn act(state: &GameState, player: PlayerId, body: ActionBody, nonce: &str) -> GameState {
    let result = reduce(state, &Action::new(body.clone(), player, nonce));
    if let Some(error) = result.error {
        panic!("{} for {player} refused: {error}", body.action_type());
    }
    result.state
}

/// The first legal play of `player`'s hand card of `def_id`.
fn play_of(state: &GameState, player: PlayerId, def_id: &str) -> ActionBody {
    let id = state.players[player]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in hand");
    legal_actions(state, player)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .expect("the card can be played")
}

/// p2 sets `trap` face-down from its hand on its own turn and ends the turn; the AI (p1) is to act.
fn after_p2_sets(trap: &str) -> GameState {
    jackioh_cards::register_all();
    let start = scenario(json!({
        "seed": "audit-hidden-face-down-set",
        "active": "p2",
        "turn": 8,
        "p1": { "field": ["core-011"], "hand": ["core-008"], "library": ["core-020", "core-053", "core-019"] },
        "p2": { "hand": [trap], "mana": 5, "library": ["core-005", "core-016", "core-012"] },
    }))
    .state()
    .clone();
    let set = act(&start, HUMAN, play_of(&start, HUMAN, trap), "set");
    // R82 ends a turn with nothing left to do; otherwise p2 ends it.
    let next = if set.active == HUMAN {
        act(&set, HUMAN, ActionBody::EndTurn, "end")
    } else {
        set
    };
    assert!(ai_to_act(&next, AI), "the AI is to act");
    next
}

/// R185 and the README's "`decide` reads only `redact`": two true states that differ only in what the
/// seat cannot see redact alike. The opponent set a (1) Trap face-down (§3.2, R33: the seat reads a card
/// back and its cost, R351): core-041 Sheepish (no tag) in one world, classic-063 Crop Dusting (Plague)
/// in the other. `play_counts::record_play` counts every play by the card's tags in
/// `players[p].game_log.played_by_tag`, a face-down Trap's included, never reset, and `redact` keeps the
/// opponent's log, so the redaction says which tags the hidden card carries (and every simulated card
/// that reads `played_this_game_with_tag` of that player reads it).
#[test]
fn redact_hides_the_tags_of_the_opponents_face_down_trap() {
    let sheepish = after_p2_sets("core-041");
    let dusting = after_p2_sets("classic-063");
    // The seat's view of the two worlds is one view (events aside): the difference is hidden from it.
    let shown = |state: &GameState| {
        let mut view = js(view_for(state, AI));
        if let Some(map) = view.as_object_mut() {
            map.remove("events");
            map.remove("defs");
        }
        view
    };
    assert_eq!(shown(&sheepish), shown(&dusting), "the two worlds look alike to the seat");
    let mut differs = Vec::new();
    diff_paths(&js(redact(&sheepish, AI)), &js(redact(&dusting, AI)), "", &mut differs);
    assert!(differs.is_empty(), "redact tells the two worlds apart at: {differs:#?}");
}

/// README, "`decide` reads only `redact`": `redact` "strips from the events its queues keep what
/// `view_for` strips of a hidden card". The AI's face-down C #9 Income Tax fires on the opponent's
/// second draw of a turn, and the opponent's keep prompt interrupts the firing, so `traps::owe_firing`
/// parks the rest of it on `state.work` as `@trapFiring` with the whole `drawn` event under
/// `resume.data.firing.event`: the drawn card's id and definition, a card in the opponent's hand.
/// `scrub_resume` scrubs `resume.data.event` only. The two worlds differ in the card p2 drew second.
#[test]
fn redact_scrubs_the_event_an_interrupted_trap_firing_keeps() {
    jackioh_cards::register_all();
    let world = |second: &str| {
        let start = scenario(json!({
            "seed": "audit-hidden-owed-firing",
            "active": "p2",
            "turn": 8,
            "p1": {
                "hand": ["core-008"],
                "backrow": [{ "def": "classic-009", "faceUp": false }],
                "library": ["core-020", "core-053", "core-019"],
            },
            "p2": { "hand": ["core-005", "core-011"], "mana": 5, "library": ["core-012", second, "core-012"] },
        }))
        .state()
        .clone();
        let drawn = act(&start, HUMAN, play_of(&start, HUMAN, "core-005"), "stockpile");
        assert_eq!(drawn.pending.as_ref().map(|pending| pending.player_id), Some(HUMAN), "the keep prompt");
        assert!(drawn.work.iter().any(|item| item.resume.data.contains_key("firing")), "the owed firing");
        drawn
    };
    let a = world("core-016");
    let b = world("core-053");
    let mut differs = Vec::new();
    diff_paths(&js(redact(&a, AI)), &js(redact(&b, AI)), "", &mut differs);
    assert!(differs.is_empty(), "redact tells the two worlds apart at: {differs:#?}");
}
