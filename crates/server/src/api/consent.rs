//! The age screen and the terms a player agrees to (SPEC §9.4, R757000, R757001; issue #757).
//!
//! `POST /api/account/consent` (`AuthLevel::User`): `{ termsVersion, birthYear?, birthMonth? }`. The
//! client asks; the server decides (CLAUDE.md rule 7):
//!
//! - **Age (R757000).** A pending account that has not passed the age check owes one. The server
//!   counts the age from the birth month and year in UTC and keeps only that the check passed, when
//!   and against which minimum, never the birth date. An answer under `MIN_ACCOUNT_AGE_YEARS`
//!   deletes the account the way `DELETE /api/account` does and answers 403 `under_age`.
//! - **Terms (R757001).** Every account owes agreement to `TERMS_VERSION`, once per version. A
//!   client that sends another version read other terms, and is told to reload (409
//!   `update_required`).
//!
//! `GET /api/auth/me` carries what is owed (`ageCheckOwed`, `termsOwed`), and `POST
//! /api/codes/redeem` refuses an account owing either (`consent_owed`) before it logs an attempt.
//! The request body is never logged: it holds a birth month and year.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::auth::remove_account;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json, now_ms};
use crate::app::App;
use crate::config::{BIRTH_YEAR_EARLIEST, MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION};
use crate::db::store::{ConsentRow, Profile, ProfileStatus, TermsSource};

// Not in SPEC: wording only. The refusal names no age (R757000), like the sign-up screen's.
const UNDER_AGE_MESSAGE: &str = "We can't create an account for you.";
const TERMS_CHANGED_MESSAGE: &str = "The terms changed. Reload the page to read them.";
const BIRTH_REQUIRED_MESSAGE: &str = "Choose the month and year you were born.";
const NANOS_PER_MS: i128 = 1_000_000;

/// The UTC year and month (1–12) at `now_ms`, epoch milliseconds.
pub fn utc_year_month(now_ms: i64) -> (i32, i32) {
    let at = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(now_ms) * NANOS_PER_MS)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    (at.year(), i32::from(u8::from(at.month())))
}

/// R757000: whole years from a birth month to `(year, month)`, counting the birth month itself as
/// not yet reached: with no day, the birthday may still be to come. The client's `ageInYears`
/// (`apps/web/src/auth/consent.ts`) is the same sum.
pub fn age_in_years(birth_year: i32, birth_month: i32, year: i32, month: i32) -> i32 {
    year - birth_year - i32::from(month <= birth_month)
}

/// R757000: a pending account that has not passed the age check. An account that was active
/// before the check existed is not asked.
pub fn age_check_owed(profile: &Profile, consent: &ConsentRow) -> bool {
    matches!(profile.status, ProfileStatus::Pending) && consent.age_checked_at.is_none()
}

/// R757001: the account has not agreed to the terms in force.
pub fn terms_owed(consent: &ConsentRow) -> bool {
    !consent.terms.iter().any(|terms| terms.version == TERMS_VERSION)
}

/// R757000, R757001: either of the two, which redemption refuses.
pub fn consent_owed(profile: &Profile, consent: &ConsentRow) -> bool {
    age_check_owed(profile, consent) || terms_owed(consent)
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(ApiErrorCode::BadRequest, message)
}

/// The birth month and year as sent: both or neither, each a whole number.
fn birth_of(body: &Value) -> Result<Option<(i32, i32)>, ApiError> {
    let number = |key: &str| -> Result<Option<i32>, ApiError> {
        match body.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .map(Some)
                .ok_or_else(|| bad_request(&format!("\"{key}\" must be a whole number"))),
        }
    };
    match (number("birthYear")?, number("birthMonth")?) {
        (Some(year), Some(month)) => Ok(Some((year, month))),
        (None, None) => Ok(None),
        _ => Err(bad_request(BIRTH_REQUIRED_MESSAGE)),
    }
}

/// A birth month that could be one: a month of the year, from `BIRTH_YEAR_EARLIEST`, not after
/// `(year, month)`.
fn birth_is_possible(birth: (i32, i32), (year, month): (i32, i32)) -> bool {
    let (birth_year, birth_month) = birth;
    (BIRTH_YEAR_EARLIEST..=year).contains(&birth_year)
        && (1..=12).contains(&birth_month)
        && (birth_year, birth_month) <= (year, month)
}

/// `POST /api/account/consent` (`AuthLevel::User`): the consent screen's and the sign-up form's
/// answer. 200 `{ ageCheckOwed: false, termsOwed: false }` once both are recorded.
pub async fn post_consent(app: &Arc<App>, req: Req) -> ApiResult {
    let caller = req
        .caller
        .as_ref()
        .ok_or_else(|| ApiError::new(ApiErrorCode::Unauthorized, "sign in first"))?;
    let profile = &caller.profile;
    let version = match req.body.get("termsVersion") {
        Some(Value::String(version)) => version.as_str(),
        _ => return Err(bad_request("\"termsVersion\" must be a string")),
    };
    if version != TERMS_VERSION {
        return Err(ApiError::new(ApiErrorCode::UpdateRequired, TERMS_CHANGED_MESSAGE));
    }
    let birth = birth_of(&req.body)?;
    let now = now_ms();

    // Read and committed at once: the deletion below opens transactions of its own, and the fake
    // store's transaction holds its lock until it ends.
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let consent = tx.consent_get(&profile.id).await?;
    tx.commit().await?;

    let mut age_passed = false;
    if let (None, Some(birth)) = (consent.age_checked_at, birth) {
        let today = utc_year_month(now);
        if !birth_is_possible(birth, today) {
            return Err(bad_request("that month and year of birth cannot be right"));
        }
        if age_in_years(birth.0, birth.1, today.0, today.1) < MIN_ACCOUNT_AGE_YEARS {
            // R757000: nothing is recorded; the account goes as "Delete my account" removes it.
            if let Err(error) = remove_account(app, caller).await {
                // No id in the line, and no birth date: only that a deletion owed did not happen.
                tracing::warn!(event = "consent.under_age.delete_failed", code = %error.code);
            }
            return Err(ApiError::new(ApiErrorCode::UnderAge, UNDER_AGE_MESSAGE));
        }
        age_passed = true;
    }
    if age_check_owed(profile, &consent) && !age_passed {
        return Err(bad_request(BIRTH_REQUIRED_MESSAGE));
    }

    let source = if matches!(profile.status, ProfileStatus::Pending) {
        TermsSource::Signup
    } else {
        TermsSource::Reaccept
    };
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    if age_passed {
        tx.consent_record_age(&profile.id, MIN_ACCOUNT_AGE_YEARS, now)
            .await?;
    }
    tx.consent_record_terms(&profile.id, TERMS_VERSION, source, now)
        .await?;
    tx.commit().await?;
    Ok(json(200, json!({ "ageCheckOwed": false, "termsOwed": false })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r757000_age_counts_the_birth_month_as_not_yet_reached() {
        assert_eq!(age_in_years(2013, 10, 2026, 10), 12);
        assert_eq!(age_in_years(2013, 10, 2026, 11), 13);
        assert_eq!(age_in_years(2013, 10, 2027, 1), 13);
        assert_eq!(age_in_years(2013, 12, 2026, 12), 12);
        assert_eq!(age_in_years(2013, 1, 2026, 2), 13);
    }

    #[test]
    fn r757000_utc_year_month_turns_at_midnight_utc() {
        // 2025-10-01T00:00:00Z.
        assert_eq!(utc_year_month(1_759_276_800_000), (2025, 10));
        assert_eq!(utc_year_month(1_759_276_800_000 - 1), (2025, 9));
    }

    #[test]
    fn r757000_a_birth_must_be_a_past_month_from_the_earliest_year() {
        let today = (2026, 10);
        assert!(birth_is_possible((2026, 10), today));
        assert!(birth_is_possible((BIRTH_YEAR_EARLIEST, 1), today));
        assert!(!birth_is_possible((2026, 11), today));
        assert!(!birth_is_possible((BIRTH_YEAR_EARLIEST - 1, 12), today));
        assert!(!birth_is_possible((2000, 0), today));
        assert!(!birth_is_possible((2000, 13), today));
    }
}
