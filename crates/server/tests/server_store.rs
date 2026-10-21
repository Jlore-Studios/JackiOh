//! The store's integration-test binary (SURFACE §1, §11.2), split from `server.rs` so that CI's
//! `db (store contract)` job, which builds and runs only this binary, stays under its five-minute
//! target. Locally `cargo test -p jackioh-server --test server_store`: every case runs against the
//! in-memory fake, and against Postgres too when `DATABASE_URL` is set, one at a time, as
//! `tests/db/run.sh` sets it.
//!
//! Of `tests/support/` it compiles `test_app` alone, the one module `store/redeem_race.rs` and
//! `store/season_start.rs` use, as `crate::support::test_app`: the request helpers and the recording
//! log beside it, in `deps`, would be dead code here.

mod store;

mod support {
    pub mod test_app;
}
