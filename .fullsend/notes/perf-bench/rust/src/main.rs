//! Rust engine benchmark: the golden replay (legal_actions for the actor, reduce, hash_state,
//! view_for x2 per step) over the first N lines of games.jsonl, then a micro-benchmark of each call
//! on one mid-game state, including the JSON round trip the WASM bindings do. Times are this
//! thread's CPU time (CLOCK_THREAD_CPUTIME_ID), which a loaded machine does not inflate as it does
//! wall time; wall time is printed beside the totals.
//!
//!   jackioh-bench <repo> [games=200] [line=2] [step=100] [iters=200] [only=all|replay|micro|loop-<op>]
use std::time::Instant;

use jackioh_engine::{
    Action, CreateGameArgs, GameState, PlayerId, begin_game, create_game, hash_state, legal_actions, reduce,
    view_for,
};
use serde_json::Value;

/// This thread's CPU time, in seconds.
fn cpu() -> f64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: a valid out-pointer for clock_gettime.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let repo = &args[1];
    let games: usize = args.get(2).map_or(200, |s| s.parse().unwrap());
    let line_at: usize = args.get(3).map_or(2, |s| s.parse().unwrap());
    let step_at: usize = args.get(4).map_or(100, |s| s.parse().unwrap());
    let iters: usize = args.get(5).map_or(200, |s| s.parse().unwrap());
    let only = args.get(6).map_or("all", String::as_str);
    jackioh_cards::register_all();
    let text = std::fs::read_to_string(format!("{repo}/crates/engine/tests/golden/games.jsonl")).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let parsed: Vec<(CreateGameArgs, Vec<Action>)> = lines
        .iter()
        .map(|g| {
            let args: CreateGameArgs = serde_json::from_value(g["args"].clone()).unwrap();
            let acts: Vec<Action> = g["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| serde_json::from_value(s["a"].clone()).unwrap())
                .collect();
            (args, acts)
        })
        .collect();

    let state_at = |line_at: usize, step_at: usize| {
        let (args, acts) = &parsed[line_at];
        let at = step_at.min(acts.len() - 1);
        let mut state = begin_game(&create_game(args)).state;
        for a in &acts[..at] {
            state = reduce(&state, a).state;
        }
        (state, acts[at].clone(), at)
    };

    if let Some(op) = only.strip_prefix("loop-") {
        // A tight loop over one call on the micro state, for the profiler.
        let (state, next, _) = state_at(line_at, step_at);
        let state_json = serde_json::to_string(&state).unwrap();
        for _ in 0..iters {
            match op {
                "view" => {
                    std::hint::black_box(view_for(&state, PlayerId::P1));
                    std::hint::black_box(view_for(&state, PlayerId::P2));
                }
                "reduce" => {
                    std::hint::black_box(reduce(&state, &next));
                }
                "legal" => {
                    std::hint::black_box(legal_actions(&state, next.player_id));
                }
                "hash" => {
                    std::hint::black_box(hash_state(&state));
                }
                "parse" => {
                    std::hint::black_box(serde_json::from_str::<GameState>(&state_json).unwrap());
                }
                _ => panic!("unknown op {op}"),
            }
        }
        return;
    }

    if only == "all" || only == "replay" {
        let (mut legal, mut red, mut hash, mut view) = (0f64, 0f64, 0f64, 0f64);
        let mut steps = 0usize;
        let wall = Instant::now();
        let cpu0 = cpu();
        for (args, acts) in parsed.iter().take(games) {
            let mut state = begin_game(&create_game(args)).state;
            for a in acts {
                let t0 = cpu();
                std::hint::black_box(legal_actions(&state, a.player_id));
                let t1 = cpu();
                let r = reduce(&state, a);
                let t2 = cpu();
                assert!(r.error.is_none(), "refused: {:?}", r.error);
                state = r.state;
                std::hint::black_box(hash_state(&state));
                let t3 = cpu();
                std::hint::black_box(view_for(&state, PlayerId::P1));
                std::hint::black_box(view_for(&state, PlayerId::P2));
                let t4 = cpu();
                legal += t1 - t0;
                red += t2 - t1;
                hash += t3 - t2;
                view += t4 - t3;
                steps += 1;
            }
        }
        let total_cpu = cpu() - cpu0;
        let wall = wall.elapsed().as_secs_f64();
        let ms = |x: f64| x * 1000.0 / steps as f64;
        println!(
            "golden replay (native): {games} games, {steps} steps, cpu {total_cpu:.2} s, wall {wall:.2} s"
        );
        println!(
            "  per step, cpu ms: legal {:.3}  reduce {:.3}  hash {:.3}  view x2 {:.3}  total {:.3}",
            ms(legal),
            ms(red),
            ms(hash),
            ms(view),
            ms(total_cpu)
        );
    }

    if only == "all" || only == "micro" {
        let (state, next, at) = state_at(line_at, step_at);
        let next = &next;
        let state_json = serde_json::to_string(&state).unwrap();
        let action_json = serde_json::to_string(next).unwrap();
        println!(
            "micro (native): line {line_at} step {at}, state {:.1} KB, next {action_json}, {iters} iters, cpu ms per call",
            state_json.len() as f64 / 1024.0,
        );
        let bench = |name: &str, run: &mut dyn FnMut()| {
            for _ in 0..iters.min(20) {
                run();
            }
            let t0 = cpu();
            for _ in 0..iters {
                run();
            }
            println!("  {name:<28} {:.3}", (cpu() - t0) * 1000.0 / iters as f64);
        };
        let p = next.player_id;
        bench("view_for(p1)", &mut || {
            std::hint::black_box(view_for(&state, PlayerId::P1));
        });
        bench("view_for(p2)", &mut || {
            std::hint::black_box(view_for(&state, PlayerId::P2));
        });
        bench("legal_actions(actor)", &mut || {
            std::hint::black_box(legal_actions(&state, p));
        });
        bench("reduce(next)", &mut || {
            std::hint::black_box(reduce(&state, next));
        });
        bench("hash_state", &mut || {
            std::hint::black_box(hash_state(&state));
        });
        bench("state.clone()", &mut || {
            std::hint::black_box(state.clone());
        });
        bench("from_str::<GameState>", &mut || {
            std::hint::black_box(serde_json::from_str::<GameState>(&state_json).unwrap());
        });
        bench("to_string(state)", &mut || {
            std::hint::black_box(serde_json::to_string(&state).unwrap());
        });
        bench("parse+view_for(p1)+to_string", &mut || {
            let s: GameState = serde_json::from_str(&state_json).unwrap();
            std::hint::black_box(serde_json::to_string(&view_for(&s, PlayerId::P1)).unwrap());
        });
        bench("parse+reduce+to_string", &mut || {
            let s: GameState = serde_json::from_str(&state_json).unwrap();
            let a: Action = serde_json::from_str(&action_json).unwrap();
            std::hint::black_box(serde_json::to_string(&reduce(&s, &a)).unwrap());
        });
    }
}
