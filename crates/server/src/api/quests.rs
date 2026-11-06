//! The daily quests an account holds, and the dev tools that give and complete one (SPEC §14.9,
//! §14.10, #1304). `GET /api/quests` answers the caller's own held quests and nobody else's (§14).
//! While the economy is live it tops the account up first (R1304000), so a player back after a long
//! absence finds `QUEST_MAX_HELD` waiting; while it is off nothing arrives (R1304002). The rhythm and
//! the payout are `economy::quests`'s.
//!
//! `POST /api/quests/:questId/replace` swaps a held quest, `QUEST_REPLACES_PER_DAY` times a UTC day,
//! while the economy is live (R2210000).
//!
//! The dev tools (`POST /api/dev/quests`, `POST /api/dev/quests/:questId/complete`) give and complete
//! a quest at once as rows marked `dev` (§14.10), so quests are tested without waiting. They answer a
//! dev account alone (`api::dev::dev_caller`, R1300000), anyone else 403 `forbidden`.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::dev::dev_caller;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::app::{App, now_ms};
use crate::config::QUEST_MAX_HELD;
use crate::db::store::Tx;
use crate::economy::grants::{Writing, internal_refusal, write_grants};
use crate::economy::quests::{
    ReplaceOutcome, complete_quest, fresh_seed, give_quest, quest_def, quest_xp, replace_quest,
    replaces_left, top_up_quests,
};
use crate::economy::store::{EconomyReason, HeldQuest, QuestBook};
use jackioh_shared::economy::ledger::LedgerGrant;

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
/// first; `nextDueAt` is `null` while the economy is off or the account holds `QUEST_MAX_HELD`.
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

/// `POST /api/quests/:questId/replace` (`AuthLevel::Active`): replaces a held quest
/// ([`replace_quest`], [[R2210000]]), answering as `GET /api/quests`. 409 while the economy is off,
/// today's replaces are spent or no other quest is left; 404 for a quest not held; a refusal writes
/// nothing. Past the requests waiting their economy turn it is 429 `rate_limited` (R2378000).
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
    let held: Vec<String> = book.held.iter().map(|quest| quest.quest_id.clone()).collect();
    let grant = give_quest(&book, &fresh_seed(), now)
        .map_err(|refusal| ApiError::new(ApiErrorCode::Conflict, refusal.message()))?;
    let LedgerGrant::Quest { quest_id, .. } = &grant else {
        return Err(ApiError::internal("a dev give drew no quest"));
    };
    let quest_id = quest_id.clone();
    write_grants(
        &mut tx,
        &Writing {
            profile_id: &profile.id,
            reference: &None,
            dev: true,
            at: now,
        },
        &[grant],
        internal_refusal,
    )
    .await?;
    let quest = quest_view(&HeldQuest {
        quest_id,
        progress: 0,
        arrived_at: now,
    });
    // R3074000: every dev tool writes its audit row in its own transaction.
    crate::api::admin::audit(
        &mut tx,
        &profile.id,
        "POST /api/dev/quests".to_string(),
        json!({}),
        json!({ "held": held }),
        json!({ "quest": quest }),
        now,
    )
    .await?;
    tx.commit().await?;
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
    let answer = json!({ "completed": def.id, "jlore": def.jlore, "xp": quest_xp(def) });
    // R3074000: every dev tool writes its audit row in its own transaction.
    crate::api::admin::audit(
        &mut tx,
        &profile.id,
        format!("POST /api/dev/quests/{}/complete", def.id),
        json!({ "questId": def.id }),
        json!({ "held": def.id }),
        answer.clone(),
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(json(200, answer))
}
