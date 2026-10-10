//! The player's game settings on the account (SPEC §9.1, R633, R634).
//!
//! The device keeps the settings in `localStorage`; an active account keeps the same on the server
//! and the client merges the two copies (R634). Two routes, both `Active` (§9.4), keyed on the
//! profile the token names and never on the body: `GET /api/settings` and `PUT /api/settings` with
//! `{ groups: { <id>: { at, values } } }`, answering `{ settings: { groups } }` after the merge.
//!
//! A PUT merges and never replaces (R634): a group replaces the stored one only when its `at` is
//! strictly later, and a group not named is left alone. The same body twice changes nothing. The
//! merge is the store's (`app.merge_player_settings`, migration 0018), under a lock on the profile.
//! The server does not know the settings, so a body is checked for shape only (`PLAYER_SETTINGS_*`
//! limits). None of it is a rule: nothing the engine does reads a setting.

use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::wire::utf16_len;
use serde::Serialize;
use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, log_info, now_ms, ok_of};
use crate::app::App;
use crate::config::{
    PLAYER_SETTINGS_BYTES_MAX, PLAYER_SETTINGS_GROUPS_MAX, PLAYER_SETTINGS_KEYS_MAX,
    PLAYER_SETTINGS_NAME_MAX_LENGTH, PLAYER_SETTINGS_TEXT_MAX_LENGTH,
};
use crate::db::store::{
    PlayerSettingValue, PlayerSettingsGroup, PlayerSettingsLimits, PlayerSettingsMergeInput,
    PlayerSettingsMergeOutcome, PlayerSettingsRow,
};

/// R633: a group id is a lower-case slug (`gameplay`, `audio`, `fx`, `cards`).
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
    words.all(|word| {
        !word.is_empty()
            && word
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    })
}

/// R633: a setting's name is a camelCase or lower-case word (`dragToPlay`, `master`).
fn is_setting_name_shape(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) && chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// What the client reads: `PlayerSettingsAccountCopy` in `apps/web/src/net/api.ts`.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsView {
    pub groups: IndexMap<String, PlayerSettingsGroup>,
}

fn settings_view(row: Option<&PlayerSettingsRow>) -> PlayerSettingsView {
    match row {
        None => PlayerSettingsView {
            groups: IndexMap::new(),
        },
        Some(row) => PlayerSettingsView {
            groups: row
                .groups
                .iter()
                .map(|(id, group)| {
                    (
                        id.clone(),
                        PlayerSettingsGroup {
                            at: group.at,
                            values: group.values.clone(),
                        },
                    )
                })
                .collect(),
        },
    }
}

/// A setting's value: a boolean, a finite number or a short text, as the store keeps it.
fn read_value(group: &str, name: &str, value: &Value) -> Result<PlayerSettingValue, ApiError> {
    let accepted = match value {
        Value::Bool(_) => true,
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::String(text) => utf16_len(text) <= PLAYER_SETTINGS_TEXT_MAX_LENGTH,
        _ => false,
    };
    if !accepted {
        return Err(bad_request(format!(
            "\"{group}.{name}\" must be a boolean, a number or a text of at most {PLAYER_SETTINGS_TEXT_MAX_LENGTH} characters"
        )));
    }
    Ok(serde_json::from_value(value.clone())?)
}

/// A whole number of epoch milliseconds, or `None`.
fn epoch_ms_of(value: Option<&Value>) -> Option<i64> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    let number = value?.as_f64()?;
    if number.fract() != 0.0 || !(0.0..=MAX_SAFE_INTEGER).contains(&number) {
        return None;
    }
    Some(number as i64)
}

/// R633, R634: `groups` is an object of at most `PLAYER_SETTINGS_GROUPS_MAX` groups, each
/// `{ at, values }`: `at` a whole number of epoch ms on the writing device's clock, `values` a flat
/// object of at most `PLAYER_SETTINGS_KEYS_MAX` settings. An `at` after the server's clock is taken
/// as now (R320's way): a fast clock could otherwise win over every later change.
pub fn read_groups(body: &Value, now: i64) -> Result<IndexMap<String, PlayerSettingsGroup>, ApiError> {
    let Some(raw) = body
        .get("groups")
        .filter(|raw| (raw).is_object())
        .and_then(Value::as_object)
    else {
        return Err(bad_request("\"groups\" must be an object of setting groups"));
    };
    if raw.len() > PLAYER_SETTINGS_GROUPS_MAX {
        return Err(bad_request(format!(
            "at most {PLAYER_SETTINGS_GROUPS_MAX} setting groups can be saved"
        )));
    }
    let mut groups: IndexMap<String, PlayerSettingsGroup> = IndexMap::new();
    for (id, group) in raw {
        if utf16_len(id) > PLAYER_SETTINGS_NAME_MAX_LENGTH || !is_group_id_shape(id) {
            return Err(bad_request(format!(
                "every group id must be a lower-case slug of at most {PLAYER_SETTINGS_NAME_MAX_LENGTH} characters"
            )));
        }
        let values_raw = group
            .get("values")
            .filter(|values| (values).is_object())
            .and_then(Value::as_object);
        let Some(values_raw) = values_raw.filter(|_| (group).is_object()) else {
            return Err(bad_request(format!(
                "\"{id}\" must be {{ at: epoch milliseconds, values: an object }}"
            )));
        };
        let Some(at) = epoch_ms_of(group.get("at")) else {
            return Err(bad_request(format!(
                "\"{id}.at\" must be a whole number of epoch milliseconds"
            )));
        };
        if values_raw.len() > PLAYER_SETTINGS_KEYS_MAX {
            return Err(bad_request(format!(
                "\"{id}\" can hold at most {PLAYER_SETTINGS_KEYS_MAX} settings"
            )));
        }
        let mut values: IndexMap<String, PlayerSettingValue> = IndexMap::new();
        for (name, value) in values_raw {
            if utf16_len(name) > PLAYER_SETTINGS_NAME_MAX_LENGTH || !is_setting_name_shape(name) {
                return Err(bad_request(format!(
                    "every setting name must be letters and digits, at most {PLAYER_SETTINGS_NAME_MAX_LENGTH} characters"
                )));
            }
            values.insert(name.clone(), read_value(id, name, value)?);
        }
        groups.insert(
            id.clone(),
            PlayerSettingsGroup {
                at: at.min(now),
                values,
            },
        );
    }
    Ok(groups)
}

// `GET` and `PUT /api/settings` are two rows of `app.rs`'s `ROUTES`, both `AuthLevel::Active`, so a
// pending account gets 403 from each (§9.4).

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
            &PlayerSettingsMergeInput {
                profile_id: profile.id.clone(),
                groups,
                at: now,
            },
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
