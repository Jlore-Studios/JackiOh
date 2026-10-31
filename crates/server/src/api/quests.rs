//! The daily quests an account holds, and the dev tools that give and complete one (SPEC §14.9,
//! §14.10, #1304).
//!
//! `GET /api/quests` answers the caller's own held quests, each with its progress, target and
//! payout, and when the next is due, and nobody else's (§14). While the economy is live it tops
//! the account up first ([[R1304000]]), so a player who shows up after a long absence finds
//! `QUEST_MAX_HELD` waiting; while it is off nothing arrives ([[R1304002]]) and nothing is due.
//! The rhythm and the payout are `economy::quests`'s.
//!
//! `POST /api/quests/:questId/replace` swaps a held quest for another, `QUEST_REPLACES_PER_DAY`
//! times a UTC day, while the economy is live: 409 `conflict` when it is off, when the day's
//! replaces are spent or when no other quest is left to draw, and 404 `not_found` for a quest the
//! account does not hold, each writing nothing ([[R2210000]]). Both routes answer `replacesLeft`, how
//! many replaces the account has left today, 0 while the economy is off.
//!
//! The dev tools (`POST /api/dev/quests`, `POST /api/dev/quests/:questId/complete`) give the caller
//! a quest at once and complete one it holds, as rows marked `dev` (§14.10), so quests are tested
//! without waiting. Like every dev tool they answer a dev account alone (`api::dev::dev_caller`,
//! R1300000), and anyone else 403 `forbidden`, writing nothing.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::dev::dev_caller;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::app::{App, now_ms};
use crate::config::QUEST_MAX_HELD;
use crate::db::store::Tx;
use crate::economy::quests::{
    ReplaceOutcome, arrival, complete_quest, fresh_seed, quest_def, quest_xp, replace_quest, replaces_left,
    top_up_quests,
};
use crate::economy::store::{EconomyReason, HeldQuest, QuestBook};

/// A held quest as the client reads it; `None` for an id the pool no longer has.
fn quest_view(held: &HeldQuest) -> Option<Value> {
    let def = quest_def(&held.quest_id)?;
    Some(json!({
        "questId": def.id,
        "title": def.title,
        "progress": held.progress,
        "target": def.target,
        "jlore": def.jlore,
        "xp": quest_xp(def),
        "arrivedAt": held.arrived_at,
    }))
}

/// The account's quests, topped up at `now` while the economy is live ([[R1304002]]).
async fn quests_at(app: &App, tx: &mut Tx<'_>, profile_id: &str, now: i64) -> Result<QuestBook, ApiError> {
    Ok(if app.economy_live {
        top_up_quests(tx, profile_id, now).await?
    } else {
        tx.economy_quests_get(profile_id).await?
    })
}

/// The answer of both quest routes: `{ quests, nextDueAt, replacesLeft }`, the held quests oldest
/// first and when the next falls due; `nextDueAt` is `null` while the economy is off and while the
/// account holds `QUEST_MAX_HELD`, since none arrives then.
fn quests_answer(app: &App, book: &QuestBook, replaces_left: i64) -> Value {
    let quests: Vec<Value> = book.held.iter().filter_map(quest_view).collect();
    let next_due_at = book
        .next_due_at
        .filter(|_| app.economy_live && book.held.len() < QUEST_MAX_HELD);
    json!({ "quests": quests, "nextDueAt": next_due_at, "replacesLeft": replaces_left })
}

/// `GET /api/quests` (`AuthLevel::Active`): the quests ([`quests_answer`]); `replacesLeft` is the
/// replaces the account may still spend today ([[R2210000]]), 0 while the economy is off.
pub async fn get_quests(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let book = quests_at(app, &mut tx, &profile.id, now).await?;
    let left = if app.economy_live {
        replaces_left(&mut tx, &profile.id, now).await?
    } else {
        0
    };
    tx.commit().await?;
    Ok(json(200, quests_answer(app, &book, left)))
}

/// `POST /api/quests/:questId/replace` (`AuthLevel::Active`): replaces a held quest with another
/// ([`replace_quest`], [[R2210000]]) and answers what `GET /api/quests` answers after it. 409 while
/// the economy is off, when today's replaces are spent and when no other quest is left to draw;
/// 404 for a quest the account does not hold. A refusal writes nothing. It runs in the account's
/// economy turn; past the requests waiting theirs it is 429 `rate_limited` (R2378000).
pub async fn post_replace_quest(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    if !app.economy_live {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "Quests can't be replaced yet.",
        ));
    }
    let not_held = || ApiError::new(ApiErrorCode::NotFound, "That quest is not held.");
    let quest_id = req.params.get("questId").cloned().ok_or_else(not_held)?;
    // R2378000: the account's turn first, so a burst waits here holding no database
    // connection, not on the economy lock `replace_quest` takes.
    let _turn = app.spends.claim(&profile.id).await?;
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    match replace_quest(&mut tx, &profile.id, &quest_id, now).await? {
        ReplaceOutcome::Replaced(_) => {}
        ReplaceOutcome::Spent => {
            return Err(ApiError::new(
                ApiErrorCode::Conflict,
                "You've already replaced a quest today. You can replace another tomorrow.",
            ));
        }
        ReplaceOutcome::NotHeld => return Err(not_held()),
        ReplaceOutcome::NoneLeft => {
            return Err(ApiError::new(
                ApiErrorCode::Conflict,
                "There's no other quest to swap in.",
            ));
        }
    }
    let book = tx.economy_quests_get(&profile.id).await?;
    let left = replaces_left(&mut tx, &profile.id, now).await?;
    tx.commit().await?;
    Ok(json(200, quests_answer(app, &book, left)))
}

/// `POST /api/dev/quests` (`AuthLevel::Active`, the dev tools): a quest at once, drawn as an
/// arrival is, marked `dev`; `{ quest }`. 409 while the account holds `QUEST_MAX_HELD`.
pub async fn dev_give_quest(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let book = quests_at(app, &mut tx, &profile.id, now).await?;
    let full = || {
        ApiError::new(
            ApiErrorCode::Conflict,
            "This account already holds the most quests it may.",
        )
    };
    if book.held.len() >= QUEST_MAX_HELD {
        return Err(full());
    }
    let held: Vec<String> = book.held.iter().map(|quest| quest.quest_id.clone()).collect();
    let (_, grant) = arrival(&profile.id, &held, fresh_seed(), EconomyReason::Dev, now).ok_or_else(full)?;
    tx.economy_quests_change(&grant).await?;
    tx.commit().await?;
    let quest = quest_view(&HeldQuest {
        quest_id: grant.quest_id,
        progress: 0,
        arrived_at: now,
    });
    Ok(json(200, json!({ "quest": quest })))
}

/// `POST /api/dev/quests/:questId/complete` (`AuthLevel::Active`, the dev tools): completes a held
/// quest and pays it as a completion does, every row marked `dev`; `{ completed, jlore, xp }`. 404
/// for a quest the account does not hold.
pub async fn dev_complete_quest(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let not_held = || ApiError::new(ApiErrorCode::NotFound, "That quest is not held.");
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    quests_at(app, &mut tx, &profile.id, now).await?;
    let def = req
        .params
        .get("questId")
        .and_then(|id| quest_def(id))
        .ok_or_else(not_held)?;
    if !complete_quest(&mut tx, &profile.id, def, EconomyReason::Dev, None, true, now).await? {
        return Err(not_held());
    }
    tx.commit().await?;
    Ok(json(
        200,
        json!({ "completed": def.id, "jlore": def.jlore, "xp": quest_xp(def) }),
    ))
}
