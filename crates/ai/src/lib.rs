//! `jackioh_ai`: the practice opponent (SPEC §9.9, docs/polish/3-ai.md; SURFACE §9). Pure and seeded
//! like the engine (CLAUDE.md rule 4, SURFACE §3): every exported name below is unique across the
//! crate, because this root re-exports every module whole, as `packages/ai/src/index.ts` did.
//!
//! The R645 emote personas (`personas.ts`) are presentation and live in the web client
//! (`apps/web/src/practice/personas.ts`, part 21), so they are not a module here. Written once by
//! part 1 (SURFACE §1); the modules are part 17's.

pub mod baselines;
pub mod candidates;
pub mod config;
pub mod decide;
pub mod deck;
pub mod determinize;
pub mod dev_run;
pub mod evaluate;
pub mod gate;
pub mod lethal;
pub mod match_;
pub mod mulligan;
pub mod observe;
pub mod reply;
pub mod search;
pub mod shadow_ban;
pub mod simulate;
pub mod sweep;
pub mod types;

pub use types::*;
pub use config::*;
pub use observe::*;
pub use determinize::*;
pub use evaluate::*;
pub use candidates::*;
pub use simulate::*;
pub use lethal::*;
pub use search::*;
pub use reply::*;
pub use mulligan::*;
pub use decide::*;
pub use deck::*;
pub use shadow_ban::*;
pub use baselines::*;
pub use match_::*;
pub use gate::*;
pub use sweep::*;
pub use dev_run::*;
