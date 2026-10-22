//! The general integration-test binary for the server (SURFACE §1, §11.2): the API, the match actor,
//! the ladder bots, the background loops' supervisor and their shared support. The store suite lives
//! in its own binary, `server_store.rs`, so that CI's Postgres job does not link the rest.

mod actor;
mod api;
mod bots;
mod loops;
mod support;
