//! The helpers every server test file carried its own copy of (#1491): the fake store behind the test
//! app (`fake`, `fake_ref`, `fake_lock`), one store call in a transaction of its own (`q!`, `store!`,
//! `try_store!`), a value as JSON (`to_json`), a client-minted id (`uuid`) and three steps of tokio's
//! paused clock (`advance`, `advance_and_yield`, `sleep_settled`). A copy that behaved differently
//! keeps a name of its own.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};

use jackioh_server::app::App;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

/// How many times `sleep_settled` yields after its sleep: as many as the actor tests' own `settle`.
const SLEEP_SETTLE_YIELDS: usize = 8;

/// The fake store behind the test app, as a handle of its own.
pub fn fake(app: &App) -> Arc<Mutex<FakeData>> {
    Arc::clone(fake_ref(app))
}

/// The fake store behind the test app, borrowed from it, so a lock on it can be held in a `let`.
pub fn fake_ref(app: &App) -> &Arc<Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data,
        Db::Pg(_) => panic!("the test app runs on the fake store"),
    }
}

/// The fake store behind the test app, locked.
pub async fn fake_lock(app: &App) -> MutexGuard<'_, FakeData> {
    fake_ref(app).lock().await
}

/// A value as its JSON.
pub fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a serialisable value")
}

/// A client-minted id: a UUID whose last twelve digits are `n`, whichever integer type `n` is.
pub fn uuid<T>(n: T) -> String
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    let n = u64::try_from(n).expect("an id's number is not negative");
    format!("00000000-0000-4000-8000-{n:012}")
}

/// Moves tokio's paused clock `ms` milliseconds forward, whichever integer type `ms` is.
pub async fn advance<T>(ms: T)
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    let ms = u64::try_from(ms).expect("time moves forward");
    tokio::time::advance(Duration::from_millis(ms)).await;
}

/// `advance`, then one yield, so the task a due timer woke gets to run.
pub async fn advance_and_yield(ms: i64) {
    advance(ms).await;
    tokio::task::yield_now().await;
}

/// Sleeps `ms` milliseconds on tokio's paused clock, so every timer due by then fires in deadline
/// order, then yields `SLEEP_SETTLE_YIELDS` times so what they woke runs. A negative `ms` sleeps 0.
pub async fn sleep_settled(ms: i64) {
    tokio::time::sleep(Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0))).await;
    for _ in 0..SLEEP_SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// One store call in its own transaction.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}
pub(crate) use q;

/// One store call in its own transaction: `store!(app, t => expr)` answers `expr` as it stands;
/// `store!(app, |t| call)` awaits `call` and insists on `Ok`; `store!(db, method(args…))` is `q!` on a
/// bare `Db` (the e2e tests' fixture store has no `App`).
macro_rules! store {
    ($app:expr, $t:ident => $call:expr) => {{
        let mut $t = $app
            .db
            .begin(None)
            .await
            .expect("the fake store opens a transaction");
        let out = $call;
        $t.commit().await.expect("the fake store commits");
        out
    }};
    ($app:expr, |$t:ident| $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("a store transaction begins");
        let out = $call
            .await
            .expect(concat!("the store answers ", stringify!($call)));
        $t.commit().await.expect("the store transaction commits");
        out
    }};
    ($db:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await.unwrap_or_else(|error| panic!("{}: {error:?}", stringify!($method)));
        t.commit().await.expect("commit");
        out
    }};
}
pub(crate) use store;

/// `store!(db, method(args…))`, answering its `Result` instead of insisting on `Ok`.
macro_rules! try_store {
    ($db:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await;
        if out.is_ok() {
            t.commit().await.expect("commit");
        }
        out
    }};
}
pub(crate) use try_store;
