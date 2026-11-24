//! Audit of the engine's combat and damage: exploratory differential run of the combat forecast
//! (`query::combat_forecast`, R3100000) against the reducer on boards of real cards.

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
const BUNKER: &str = "meditative-097-5"; // 5/10 Armor 3, Can't attack, First Strike, lane strike x3
const FUSE: &str = "stinkyscapeplus-107"; // 2/5 Armor 3, Ranged, Pierce, sweep
const OCT: &str = "stinkyscapeplus-123"; // 4/16 Can't attack, Flying, damage pool 5
const VOLCANIC: &str = "stinkyscapeplus-151"; // 5/1 Damage dealt Burns
const NOXIOUS: &str = "stinkyscapeplus-153"; // 3/1 Damage dealt is Toxic
const SCORCHING: &str = "stinkyscapeplus-150"; // 3/1 Damage also Burns
const BANISHER: &str = "meditative-072"; // 2/1 exiles what it damages
const ANIME: &str = "classicplus-011"; // 3/3 hero and this take at most {cap}
const SCORCH: &str = "stinkyscapeplus-037"; // 1/5 Ranged, Pierce, only across
const MACE: &str = "stinkyscapeplus-111"; // 2/2 Ranged, Pierce, also Burns
const REIGN: &str = "stinkyscapeplus-114"; // 12/8 Ranged Windfury Pierce Cleave Armor 4
const TOP_LOSER: &str = "classicplus-019-1"; // 3/3 only its lane attacks it
const PROJECTOR: &str = "stinkyscapeplus-042"; // Field Spell, damage pool 5

fn defs() -> Vec<&'static str> {
    vec![
        VANILLA, VANILLA, VANILLA, VANILLA, BUNKER, FUSE, OCT, VOLCANIC, NOXIOUS, SCORCHING, BANISHER, ANIME,
        SCORCH, MACE, REIGN, TOP_LOSER,
    ]
}

fn cannot_defend(def: &str) -> bool {
    [FUSE, OCT, SCORCH, TOP_LOSER].contains(&def)
}

fn grants() -> Vec<Keyword> {
    vec![
        Keyword::FirstStrike,
        Keyword::Poisonous,
        Keyword::DivineShield,
        Keyword::Pierce,
        Keyword::Ranged,
        Keyword::Trample,
        Keyword::Armor { n: 1 },
        Keyword::Armor { n: 3 },
        Keyword::Indestructible,
        Keyword::Lifesteal,
        Keyword::Reborn,
        Keyword::Cleave,
        Keyword::Windfury,
        Keyword::Flying,
        Keyword::Taunt,
        Keyword::Deft,
    ]
}

fn side(rng: &mut Rng, pool: bool) -> Value {
    let all = defs();
    let mut field = Vec::new();
    for lane in 1..=5 {
        if !rng.chance(0.55) {
            continue;
        }
        let def = rng.pick(&all).copied().unwrap_or(VANILLA);
        let mut entry = json!({ "def": def, "lane": lane, "radiant": rng.chance(0.25) });
        if def == VANILLA && rng.chance(0.7) {
            entry["statsOverride"] = json!({ "attack": rng.int(9), "health": 1 + rng.int(9) });
        }
        field.push(entry);
    }
    let mut out = json!({
        "field": field,
        "hand": [VANILLA],
        "library": [VANILLA, VANILLA, VANILLA],
    });
    if pool {
        out["backrow"] = json!([PROJECTOR]);
    }
    out
}

fn dress(state: &mut GameState, rng: &mut Rng) {
    let pool = grants();
    let ids: Vec<(String, String)> = [P1, P2]
        .into_iter()
        .flat_map(|player| {
            state.players[player]
                .units
                .iter()
                .flatten()
                .flatten()
                .map(|card| (card.id.clone(), card.def_id.clone()))
                .collect::<Vec<_>>()
        })
        .collect();
    for (id, def) in ids {
        let mut keywords = Vec::new();
        for _ in 0..rng.int(3) {
            if let Some(keyword) = rng.pick(&pool) {
                keywords.push(keyword.clone());
            }
        }
        let never_def = cannot_defend(&def) || keywords.contains(&Keyword::Indestructible);
        let to_def = !never_def && rng.chance(0.25);
        let hurt = rng.chance(0.4);
        {
            let unit = find_instance_mut(state, &id).expect("unit");
            unit.granted_keywords.extend(keywords);
            if to_def {
                unit.position = Some(Position::Def);
            }
        }
        let health = unit_view(state, find_instance(state, &id).expect("unit")).max_health;
        if hurt && health > 1 {
            let damage = rng.int(health - 1);
            find_instance_mut(state, &id).expect("unit").damage = damage;
        }
    }
}

fn plain(events: &[GameEvent], after: &GameState) -> bool {
    if after.pending.is_some() {
        return false;
    }
    let declared = events
        .iter()
        .filter(|event| matches!(event, GameEvent::AttackDeclared { .. }))
        .count();
    let odd = events.iter().any(|event| {
        matches!(event, GameEvent::AttackDeclared { forced, instead_of, .. } if *forced || instead_of.is_some())
    });
    if declared != 1 || odd {
        return false;
    }
    !events.iter().any(|event| {
        matches!(
            event,
            GameEvent::HookResolving { .. }
                | GameEvent::TrapFired { .. }
                | GameEvent::AttackCancelled { .. }
                | GameEvent::Redirected { .. }
                | GameEvent::TurnEnded { .. }
                | GameEvent::TurnAutoEnded { .. }
                | GameEvent::TurnCutShort { .. }
        )
    })
}

#[test]
fn explore_forecast_against_real_cards() {
    jackioh_cards::register_all();
    const CASES: i32 = 700;
    let mut checked = 0;
    let mut skipped = 0;
    let mut drifts = 0;
    for index in 0..CASES {
        let seed = format!("audit-combat-{index}");
        let mut rng = Rng::new(&seed, 0);
        let pool1 = rng.chance(0.2);
        let p1 = side(&mut rng, pool1);
        let pool2 = rng.chance(0.3);
        let mut p2 = side(&mut rng, pool2);
        p2["health"] = json!(1 + rng.int(14));
        p2["armor"] = json!(rng.int(3));
        let mut state = scenario(json!({ "seed": seed, "active": "p1", "turn": 9, "p1": p1, "p2": p2 }))
            .state()
            .clone();
        dress(&mut state, &mut rng);
        if rng.chance(0.15) {
            state.slashed_lane = Some(3);
        }
        for forecast in combat_forecast(&state, P1).attacks {
            let action = Action::new(
                ActionBody::Attack {
                    attacker_id: forecast.attacker_id.clone(),
                    target_id: forecast.target_id.clone(),
                },
                P1,
                "audit-attack",
            );
            let after = reduce(&state, &action);
            if let Some(error) = &after.error {
                println!("LISTED BUT REFUSED {seed}: {} -> {}: {error}", forecast.attacker_id, forecast.target_id);
                drifts += 1;
                continue;
            }
            if !plain(&after.events, &after.state) {
                skipped += 1;
                continue;
            }
            let splashed: Vec<String> = forecast.splash.iter().map(|hit| hit.instance_id.clone()).collect();
            let played = PlayedAttack::read(
                &after.events,
                &after.state,
                &forecast.attacker_id,
                &forecast.target_id,
                &splashed,
            );
            let said = PlayedAttack::forecast(&forecast);
            checked += 1;
            if played != said {
                drifts += 1;
                if drifts <= 40 {
                    let attacker = find_instance(&state, &forecast.attacker_id).expect("attacker");
                    let a = unit_view(&state, attacker);
                    let target = find_instance(&state, &forecast.target_id);
                    let t = target.map(|unit| unit_view(&state, unit));
                    println!(
                        "DRIFT {seed}: {} ({}) -> {} ({:?})\n  forecast {said:?}\n  played   {played:?}\n  attacker {}/{} {:?}\n  target {:?}",
                        forecast.attacker_id,
                        attacker.def_id,
                        forecast.target_id,
                        target.map(|unit| unit.def_id.clone()),
                        a.attack,
                        a.health,
                        a.keywords,
                        t.map(|view| (view.attack, view.health, view.position, view.keywords)),
                    );
                }
            }
        }
    }
    println!("checked {checked}, skipped {skipped}, drifts {drifts}");
    assert_eq!(drifts, 0);
}

const ALL_UNITS: &str = include_str!("/tmp/claude-0/-home-user-JackiOh/ea3d4079-b773-5a4f-b723-2c6f129bcd0b/scratchpad/unit_ids.txt");

fn wide_side(rng: &mut Rng, all: &[&str]) -> Value {
    let mut field = Vec::new();
    for lane in 1..=5 {
        if !rng.chance(0.5) {
            continue;
        }
        let def = rng.pick(all).copied().unwrap_or(VANILLA);
        field.push(json!({ "def": def, "lane": lane, "radiant": rng.chance(0.25) }));
    }
    json!({ "field": field, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] })
}

#[test]
fn explore_forecast_against_every_unit() {
    jackioh_cards::register_all();
    let all: Vec<&str> = ALL_UNITS.lines().filter(|line| !line.is_empty()).collect();
    const CASES: i32 = 3000;
    let (mut checked, mut skipped, mut drifts, mut setup_panics, mut combat_panics) = (0, 0, 0, 0, 0);
    let mut by_def: std::collections::BTreeMap<String, i32> = Default::default();
    std::panic::set_hook(Box::new(|_| {}));
    for index in 0..CASES {
        let seed = format!("audit-wide-{index}");
        let mut rng = Rng::new(&seed, 0);
        let p1 = wide_side(&mut rng, &all);
        let mut p2 = wide_side(&mut rng, &all);
        p2["health"] = json!(1 + rng.int(14));
        p2["armor"] = json!(rng.int(3));
        let opts = json!({ "seed": seed, "active": "p1", "turn": 9, "p1": p1, "p2": p2 });
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let state = scenario(opts.clone()).state().clone();
            let forecast = combat_forecast(&state, P1);
            (state, forecast)
        }));
        let Ok((state, forecast)) = built else {
            setup_panics += 1;
            continue;
        };
        for forecast in forecast.attacks {
            let action = Action::new(
                ActionBody::Attack {
                    attacker_id: forecast.attacker_id.clone(),
                    target_id: forecast.target_id.clone(),
                },
                P1,
                "audit-attack",
            );
            let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&state, &action)));
            let Ok(after) = ran else {
                combat_panics += 1;
                let attacker = find_instance(&state, &forecast.attacker_id).map(|c| c.def_id.clone());
                let target = find_instance(&state, &forecast.target_id).map(|c| c.def_id.clone());
                eprintln!("PANIC {seed}: {attacker:?} -> {target:?} ({})", forecast.target_id);
                continue;
            };
            if let Some(error) = &after.error {
                eprintln!("LISTED BUT REFUSED {seed}: {} -> {}: {error}", forecast.attacker_id, forecast.target_id);
                drifts += 1;
                continue;
            }
            if !plain(&after.events, &after.state) {
                skipped += 1;
                continue;
            }
            let splashed: Vec<String> = forecast.splash.iter().map(|hit| hit.instance_id.clone()).collect();
            let played = PlayedAttack::read(
                &after.events,
                &after.state,
                &forecast.attacker_id,
                &forecast.target_id,
                &splashed,
            );
            let said = PlayedAttack::forecast(&forecast);
            checked += 1;
            if played != said {
                drifts += 1;
                let attacker = find_instance(&state, &forecast.attacker_id).expect("attacker");
                let target = find_instance(&state, &forecast.target_id);
                let key = format!(
                    "{} -> {}",
                    attacker.def_id,
                    target.map_or("hero".to_string(), |unit| unit.def_id.clone())
                );
                *by_def.entry(key).or_default() += 1;
                if true {
                    let a = unit_view(&state, attacker);
                    let t = target.map(|unit| unit_view(&state, unit));
                    let board: Vec<String> = [P1, P2]
                        .into_iter()
                        .flat_map(|player| {
                            state.players[player].units.iter().flatten().flatten().map(move |c| {
                                format!("{player:?}:{}{}", c.def_id, if c.radiant { "*" } else { "" })
                            })
                        })
                        .collect();
                    eprintln!(
                        "DRIFT {seed}: {} ({}) -> {} ({:?})\n  forecast {said:?}\n  played   {played:?}\n  attacker {}/{} {:?}\n  target {:?}\n  board {board:?}",
                        forecast.attacker_id,
                        attacker.def_id,
                        forecast.target_id,
                        target.map(|unit| unit.def_id.clone()),
                        a.attack,
                        a.health,
                        a.keywords,
                        t.map(|view| (view.attack, view.health, view.position, view.keywords)),
                    );
                }
            }
        }
    }
    let _ = std::panic::take_hook();
    eprintln!("checked {checked}, skipped {skipped}, drifts {drifts}, setup panics {setup_panics}, combat panics {combat_panics}");
    eprintln!("{by_def:#?}");
    assert_eq!(drifts + combat_panics, 0);
}
