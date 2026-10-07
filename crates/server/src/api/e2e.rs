//! BUILD M8's `E2E=1` test server: the fixture accounts, the fixture invite codes and R144's reseed
//! at boot. Port of `apps/server/src/api/e2e.ts`; the auth provider that honours the fixture tokens
//! is `crate::auth::E2eAuth` (SURFACE §11.1), built from `E2E_ACCOUNTS` and `auth_user_of` here.
//!
//! BUILD M8's preamble: "Cypress runs against `apps/web` in `E2E=1` mode (hotseat route and a test
//! server with fixture accounts)." `e2e/README.md`'s assumption A6 spells out which accounts:
//! "fixture accounts `e2e-p1`, `e2e-p2` (active, own every card) and `e2e-pending` (pending,
//! verified email); seeded invite codes — one good, one missing, one expired, one exhausted."
//!
//! SPEC §11 R144: "In end-to-end mode the server reseeds its fixture accounts and invite codes at
//! boot, so a spec that activates the pending account or spends an invite code is repeatable.
//! Without this the invite-gate spec passes once and fails on every later run, which is
//! indistinguishable from a regression." Spec 10 does exactly that: it flips `e2e-pending` to
//! active and spends the good code.
//!
//! NOTHING HERE IS REACHABLE IN PRODUCTION. `app.rs` builds these only when `env.e2e` is true, and
//! `env.rs` already refuses `E2E` together with `NODE_ENV=production`:
//!
//!     "E2E: must not be set together with NODE_ENV=production — BUILD M8's fixture accounts and
//!      seeded games must never be reachable in a production deployment."
//!
//! THE VALUES BELOW ARE A CONTRACT WITH `e2e/support/config.ts`, which is the suite's own source of
//! truth and cannot be imported from here. They are transcribed from its defaults. That file also
//! lets CI override every one of them (`--expose p1Token=…`, `--expose goodCode=…`); an override
//! with no matching change here would seed one set and ask for another, so the two move together.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::json;

use crate::api::crypto::{hashes_for_pepper, is_well_formed_code, normalize_code, system_ids};
use crate::api::http::{ApiError, log_info, now_ms};
use crate::app::App;
use crate::auth::{AuthUser, Session};
use crate::config::{INVITE_CODE_LENGTH, RATING_START};
use crate::db::store::{Db, InviteCode, ProfileCreateInput, ProfileStatus};

// ---------------------------------------------------------------------------
// Fixture accounts (`e2e/support/config.ts`, `accounts`)
// ---------------------------------------------------------------------------

/// What `profiles.status` must be once the reseed is done (§9.4).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum E2EAccountStatus {
    Pending,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2EAccount {
    /// The managed-auth user id. `profiles.user_id` is resolved from it, as in production.
    pub user_id: &'static str,
    pub email: &'static str,
    pub password: &'static str,
    /// The static bearer token this account's requests carry.
    pub token: &'static str,
    /// What `profiles.status` must be once the reseed is done (§9.4).
    pub status: E2EAccountStatus,
}

/// §9.4 makes a verified email a precondition of redemption ("reject unless the account is pending
/// with a verified email"), and spec 10 asserts `emailVerified === true` on the *pending* account
/// before it redeems. So all three fixtures are verified; what distinguishes `e2e-pending` is its
/// `profiles.status`, which is the only thing §9.4's gate reads.
pub const E2E_ACCOUNTS: &[E2EAccount] = &[
    E2EAccount {
        user_id: "e2e-p1",
        email: "e2e-p1@jackioh.test",
        password: "e2e-p1-password",
        token: "e2e-token-p1",
        status: E2EAccountStatus::Active,
    },
    E2EAccount {
        user_id: "e2e-p2",
        email: "e2e-p2@jackioh.test",
        password: "e2e-p2-password",
        token: "e2e-token-p2",
        status: E2EAccountStatus::Active,
    },
    E2EAccount {
        user_id: "e2e-pending",
        email: "e2e-pending@jackioh.test",
        password: "e2e-pending-password",
        token: "e2e-token-pending",
        status: E2EAccountStatus::Pending,
    },
];

// ---------------------------------------------------------------------------
// Fixture invite codes (`e2e/support/config.ts`, `inviteCodes`)
// ---------------------------------------------------------------------------

/// One code per §9.4 failure mode plus one that works. `missing` is the one that must NOT exist:
/// seeding it would turn spec 10's "missing" sample into an "already used" sample and the three
/// kinds would stop being the three kinds.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum E2EInviteCodeKind {
    Good,
    Missing,
    Expired,
    Exhausted,
}

impl E2EInviteCodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            E2EInviteCodeKind::Good => "good",
            E2EInviteCodeKind::Missing => "missing",
            E2EInviteCodeKind::Expired => "expired",
            E2EInviteCodeKind::Exhausted => "exhausted",
        }
    }
}

/// TS `Record<E2EInviteCodeKind, string>`: the plaintext of each fixture code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2EInviteCodes {
    pub good: &'static str,
    pub missing: &'static str,
    pub expired: &'static str,
    pub exhausted: &'static str,
}

impl E2EInviteCodes {
    /// TS `codes[kind]`.
    pub fn of(&self, kind: E2EInviteCodeKind) -> &'static str {
        match kind {
            E2EInviteCodeKind::Good => self.good,
            E2EInviteCodeKind::Missing => self.missing,
            E2EInviteCodeKind::Expired => self.expired,
            E2EInviteCodeKind::Exhausted => self.exhausted,
        }
    }

    /// TS `Object.entries(codes)`, in the record's order.
    pub fn entries(&self) -> [(E2EInviteCodeKind, &'static str); 4] {
        [
            (E2EInviteCodeKind::Good, self.good),
            (E2EInviteCodeKind::Missing, self.missing),
            (E2EInviteCodeKind::Expired, self.expired),
            (E2EInviteCodeKind::Exhausted, self.exhausted),
        ]
    }
}

pub const E2E_INVITE_CODES: E2EInviteCodes = E2EInviteCodes {
    good: "ABCD-EFGH-JKMN-PQRS",
    missing: "ZZZZ-ZZZZ-ZZZZ-ZZZZ",
    expired: "XPRD-XPRD-XPRD-XPRD",
    exhausted: "XHST-XHST-XHST-XHST",
};

/// Not in SPEC, and no R-row: a fixture value with no consequence. Any past instant makes the code
/// expired, which is all R144's reseed needs; the rule it exercises is §9.4 step 5's rejection.
const EXPIRED_CODE_AGE_MS: i64 = 60 * 60 * 1000;

/// Not in SPEC, and no R-row: the fixtures take the default rather than choosing. `max_uses` of one
/// matches `DEFAULT_INVITE_CODE_MAX_USES` in `codes.rs` — where the proposed ruling for that default
/// is written out — and the db agent's `max_uses int not null default 1`, so the exhausted fixture
/// is exhausted at one use. Restating the number is all this does; it decides nothing.
const FIXTURE_CODE_MAX_USES: i32 = 1;

// ---------------------------------------------------------------------------
// The auth provider's halves (`crate::auth::E2eAuth` holds the provider itself)
// ---------------------------------------------------------------------------

/// The identity a fixture token verifies as.
pub fn auth_user_of(account: &E2EAccount) -> AuthUser {
    AuthUser {
        user_id: account.user_id.to_string(),
        email: Some(account.email.to_string()),
        // §9.4 step 1's precondition. `app_metadata` stays empty: nothing here is an authorization
        // claim, and `profiles.status` remains the only authority on what an account may do.
        email_verified: true,
        app_metadata: Default::default(),
    }
}

/// The session a fixture email and password sign in to (`E2E=1` only).
pub fn session_of(account: &E2EAccount) -> Session {
    Session {
        access_token: account.token.to_string(),
        refresh_token: None,
        expires_at: None,
        user: auth_user_of(account),
    }
}

// ---------------------------------------------------------------------------
// R144: the reseed
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct E2ESeedSummary {
    /// userId -> the profile id the reseed created, for the boot log.
    pub profiles: IndexMap<String, String>,
    /// How many cards R111's launch grant handed each active fixture.
    pub granted_cards: usize,
    /// The kinds actually written to `invite_codes`; `missing` is never among them.
    pub codes: Vec<E2EInviteCodeKind>,
}

/// TS `seedE2EFixtures`'s `options`: the fixtures to seed in place of the contract's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2ESeedOptions {
    pub accounts: &'static [E2EAccount],
    pub codes: E2EInviteCodes,
}

impl Default for E2ESeedOptions {
    fn default() -> E2ESeedOptions {
        E2ESeedOptions { accounts: E2E_ACCOUNTS, codes: E2E_INVITE_CODES }
    }
}

/// R144 with the contract's fixtures: what `app.rs` runs at boot under `E2E=1`, before the port
/// opens, so no request can land on half a fixture set and so spec 10 is repeatable run after run.
pub async fn seed_e2e_fixtures(app: &App) -> Result<E2ESeedSummary, ApiError> {
    seed_e2e_fixtures_with(app, E2ESeedOptions::default()).await
}

/// R144. Wipes every row and rewrites the fixtures, so running it twice leaves exactly the state
/// running it once does: `e2e-pending` pending again, the good code unspent again, spec 10
/// repeatable.
///
/// The accounts are created `pending` and then flipped with `profiles_set_status`, which is where
/// the in-memory store carries R111's `pending → active` trigger — so "own every card" is produced
/// by the same grant a redemption produces, not by a seeding shortcut that could disagree with it.
pub async fn seed_e2e_fixtures_with(app: &App, options: E2ESeedOptions) -> Result<E2ESeedSummary, ApiError> {
    let accounts = options.accounts;
    let codes = options.codes;
    let now = now_ms();

    assert_fixture_codes_are_redeemable(&codes)?;

    let Db::Fake(store) = &app.db else {
        return Err(ApiError::internal("R144's reseed needs the in-memory store; this server holds Postgres"));
    };
    store.lock().await.reset();

    let mut profiles: IndexMap<String, String> = IndexMap::new();
    let mut granted_cards = 0;
    for account in accounts {
        let mut tx = app.db.begin(None).await?;
        let profile = tx
            .profiles_create(&ProfileCreateInput {
                user_id: account.user_id.to_string(),
                email: account.email.to_string(),
                rating: f64::from(RATING_START),
                at: now,
                display_name: None,
            })
            .await?;
        if account.status == E2EAccountStatus::Active {
            tx.profiles_set_status(&profile.id, ProfileStatus::Active).await?;
        }
        tx.commit().await?;
        if account.status == E2EAccountStatus::Active {
            granted_cards = store.lock().await.grants_for(&profile.id).len();
        }
        profiles.insert(account.user_id.to_string(), profile.id.clone());
    }

    let mut written: Vec<E2EInviteCodeKind> = Vec::new();
    for kind in [E2EInviteCodeKind::Good, E2EInviteCodeKind::Expired, E2EInviteCodeKind::Exhausted] {
        let code = invite_code_for(app, kind, codes.of(kind), now);
        let mut tx = app.db.begin(None).await?;
        tx.codes_insert(&code).await?;
        tx.commit().await?;
        written.push(kind);
    }

    log_info(
        "e2e.fixtures.seeded",
        json!({
            "profiles": profiles,
            "grantedCards": granted_cards,
            "codes": written,
            // The plaintext of a fixture code is not a secret — it is checked into
            // `e2e/support/config.ts` — but it is still never logged, so nothing teaches an operator
            // that logging one is normal.
            "missingCodeIsAbsent": true,
        }),
    );

    Ok(E2ESeedSummary { profiles, granted_cards, codes: written })
}

fn invite_code_for(app: &App, kind: E2EInviteCodeKind, plain: &str, now: i64) -> InviteCode {
    // §9.4: "stored hashed". `Hashes::code` normalises (upper case, separators dropped) before
    // hashing, exactly as redemption in codes.rs does on the way in, so a typed
    // `abcd efgh jkmn pqrs` finds the same row as `ABCD-EFGH-JKMN-PQRS`.
    let base = InviteCode {
        id: system_ids.uuid(),
        code_hash: hashes_for_pepper(&app.env.code_pepper).code(plain),
        max_uses: FIXTURE_CODE_MAX_USES,
        uses: 0,
        revoked: false,
        expires_at: None,
        created_at: now,
    };
    match kind {
        E2EInviteCodeKind::Expired => InviteCode { expires_at: Some(now - EXPIRED_CODE_AGE_MS), ..base },
        E2EInviteCodeKind::Exhausted => InviteCode { uses: FIXTURE_CODE_MAX_USES, ..base },
        E2EInviteCodeKind::Good | E2EInviteCodeKind::Missing => base,
    }
}

/// A fixture code that `codes.rs` would reject as *malformed* would still answer §9.4's identical
/// error, so spec 10 would stay green while testing nothing — the good code included. Checked at
/// boot, loudly, against the same predicate redemption uses.
fn assert_fixture_codes_are_redeemable(codes: &E2EInviteCodes) -> Result<(), ApiError> {
    let mut seen: IndexMap<String, E2EInviteCodeKind> = IndexMap::new();
    for (kind, plain) in codes.entries() {
        let normalized = normalize_code(plain);
        if !is_well_formed_code(&normalized, INVITE_CODE_LENGTH as usize) {
            return Err(ApiError::internal(format!(
                "the {} end-to-end invite code is not a code §9.4 would mint: {} characters from CODE_ALPHABET (R104) are required",
                kind.as_str(),
                INVITE_CODE_LENGTH
            )));
        }
        if let Some(clash) = seen.get(&normalized) {
            return Err(ApiError::internal(format!(
                "the {} and {} end-to-end invite codes are the same code; spec 10 needs four distinct ones",
                kind.as_str(),
                clash.as_str()
            )));
        }
        seen.insert(normalized, kind);
    }
    Ok(())
}
