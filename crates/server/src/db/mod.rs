//! Persistence (SURFACE §11.1, §11.2): the `Db`/`Tx` enums (`store`), Postgres (`pg`), the fake
//! (`fake`) and the migration runner (`migrate`), laid out by SURFACE §1. `store`'s types are
//! re-exported so `db::Db`, `db::Profile` and the rest resolve as SURFACE §11.2 writes them.

pub mod fake;
pub mod migrate;
pub mod pg;
pub mod store;

pub use store::*;
