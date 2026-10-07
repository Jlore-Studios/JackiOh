//! `src/cli/season_start.rs` — R609's season-open admin script — driven against the in-memory
//! store, where `--dry-run`'s rollback is the observable half: the report must describe the reset
//! exactly while the store holds none of its writes.
//!
//! The port of `apps/server/test/db/season-start.test.ts`. TS's `createTestDeps()` is
//! `support::deps::test_app()` (a fake store and the test patch version); its store's
//! `seedProfile` is `FakeData::seed_profile`, reached through the `Db::Fake` handle's lock.

use jackioh_server::api::ranked::{SeasonDeps, rate_ranked_game};
use jackioh_server::cli::season_start::{SeasonStartOptions, parse_season_start_args, start_season};
use jackioh_server::config::SEASON_RESET_STRENGTH;
use jackioh_server::db::store::Db;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::support::deps::{TEST_CATALOG_VERSION, test_app};

/// One transaction around one store call, as TS's store gave every method called outside
/// `store.tx`: begin, the call, commit. Answers the call's value, or the first error's text.
macro_rules! once {
    ($db:expr, $sub:expr, |$tx:ident| $call:expr) => {
        async {
            let mut $tx = $db.begin($sub).await.map_err(|error| error.to_string())?;
            let value = $call.await.map_err(|error| error.to_string())?;
            $tx.commit().await.map_err(|error| error.to_string())?;
            Ok::<_, String>(value)
        }
        .await
    };
}

fn de<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn js<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a row serialises")
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// `toBeCloseTo(expected, 9)`: within half of 1e-9.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.5e-9,
        "{actual} is not close to {expected}"
    );
}

/// R609: `v<major>.<minor>` of a patch version (`ranked/season.rs`'s `season_id_of`, copied so the
/// expectation is computed here rather than by the code under test).
fn season_id_of(patch_version: &str) -> String {
    let rest = patch_version
        .strip_prefix('v')
        .unwrap_or_else(|| panic!("{patch_version:?} is not a patch version"));
    let mut parts = rest.splitn(3, ['.', '-']);
    let major: u64 = parts
        .next()
        .and_then(|part| part.parse().ok())
        .expect("a major version");
    let minor: u64 = parts
        .next()
        .map(|part| part.chars().take_while(char::is_ascii_digit).collect::<String>())
        .and_then(|digits| digits.parse().ok())
        .expect("a minor version");
    format!("v{major}.{minor}")
}

fn player(profile_id: &str) -> Value {
    json!({ "kind": "player", "profileId": profile_id })
}

/// The two rated profiles.
const A: &str = "profile-a";
const B: &str = "profile-b";

/// TS `deps.store.seedProfile(input)`: an active profile written straight into the fake's tables.
async fn seed_profile(db: &Db, input: Value) {
    let Db::Fake(data) = db else {
        panic!("the test app's store is the fake")
    };
    data.lock().await.seed_profile(input);
}

async fn profile_of(db: &Db, profile_id: &str) -> Value {
    let found =
        once!(db, Some(profile_id), |tx| tx.profiles_get_by_id(profile_id)).expect("profiles.getById");
    js(&found.expect("the profile exists"))
}

async fn season_ids(db: &Db) -> Vec<String> {
    let seasons = once!(db, None, |tx| tx.ranked_seasons()).expect("ranked.seasons");
    seasons
        .iter()
        .map(|season| js(season)["id"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// One rated game between the two profiles: what `ratedPlayers` (and so the reset) reads.
async fn rate(app: &std::sync::Arc<jackioh_server::app::App>, at: i64) {
    let input = json!({
        "id": format!("m-{at}"),
        "kind": "match",
        "catalogVersion": "test-1",
        "sides": [player(A), player(B)],
        "winnerSide": 0,
        "reason": "hero-death",
        "at": at,
    });
    let mut tx = app.db.begin(None).await.expect("store.tx");
    rate_ranked_game(&mut tx, app, &de(input))
        .await
        .expect("rateRankedGame");
    tx.commit().await.expect("commit");
}

/// Two rated players and their season — the state a mid-version bump finds.
async fn rated_pair(app: &std::sync::Arc<jackioh_server::app::App>) {
    seed_profile(
        &app.db,
        json!({ "id": A, "rating": 1200, "ratingDeviation": 100 }),
    )
    .await;
    seed_profile(&app.db, json!({ "id": B, "rating": 800, "ratingDeviation": 100 })).await;
    rate(app, 1).await;
}

fn deps(patch_version: &str) -> SeasonDeps {
    SeasonDeps {
        patch_version: patch_version.to_string(),
    }
}

/// The version of the next season's build: one minor past the compiled-in catalog version, which
/// is the season the test app rates in. TS rated its tests in `TEST_PATCH_VERSION` (`v0.1.1`) and
/// opened `v0.2.0`'s season over it; the Rust server rates in `jackioh_cards::catalog_version()`'s
/// season (SURFACE §11.3), so the season these tests open is the one after that.
fn next_patch() -> String {
    let current = season_id_of(TEST_CATALOG_VERSION);
    let (major, minor) = current
        .trim_start_matches('v')
        .split_once('.')
        .expect("v<major>.<minor>");
    let minor: u64 = minor.parse().expect("a minor version");
    format!("v{major}.{}.0", minor + 1)
}

/// `next_patch`'s season (TS's `"v0.2"`).
fn next_season() -> String {
    season_id_of(&next_patch())
}

mod season_start_args {
    use super::*;

    #[test]
    fn takes_only_dry_run() {
        assert!(!parse_season_start_args(&args(&[])).expect("no arguments").dry_run);
        assert!(
            parse_season_start_args(&args(&["--dry-run"]))
                .expect("--dry-run")
                .dry_run
        );
        assert!(
            parse_season_start_args(&args(&["--dry-run", "--dry-run"]))
                .expect("twice")
                .dry_run
        );
        let apply = parse_season_start_args(&args(&["--apply"])).expect_err("--apply is refused");
        assert!(apply.to_string().contains("--apply"), "{apply}");
        let version = parse_season_start_args(&args(&["v0.2"])).expect_err("a bare version is refused");
        assert!(version.to_string().contains("v0.2"), "{version}");
    }
}

mod r609_start_season {
    use super::*;

    #[tokio::test]
    async fn opens_the_builds_season_for_real_when_dry_run_is_absent() {
        let app = test_app().await;
        rated_pair(&app).await;

        let opened = js(&start_season(
            &app.db,
            &deps(&next_patch()),
            SeasonStartOptions { dry_run: false },
        )
        .await
        .expect("startSeason"));
        assert_eq!(opened["season"]["id"], next_season().as_str());
        assert_eq!(opened["opened"], true);
        assert_eq!(opened["reset"]["players"].as_i64(), Some(2));
        assert_eq!(
            season_ids(&app.db).await,
            vec![season_id_of(TEST_CATALOG_VERSION), next_season()]
        );

        // And it is idempotent: the season the server boot would open is already there.
        let again = js(&start_season(
            &app.db,
            &deps(&next_patch()),
            SeasonStartOptions { dry_run: false },
        )
        .await
        .expect("startSeason again"));
        assert_eq!(again["opened"], false);
        assert_eq!(again["reset"], Value::Null);
    }

    #[tokio::test]
    async fn dry_run_reports_the_reset_and_rolls_every_write_back() {
        let app = test_app().await;
        rated_pair(&app).await;
        let (a, b) = (profile_of(&app.db, A).await, profile_of(&app.db, B).await);
        let rating = |profile: &Value| profile["rating"].as_f64().expect("a rating");
        let mean = (rating(&a) + rating(&b)) / 2.0;

        let opened = js(&start_season(
            &app.db,
            &deps(&next_patch()),
            SeasonStartOptions { dry_run: true },
        )
        .await
        .expect("startSeason --dry-run"));

        // The report is the real reset's: two players, pulled toward their mean by the strength.
        assert_eq!(opened["season"]["id"], next_season().as_str());
        assert_eq!(opened["opened"], true);
        assert_eq!(opened["reset"]["players"].as_i64(), Some(2));
        assert_close(opened["reset"]["mean"].as_f64().expect("a mean"), mean);
        assert_close(
            opened["reset"]["highestAfter"]
                .as_f64()
                .expect("a highest rating"),
            mean + (rating(&a) - mean) * (1.0 - SEASON_RESET_STRENGTH),
        );

        // …and nothing it described survives: no season row, untouched ratings.
        assert_eq!(
            season_ids(&app.db).await,
            vec![season_id_of(TEST_CATALOG_VERSION)]
        );
        let (a_after, b_after) = (profile_of(&app.db, A).await, profile_of(&app.db, B).await);
        assert_eq!(a_after["rating"], a["rating"]);
        assert_eq!(a_after["ratingDeviation"], a["ratingDeviation"]);
        assert_eq!(b_after["rating"], b["rating"]);
        assert_eq!(b_after["ratingDeviation"], b["ratingDeviation"]);

        // A real run right after does exactly what the dry run reported.
        let applied = js(&start_season(
            &app.db,
            &deps(&next_patch()),
            SeasonStartOptions { dry_run: false },
        )
        .await
        .expect("startSeason"));
        assert_eq!(applied["opened"], true);
        assert_eq!(applied["reset"]["players"].as_i64(), Some(2));
        assert_close(
            rating(&profile_of(&app.db, A).await),
            mean + (rating(&a) - mean) * (1.0 - SEASON_RESET_STRENGTH),
        );
    }

    #[tokio::test]
    async fn dry_run_on_an_already_open_season_reports_it_as_unopened_and_writes_nothing() {
        let app = test_app().await;
        rated_pair(&app).await;
        let opened = js(&start_season(
            &app.db,
            &deps(TEST_CATALOG_VERSION),
            SeasonStartOptions { dry_run: true },
        )
        .await
        .expect("startSeason --dry-run"));
        assert_eq!(
            opened["season"]["id"],
            season_id_of(TEST_CATALOG_VERSION).as_str()
        );
        assert_eq!(opened["opened"], false);
        assert_eq!(opened["reset"], Value::Null);
        assert_eq!(
            season_ids(&app.db).await,
            vec![season_id_of(TEST_CATALOG_VERSION)]
        );
    }
}
