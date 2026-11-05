//! The cosmetics registry (#1295, #1297, SPEC §11 R1295000) is `jackioh_shared::cosmetics` since #3104:
//! `crates/shared/cosmetics.json` and its check, the offers, the ownership rules and the packs' droppable
//! lists, one copy for the server, the browser and every host that is not Rust. The server names them here
//! as it always has; `crates/shared/src/cosmetics.rs` holds the rulings' text and their tests, and
//! `tests/export_cosmetics.rs` writes the registry into `apps/web/src/wire/generated/cosmetics.ts`.

pub use jackioh_shared::cosmetics::*;
