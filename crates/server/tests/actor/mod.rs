//! The server's ported tests: `apps/server/test/match/*.test.ts` (`match/` is `actor/`), part 19.
//! Every WebSocket message keeps TypeScript's shape, and a match plays end to end over the socket
//! (docs/v0.3.0/README.md V14, with e2e specs 05 and 06).
//! Written once by part 1 (SURFACE §1).

pub mod aim;
pub mod clock;
pub mod dealt_deck;
pub mod engine_real;
pub mod glitch;
pub mod heartbeat;
pub mod last_boards;
pub mod match_actor;
pub mod recovery;
pub mod rooms;
pub mod series_recovery;
pub mod telemetry;
pub mod ws_server;
