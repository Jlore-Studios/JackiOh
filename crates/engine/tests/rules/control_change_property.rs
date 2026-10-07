//! Port of `packages/engine/test/control-change.property.test.ts`.
//!
//! R171 over random boards (SPEC §4.1, §11; docs/polish/4-edge-cases.md "fast-check properties").
//!
//! Each case builds a random board — per side and per lane, optionally one keyword body in either
//! position, entered this turn, last turn or never marked, with any exertion, and optionally a Stack
//! card on top of it — then applies one to three random control changes for either player: a steal
//! of one of the actor's enemy top units, Steal all, the board swap, or a rotation in either
//! direction on either face. The actor may be the inactive player, which is the opponent's-turn case.
//!
//! Four properties, each read from outside the code under test:
//!   P1 bookkeeping: a card that got a `controlChanged` and is still on the field took this turn and
//!      a fresh exertion; every other card on the field kept exactly what it started with; a card's
//!      controller only changes with a `controlChanged`; nobody's owner changes.
//!   P2 the §6.1 oracle: a unit that crossed, or started the turn freshly entered, is sick, so with
//!      neither Rush nor Charge it has no target and without Charge it cannot aim at the hero; one
//!      that crossed with Charge and nothing else stopping it has a target (the fresh exertion).
//!   P3 `legalActions` offers exactly the attacks `reduce` accepts.
//!   P4 R53: a forced attack by a unit that crossed still happens and spends nothing.
//!
//! Every run is reproducible from PROPERTY_SEED (CLAUDE.md rules 4 and 9 in spirit).
//!
//! fast-check is a TS library with no Rust counterpart in the workspace (SURFACE §2), so the cases
//! are drawn here from the engine's own seeded `Rng`, one stream per run (`"<PROPERTY_SEED>:<run>"`),
//! with the arbitraries' shapes and frequencies kept (fast-check's `fc.option` builds its nil one
//! time in five); a failing run names its index, which reproduces it alone. The run counts are TS's.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{rotate, steal, steal_all, swap_board};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{charger, deft_duelist, pacifist, plain, rusher, stacker, taunter, zero_attack};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

/// The one seed every property runs from, so a failure reproduces exactly.
const PROPERTY_SEED: u32 = 20260922;
/// P1, P2 and P4 build a board and apply a few effects: cheap.
const BOOKKEEPING_RUNS: u32 = 300;
const ORACLE_RUNS: u32 = 300;
const FORCED_RUNS: u32 = 150;
/// P3 calls `reduce` for every attacker and target, and each call clones the state.
const AGREEMENT_RUNS: u32 = 60;

/// The keyword bodies of `fixtures/combat` a unit zone is filled from.
fn bodies() -> Vec<CardDef> {
    vec![plain.clone(), rusher.clone(), charger.clone(), deft_duelist.clone(), taunter.clone(), pacifist.clone(), zero_attack.clone()]
}

const FRESH: Exertion = Exertion {
    attacked: false,
    switched: false,
    attacks: None,
};

#[derive(Clone, Copy, Debug)]
enum Entry {
    Now,
    Last,
    Unset,
}

#[derive(Clone, Copy, Debug)]
struct Marks {
    entry: Entry,
    attacked: bool,
    switched: bool,
}

#[derive(Clone, Debug)]
struct UnitSpec {
    marks: Marks,
    body: usize,
    position: Position,
    top: Option<Marks>,
}

type Board = PerPlayer<Vec<Option<UnitSpec>>>;

#[derive(Clone, Debug)]
enum Verb {
    Steal {
        actor: PlayerId,
        pick: usize,
    },
    StealAll {
        actor: PlayerId,
    },
    SwapBoard {
        actor: PlayerId,
    },
    Rotate {
        actor: PlayerId,
        direction: RotationDirection,
        radiant: bool,
    },
}

impl Verb {
    fn actor(&self) -> PlayerId {
        match *self {
            Verb::Steal { actor, .. }
            | Verb::StealAll { actor }
            | Verb::SwapBoard { actor }
            | Verb::Rotate { actor, .. } => actor,
        }
    }
}

#[derive(Clone, Debug)]
struct Case {
    board: Board,
    verbs: Vec<Verb>,
}

// ---- The arbitraries (fast-check's, drawn from a seeded `Rng`) ----------------------------------

/// `fc.option(arb)`: nil one time in five (fast-check's default `freq`).
fn option<T>(rng: &mut Rng, arb: impl FnOnce(&mut Rng) -> T) -> Option<T> {
    if rng.int(5) == 0 { None } else { Some(arb(rng)) }
}

fn entry_arb(rng: &mut Rng) -> Entry {
    match rng.int(3) {
        0 => Entry::Now,
        1 => Entry::Last,
        _ => Entry::Unset,
    }
}

fn marks_arb(rng: &mut Rng) -> Marks {
    Marks {
        entry: entry_arb(rng),
        attacked: rng.coin(),
        switched: rng.coin(),
    }
}

fn unit_arb(rng: &mut Rng) -> UnitSpec {
    let body = rng.int(bodies().len() as i32) as usize;
    let position = if rng.coin() { Position::Atk } else { Position::Def };
    let marks = marks_arb(rng);
    let top = option(rng, marks_arb);
    UnitSpec {
        marks,
        body,
        position,
        top,
    }
}

fn side_arb(rng: &mut Rng) -> Vec<Option<UnitSpec>> {
    (0..UNIT_ZONES).map(|_| option(rng, unit_arb)).collect()
}

fn actor_arb(rng: &mut Rng) -> PlayerId {
    if rng.coin() { P1 } else { P2 }
}

fn verb_arb(rng: &mut Rng) -> Verb {
    match rng.int(4) {
        0 => Verb::Steal {
            actor: actor_arb(rng),
            pick: rng.int(UNIT_ZONES) as usize,
        },
        1 => Verb::StealAll { actor: actor_arb(rng) },
        2 => Verb::SwapBoard { actor: actor_arb(rng) },
        _ => Verb::Rotate {
            actor: actor_arb(rng),
            direction: if rng.coin() {
                RotationDirection::Left
            } else {
                RotationDirection::Right
            },
            radiant: rng.coin(),
        },
    }
}

fn case_arb(run: u32) -> Case {
    let mut rng = Rng::new(&format!("{PROPERTY_SEED}:{run}"), 0);
    let board = PerPlayer::new(side_arb(&mut rng), side_arb(&mut rng));
    let count = 1 + rng.int(3);
    let verbs = (0..count).map(|_| verb_arb(&mut rng)).collect();
    Case { board, verbs }
}

// ---------------------------------------------------------------------------
// Building and running a case.
// ---------------------------------------------------------------------------

/// TS's module `let nonce = 0`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("ccp{nonce}")))
}

/// Past both mulligans, in p1's main phase of turn 1, with an empty board (rulings-b's `playing`).
fn playing() -> GameState {
    let mut state = begin_game(&new_game("control-change-property", None)).state;
    for player in PLAYER_IDS {
        let keep: Vec<String> = state.players[player].hand.iter().map(|c| c.id.clone()).collect();
        let result = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
        if let Some(error) = result.error {
            panic!("{error}");
        }
        state = result.state;
    }
    state
}

/// TS's module `const BASE = playing()`. Built once per test, on the test's own thread, because the
/// fixture registries `new_game` installs are the testkit's thread-local override (SURFACE §8).
fn base() -> GameState {
    playing()
}

fn mark(card: &mut CardInstance, marks: Marks, turn: i32) {
    card.summoned_turn = match marks.entry {
        Entry::Now => Some(turn),
        Entry::Last => Some(turn - 1),
        Entry::Unset => None,
    };
    card.exertion = Exertion {
        attacked: marks.attacked,
        switched: marks.switched,
        attacks: None,
    };
}

/// Every card on the field, dormant pile cards and backrow cards included.
fn field_cards(state: &GameState) -> Vec<CardInstance> {
    PLAYER_IDS
        .into_iter()
        .flat_map(|player| {
            let side = &state.players[player];
            let units = side.units.iter().flatten().flatten().cloned();
            let backrow = side.backrow.iter().flatten().cloned();
            units.chain(backrow).collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    summoned_turn: Option<i32>,
    exertion: Exertion,
    controller: PlayerId,
    owner: PlayerId,
}

fn build(base: &GameState, board: &Board) -> (GameState, IndexMap<String, Snapshot>) {
    let mut state = base.clone();
    let turn = state.turn;
    let bodies = bodies();
    for player in PLAYER_IDS {
        for (index, spec) in board[player].iter().enumerate() {
            let Some(spec) = spec else {
                continue;
            };
            let lane = index as i32 + 1;
            let def = bodies.get(spec.body).cloned().unwrap_or_else(|| plain.clone());
            let body = put(&mut state, &def.id, slot(player, Row::Units, lane), json!({}));
            {
                let held = find_instance_mut(&mut state, &body.id).expect("on the field");
                held.position = Some(spec.position);
                mark(held, spec.marks, turn);
            }
            let Some(top_marks) = spec.top else {
                continue;
            };
            let top = new_instance(&mut state, &stacker.id, player, Zone::Hand { player });
            let top_id = top.id.clone();
            if !place_on_field(&mut state, top, slot(player, Row::Units, lane), json_as(json!({ "stack": true }))) {
                panic!("could not stack");
            }
            mark(find_instance_mut(&mut state, &top_id).expect("stacked"), top_marks, turn);
        }
    }
    let start: IndexMap<String, Snapshot> = field_cards(&state)
        .into_iter()
        .map(|card| {
            (
                card.id.clone(),
                Snapshot {
                    summoned_turn: card.summoned_turn,
                    exertion: card.exertion,
                    controller: card.controller,
                    owner: card.owner,
                },
            )
        })
        .collect();
    (state, start)
}

fn effect_of(state: &GameState, verb: &Verb) -> Option<Effect> {
    match verb {
        Verb::Steal { actor, pick } => {
            let enemies: Vec<String> = active_units_of(state, opponent_of(*actor))
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let target = enemies.get(pick % enemies.len().max(1))?;
            Some(steal(json_as(json!({ "instanceId": target }))))
        }
        Verb::StealAll { .. } => Some(steal_all(Default::default())),
        Verb::SwapBoard { .. } => Some(swap_board()),
        Verb::Rotate {
            direction, radiant, ..
        } => Some(rotate(json_as(json!({ "direction": direction, "radiant": radiant })))),
    }
}

/// Apply the verbs in order, each for its actor, and return every event with the crossed ids.
fn apply(state: &mut GameState, verbs: &[Verb]) -> (Vec<GameEvent>, IndexSet<String>) {
    let mut events: Vec<GameEvent> = Vec::new();
    for verb in verbs {
        let Some(effect) = effect_of(state, verb) else {
            continue;
        };
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let sink = EngineSink::new(state, &mut events, &mut rng);
            let mut ctx = make_context(
                sink,
                None,
                HookOptions {
                    controller: Some(verb.actor()),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        state.rng_cursor = rng.cursor();
    }
    let crossed: IndexSet<String> = events_of_type(&events, GameEventType::ControlChanged)
        .iter()
        .map(|event| match event {
            GameEvent::ControlChanged { instance_id, .. } => instance_id.clone(),
            other => panic!("not a controlChanged event: {other:?}"),
        })
        .collect();
    (events, crossed)
}

struct Ran {
    state: GameState,
    start: IndexMap<String, Snapshot>,
    crossed: IndexSet<String>,
}

fn run(base: &GameState, sample: &Case) -> Ran {
    let (mut state, start) = build(base, &sample.board);
    let (_, crossed) = apply(&mut state, &sample.verbs);
    Ran { state, start, crossed }
}

// ---------------------------------------------------------------------------
// The properties.
// ---------------------------------------------------------------------------

/// `describe("R171 over random boards and random control changes (fast-check)")`.
mod r171_over_random_boards_and_random_control_changes_fast_check {
    use super::*;

    #[test]
    fn r171_r747_p1_only_a_card_that_changed_sides_takes_this_turn_and_a_fresh_exertion_and_no_owner_changes_but_a_bounce_s(
    ) {
        let base = base();
        for case in 0..BOOKKEEPING_RUNS {
            let sample = case_arb(case);
            let Ran { state, start, crossed } = run(&base, &sample);
            let turn = state.turn;
            for card in field_cards(&state) {
                let Some(before) = start.get(&card.id) else {
                    panic!("run {case}: {} appeared on the field", card.id);
                };
                assert_eq!(
                    card.controller,
                    card.zone.player(),
                    "run {case}: {} controller vs its zone",
                    card.id
                );
                if card.controller != before.controller {
                    assert!(crossed.contains(&card.id), "run {case}: {} changed side silently", card.id);
                }
                if crossed.contains(&card.id) {
                    assert_eq!(card.summoned_turn, Some(turn), "run {case}: {} crossed", card.id);
                    assert_eq!(card.exertion, FRESH, "run {case}: {} crossed", card.id);
                } else {
                    assert_eq!(card.summoned_turn, before.summoned_turn, "run {case}: {} stayed", card.id);
                    assert_eq!(card.exertion, before.exertion, "run {case}: {} stayed", card.id);
                }
            }
            for (id, before) in &start {
                let card = find_instance(&state, id);
                // R747: a card a bounce took off the field is in its controller's hand and theirs now (R12);
                // every other card is still its first owner's.
                match card {
                    Some(card) if card.zone.z() == ZoneName::Hand => {
                        assert_eq!(card.owner, card.zone.player(), "run {case}: {id} owner in a hand");
                    }
                    _ => assert_eq!(card.map(|c| c.owner), Some(before.owner), "run {case}: {id} owner"),
                }
            }
        }
    }

    #[test]
    fn r171_p2_a_unit_that_crossed_is_sick_like_one_summoned_this_turn_and_a_charge_unit_that_crossed_can_attack() {
        let base = base();
        for case in 0..ORACLE_RUNS {
            let sample = case_arb(case);
            let Ran { state, start, crossed } = run(&base, &sample);
            let turn = state.turn;
            let units: Vec<CardInstance> = active_units_of(&state, state.active)
                .iter()
                .map(|card| find_instance(&state, &card.id).cloned().expect("on the field"))
                .collect();
            for unit in &units {
                let view = unit_view(&state, unit);
                let rush = has_keyword(&view.keywords, KeywordKind::Rush);
                let charge = has_keyword(&view.keywords, KeywordKind::Charge);
                let moved = crossed.contains(&unit.id);
                let sick = moved || start.get(&unit.id).is_some_and(|before| before.summoned_turn == Some(turn));
                let targets = attack_targets(&state, unit);
                let who = format!("run {case}: {} ({})", unit.id, unit.def_id);
                if sick && !rush && !charge {
                    assert!(targets.is_empty(), "{who} is sick");
                }
                if sick && !charge {
                    assert!(
                        !targets.iter().any(|target| matches!(target, AttackTarget::Hero { .. })),
                        "{who} aims at the hero"
                    );
                }
                if moved
                    && charge
                    && view.position == Position::Atk
                    && view.attack > 0
                    && !has_keyword(&view.keywords, KeywordKind::CantAttack)
                {
                    assert!(!targets.is_empty(), "{who} crossed with Charge");
                }
            }
        }
    }

    #[test]
    fn r171_p3_legalactions_offers_an_attack_exactly_when_reduce_accepts_it() {
        let base = base();
        for case in 0..AGREEMENT_RUNS {
            let sample = case_arb(case);
            let Ran { state, .. } = run(&base, &sample);
            let player = state.active;
            let enemy = opponent_of(player);
            let offered: IndexSet<String> = legal_actions(&state, player)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Attack {
                        attacker_id,
                        target_id,
                    } => Some(format!("{attacker_id}>{target_id}")),
                    _ => None,
                })
                .collect();
            let mut target_ids: Vec<String> =
                active_units_of(&state, enemy).iter().map(|card| card.id.clone()).collect();
            target_ids.push(format!("hero-{enemy}"));
            let attackers: Vec<(String, String)> = active_units_of(&state, player)
                .iter()
                .map(|card| (card.id.clone(), card.def_id.clone()))
                .collect();
            for (attacker_id, attacker_def) in &attackers {
                for target_id in &target_ids {
                    let result = act(
                        &state,
                        json!({ "type": "attack", "playerId": player, "attackerId": attacker_id, "targetId": target_id }),
                    );
                    assert_eq!(
                        result.error.is_none(),
                        offered.contains(&format!("{attacker_id}>{target_id}")),
                        "run {case}: {attacker_id} ({attacker_def}) → {target_id}: {}",
                        result.error.as_deref().unwrap_or("accepted")
                    );
                }
            }
        }
    }

    #[test]
    fn r171_r53_p4_a_unit_that_crossed_is_still_made_to_attack_by_a_forced_attack_which_spends_nothing() {
        let base = base();
        for case in 0..FORCED_RUNS {
            let sample = case_arb(case);
            let Ran { state, crossed, .. } = run(&base, &sample);
            for id in &crossed {
                let mut probe = state.clone();
                let Some(attacker) = find_instance(&probe, id).cloned() else {
                    continue;
                };
                if !is_active_on_field(&probe, &attacker) {
                    continue;
                }
                if !matches!(attacker.zone, Zone::Field { row: Row::Units, .. }) {
                    continue;
                }
                let enemy = opponent_of(attacker.controller);
                let first = active_units_of(&probe, enemy)
                    .first()
                    .map(|card| find_instance(&probe, &card.id).cloned().expect("on the field"));
                let target: AttackTarget = match first {
                    None => AttackTarget::Hero { player: enemy },
                    Some(instance) => AttackTarget::Unit { instance },
                };
                let before = attacker.exertion;

                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = Rng::new(&probe.seed, probe.rng_cursor);
                {
                    let mut sink = EngineSink::new(&mut probe, &mut events, &mut rng);
                    force_attack(&mut sink, &attacker, &target);
                }

                let forced = events_of_type(&events, GameEventType::AttackDeclared)
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::AttackDeclared { attacker_id, forced: true, .. } if attacker_id == id)
                    })
                    .count();
                assert_eq!(forced, 1, "run {case}: {id} forced");
                assert_eq!(
                    find_instance(&probe, id).map(|card| card.exertion),
                    Some(before),
                    "run {case}: {id} spent nothing"
                );
            }
        }
    }
}
