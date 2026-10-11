//! The match actor (SURFACE §11.1): one module per `apps/server/src/match/<x>.ts` (`match` is a
//! Rust keyword; `actor.ts` is `match_actor.rs`), part 19.
//! Written once by part 1 (SURFACE §1).

pub mod clock;
pub mod contracts;
pub mod engine;
pub mod match_actor;
pub mod protocol;
pub mod registry;
pub mod rooms;
pub mod telemetry;
pub mod ws_server;
