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
//! - `ranked`: the ladder's pure rules (part 18). `db`: the store and migrations (part 20).
//! - `cli`: the command-line tools `main.rs` dispatches to (part 20).

pub mod actor;
pub mod api;
pub mod app;
pub mod auth;
pub mod cli;
pub mod config;
pub mod db;
pub mod env;
pub mod ranked;
