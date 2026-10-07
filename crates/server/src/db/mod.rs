//! Persistence (SURFACE §11.1, §11.2): the `Db`/`Tx` enums (`store`), Postgres (`pg`), the fake
//! (`fake`) and the migration runner (`migrate`), part 20.
//! Written once by part 1 (SURFACE §1). `store`'s types are re-exported so `db::Db`, `db::Profile`
//! and the rest resolve as SURFACE §11.2 writes them (part 31).

pub mod fake;
pub mod migrate;
pub mod pg;
pub mod store;

pub use store::*;
