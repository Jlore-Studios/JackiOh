//! The player's game settings on the account (SPEC §9.1, R633, R634). Port of
//! `apps/server/src/api/settings.ts`.
//!
//! The device keeps the switches, volumes and choices a player has set in the settings dialog in
//! `localStorage`. An active account keeps the same on the server too, so a player who signs in on
//! another device, or clears site data, finds them there, and the client merges the two copies
//! (R634). This file is the account's half: two routes, both `Active` (a pending account has the
//! code screen and nothing else, §9.4), both keyed on the profile the verified token names and
//! never on anything in the body.
//!
//!   GET /api/settings  -> { settings: { groups } }
//!   PUT /api/settings  <- { groups: { <id>: { at, values } } }
//!                      -> { settings: { groups } } as it stands after the merge
//!
//! A group is one of the client's stores (gameplay, audio, effects, card display): `values` is a
//! flat object of its settings and `at` the time, on the writing device's clock, that it last
//! changed. A PUT merges and never replaces (R634): each group sent replaces the stored one only
//! when its `at` is strictly later, and a group the write does not name is left alone, so a stale
//! device can never undo a newer change and a change to one group never undoes another's. The same
//! body sent twice changes nothing, so a client may retry it freely. The merge is the store's
//! (`app.merge_player_settings`, migration 0018), under a lock on the profile, so two devices
//! writing at once cannot lose each other's group.
//!
//! The server does not know the settings: they are the client's, and a setting added there needs no
//! change here. So a body is checked for its shape only — at most `PLAYER_SETTINGS_GROUPS_MAX`
//! groups, each a slug of at most `PLAYER_SETTINGS_NAME_MAX_LENGTH` characters holding at most
//! `PLAYER_SETTINGS_KEYS_MAX` settings named by short words, each a boolean, a finite number or a
//! text of at most `PLAYER_SETTINGS_TEXT_MAX_LENGTH` characters — and an account holds at most
//! `PLAYER_SETTINGS_BYTES_MAX` bytes of them. None of it is a rule: nothing the engine does reads
//! a setting.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, log_info, now_ms, ok_of};
use crate::app::App;
use crate::config::{
    PLAYER_SETTINGS_BYTES_MAX, PLAYER_SETTINGS_GROUPS_MAX, PLAYER_SETTINGS_KEYS_MAX, PLAYER_SETTINGS_NAME_MAX_LENGTH,
    PLAYER_SETTINGS_TEXT_MAX_LENGTH,
};
use crate::db::store::{
    PlayerSettingValue, PlayerSettingsGroup, PlayerSettingsLimits, PlayerSettingsMergeInput,
    PlayerSettingsMergeOutcome, PlayerSettingsRow, Profile,
};

/// R633: a group id is a lower-case slug (`gameplay`, `audio`, `fx`, `cards`).
///
/// TS `/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u`.
fn is_group_id_shape(id: &str) -> bool {
    let mut words = id.split('-');
    let Some(head) = words.next() else {
        return false;
    };
    let mut head_chars = head.chars();
    if !head_chars.next().is_some_and(|ch| ch.is_ascii_lowercase()) {
        return false;
    }
    if !head_chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return false;
    }
    words.all(|word| !word.is_empty() && word.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()))
}

/// R633: a setting's name is a camelCase or lower-case word (`dragToPlay`, `master`).
///
/// TS `/^[a-zA-Z][a-zA-Z0-9]*$/u`.
fn is_setting_name_shape(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) && chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// TS `string.length`: UTF-16 code units.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// What the client reads: `PlayerSettingsAccountCopy` in `apps/web/src/net/api.ts`.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsView {
    pub groups: IndexMap<String, PlayerSettingsGroup>,
}

fn settings_view(row: Option<&PlayerSettingsRow>) -> PlayerSettingsView {
    match row {
        None => PlayerSettingsView { groups: IndexMap::new() },
        Some(row) => PlayerSettingsView {
            groups: row
                .groups
                .iter()
                .map(|(id, group)| (id.clone(), PlayerSettingsGroup { at: group.at, values: group.values.clone() }))
                .collect(),
        },
    }
}

fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// A setting's value: a boolean, a finite number or a short text, as the store keeps it.
fn read_value(group: &str, name: &str, value: &Value) -> Result<PlayerSettingValue, ApiError> {
    let accepted = match value {
        Value::Bool(_) => true,
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::String(text) => utf16_len(text) <= PLAYER_SETTINGS_TEXT_MAX_LENGTH as usize,
        _ => false,
    };
    if !accepted {
        return Err(bad_request(format!(
            "\"{group}.{name}\" must be a boolean, a number or a text of at most {PLAYER_SETTINGS_TEXT_MAX_LENGTH} characters"
        )));
    }
    Ok(serde_json::from_value(value.clone())?)
}

/// TS `typeof at === "number" && Number.isSafeInteger(at) && at >= 0`.
fn epoch_ms_of(value: Option<&Value>) -> Option<i64> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    let number = value?.as_f64()?;
    if number.fract() != 0.0 || number < 0.0 || number > MAX_SAFE_INTEGER {
        return None;
    }
    Some(number as i64)
}

/// R633, R634: `groups` is an object of at most `PLAYER_SETTINGS_GROUPS_MAX` groups, each
/// `{ at, values }`: `at` a whole number of epoch milliseconds, the writing device's clock, and
/// `values` a flat object of at most `PLAYER_SETTINGS_KEYS_MAX` settings. An `at` after the server's
/// own clock is taken as now, as R320 does for a choice: a device whose clock runs ahead could
/// otherwise make its settings win over every later change for as long as its clock stays ahead.
pub fn read_groups(body: &Value, now: i64) -> Result<IndexMap<String, PlayerSettingsGroup>, ApiError> {
    let Some(raw) = body.get("groups").filter(|raw| is_record(raw)).and_then(Value::as_object) else {
        return Err(bad_request("\"groups\" must be an object of setting groups"));
    };
    if raw.len() > PLAYER_SETTINGS_GROUPS_MAX as usize {
        return Err(bad_request(format!("at most {PLAYER_SETTINGS_GROUPS_MAX} setting groups can be saved")));
    }
    let mut groups: IndexMap<String, PlayerSettingsGroup> = IndexMap::new();
    for (id, group) in raw {
        if utf16_len(id) > PLAYER_SETTINGS_NAME_MAX_LENGTH as usize || !is_group_id_shape(id) {
            return Err(bad_request(format!(
                "every group id must be a lower-case slug of at most {PLAYER_SETTINGS_NAME_MAX_LENGTH} characters"
            )));
        }
        let values_raw = group.get("values").filter(|values| is_record(values)).and_then(Value::as_object);
        let Some(values_raw) = values_raw.filter(|_| is_record(group)) else {
            return Err(bad_request(format!("\"{id}\" must be {{ at: epoch milliseconds, values: an object }}")));
        };
        let Some(at) = epoch_ms_of(group.get("at")) else {
            return Err(bad_request(format!("\"{id}.at\" must be a whole number of epoch milliseconds")));
        };
        if values_raw.len() > PLAYER_SETTINGS_KEYS_MAX as usize {
            return Err(bad_request(format!("\"{id}\" can hold at most {PLAYER_SETTINGS_KEYS_MAX} settings")));
        }
        let mut values: IndexMap<String, PlayerSettingValue> = IndexMap::new();
        for (name, value) in values_raw {
            if utf16_len(name) > PLAYER_SETTINGS_NAME_MAX_LENGTH as usize || !is_setting_name_shape(name) {
                return Err(bad_request(format!(
                    "every setting name must be letters and digits, at most {PLAYER_SETTINGS_NAME_MAX_LENGTH} characters"
                )));
            }
            values.insert(name.clone(), read_value(id, name, value)?);
        }
        groups.insert(id.clone(), PlayerSettingsGroup { at: at.min(now), values });
    }
    Ok(groups)
}

/// The caller behind a route that declares `AuthLevel::Active` (a private copy of
/// `collection.rs`'s `callerProfile`).
fn caller_profile(req: &Req) -> Result<&Profile, ApiError> {
    req.caller.as_ref().map(|caller| &caller.profile).ok_or_else(|| bad_request("this endpoint needs a signed-in profile"))
}

// TS's `createSettingsRoutes()` is two rows of `app.rs`'s `ROUTES`, `GET` and `PUT /api/settings`
// (→ get_settings, put_settings). Both `AuthLevel::Active`, so a pending account gets 403 from each
// (§9.4).

/// `GET /api/settings`.
pub async fn get_settings(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let row = tx.player_settings_get(&profile.id).await?;
    tx.commit().await?;
    ok_of(&json!({ "settings": settings_view(row.as_ref()) }))
}

/// `PUT /api/settings`: the groups sent, merged into the account's (R634).
pub async fn put_settings(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let now = now_ms();
    let groups = read_groups(&req.body, now)?;
    let sent = groups.len();

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let outcome = tx
        .player_settings_merge(
            &PlayerSettingsMergeInput { profile_id: profile.id.clone(), groups, at: now },
            &PlayerSettingsLimits {
                max_groups: PLAYER_SETTINGS_GROUPS_MAX as i64,
                max_bytes: PLAYER_SETTINGS_BYTES_MAX as i64,
            },
        )
        .await?;
    tx.commit().await?;
    let settings = match outcome {
        PlayerSettingsMergeOutcome::Limit => {
            return Err(ApiError::with_details(
                ApiErrorCode::Conflict,
                format!(
                    "This account already holds the most settings it can ({PLAYER_SETTINGS_GROUPS_MAX} groups, {PLAYER_SETTINGS_BYTES_MAX} bytes)."
                ),
                json!({ "groups": PLAYER_SETTINGS_GROUPS_MAX, "bytes": PLAYER_SETTINGS_BYTES_MAX }),
            ));
        }
        PlayerSettingsMergeOutcome::Merged { settings } => settings,
    };
    log_info(
        "settings.merged",
        json!({ "profileId": profile.id, "sent": sent, "held": settings.groups.len() }),
    );
    ok_of(&json!({ "settings": settings_view(Some(&settings)) }))
}
