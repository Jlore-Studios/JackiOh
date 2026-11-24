//! Audit of the engine's Duos mode (`crates/engine/src/duos/`, `duos_life.rs`). Temporary.

use jackioh_engine::config::DUOS_HUMAN_TEAM;
use jackioh_engine::duos::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::{A1, A2, B1, B2};

const VANILLA: &str = "core-008"; // Mr. Vanilla
const BALTHASAR: &str = "stinkyscape-018-2";

fn deck() -> Vec<String> {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 22, 23, 24, 25, 26, 28,
    ]
    .iter()
    .map(|n| format!("core-{n:03}"))
    .collect()
}

fn begun_with(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    jackioh_cards::register_all();
    let decks = DuosDecks {
        a1: deck(),
        a2: deck(),
        b1: deck(),
        b2: deck(),
    };
    duos_scenario(seed, decks, teams)
}

/// What `duos_create` puts on a duel's side (`duos_seat`): its team's own HP, unspent, on a full pool.
fn duos_side(s: &mut Scenario, player: PlayerId, own: i32) {
    let side = &mut s.state_mut().players[player];
    side.hero.health = own;
    side.auto_end_turn = Some(false);
    side.duos = Some(DuosLife {
        pool: DUOS_HUMAN_TEAM.pool_health,
        pool_cap: DUOS_HUMAN_TEAM.pool_health,
        spent: false,
    });
}

fn balthasar_votes(own: i32, duos: bool) -> usize {
    jackioh_cards::register_all();
    let library = vec![VANILLA; 4];
    let mut s = scenario(json!({
        "active": "p2",
        "p1": { "hand": [VANILLA], "backrow": [BALTHASAR], "health": own, "library": library },
        "p2": { "hand": [VANILLA], "library": library },
    }));
    if duos {
        duos_side(&mut s, P1, own);
        duos_side(&mut s, P2, DUOS_HUMAN_TEAM.own_health);
    }
    s.end_turn();
    assert_eq!(s.state().active, P1);
    s.last_events()
        .iter()
        .filter(|event| matches!(event, GameEvent::VoteCast { def_id, .. } if def_id == BALTHASAR))
        .count()
}

/// §13.2: "Each duel hero starts at its seat's own HP ... instead of at 30 health"; §13.4: a seat starts
/// with its team's `own_health` (20). BALTHASAR: "Vote to remove you if your hero has less than half its
/// starting health" (R1866002: "the health that hero started the game with"). Half of 20 is 10, so a
/// Duos hero on 14 of its 20 own HP has not lost half and BALTHASAR casts no vote.
#[test]
fn balthasar_reads_a_duos_heros_starting_health_as_its_own_hp() {
    // The control: outside Duos the hero started on 30, and 14 is below half.
    assert_eq!(balthasar_votes(14, false), 1, "1v1: 14 of 30 is below half");
    // In Duos, 9 of 20 is below half.
    assert_eq!(balthasar_votes(9, true), 1, "Duos: 9 of 20 is below half");
    // In Duos, 14 of 20 is not.
    assert_eq!(balthasar_votes(14, true), 0, "Duos: 14 of 20 is above half");
}

// ---- probe: random play with the real catalog (temporary) ----

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

fn random_decks(rng: &mut Lcg) -> DuosDecks {
    let pool: Vec<String> = jackioh_engine::registered_catalog()
        .values()
        .filter(|def| jackioh_engine::deckable(def))
        .map(|def| def.id.clone())
        .collect();
    let mut deal = || -> Vec<String> {
        let mut left = pool.clone();
        (0..DECK_SIZE).map(|_| left.swap_remove(rng.next(left.len()))).collect()
    };
    DuosDecks {
        a1: deal(),
        a2: deal(),
        b1: deal(),
        b2: deal(),
    }
}

fn pools_agree(state: &DuosState) -> Result<(), String> {
    for team in [Team::A, Team::B] {
        let pool = state.teams[team].pool;
        for duel in 0..2 {
            let side = &state.duels[duel].players[team.player()];
            let Some(life) = side.duos else {
                return Err(format!("duel {duel} team {team}: no duos life"));
            };
            if life.pool != pool {
                return Err(format!("duel {duel} team {team}: pool {} vs team {pool}", life.pool));
            }
            if life.spent && side.hero.health != pool {
                return Err(format!("duel {duel} team {team}: spent health {} vs pool {pool}", side.hero.health));
            }
            if !life.spent && side.hero.health <= 0 && state.result.is_none() {
                return Err(format!("duel {duel} team {team}: unspent at {}", side.hero.health));
            }
        }
    }
    Ok(())
}

fn play_one(seed: u64, max_steps: usize) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rng = Lcg(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let free = DuosTeamRules {
        pass_cost: 0,
        ..DUOS_HUMAN_TEAM
    };
    let options = DuosCreateOptions {
        seed: format!("audit-probe-{seed}"),
        decks: random_decks(&mut rng),
        teams: Some(PerTeam::new(free, DUOS_HUMAN_TEAM)),
        ..DuosCreateOptions::default()
    };
    let mut state = duos_begin(&duos_create(&options)).state;
    let mut monitor = create_duos_invariant_monitor(&state);
    let mut log: Vec<DuosAction> = Vec::new();
    let mut nonce = 0u64;
    for step in 0..max_steps {
        if state.result.is_some() {
            break;
        }
        let seats = duos_seats_to_act(&state);
        if seats.is_empty() {
            found.push(format!("seed {seed} step {step}: stuck, nobody to act, round {}", state.round));
            break;
        }
        let seat = seats[rng.next(seats.len())];
        let legal: Vec<DuosActionBody> = duos_legal_actions(&state, seat)
            .into_iter()
            .filter(|body| *body != DuosActionBody::Concede)
            .collect();
        // Every listed action must be accepted (a sample of them).
        for _ in 0..3 {
            let body = legal[rng.next(legal.len())].clone();
            let action = DuosAction {
                seat,
                nonce: format!("try-{nonce}"),
                body: body.clone(),
            };
            nonce += 1;
            let tried = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| duos_reduce(&state, &action)));
            match tried {
                Err(_) => found.push(format!("seed {seed} step {step}: PANIC on {seat} {body:?}")),
                Ok(result) => {
                    if let Some(error) = result.error {
                        found.push(format!("seed {seed} step {step}: listed but refused: {seat} {body:?}: {error}"));
                    }
                }
            }
        }
        // Prefer a pass one time in three, end turn rarely.
        let passes: Vec<&DuosActionBody> = legal
            .iter()
            .filter(|body| matches!(body, DuosActionBody::Pass { .. }))
            .collect();
        let non_end: Vec<&DuosActionBody> = legal
            .iter()
            .filter(|body| {
                !matches!(body, DuosActionBody::Pass { .. })
                    && **body != DuosActionBody::Duel { body: ActionBody::EndTurn }
            })
            .collect();
        let roll = rng.next(10);
        let body = if !passes.is_empty() && roll < 3 {
            passes[rng.next(passes.len())].clone()
        } else if !non_end.is_empty() && roll < 8 {
            non_end[rng.next(non_end.len())].clone()
        } else {
            legal[rng.next(legal.len())].clone()
        };
        let action = DuosAction {
            seat,
            nonce: format!("n-{nonce}"),
            body: body.clone(),
        };
        nonce += 1;
        for line in monitor.before(&state, seat, &body) {
            found.push(format!("seed {seed} step {step}: before {seat} {body:?}: {line}"));
        }
        let applied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| duos_reduce(&state, &action)));
        let result = match applied {
            Err(_) => {
                found.push(format!("seed {seed} step {step}: PANIC applying {seat} {body:?}"));
                break;
            }
            Ok(result) => result,
        };
        if let Some(error) = result.error {
            found.push(format!("seed {seed} step {step}: chosen refused: {seat} {body:?}: {error}"));
            break;
        }
        state = result.state;
        log.push(action);
        for line in monitor.after(&result.events, &state) {
            found.push(format!("seed {seed} step {step}: after {seat} {body:?}: {line}"));
        }
        if matches!(body, DuosActionBody::Pass { .. }) || step % 5 == 0 {
            for line in monitor.hidden(&state) {
                found.push(format!("seed {seed} step {step}: hidden after {seat} {body:?}: {line}"));
            }
        }
        if let Err(why) = pools_agree(&state) {
            found.push(format!("seed {seed} step {step}: after {seat} {body:?}: {why}"));
        }
        if step % 7 == 0 {
            let json = serde_json::to_string(&state).expect("serializes");
            let back: DuosState = serde_json::from_str(&json).expect("deserializes");
            if back != state {
                found.push(format!("seed {seed} step {step}: JSON round trip changed the state"));
            } else if duos_hash(&back) != duos_hash(&state) {
                found.push(format!("seed {seed} step {step}: JSON round trip changed the hash"));
            }
            for viewer in DUOS_SEATS {
                let view = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| duos_view_for(&state, viewer)));
                match view {
                    Err(_) => found.push(format!("seed {seed} step {step}: view PANIC for {viewer}")),
                    Ok(view) => {
                        for leak in check_duos_view_redaction(&view) {
                            found.push(format!("seed {seed} step {step}: {leak}"));
                        }
                    }
                }
            }
        }
        if found.len() > 8 {
            break;
        }
    }
    let folded = duos_fold(&DuosReplayInput {
        options: options.clone(),
        log: log.clone(),
    });
    if folded.refused_at.is_some() || folded.hash != duos_hash(&state) {
        found.push(format!(
            "seed {seed}: fold differs: refused_at {:?}, {} vs live {}",
            folded.refused_at,
            folded.hash,
            duos_hash(&state)
        ));
    }
    if state.result.as_ref().is_some_and(|result| result.reason != "poolEmptied") {
        for duel in 0..2 {
            let game = &state.duels[duel];
            eprintln!(
                "  duel {duel}: result {:?} turn {} healths {:?}/{:?} duos {:?}/{:?} last {:?}",
                game.result,
                game.turn,
                game.players.p1.hero.health,
                game.players.p2.hero.health,
                game.players.p1.duos,
                game.players.p2.duos,
                log.last()
            );
        }
        eprintln!("  teams {:?}", state.teams);
    }
    eprintln!(
        "seed {seed}: {} actions, round {}, result {:?}, findings {}",
        log.len(),
        state.round,
        state.result,
        found.len()
    );
    found
}

#[test]
fn probe_random_play() {
    jackioh_cards::register_all();
    let mut all: Vec<String> = Vec::new();
    for seed in [10u64, 109] {
        all.extend(play_one(seed, 500));
    }
    for line in &all {
        eprintln!("FOUND {line}");
    }
    assert!(all.is_empty(), "{} findings", all.len());
    let _ = (A1, A2, B1, B2, VANILLA, begun_with as fn(&str, Option<PerTeam<DuosTeamRules>>) -> DuosState);
}
