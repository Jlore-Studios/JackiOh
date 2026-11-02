//! The helpers every server test file carried its own copy of (#1491, #2385): the fake store behind the
//! test app (`fake`, `fake_ref`, `fake_lock`), its tables (`table`, `fake_read`) and its injected faults
//! (`fail_on`, `stop_failing`) and its call log (`record_calls`), one store call in a transaction of
//! its own (`q!`, `store!`, `try_store!`), a value as JSON and back (`to_json`, `from`), a
//! client-minted id (`uuid`), an HMAC-SHA256 as hex (`hmac_sha256_hex`), three steps of tokio's
//! paused clock (`advance`, `advance_and_yield`, `sleep_settled`), a signed-in active account
//! (`Account`, `account`) and two of them made friends (`befriend`), the cards a view names
//! (`named_cards`) and TS's `toMatchObject` (`assert_match`; #2160). A copy that behaved differently keeps a name of its own;
//! `crates/tools/tests/acceptance.rs` (`SHARED_TEST_HELPERS`) refuses a new copy.

use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, MutexGuard};

use jackioh_server::app::App;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StoreError};

use crate::support::deps::{add_user, call};

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
    serde_json::to_value(value).expect("a serializable value")
}

/// A JSON answer as the type it holds, panicking with the error and the JSON when it is not one.
pub fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// One table of the fake store: rows as JSON.
pub async fn table(app: &App, pick: impl Fn(&FakeData) -> Value) -> Vec<Value> {
    let data = fake(app);
    let data = data.lock().await;
    match pick(&data) {
        Value::Array(rows) => rows,
        other => panic!("not a table: {other}"),
    }
}

/// One read of the fake's raw tables, as JSON: a table, a row or a count.
pub async fn fake_read(app: &App, read: impl Fn(&FakeData) -> Value) -> Value {
    let data = fake_ref(app).lock().await;
    read(&data)
}

/// Every store method named `method` fails with `message` until `stop_failing`.
pub async fn fail_on(app: &App, method: &'static str, message: &'static str) {
    fake(app).lock().await.on_call = Some(Arc::new(move |called: &str| {
        if called == method {
            Err(StoreError::Other(message.to_string()))
        } else {
            Ok(())
        }
    }));
}

/// Every store method answers again.
pub async fn stop_failing(app: &App) {
    fake(app).lock().await.on_call = None;
}

/// Every store method called from now on, by name and in order, until `stop_failing`.
pub async fn record_calls(app: &App) -> Arc<std::sync::Mutex<Vec<String>>> {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = Arc::clone(&calls);
    fake(app).lock().await.on_call = Some(Arc::new(move |called: &str| {
        seen.lock().expect("call log").push(called.to_string());
        Ok(())
    }));
    calls
}

/// HMAC-SHA256 as lower-case hex (RFC 2104 over `sha2`, so a test needs no MAC crate's API).
pub fn hmac_sha256_hex(key: &str, message: &str) -> String {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key.as_bytes());
        block[..digest.len()].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key.as_bytes());
    }
    let inner_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
    let outer_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message.as_bytes());
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(&inner[..]);
    outer
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

/// One signed-in account: its profile id, its username as shown (`name#tag`) and its token.
pub struct Account {
    pub id: String,
    pub name: String,
    pub token: String,
}

/// An active account, username `base#n`, its profile seeded on the fake store and its user signed in.
pub async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    let tag = i64::from(n);
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
        "username": base,
        "usernameTag": tag,
        "usernamePrompted": true,
    }));
    let token = add_user(app, &user_id, &format!("{user_id}@example.test"), true);
    Account {
        id,
        name: format!("{base}#{tag}"),
        token,
    }
}

/// `a` asks `b` to be friends by username (201) and `b` accepts (200).
pub async fn befriend(app: &Arc<App>, a: &Account, b: &Account) {
    let (status, _headers, body) = call(
        app,
        "POST",
        "/api/friends/requests",
        Some(&a.token),
        json!({ "username": b.name }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let request_id = body["requestId"].as_str().expect("a request id").to_string();
    let (status, _headers, body) = call(
        app,
        "POST",
        &format!("/api/friends/requests/{request_id}/accept"),
        Some(&b.token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

/// Every `(instanceId, defId)` a view JSON names, wherever it names one.
pub fn named_cards(view: &Value) -> Vec<(String, String)> {
    let mut named = Vec::new();
    let mut stack = vec![view];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(fields) => {
                if let (Some(Value::String(instance)), Some(Value::String(def))) =
                    (fields.get("instanceId"), fields.get("defId"))
                {
                    named.push((instance.clone(), def.clone()));
                }
                stack.extend(fields.values());
            }
            Value::Array(entries) => stack.extend(entries),
            _ => {}
        }
    }
    named
}

/// TS's `toMatchObject`: every key `expected` names holds the same value in `actual` (objects
/// partially, arrays element for element), and nothing else is compared.
pub fn assert_match(actual: &Value, expected: &Value) {
    fn matches(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches(got, value))
            }
            _ => actual == expected,
        }
    }
    assert!(matches(actual, expected), "expected {actual} to match {expected}");
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
