//! The Postgres round trips a request costs (#3208), counted by a proxy between the pool and the database:
//! every ReadyForQuery the server sends is one round trip a client waited for (a simple query, a Sync, a
//! ping). The pool is `pg::pool_options`', the one `Db::Pg` runs on, at size 1 so every count is on one
//! connection, with each statement prepared by a first, uncounted pass. Then the pool's checkout ping and
//! heartbeat, and that a ban applies to the next request, as no profile is kept between requests. Without
//! `DATABASE_URL` every case returns at once, so `cargo test` stays hermetic.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::body::Body;
use jackioh_server::auth::Auth;
use jackioh_server::config::POOL_PING_AFTER_IDLE_MS;
use jackioh_server::db::pg;
use jackioh_server::db::store::Db;
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::{PgConnectOptions, PgPool, PgSslMode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tower::ServiceExt;

use crate::support::test_app::{TestAppOptions, test_app_with};

/// `GET /api/settings`, an authenticated read of one row: the caller lookup's transaction, then the
/// handler's. Before #3208, 14: on each, the checkout ping, `BEGIN`, `begin`'s session statement,
/// `run_as`'s again, the select and `COMMIT` (12 on the request's path), then sqlx's release ping, which
/// runs on a task of its own after the connection is dropped. After, 8: `BEGIN` with the session statement,
/// the select and `COMMIT` on each (6 on the path), and the two release pings.
const ONE_ROW_READ_ROUND_TRIPS: usize = 8;

/// `begin`, `profiles_get_by_id`, `player_settings_get` and `commit` for one profile. Before #3208, 9: the
/// checkout ping, `BEGIN`, the session statement three times, the two selects, `COMMIT` and the release
/// ping. After, 5: `BEGIN` with the session statement, the two selects, `COMMIT` and the release ping.
const TWO_METHOD_ROUND_TRIPS: usize = 5;

/// The checkout ping's threshold in `pings_a_connection_idle_past_the_threshold_and_not_one_checked_out_at_once`,
/// short so the test waits little; production's is `POOL_PING_AFTER_IDLE_MS`.
const TEST_PING_AFTER_IDLE_MS: u64 = 200;

/// How often `settle` looks at the pool, and how many looks it takes before it gives up.
const SETTLE_POLL_MS: u64 = 10;
const SETTLE_TRIES: usize = 200;

/// The wire's ReadyForQuery message type, and its header: a type byte, then a big-endian length that
/// counts itself.
const READY_FOR_QUERY: u8 = b'Z';
const HEADER_BYTES: usize = 5;
const LENGTH_BYTES: usize = 4;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|url| !url.is_empty())
}

/// A TCP proxy in front of the database that counts the connections it is asked for and the
/// ReadyForQuery messages the server sends back.
struct Proxy {
    port: u16,
    opened: Arc<AtomicUsize>,
    ready: Arc<AtomicUsize>,
    accepting: JoinHandle<()>,
}

impl Proxy {
    async fn start(url: &str) -> Proxy {
        let upstream = PgConnectOptions::from_str(url).expect("DATABASE_URL parses");
        let target = format!("{}:{}", upstream.get_host(), upstream.get_port());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("the proxy's port");
        let port = listener.local_addr().expect("the proxy's address").port();
        let opened = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(AtomicUsize::new(0));
        let (counted, readied) = (opened.clone(), ready.clone());
        let accepting = tokio::spawn(async move {
            while let Ok((client, _)) = listener.accept().await {
                counted.fetch_add(1, Ordering::SeqCst);
                let (target, readied) = (target.clone(), readied.clone());
                tokio::spawn(async move {
                    let Ok(server) = TcpStream::connect(&target).await else {
                        return;
                    };
                    let (mut client_read, mut client_write) = client.into_split();
                    let (mut server_read, mut server_write) = server.into_split();
                    let upstream = tokio::spawn(async move {
                        let _ = tokio::io::copy(&mut client_read, &mut server_write).await;
                    });
                    let _ = forward_counting(&mut server_read, &mut client_write, &readied).await;
                    upstream.abort();
                });
            }
        });
        Proxy {
            port,
            opened,
            ready,
            accepting,
        }
    }

    /// The database's options, through the proxy. TLS off, so the server's stream is framed from its
    /// first byte.
    fn options(&self, url: &str) -> PgConnectOptions {
        PgConnectOptions::from_str(url)
            .expect("DATABASE_URL parses")
            .host("127.0.0.1")
            .port(self.port)
            .ssl_mode(PgSslMode::Disable)
    }

    fn ready(&self) -> usize {
        self.ready.load(Ordering::SeqCst)
    }

    fn opened(&self) -> usize {
        self.opened.load(Ordering::SeqCst)
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.accepting.abort();
    }
}

/// Server to client, one message at a time, a ReadyForQuery counted before it is passed on.
async fn forward_counting(
    from: &mut OwnedReadHalf,
    to: &mut OwnedWriteHalf,
    ready: &AtomicUsize,
) -> std::io::Result<()> {
    let mut header = [0u8; HEADER_BYTES];
    loop {
        from.read_exact(&mut header).await?;
        let mut length = [0u8; LENGTH_BYTES];
        length.copy_from_slice(&header[1..]);
        let length = usize::try_from(u32::from_be_bytes(length)).unwrap_or(0);
        let mut body = vec![0u8; length.saturating_sub(LENGTH_BYTES)];
        from.read_exact(&mut body).await?;
        if header[0] == READY_FOR_QUERY {
            ready.fetch_add(1, Ordering::SeqCst);
        }
        to.write_all(&header).await?;
        to.write_all(&body).await?;
    }
}

/// Waits until every connection the pool holds is back in it: a dropped connection is returned by a
/// task of its own, after sqlx's release ping.
async fn settle(pool: &PgPool) {
    for _ in 0..SETTLE_TRIES {
        if pool.num_idle() == pool.size() as usize {
            return;
        }
        tokio::time::sleep(Duration::from_millis(SETTLE_POLL_MS)).await;
    }
    panic!(
        "the pool never settled: {} idle of {}",
        pool.num_idle(),
        pool.size()
    );
}

/// A raw superuser pool over a clean slate, with the fixture catalog the profile triggers read.
async fn admin(url: &str) -> PgPool {
    let admin = PgPool::connect(url).await.expect("the admin connection");
    sqlx::raw_sql("truncate public.player_settings, public.profiles, auth.users restart identity cascade")
        .execute(&admin)
        .await
        .expect("truncate");
    super::contract::seed_cards(&admin).await;
    admin
}

/// Production's checkout-ping threshold.
fn ping_after_idle() -> Duration {
    Duration::from_millis(u64::try_from(POOL_PING_AFTER_IDLE_MS).expect("a positive threshold"))
}

/// The store's pool, through the proxy, as `pg::create_postgres_store` builds it, at size 1.
fn store_pool(proxy: &Proxy, url: &str) -> PgPool {
    pg::pool_options(1, ping_after_idle()).connect_lazy_with(proxy.options(url))
}

/// The test app over a Postgres pool: the fixture auth, no fixtures written.
async fn app_over(pool: &PgPool) -> Arc<jackioh_server::app::App> {
    test_app_with(TestAppOptions {
        db: Some(Db::Pg(pool.clone())),
        ..TestAppOptions::default()
    })
    .await
}

/// An active account the app's fixture auth signs in: its auth user, the profile 0001's trigger made
/// for it, and its bearer token.
async fn active_user(admin: &PgPool, app: &jackioh_server::app::App) -> (String, String) {
    let email = format!("{}@example.test", uuid::Uuid::new_v4());
    let row =
        sqlx::query("insert into auth.users (email, email_confirmed_at) values ($1, now()) returning id")
            .bind(&email)
            .fetch_one(admin)
            .await
            .expect("insert the auth user");
    let id = row
        .try_get::<uuid::Uuid, _>("id")
        .expect("the auth user's id")
        .to_string();
    sqlx::query("update public.profiles set status = 'active' where id = $1::uuid")
        .bind(&id)
        .execute(admin)
        .await
        .expect("activate the profile");
    let Auth::E2e(auth) = &app.auth else {
        panic!("the test app signs in with the fixture auth");
    };
    let token = auth.add_user(&id, &email, true);
    (id, token)
}

/// One `GET` through the real router: its status and its JSON body.
async fn get(app: &Arc<jackioh_server::app::App>, path: &str, token: &str) -> (u16, Value) {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("https://server.test{path}"))
        .header("x-forwarded-for", "203.0.113.7")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("a test request");
    let response = jackioh_server::app::router(app.clone())
        .oneshot(request)
        .await
        .unwrap_or_else(|error: std::convert::Infallible| match error {});
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("a response body");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

#[tokio::test]
async fn pins_a_one_row_authenticated_read() {
    let Some(url) = database_url() else { return };
    let admin = admin(&url).await;
    let proxy = Proxy::start(&url).await;
    let pool = store_pool(&proxy, &url);
    let app = app_over(&pool).await;
    let (_, token) = active_user(&admin, &app).await;

    // The first request opens the connection and prepares the statements; the second is the one counted.
    assert_eq!(get(&app, "/api/settings", &token).await.0, 200);
    settle(&pool).await;
    let (ready, opened) = (proxy.ready(), proxy.opened());
    assert_eq!(get(&app, "/api/settings", &token).await.0, 200);
    settle(&pool).await;
    assert_eq!(proxy.ready() - ready, ONE_ROW_READ_ROUND_TRIPS);
    assert_eq!(proxy.opened() - opened, 0, "a warm connection serves the request");
    pool.close().await;
    admin.close().await;
}

#[tokio::test]
async fn pins_a_two_method_transaction() {
    let Some(url) = database_url() else { return };
    let admin = admin(&url).await;
    let proxy = Proxy::start(&url).await;
    let pool = store_pool(&proxy, &url);
    let app = app_over(&pool).await;
    let (profile_id, _) = active_user(&admin, &app).await;

    // The first pass opens the connection and prepares the statements; the second is the one counted.
    let mut counted = Vec::new();
    for _ in 0..2 {
        settle(&pool).await;
        let ready = proxy.ready();
        let mut tx = app.db.begin(Some(&profile_id)).await.expect("begin");
        tx.profiles_get_by_id(&profile_id)
            .await
            .expect("profiles_get_by_id");
        tx.player_settings_get(&profile_id)
            .await
            .expect("player_settings_get");
        tx.commit().await.expect("commit");
        settle(&pool).await;
        counted.push(proxy.ready() - ready);
    }
    assert_eq!(counted[1], TWO_METHOD_ROUND_TRIPS);
    assert_eq!(proxy.opened(), 1);
    pool.close().await;
    admin.close().await;
}

#[tokio::test]
async fn pings_a_connection_idle_past_the_threshold_and_not_one_checked_out_at_once() {
    let Some(url) = database_url() else { return };
    let proxy = Proxy::start(&url).await;
    let threshold = Duration::from_millis(TEST_PING_AFTER_IDLE_MS);
    let pool = pg::pool_options(1, threshold).connect_lazy_with(proxy.options(&url));
    drop(pool.acquire().await.expect("open the connection"));
    settle(&pool).await;

    let ready = proxy.ready();
    let conn = pool.acquire().await.expect("checked out at once");
    assert_eq!(
        proxy.ready() - ready,
        0,
        "a connection idle under the threshold is not pinged"
    );
    drop(conn);
    settle(&pool).await;

    tokio::time::sleep(threshold * 2).await;
    let ready = proxy.ready();
    let conn = pool.acquire().await.expect("checked out after idling");
    assert_eq!(
        proxy.ready() - ready,
        1,
        "a connection idle past the threshold is pinged"
    );
    drop(conn);
    settle(&pool).await;
    assert_eq!(proxy.opened(), 1, "the same connection throughout");
    pool.close().await;
}

#[tokio::test]
async fn the_heartbeat_pings_each_warm_connection_and_keeps_it() {
    let Some(url) = database_url() else { return };
    let proxy = Proxy::start(&url).await;
    let pool = store_pool(&proxy, &url);
    drop(pool.acquire().await.expect("open the connection"));
    settle(&pool).await;

    let (ready, opened) = (proxy.ready(), proxy.opened());
    assert_eq!(pg::heartbeat_tick(&pool).await, 1);
    settle(&pool).await;
    // Its ping, then sqlx's release ping as the connection goes back.
    assert_eq!(proxy.ready() - ready, 2);
    assert_eq!(proxy.opened() - opened, 0);
    assert_eq!(pool.size(), 1);
    pool.close().await;
}

#[tokio::test]
async fn the_heartbeat_leaves_a_pool_above_its_warm_size_alone() {
    let Some(url) = database_url() else { return };
    let pool = pg::pool_options(3, ping_after_idle())
        .connect_lazy_with(PgConnectOptions::from_str(&url).expect("DATABASE_URL parses"));
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(pool.acquire().await.expect("a connection"));
    }
    drop(held);
    settle(&pool).await;
    assert_eq!(pg::heartbeat_tick(&pool).await, 0);
    assert_eq!(pool.size(), 3);
    pool.close().await;
}

#[tokio::test]
async fn a_ban_applies_to_the_next_request() {
    let Some(url) = database_url() else { return };
    let admin = admin(&url).await;
    let pool = pg::pool_options(1, ping_after_idle())
        .connect_lazy_with(PgConnectOptions::from_str(&url).expect("DATABASE_URL parses"));
    let app = app_over(&pool).await;
    let (id, token) = active_user(&admin, &app).await;
    assert_eq!(get(&app, "/api/settings", &token).await.0, 200);

    sqlx::query("update public.profiles set status = 'banned' where id = $1::uuid")
        .bind(&id)
        .execute(&admin)
        .await
        .expect("ban the profile");
    let (status, body) = get(&app, "/api/settings", &token).await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "account_banned");
    pool.close().await;
    admin.close().await;
}

/// A login role that is no member of `service_role`, so `pg::begin`'s session statement fails for it, and
/// the statement that makes it once per cluster (the shards' databases share their roles).
const OUTSIDER_ROLE: &str = "store_probe_outsider";
const OUTSIDER_PASSWORD: &str = "outsider";
const OUTSIDER_SETUP: &str = "do $$ begin
  if not exists (select from pg_roles where rolname = 'store_probe_outsider') then
    create role store_probe_outsider login password 'outsider';
  end if;
end $$";

/// `pg::begin` sends its session statement and `BEGIN` as one message; a failed session statement must
/// leave the connection outside any transaction, as a failed `BEGIN` would, since sqlx then neither rolls
/// it back nor closes it, and hands it to the next request.
#[tokio::test]
async fn a_begin_whose_session_statement_fails_leaves_its_connection_out_of_any_transaction() {
    let Some(url) = database_url() else { return };
    let superuser = PgPool::connect(&url).await.expect("the superuser connection");
    sqlx::raw_sql(OUTSIDER_SETUP)
        .execute(&superuser)
        .await
        .expect("the outsider role");
    superuser.close().await;
    let options = PgConnectOptions::from_str(&url)
        .expect("DATABASE_URL parses")
        .username(OUTSIDER_ROLE)
        .password(OUTSIDER_PASSWORD);
    let pool = pg::pool_options(1, ping_after_idle()).connect_lazy_with(options);
    let backend = |row: &sqlx::postgres::PgRow| row.try_get::<i32, _>("pid").expect("pid");
    let before = sqlx::query("select pg_backend_pid() as pid")
        .fetch_one(&pool)
        .await
        .expect("the outsider connects");
    let before = backend(&before);
    settle(&pool).await;

    let subject = uuid::Uuid::new_v4().to_string();
    let Err(error) = Db::Pg(pool.clone()).begin(Some(&subject)).await else {
        panic!("a role that is no member of service_role cannot switch to it");
    };
    assert!(error.to_string().contains("permission denied"), "{error}");
    settle(&pool).await;

    // The same connection answers, and outside any transaction: a local setting made by one statement is
    // gone by the next, which it would not be inside a block left open.
    sqlx::query("select set_config('jackioh.probe_left_open', 'open', true)")
        .execute(&pool)
        .await
        .expect("the connection is not left in a failed transaction");
    let after = sqlx::query(
        "select pg_backend_pid() as pid, \
         coalesce(current_setting('jackioh.probe_left_open', true), '') as left_open",
    )
    .fetch_one(&pool)
    .await
    .expect("the connection still answers");
    assert_eq!(backend(&after), before, "the same connection");
    assert_eq!(
        after.try_get::<String, _>("left_open").expect("left_open"),
        "",
        "no transaction was left open on it"
    );
    pool.close().await;
}
