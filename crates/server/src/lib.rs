//! The JackiOh server (SURFACE §11): one process that runs the HTTP API (`api`) and one match actor
//! per live match (`actor`), over one store (`db`). Its routes, statuses, headers, bodies, frames,
//! close codes and rows are SURFACE §11.3's.
//!
//! Module tree (SURFACE §11.1):
//!
//! - `env`: every environment variable, its default and its refusal.
//! - `config`: every server constant.
//! - `app`: `App`, the route table, boot and the background loops.
//! - `auth`: the `Auth` enum, Supabase and the E2E fixtures.
//! - `api`: the routes. `actor`: the match lifecycle.
//! - `bots`: the ladder bots' personas, seat, fallback, pool (§9.12, R1451, R1452, R1453, R1454; #638).
//! - `ranked`: the ladder's pure rules. `db`: the store and migrations.
//! - `seats`: the seat ledger, the one game each profile holds (R1523000; #1523).
//! - `cli`: the command-line tools `main.rs` dispatches to.
//! - `loops`: the loops' supervisor, logging and restarting one that panics (#842) or returns (#1093).
//! - `username`: a username's checks, stored form and key (SPEC §9.4, R1432–R1434; #579).
//! - `bug_reports`: a match its own log does not reproduce, filed as a GitHub issue for the night bot.
//! - `cosmetics`: the cosmetics registry, `cosmetics.json`, and its check (#1295, R1295000).
//! - `loadout`: the cosmetics each seat of a match wears, frozen as it starts, and when a card's own
//!   picks may be sent (#1834, R1834002, R1834003).
//! - `economy`: the economy's holdings and ledgers (SPEC §14.11, #1299), its math, level curve and pity
//!   (#1298), and the card packs (#1301).

pub mod actor;
pub mod api;
pub mod app;
pub mod auth;
pub mod bots;
pub mod bug_reports;
pub mod cli;
pub mod config;
pub mod cosmetics;
pub mod db;
// Economy (SPEC §14, #1299)
pub mod economy;
// Duos co-op (#946)
// Fullsend SPEC S6: the Duos co-op rooms, their socket and their four-seat match actor.
pub mod duos;
// end Duos co-op (#946)
pub mod env;
pub mod loadout;
pub mod loops;
pub mod ranked;
pub mod seats;
// Social: friends, presence, challenges, chat (#937, #938, #941); SPEC S7 (#936).
pub mod social;
pub mod username;
