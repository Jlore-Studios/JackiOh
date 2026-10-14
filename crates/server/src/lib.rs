//! The JackiOh server (SURFACE §11): one process that runs the HTTP API (`api`) and one match actor
//! per live match (`actor`), over one store (`db`). It serves the same routes, statuses, headers,
//! bodies, frames, close codes and rows as `apps/server`, with SURFACE §11.3's deltas only.
//!
//! Module tree (SURFACE §11.1), written once by part 1:
//!
//! - `env`: every environment variable, its default and its refusal (← `src/env.ts`, part 18).
//! - `config`: every server constant (← `src/config.ts`, part 18).
//! - `app`: `App`, the route table, boot and the background loops (← `src/index.ts`, part 18).
//! - `auth`: the `Auth` enum, Supabase and the E2E fixtures (part 18).
//! - `api`: the routes (parts 18 and 19). `actor`: the match lifecycle (part 19).
//! - `bots`: the ladder bots: their personas, seat, fallback and pool (SPEC §9.12, R1453, R1454,
//!   R1451, R1452; #638).
//! - `ranked`: the ladder's pure rules (part 18). `db`: the store and migrations (part 20).
//! - `cli`: the command-line tools `main.rs` dispatches to (part 20).
//! - `loops`: the background loops' supervisor, which logs a loop that panics and starts it again
//!   (#842).
//! - `username`: a username's checks, stored form and key (SPEC §9.4, R1432–R1434; #579).

pub mod actor;
pub mod api;
pub mod app;
pub mod auth;
pub mod bots;
pub mod cli;
pub mod config;
pub mod db;
// ---- Duos co-op (#946) ----
// Fullsend SPEC S6: the Duos co-op rooms, their socket and their four-seat match actor.
pub mod duos;
// ---- end Duos co-op (#946) ----
pub mod env;
pub mod loops;
pub mod ranked;
// ---- Social: friends, presence, challenges, chat (#937, #938, #941) ---- SPEC S7 (#936).
pub mod social;
pub mod username;
