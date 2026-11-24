//! Audit: hidden information (R185) — `redact`, `determinize` and the entry points that read the true
//! state only through them. Each test asserts the documented behaviour and fails on the bug it proves.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serializable")
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

// ---------------------------------------------------------------------------------------------
// EXPLORATION (temporary)
// ---------------------------------------------------------------------------------------------

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

fn find_string(v: &Value, needle: &str, path: &str, out: &mut Vec<String>) {
    match v {
        Value::String(text) => {
            if text.contains(needle) {
                out.push(path.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                find_string(item, needle, &format!("{path}[]"), out);
            }
        }
        Value::Object(fields) => {
            for (key, item) in fields {
                if key.contains(needle) {
                    out.push(format!("{path}.<key>"));
                }
                find_string(item, needle, &format!("{path}.{key}"), out);
            }
        }
        _ => {}
    }
}

fn all_set_decks(seed: &str) -> (Vec<String>, Vec<String>) {
    let mut pool: Vec<String> = query(&CatalogQueryArgs::default())
        .iter()
        .map(|def| def.id.clone())
        .collect();
    pool.sort();
    let shuffled = create_rng(&format!("audit-decks:{seed}"), 0).shuffle(&pool);
    let n = DECK_SIZE as usize;
    (shuffled[..n].to_vec(), shuffled[n..2 * n].to_vec())
}

fn every_instance(state: &GameState) -> Vec<&CardInstance> {
    let mut out = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend(side.hand.iter());
        out.extend(side.library.iter());
        out.extend(side.graveyard.iter());
        out.extend(side.exile.iter());
        out.extend(side.resolving.iter());
        out.extend(side.units.iter().flatten().flatten());
        out.extend(side.backrow.iter().flatten());
        out.extend(side.backrow_piles.iter().flatten().flatten());
        out.extend(side.carried.iter().flatten().flatten());
    }
    out
}

#[test]
fn explore_taint() {
    jackioh_cards::register_all();
    let mut tally: indexmap::IndexMap<String, (usize, String)> = indexmap::IndexMap::new();
    let games: usize = std::env::var("AUDIT_GAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    for game in 0..games {
        let seed = format!("audit-taint-{game}");
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.clone(),
            decks: all_set_decks(&seed),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut policy = create_rng(&format!("audit-policy:{seed}"), 0);
        let mut n = 0;
        while state.result.is_none() && n < 400 {
            let Some(player) = seat_to_act(&state) else { break };
            let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else { break };
            let result = reduce(&state, &Action::new(chosen, player, format!("rp-{n}")));
            if result.error.is_some() {
                break;
            }
            state = result.state;
            n += 1;
            if n % 3 != 0 {
                continue;
            }
            for seat in PLAYER_IDS {
                let hidden = hidden_instance_ids(&state, seat);
                let seen = redact(&state, seat);
                let v = js(&seen);
                let visible: indexmap::IndexSet<String> = every_instance(&state)
                    .into_iter()
                    .filter(|card| !hidden.contains(&card.id))
                    .map(|card| card.def_id.clone())
                    .collect();
                for id in &hidden {
                    let Some(card) = find_instance(&state, id) else { continue };
                    if visible.contains(&card.def_id) {
                        continue;
                    }
                    let mut hits = Vec::new();
                    find_string(&v, &card.def_id, "", &mut hits);
                    for hit in hits {
                        let entry = tally.entry(hit).or_insert((0, format!("{seed} n={n} seat={seat} card={} def={} zone={:?}", card.id, card.def_id, card.zone)));
                        entry.0 += 1;
                    }
                }
            }
        }
    }
    for (path, (count, example)) in &tally {
        println!("TAINT {count:5} {path}   e.g. {example}");
    }
}
