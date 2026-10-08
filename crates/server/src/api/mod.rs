//! The HTTP API (SURFACE §11.1): one module per `apps/server/src/api/<x>.ts`, parts 18 and 19.
//! Written once by part 1 (SURFACE §1).

pub mod auth;
pub mod catalog;
pub mod codes;
pub mod collection;
pub mod cors;
pub mod crypto;
pub mod decks;
pub mod e2e;
pub mod game_records;
pub mod http;
pub mod queue;
pub mod ranked;
pub mod rematch;
pub mod replays;
pub mod results;
pub mod retention;
pub mod series;
pub mod series_rules;
pub mod settings;
pub mod stats;
pub mod tutorial;
