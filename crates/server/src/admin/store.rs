//! The dev dashboard's store (SPEC §14.10, §14.13; migration 0080; issue #3074): the audit of every
//! dev and admin action (R3074000), the hand-set ratings and their ledger (R3074001), and the news
//! posts (R3074003), over Postgres (`pg` below) or the in-memory fake (`fake` below, whose tables are
//! `FakeTables::admin`).
//!
//! Same shape as `economy::store`: each store method is a `Tx` method `admin_<table>_<verb>`
//! dispatching to this file's `pg::<name>` or `fake::<name>`, in the caller's transaction. What the
//! schema's checks refuse is refused here first, as `StoreError::Other("admin: …")`, so the fake
//! refuses it too. Timestamps are epoch milliseconds.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::store::{StoreError, StoreResult, Tx};
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::Ladder;

/// The reason every `rating_grants` row has (§14.11): a dev account's own tool.
pub const RATING_GRANT_REASON: &str = "dev";

/// The prefix every post's id starts with, which no `news.json` entry may (`economy::news::parse`),
/// so a claim never mistakes one for the other (migration 0080's `news_posts_id_check`).
pub const NEWS_POST_ID_PREFIX: &str = "post-";

/// R3074000: one row of `public.admin_actions` as it is written: who (a profile id; `None` once
/// the account is deleted), which route (`<METHOD> <path>`, the path as the request named it), the
/// request, the values before and after, and when.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewAdminAction {
    pub profile_id: String,
    pub route: String,
    pub request: Value,
    pub before: Value,
    pub after: Value,
    pub at: i64,
}

/// R3074000: one row of `public.admin_actions` as it is read back, with its id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminAction {
    pub id: i64,
    pub profile_id: Option<String>,
    pub route: String,
    pub request: Value,
    pub before: Value,
    pub after: Value,
    pub at: i64,
}

/// R3074001: one row of `public.hand_set_ratings`: the account holds a hand-set rating on
/// `ladder`, and `earned` is the rating its games had earned when it was first set.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HandSetRating {
    pub profile_id: String,
    pub ladder: Ladder,
    pub earned: Glicko,
    pub set_at: i64,
}

/// R3074001: what a `rating_grants` row records: a rating set by hand, or a reset to the earned one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RatingChange {
    Set,
    Reset,
}

impl RatingChange {
    fn as_str(self) -> &'static str {
        match self {
            RatingChange::Set => "set",
            RatingChange::Reset => "reset",
        }
    }

    fn from_literal(text: &str) -> Option<RatingChange> {
        [RatingChange::Set, RatingChange::Reset]
            .into_iter()
            .find(|change| change.as_str() == text)
    }
}

/// R3074001, §14.11: one row of `public.rating_grants`, the rating ledger: the whole Glicko-2
/// triple before and after, its reason [`RATING_GRANT_REASON`] and the `dev` mark.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RatingGrant {
    pub profile_id: String,
    pub ladder: Ladder,
    pub change: RatingChange,
    pub before: Glicko,
    pub after: Glicko,
    pub dev: bool,
    pub at: i64,
}

/// R3074003: one row of `public.news_posts`: a post a dev wrote from the dashboard, plain text,
/// paying no reward. `posted_by` is `None` once its author's account is deleted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewsPost {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub posted_at: i64,
    pub posted_by: Option<String>,
    pub taken_down_at: Option<i64>,
}

/// The fake's tables (`FakeTables::admin`), each in the order it was written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdminTables {
    pub actions: Vec<AdminAction>,
    pub hand_set: Vec<HandSetRating>,
    pub rating_grants: Vec<RatingGrant>,
    pub posts: Vec<NewsPost>,
}

// What both stores refuse alike: migration 0080's checks

fn refused(why: String) -> StoreError {
    StoreError::Other(format!("admin: {why}"))
}

/// `admin_actions_route_check`: a dev or admin route, named with its method.
fn check_route(route: &str) -> Result<(), StoreError> {
    let (method, path) = route.split_once(' ').unwrap_or(("", ""));
    let method_ok = ["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&method);
    let path_ok = path.starts_with("/api/dev/") || path.starts_with("/api/admin/");
    if method_ok && path_ok {
        Ok(())
    } else {
        Err(refused(format!("{route:?} is no dev or admin route")))
    }
}

/// `rating_grants_dev_check`: a dev reason is always marked `dev`.
fn check_dev(dev: bool) -> Result<(), StoreError> {
    if dev {
        Ok(())
    } else {
        Err(refused(
            "a hand-set rating needs the dev mark (§14.10)".to_string(),
        ))
    }
}

/// `news_posts_id_check` and `news_posts_text_check`.
fn check_post(post: &NewsPost) -> Result<(), StoreError> {
    let slug = post.id.strip_prefix(NEWS_POST_ID_PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && rest.split('-').all(|part| {
                !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    });
    if !slug {
        return Err(refused(format!(
            "a post's id is {NEWS_POST_ID_PREFIX}<slug>, not {:?}",
            post.id
        )));
    }
    if post.title.trim().is_empty() || post.summary.trim().is_empty() {
        return Err(refused("a post has a title and a summary".to_string()));
    }
    Ok(())
}

// The store methods

macro_rules! admin_dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// R3074000: appends one audit row; answers its id.
    pub async fn admin_actions_append(&mut self, action: &NewAdminAction) -> StoreResult<i64> {
        check_route(&action.route)?;
        admin_dispatch!(self, actions_append(action))
    }

    /// R3074000: the newest `limit` audit rows, newest first (of two at once, the later written).
    pub async fn admin_actions_recent(&mut self, limit: i64) -> StoreResult<Vec<AdminAction>> {
        admin_dispatch!(self, actions_recent(limit))
    }

    /// R3074001: the ladders on which the profile holds a hand-set rating, Constructed first.
    pub async fn admin_hand_set_get(&mut self, profile_id: &str) -> StoreResult<Vec<HandSetRating>> {
        admin_dispatch!(self, hand_set_get(profile_id))
    }

    /// R3074001: marks the ladder hand-set, keeping `earned`. False, writing nothing, when it is
    /// hand-set already: the earned rating kept is the one from before the first hand-set rating.
    pub async fn admin_hand_set_keep(&mut self, row: &HandSetRating) -> StoreResult<bool> {
        admin_dispatch!(self, hand_set_keep(row))
    }

    /// R3074001: ends the ladder's hand-set rating, answering the row it removed, or `None` when
    /// the ladder was not hand-set.
    pub async fn admin_hand_set_clear(
        &mut self,
        profile_id: &str,
        ladder: Ladder,
    ) -> StoreResult<Option<HandSetRating>> {
        admin_dispatch!(self, hand_set_clear(profile_id, ladder))
    }

    /// R3074001, §14.11: appends one rating ledger row, always marked `dev`.
    pub async fn admin_rating_grants_append(&mut self, grant: &RatingGrant) -> StoreResult<()> {
        check_dev(grant.dev)?;
        admin_dispatch!(self, rating_grants_append(grant))
    }

    /// R3074001: the profile's rating ledger rows, oldest first.
    pub async fn admin_rating_grants_of(&mut self, profile_id: &str) -> StoreResult<Vec<RatingGrant>> {
        admin_dispatch!(self, rating_grants_of(profile_id))
    }

    /// R3074003: writes a new post. Its id must be free.
    pub async fn admin_posts_create(&mut self, post: &NewsPost) -> StoreResult<()> {
        check_post(post)?;
        admin_dispatch!(self, posts_create(post))
    }

    /// R3074003: the post, taken down or not, or `None`.
    pub async fn admin_posts_get(&mut self, id: &str) -> StoreResult<Option<NewsPost>> {
        admin_dispatch!(self, posts_get(id))
    }

    /// R3074003: the newest `limit` posts, taken down or not, newest first (of two posted at once,
    /// the one with the lower id first).
    pub async fn admin_posts_recent(&mut self, limit: i64) -> StoreResult<Vec<NewsPost>> {
        admin_dispatch!(self, posts_recent(limit))
    }

    /// R3074003, R2875001: the posts standing in `[from, until]` by `posted_at` and not taken down,
    /// newest first: what `GET /api/news` reads beside `news.json`.
    pub async fn admin_posts_standing(&mut self, from: i64, until: i64) -> StoreResult<Vec<NewsPost>> {
        admin_dispatch!(self, posts_standing(from, until))
    }

    /// R3074003: takes the post down at `at`, answering it as it now stands; `None`, writing
    /// nothing, when no post has the id or it is down already.
    pub async fn admin_posts_take_down(&mut self, id: &str, at: i64) -> StoreResult<Option<NewsPost>> {
        admin_dispatch!(self, posts_take_down(id, at))
    }
}

/// Migration 0080's foreign keys, in the fake (`db::fake::remove_profile_rows`): the hand-set
/// ratings and the rating ledger go with their account; an audit row and a post stay and forget it.
pub fn remove_admin_rows(tables: &mut AdminTables, profile_id: &str) {
    tables.hand_set.retain(|row| row.profile_id != profile_id);
    tables.rating_grants.retain(|row| row.profile_id != profile_id);
    for action in &mut tables.actions {
        if action.profile_id.as_deref() == Some(profile_id) {
            action.profile_id = None;
        }
    }
    for post in &mut tables.posts {
        if post.posted_by.as_deref() == Some(profile_id) {
            post.posted_by = None;
        }
    }
}

/// `rows` newest first by `at`, then by `tie` (a later id, or a lower one).
fn newest_first<T>(
    mut rows: Vec<T>,
    at: impl Fn(&T) -> i64,
    tie: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> Vec<T> {
    rows.sort_by(|a, b| at(b).cmp(&at(a)).then_with(|| tie(a, b)));
    rows
}

// Postgres (migration 0080)

mod pg {
    use sqlx::Row;
    use sqlx::postgres::PgRow;
    use sqlx::types::Json;
    use time::OffsetDateTime;

    use super::{
        AdminAction, HandSetRating, NewAdminAction, NewsPost, RATING_GRANT_REASON, RatingChange, RatingGrant,
    };
    use crate::db::pg::{PgTx, ms_of, run_as};
    use crate::db::store::StoreError;
    use crate::ranked::glicko2::Glicko;
    use crate::ranked::ladder::Ladder;

    fn ladder_of(row: &PgRow) -> Result<Ladder, StoreError> {
        let text: String = row.try_get("format")?;
        Ladder::from_literal(&text).ok_or_else(|| StoreError::from(format!("admin: no ladder is {text:?}")))
    }

    fn action_of(row: &PgRow) -> Result<AdminAction, StoreError> {
        Ok(AdminAction {
            id: row.try_get("id")?,
            profile_id: row
                .try_get::<Option<uuid::Uuid>, _>("profile_id")?
                .map(|id| id.to_string()),
            route: row.try_get("route")?,
            request: row.try_get::<Json<serde_json::Value>, _>("request")?.0,
            before: row.try_get::<Json<serde_json::Value>, _>("before_value")?.0,
            after: row.try_get::<Json<serde_json::Value>, _>("after_value")?.0,
            at: ms_of(row.try_get("at")?),
        })
    }

    fn hand_set_of(row: &PgRow) -> Result<HandSetRating, StoreError> {
        Ok(HandSetRating {
            profile_id: row.try_get::<uuid::Uuid, _>("profile_id")?.to_string(),
            ladder: ladder_of(row)?,
            earned: Glicko {
                rating: row.try_get("earned_rating")?,
                deviation: row.try_get("earned_deviation")?,
                volatility: row.try_get("earned_volatility")?,
            },
            set_at: ms_of(row.try_get("set_at")?),
        })
    }

    fn post_of(row: &PgRow) -> Result<NewsPost, StoreError> {
        Ok(NewsPost {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            summary: row.try_get("summary")?,
            posted_at: ms_of(row.try_get("posted_at")?),
            posted_by: row
                .try_get::<Option<uuid::Uuid>, _>("posted_by")?
                .map(|id| id.to_string()),
            taken_down_at: row
                .try_get::<Option<OffsetDateTime>, _>("taken_down_at")?
                .map(ms_of),
        })
    }

    /// A post's columns, as `post_of` reads them; a macro so each statement stays one literal.
    macro_rules! post_columns {
        () => {
            "id, title, summary, posted_at, posted_by, taken_down_at"
        };
    }

    pub async fn actions_append(t: &mut PgTx<'_>, action: &NewAdminAction) -> Result<i64, StoreError> {
        run_as(t, Some(action.profile_id.as_str())).await?;
        let id: i64 = sqlx::query_scalar(
            "insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
             values ($1::uuid, $2::text, $3::jsonb, $4::jsonb, $5::jsonb,
                     to_timestamp($6::double precision / 1000.0))
             returning id",
        )
        .bind(&action.profile_id)
        .bind(&action.route)
        .bind(Json(&action.request))
        .bind(Json(&action.before))
        .bind(Json(&action.after))
        .bind(action.at)
        .fetch_one(&mut **t)
        .await?;
        Ok(id)
    }

    pub async fn actions_recent(t: &mut PgTx<'_>, limit: i64) -> Result<Vec<AdminAction>, StoreError> {
        run_as(t, None).await?;
        let rows = sqlx::query(
            "select id, profile_id, route, request, before_value, after_value, at
               from public.admin_actions
              order by at desc, id desc
              limit $1::bigint",
        )
        .bind(limit)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(action_of).collect()
    }

    pub async fn hand_set_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<HandSetRating>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = sqlx::query(
            "select profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at
               from public.hand_set_ratings
              where profile_id = $1::uuid
              order by case format when 'constructed' then 0 else 1 end",
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(hand_set_of).collect()
    }

    pub async fn hand_set_keep(t: &mut PgTx<'_>, row: &HandSetRating) -> Result<bool, StoreError> {
        run_as(t, Some(row.profile_id.as_str())).await?;
        let done = sqlx::query(
            "insert into public.hand_set_ratings
               (profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at)
             values ($1::uuid, $2::text, $3::double precision, $4::double precision,
                     $5::double precision, to_timestamp($6::double precision / 1000.0))
             on conflict (profile_id, format) do nothing",
        )
        .bind(&row.profile_id)
        .bind(row.ladder.as_str())
        .bind(row.earned.rating)
        .bind(row.earned.deviation)
        .bind(row.earned.volatility)
        .bind(row.set_at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn hand_set_clear(
        t: &mut PgTx<'_>,
        profile_id: &str,
        ladder: Ladder,
    ) -> Result<Option<HandSetRating>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let row = sqlx::query(
            "delete from public.hand_set_ratings
              where profile_id = $1::uuid and format = $2::text
              returning profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at",
        )
        .bind(profile_id)
        .bind(ladder.as_str())
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(hand_set_of).transpose()
    }

    pub async fn rating_grants_append(t: &mut PgTx<'_>, grant: &RatingGrant) -> Result<(), StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        sqlx::query(
            "insert into public.rating_grants
               (profile_id, format, kind, rating_before, deviation_before, volatility_before,
                rating_after, deviation_after, volatility_after, reason, dev, at)
             values ($1::uuid, $2::text, $3::text, $4::double precision, $5::double precision,
                     $6::double precision, $7::double precision, $8::double precision,
                     $9::double precision, $10::text, $11::boolean,
                     to_timestamp($12::double precision / 1000.0))",
        )
        .bind(&grant.profile_id)
        .bind(grant.ladder.as_str())
        .bind(grant.change.as_str())
        .bind(grant.before.rating)
        .bind(grant.before.deviation)
        .bind(grant.before.volatility)
        .bind(grant.after.rating)
        .bind(grant.after.deviation)
        .bind(grant.after.volatility)
        .bind(RATING_GRANT_REASON)
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    pub async fn rating_grants_of(
        t: &mut PgTx<'_>,
        profile_id: &str,
    ) -> Result<Vec<RatingGrant>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = sqlx::query(
            "select profile_id, format, kind, rating_before, deviation_before, volatility_before,
                    rating_after, deviation_after, volatility_after, dev, at
               from public.rating_grants
              where profile_id = $1::uuid
              order by at, id",
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter()
            .map(|row| {
                let kind: String = row.try_get("kind")?;
                Ok(RatingGrant {
                    profile_id: row.try_get::<uuid::Uuid, _>("profile_id")?.to_string(),
                    ladder: ladder_of(row)?,
                    change: RatingChange::from_literal(&kind)
                        .ok_or_else(|| StoreError::from(format!("admin: no rating change is {kind:?}")))?,
                    before: Glicko {
                        rating: row.try_get("rating_before")?,
                        deviation: row.try_get("deviation_before")?,
                        volatility: row.try_get("volatility_before")?,
                    },
                    after: Glicko {
                        rating: row.try_get("rating_after")?,
                        deviation: row.try_get("deviation_after")?,
                        volatility: row.try_get("volatility_after")?,
                    },
                    dev: row.try_get("dev")?,
                    at: ms_of(row.try_get("at")?),
                })
            })
            .collect()
    }

    pub async fn posts_create(t: &mut PgTx<'_>, post: &NewsPost) -> Result<(), StoreError> {
        run_as(t, post.posted_by.as_deref()).await?;
        sqlx::query(
            "insert into public.news_posts (id, title, summary, posted_at, posted_by, taken_down_at)
             values ($1::text, $2::text, $3::text, to_timestamp($4::double precision / 1000.0),
                     $5::uuid, case when $6::bigint is null then null
                                    else to_timestamp($6::double precision / 1000.0) end)",
        )
        .bind(&post.id)
        .bind(&post.title)
        .bind(&post.summary)
        .bind(post.posted_at)
        .bind(&post.posted_by)
        .bind(post.taken_down_at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    pub async fn posts_get(t: &mut PgTx<'_>, id: &str) -> Result<Option<NewsPost>, StoreError> {
        run_as(t, None).await?;
        let row = sqlx::query(concat!(
            "select ",
            post_columns!(),
            " from public.news_posts where id = $1::text"
        ))
        .bind(id)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(post_of).transpose()
    }

    pub async fn posts_recent(t: &mut PgTx<'_>, limit: i64) -> Result<Vec<NewsPost>, StoreError> {
        run_as(t, None).await?;
        let rows = sqlx::query(concat!(
            "select ",
            post_columns!(),
            r#" from public.news_posts
                order by posted_at desc, id collate "C"
                limit $1::bigint"#
        ))
        .bind(limit)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(post_of).collect()
    }

    pub async fn posts_standing(
        t: &mut PgTx<'_>,
        from: i64,
        until: i64,
    ) -> Result<Vec<NewsPost>, StoreError> {
        run_as(t, None).await?;
        let rows = sqlx::query(concat!(
            "select ",
            post_columns!(),
            r#" from public.news_posts
                where taken_down_at is null
                  and posted_at >= to_timestamp($1::double precision / 1000.0)
                  and posted_at <= to_timestamp($2::double precision / 1000.0)
                order by posted_at desc, id collate "C""#
        ))
        .bind(from)
        .bind(until)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(post_of).collect()
    }

    pub async fn posts_take_down(
        t: &mut PgTx<'_>,
        id: &str,
        at: i64,
    ) -> Result<Option<NewsPost>, StoreError> {
        run_as(t, None).await?;
        let row = sqlx::query(concat!(
            "update public.news_posts
                set taken_down_at = to_timestamp($2::double precision / 1000.0)
              where id = $1::text and taken_down_at is null
              returning ",
            post_columns!()
        ))
        .bind(id)
        .bind(at)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(post_of).transpose()
    }
}

// The in-memory fake (`FakeTables::admin`)

mod fake {
    use super::{
        AdminAction, AdminTables, HandSetRating, NewAdminAction, NewsPost, RatingGrant, newest_first,
    };
    use crate::db::fake::FakeTx;
    use crate::db::store::StoreError;
    use crate::ranked::ladder::Ladder;

    /// The unit fake's two seams (`db::fake`'s `call`): its fault-injection hook, then its race hook,
    /// each told the method's name.
    fn call<'t>(f: &'t mut FakeTx<'_>, method: &str) -> Result<&'t mut AdminTables, StoreError> {
        let data = f.data();
        if let Some(hook) = data.on_call.clone() {
            hook(method)?;
        }
        if let Some(hook) = data.before_call.clone() {
            hook(method, &mut data.tables);
        }
        Ok(&mut f.tables().admin)
    }

    pub fn actions_append(f: &mut FakeTx<'_>, action: &NewAdminAction) -> Result<i64, StoreError> {
        let tables = call(f, "admin.actions.append")?;
        let id = tables.actions.iter().map(|row| row.id).max().unwrap_or(0) + 1;
        tables.actions.push(AdminAction {
            id,
            profile_id: Some(action.profile_id.clone()),
            route: action.route.clone(),
            request: action.request.clone(),
            before: action.before.clone(),
            after: action.after.clone(),
            at: action.at,
        });
        Ok(id)
    }

    pub fn actions_recent(f: &mut FakeTx<'_>, limit: i64) -> Result<Vec<AdminAction>, StoreError> {
        let tables = call(f, "admin.actions.recent")?;
        let rows = newest_first(tables.actions.clone(), |row| row.at, |a, b| b.id.cmp(&a.id));
        Ok(rows
            .into_iter()
            .take(usize::try_from(limit).unwrap_or(0))
            .collect())
    }

    pub fn hand_set_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<HandSetRating>, StoreError> {
        let tables = call(f, "admin.handSet.get")?;
        let mut rows: Vec<HandSetRating> = tables
            .hand_set
            .iter()
            .filter(|row| row.profile_id == profile_id)
            .cloned()
            .collect();
        rows.sort_by_key(|row| Ladder::ALL.iter().position(|ladder| *ladder == row.ladder));
        Ok(rows)
    }

    /// A ladder hand-set already writes nothing, as Postgres's `on conflict do nothing` does.
    pub fn hand_set_keep(f: &mut FakeTx<'_>, row: &HandSetRating) -> Result<bool, StoreError> {
        let tables = call(f, "admin.handSet.keep")?;
        if tables
            .hand_set
            .iter()
            .any(|held| held.profile_id == row.profile_id && held.ladder == row.ladder)
        {
            return Ok(false);
        }
        tables.hand_set.push(row.clone());
        Ok(true)
    }

    pub fn hand_set_clear(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        ladder: Ladder,
    ) -> Result<Option<HandSetRating>, StoreError> {
        let tables = call(f, "admin.handSet.clear")?;
        let at = tables
            .hand_set
            .iter()
            .position(|held| held.profile_id == profile_id && held.ladder == ladder);
        Ok(at.map(|at| tables.hand_set.remove(at)))
    }

    pub fn rating_grants_append(f: &mut FakeTx<'_>, grant: &RatingGrant) -> Result<(), StoreError> {
        let tables = call(f, "admin.ratingGrants.append")?;
        tables.rating_grants.push(grant.clone());
        Ok(())
    }

    pub fn rating_grants_of(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<RatingGrant>, StoreError> {
        let tables = call(f, "admin.ratingGrants.of")?;
        let mut rows: Vec<RatingGrant> = tables
            .rating_grants
            .iter()
            .filter(|row| row.profile_id == profile_id)
            .cloned()
            .collect();
        // Stable: rows written at one time keep the order they were written in, as `order by at, id`.
        rows.sort_by_key(|row| row.at);
        Ok(rows)
    }

    /// A taken id is refused, as the table's key refuses it.
    pub fn posts_create(f: &mut FakeTx<'_>, post: &NewsPost) -> Result<(), StoreError> {
        let tables = call(f, "admin.posts.create")?;
        if tables.posts.iter().any(|held| held.id == post.id) {
            return Err(StoreError::Other(format!("admin: post {:?} exists", post.id)));
        }
        tables.posts.push(post.clone());
        Ok(())
    }

    pub fn posts_get(f: &mut FakeTx<'_>, id: &str) -> Result<Option<NewsPost>, StoreError> {
        let tables = call(f, "admin.posts.get")?;
        Ok(tables.posts.iter().find(|post| post.id == id).cloned())
    }

    fn by_id(a: &NewsPost, b: &NewsPost) -> std::cmp::Ordering {
        a.id.as_bytes().cmp(b.id.as_bytes())
    }

    pub fn posts_recent(f: &mut FakeTx<'_>, limit: i64) -> Result<Vec<NewsPost>, StoreError> {
        let tables = call(f, "admin.posts.recent")?;
        let rows = newest_first(tables.posts.clone(), |post| post.posted_at, by_id);
        Ok(rows
            .into_iter()
            .take(usize::try_from(limit).unwrap_or(0))
            .collect())
    }

    pub fn posts_standing(f: &mut FakeTx<'_>, from: i64, until: i64) -> Result<Vec<NewsPost>, StoreError> {
        let tables = call(f, "admin.posts.standing")?;
        let standing: Vec<NewsPost> = tables
            .posts
            .iter()
            .filter(|post| post.taken_down_at.is_none() && (from..=until).contains(&post.posted_at))
            .cloned()
            .collect();
        Ok(newest_first(standing, |post| post.posted_at, by_id))
    }

    pub fn posts_take_down(f: &mut FakeTx<'_>, id: &str, at: i64) -> Result<Option<NewsPost>, StoreError> {
        let tables = call(f, "admin.posts.takeDown")?;
        let Some(post) = tables
            .posts
            .iter_mut()
            .find(|post| post.id == id && post.taken_down_at.is_none())
        else {
            return Ok(None);
        };
        post.taken_down_at = Some(at);
        Ok(Some(post.clone()))
    }
}
