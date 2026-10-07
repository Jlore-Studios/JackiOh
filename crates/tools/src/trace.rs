//! `cargo jackioh trace <matchup> <n> [gate|full] [--series name]` (SURFACE §12): a tuning aid. Port
//! of `packages/ai/scripts/trace.ts`.
//!
//! Replays game n of a matchup, set up as the gate sets it up but from the tuning series
//! (AI_TUNING_SERIES; `--series <name>`, or `SERIES=<name>` as TS read it, picks another), and prints
//! it turn by turn. Writes no file.
//!
//! ```text
//! cargo jackioh trace ai-vs-greedy 3 [gate|full]
//! ```
//!
//! Not ported: bench.ts's `OVERRIDE_<CONFIG>` and `BAN` knobs, which TS applied by assigning into the
//! AI's config objects and its shadow-ban list at run time. In Rust those are constants (SURFACE §3:
//! no mutable statics), so a tuning run changes them in `crates/ai/src/config.rs` or
//! `shadow_ban.rs` and rebuilds.

use anyhow::Result;

use jackioh_ai::{AI_BUDGET, AI_EVAL, AI_GATE_BUDGET, AI_TUNING_SERIES, MatchHooks, NextSwing, evaluate, game_config, play_match};
use jackioh_engine::layers::unit_view;
use jackioh_engine::state::find_instance;
use jackioh_engine::zones::active_units_of;
use jackioh_engine::{ActionBody, GameState, PLAYER_IDS, Phase, PlayerId, Position};

use crate::gate::{parse_matchup, subject_seat_of};
use crate::sweep::to_fixed;

/// The evaluation prints with this many decimals.
const EVAL_DECIMALS: usize = 1;

/// `cargo jackioh trace …`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The matchup: ai-vs-random, ai-vs-greedy or hard-vs-easy.
    #[arg(default_value = "ai-vs-greedy")]
    pub matchup: String,
    /// The game's number (1-based); the subject sits p1 when it is odd.
    #[arg(default_value_t = 1)]
    pub n: i32,
    /// `full` plays at AI_BUDGET; anything else (or nothing) at AI_GATE_BUDGET.
    pub budget: Option<String>,
    /// The seed series (also `SERIES`); default AI_TUNING_SERIES.
    #[arg(long, value_name = "NAME")]
    pub series: Option<String>,
}

/// A card's name, by its def (a transient one first, as `findDef` reads it), else its id.
fn name_of(state: &GameState, def_id: &str) -> String {
    state
        .transient_defs
        .get(def_id)
        .or_else(|| jackioh_cards::CATALOG.get(def_id))
        .map_or_else(|| def_id.to_string(), |def| def.name.clone())
}

/// One seat's side of the board on one line: hero, mana, hand and library sizes, its units with
/// their stats and keyword initials, and its backrow.
fn board(state: &GameState, p: PlayerId) -> String {
    let units: Vec<String> = active_units_of(state, p)
        .iter()
        .map(|u| {
            let v = unit_view(state, u);
            let kw: String =
                v.keywords.iter().filter_map(|keyword| keyword.kind().as_str().chars().next()).collect();
            format!(
                "{}{} {}/{}{}{}",
                name_of(state, &u.def_id),
                if u.radiant { "*" } else { "" },
                v.attack,
                v.health,
                if v.position == Position::Def { " DEF" } else { "" },
                if kw.is_empty() { String::new() } else { format!(" [{kw}]") }
            )
        })
        .collect();
    let side = &state.players[p];
    let back: Vec<String> = side.backrow.iter().flatten().map(|card| name_of(state, &card.def_id)).collect();
    format!(
        "{p} hp {} mana {}/{} hand {} lib {} | {}{}",
        side.hero.health,
        side.mana.current,
        side.mana.max,
        side.hand.len(),
        side.library.len(),
        units.join(", "),
        if back.is_empty() { String::new() } else { format!(" || {}", back.join(", ")) }
    )
}

/// An action in words: what is played, what attacks what, what switches; anything else as its JSON.
fn describe(state: &GameState, _seat: PlayerId, action: &ActionBody) -> String {
    match action {
        ActionBody::Play { instance_id, targets, .. } => {
            let card = find_instance(state, instance_id);
            let targets = match targets {
                Some(targets) => serde_json::to_string(targets).unwrap_or_default(),
                None => String::new(),
            };
            let name = card.map_or_else(|| instance_id.clone(), |card| name_of(state, &card.def_id));
            format!("play {name} {targets}")
        }
        ActionBody::Attack { attacker_id, target_id } => {
            let attacker = find_instance(state, attacker_id)
                .map_or_else(|| attacker_id.clone(), |card| name_of(state, &card.def_id));
            let target = if target_id.starts_with("hero") {
                target_id.clone()
            } else {
                find_instance(state, target_id).map_or_else(|| target_id.clone(), |card| name_of(state, &card.def_id))
            };
            format!("attack {attacker} -> {target}")
        }
        ActionBody::SwitchPosition { instance_id } => {
            let name = find_instance(state, instance_id).map_or_else(String::new, |card| name_of(state, &card.def_id));
            format!("switch {name}")
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// `cargo jackioh trace`.
pub fn run(args: Args) -> Result<()> {
    jackioh_cards::register_all();
    let matchup = parse_matchup(&args.matchup)?;
    let n = args.n;
    let series = args
        .series
        .or_else(|| std::env::var("SERIES").ok())
        .unwrap_or_else(|| AI_TUNING_SERIES.to_string());
    let budget = if args.budget.as_deref() == Some("full") { AI_BUDGET } else { AI_GATE_BUDGET };
    let config = game_config(matchup, n, budget, &series);
    let subject = subject_seat_of(n);
    println!("subject {subject}; decks:");
    for p in PLAYER_IDS {
        let deck = match p {
            PlayerId::P1 => &config.decks.0,
            PlayerId::P2 => &config.decks.1,
        };
        println!("  {p} {}", deck.join(" "));
    }

    let mut last_turn: i32 = -1;
    let mut hooks = MatchHooks {
        after_action: Some(Box::new(|before: &GameState, _after: &GameState, seat: PlayerId, action: &ActionBody| {
            if before.turn != last_turn && before.phase == Phase::Main {
                last_turn = before.turn;
                let eval = evaluate(before, subject, NextSwing::Enemy, &AI_EVAL);
                println!(
                    "\n== turn {} active {} (eval for subject {})",
                    before.turn,
                    before.active,
                    to_fixed(eval, EVAL_DECIMALS)
                );
                for p in PLAYER_IDS {
                    println!("   {}", board(before, p));
                }
                let hand: Vec<String> =
                    before.players[before.active].hand.iter().map(|card| name_of(before, &card.def_id)).collect();
                println!("   hand({}): {}", before.active, hand.join(", "));
            }
            let who = if seat == subject { "AI " } else { "OPP" };
            println!("  {who} {}", describe(before, seat, action));
        })),
        ..MatchHooks::default()
    };
    let record = play_match(&config, &mut hooks);
    println!(
        "\nresult {} turns {}",
        serde_json::to_string(&record.result).unwrap_or_default(),
        record.turns
    );
    Ok(())
}
