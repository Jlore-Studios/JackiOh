// THROWAWAY (#874 debugging) — delete before finishing.
use jackioh_ai::{AI_GATE_BUDGET, AiOptions, MatchHooks, decide, game_config, play_match};
use jackioh_engine::{ActionBody, GameState, PlayerId, Rng};

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static CUR: AtomicUsize = AtomicUsize::new(0);
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static IN: AtomicBool = AtomicBool::new(false);
static BIG: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
const LIMIT: usize = 2_000_000_000;
struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let now = CUR.fetch_add(l.size(), Ordering::Relaxed) + l.size();
        if l.size() >= 200_000 && BIG.fetch_add(1, Ordering::Relaxed) < 12 && !IN.swap(true, Ordering::SeqCst) {
            let bt = std::backtrace::Backtrace::force_capture().to_string();
            let frames: Vec<String> = bt.lines().filter(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit()) && l.contains("jackioh_")).take(10).map(|l| l.split(": ").nth(1).unwrap_or("").replace("jackioh_engine::", "E::").replace("jackioh_ai::", "A::")).collect();
            eprintln!("BIG {}B cur={}MB t={:?}: {}", l.size(), now / 1_000_000, START.get().map(|s| s.elapsed()), frames.join(" <- "));
            IN.store(false, Ordering::SeqCst);
        }
        if now > LIMIT {
            let k = COUNT.fetch_add(1, Ordering::Relaxed);
            if k % 1_000_000 == 0 && !IN.swap(true, Ordering::SeqCst) {
                let bt = std::backtrace::Backtrace::force_capture().to_string();
                let frames: Vec<String> = bt.lines().filter(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit()) && l.contains("jackioh_")).take(14).map(|l| l.split(": ").nth(1).unwrap_or("").replace("jackioh_engine::", "E::").replace("jackioh_ai::", "A::")).collect();
                eprintln!("SAMPLE {} cur={}MB t={:?}: {}", k / 1_000_000, now / 1_000_000, START.get().map(|s| s.elapsed()), frames.join(" <- "));
                IN.store(false, Ordering::SeqCst);
                if k / 1_000_000 >= 25 { std::process::exit(7); }
            }
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        CUR.fetch_sub(l.size(), Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

fn main() {
    START.get_or_init(std::time::Instant::now);
    jackioh_cards::register_all();
    let mode = std::env::args().nth(1).unwrap();
    let n: i32 = std::env::args().nth(2).unwrap().parse().unwrap();
    let matchup = jackioh_ai::Matchup::AiVsGreedy;
    let seed = format!("gate:v3:ai-vs-greedy:{n}");
    if mode == "capture" {
        let from_turn: i32 = std::env::args().nth(3).unwrap().parse().unwrap();
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(
                |_before: &GameState, after: &GameState, _actor: PlayerId, _a: &ActionBody| {
                    if after.turn >= from_turn {
                        std::fs::write("/tmp/state874.json", serde_json::to_string(after).unwrap()).unwrap();
                    }
                },
            )),
            ..MatchHooks::default()
        };
        let cfg = game_config(matchup, n, AI_GATE_BUDGET, "gate:v3");
        play_match(&cfg, &mut hooks);
    } else {
        let state: GameState = serde_json::from_str(&std::fs::read_to_string("/tmp/state874.json").unwrap()).unwrap();
        let seat = jackioh_engine::seat_to_act(&state).unwrap();
        println!("seat {seat} turn {}", state.turn);
        let mut options = AiOptions {
            rng: Rng::new(&format!("{seed}:ctl:{seat}"), 0),
            budget: AI_GATE_BUDGET,
            should_stop: None,
        };
        let d = decide(&state, seat, &mut options);
        println!("{:?}", d.map(|d| d.stats));
    }
}
