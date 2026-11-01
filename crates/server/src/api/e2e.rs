//! BUILD M8's `E2E=1` test server: the fixture accounts and invite codes and R144's reseed, at boot
//! and on `POST /api/e2e/reseed`; `crate::auth::E2eAuth` (SURFACE §11.1), built from `E2E_ACCOUNTS`
//! here, honors the fixture tokens. `e2e-p1` and `e2e-p2` are active and own every card (a
//! redemption grants only the starter cards, R1310000, so they are granted the rest), `e2e-pending`
//! is pending, and R1310004's `e2e-econ` is a dev account in end-to-end mode alone ([`is_e2e_dev`]).
//!
//! SPEC §11 R144: the reseed makes a spec that activates the pending account or spends a code
//! repeatable against one server with no restart. The values below are a contract with
//! `e2e/support/config.ts`, transcribed from its defaults; CI may override them there, so the two
//! move together. Nothing here is reachable in production: `app.rs` builds it only under `env.e2e`,
//! and `env.rs` refuses `E2E` with `NODE_ENV=production`:
//! ```text
//! "E2E: must not be set together with NODE_ENV=production — BUILD M8's fixture accounts and
//!  seeded games must never be reachable in a production deployment."
//! ```

use std::sync::Arc;

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::json;

use crate::api::collection::grant_entire_catalog;
use crate::api::crypto::{hashes_for_pepper, is_well_formed_code, normalize_code, system_ids};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, log_info, now_ms, ok_of};
use crate::app::App;
use crate::config::{INVITE_CODE_LENGTH, MIN_ACCOUNT_AGE_YEARS, RATING_START, TERMS_VERSION};
use crate::db::store::{Db, InviteCode, ProfileCreateInput, ProfileStatus, TermsSource};

// Fixture accounts (`e2e/support/config.ts`, `accounts`)

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
    /// R1435: the fixed username the reseed gives the account, bare, with the prompt after
    /// activation already answered, so no end-to-end spec meets the prompt.
    pub username: &'static str,
    /// R1310004: whether the server counts the account as a dev account (§14.10) in end-to-end
    /// mode, as `DEV_PROFILE_IDS` would list it: a fixture's profile id is drawn afresh at every
    /// reseed, so no list could name it.
    pub dev: bool,
}

/// §9.4 makes a verified email a precondition of redemption, and spec 10 asserts
/// `emailVerified === true` on the pending account before it redeems, so every fixture is verified;
/// `profiles.status`, the only thing §9.4's gate reads, tells them apart.
pub const E2E_ACCOUNTS: &[E2EAccount] = &[
    E2EAccount {
        user_id: "e2e-p1",
        email: "e2e-p1@jackioh.test",
        password: "e2e-p1-password",
        token: "e2e-token-p1",
        status: E2EAccountStatus::Active,
        username: "e2e_p1",
        dev: false,
    },
    E2EAccount {
        user_id: "e2e-p2",
        email: "e2e-p2@jackioh.test",
        password: "e2e-p2-password",
        token: "e2e-token-p2",
        status: E2EAccountStatus::Active,
        username: "e2e_p2",
        dev: false,
    },
    E2EAccount {
        user_id: "e2e-pending",
        email: "e2e-pending@jackioh.test",
        password: "e2e-pending-password",
        token: "e2e-token-pending",
        status: E2EAccountStatus::Pending,
        username: "e2e_pending",
        dev: false,
    },
    // R1310004: the economy loop's (spec 45), which redeems the good code for the starter cards
    // (R1310000) and moves its own bars with the dev tools (R1310003).
    E2EAccount {
        user_id: "e2e-econ",
        email: "e2e-econ@jackioh.test",
        password: "e2e-econ-password",
        token: "e2e-token-econ",
        status: E2EAccountStatus::Pending,
        username: "e2e_econ",
        dev: true,
    },
];

/// R1310004: whether `user_id` is a fixture account the end-to-end server counts as a dev account.
/// `api::dev::is_dev` asks only when `env.e2e` is set, which `env.rs` refuses in production.
pub fn is_e2e_dev(user_id: &str) -> bool {
    E2E_ACCOUNTS
        .iter()
        .any(|account| account.dev && account.user_id == user_id)
}

// Fixture invite codes (`e2e/support/config.ts`, `inviteCodes`)

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

/// Plaintext of each fixture code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2EInviteCodes {
    pub good: &'static str,
    pub missing: &'static str,
    pub expired: &'static str,
    pub exhausted: &'static str,
}

impl E2EInviteCodes {
    /// Code for `kind`.
    pub fn of(&self, kind: E2EInviteCodeKind) -> &'static str {
        match kind {
            E2EInviteCodeKind::Good => self.good,
            E2EInviteCodeKind::Missing => self.missing,
            E2EInviteCodeKind::Expired => self.expired,
            E2EInviteCodeKind::Exhausted => self.exhausted,
        }
    }

    /// Entries in kind order.
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

/// R161's default, which the fixtures take rather than choosing: `max_uses` of one matches
/// `DEFAULT_INVITE_CODE_MAX_USES` in `codes.rs` and the db agent's `max_uses int not null default 1`,
/// so the exhausted fixture is exhausted at one use.
const FIXTURE_CODE_MAX_USES: i64 = 1;

// R144: the reseed

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct E2ESeedSummary {
    /// userId -> the profile id the reseed created, for the boot log.
    pub profiles: IndexMap<String, String>,
    /// How many cards each active fixture was granted: R111's starter cards (R1310000) and then the
    /// rest of the catalog (R1310004), every card.
    pub granted_cards: usize,
    /// The kinds actually written to `invite_codes`; `missing` is never among them.
    pub codes: Vec<E2EInviteCodeKind>,
}

/// Fixtures to seed in place of defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2ESeedOptions {
    pub accounts: &'static [E2EAccount],
    pub codes: E2EInviteCodes,
}

impl Default for E2ESeedOptions {
    fn default() -> E2ESeedOptions {
        E2ESeedOptions {
            accounts: E2E_ACCOUNTS,
            codes: E2E_INVITE_CODES,
        }
    }
}

/// R144 with the contract's fixtures: what `app.rs` runs at boot under `E2E=1` (and `post_reseed` on
/// demand), before the port opens, so no request can land on half a fixture set and so spec 10 is
/// repeatable run after run.
pub async fn seed_e2e_fixtures(app: &App) -> Result<E2ESeedSummary, ApiError> {
    seed_e2e_fixtures_with(app, E2ESeedOptions::default()).await
}

/// R144. Wipes every row and rewrites the fixtures, so running it twice leaves the state running it
/// once does. Accounts are created `pending` and flipped with `profiles_set_status`, where the
/// in-memory store carries R111's trigger, so an active fixture holds a redemption's starter grant
/// (R1310000); R1310004 then grants it the rest of the catalog.
pub async fn seed_e2e_fixtures_with(app: &App, options: E2ESeedOptions) -> Result<E2ESeedSummary, ApiError> {
    let accounts = options.accounts;
    let codes = options.codes;
    let now = now_ms();

    assert_fixture_codes_are_redeemable(&codes)?;

    let Db::Fake(store) = &app.db else {
        return Err(ApiError::internal(
            "R144's reseed needs the in-memory store; this server holds Postgres",
        ));
    };
    store.lock().await.reset();
    // R1523000: the wipe took the seat epoch and every seat with it, so the ledger boots again on
    // the empty store, as a restart over it would.
    crate::seats::boot(app).await?;

    let mut profiles: IndexMap<String, String> = IndexMap::new();
    let mut granted_cards = 0;
    for account in accounts {
        let mut tx = app.db.begin(None).await?;
        let profile = tx
            .profiles_create(&ProfileCreateInput {
                user_id: account.user_id.to_string(),
                email: account.email.to_string(),
                rating: RATING_START,
                at: now,
            })
            .await?;
        if account.status == E2EAccountStatus::Active {
            tx.profiles_set_status(&profile.id, ProfileStatus::Active).await?;
        }
        // R757000, R757001: each fixture signed up through the age screen and the terms box, so no
        // spec meets the consent screen unless it asks for it.
        tx.consent_record_age(&profile.id, MIN_ACCOUNT_AGE_YEARS, now)
            .await?;
        tx.consent_record_terms(&profile.id, TERMS_VERSION, TermsSource::Signup, now)
            .await?;
        tx.commit().await?;
        // R1435: a fixed name and the prompt answered, as `seed-accounts` names its accounts.
        store.lock().await.name_fixture(&profile.id, account.username);
        if account.status == E2EAccountStatus::Active {
            // R1310004: every card, past the starter cards the activation granted.
            grant_entire_catalog(app, &profile.id, None).await?;
            granted_cards = store.lock().await.grants_for(&profile.id).len();
        }
        profiles.insert(account.user_id.to_string(), profile.id.clone());
    }

    let mut written: Vec<E2EInviteCodeKind> = Vec::new();
    for kind in [
        E2EInviteCodeKind::Good,
        E2EInviteCodeKind::Expired,
        E2EInviteCodeKind::Exhausted,
    ] {
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

    Ok(E2ESeedSummary {
        profiles,
        granted_cards,
        codes: written,
    })
}

// R144: the reseed on demand

/// The reseed's path. A contract with `e2e/support/config.ts`'s `E2E_RESEED_PATH`, which spec 10
/// posts to, like the fixtures above.
pub const E2E_RESEED_PATH: &str = "/api/e2e/reseed";

/// `POST /api/e2e/reseed` (`AuthLevel::None`): R144's reseed on demand, answering the
/// `E2ESeedSummary`. Outside end-to-end mode it answers what `api::http::dispatch` answers for a
/// path nobody serves, so a production server neither runs it nor shows that it exists. It wipes
/// every row: the specs run one at a time, and spec 10 calls it before any request of its own.
pub async fn post_reseed(app: &Arc<App>, _req: Req) -> ApiResult {
    if !app.env.e2e {
        return Err(ApiError::new(ApiErrorCode::NotFound, "no such endpoint"));
    }
    ok_of(&seed_e2e_fixtures(app).await?)
}

fn invite_code_for(app: &App, kind: E2EInviteCodeKind, plain: &str, now: i64) -> InviteCode {
    // §9.4: "stored hashed". `Hashes::code` normalizes (upper case, separators dropped) before
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
        E2EInviteCodeKind::Expired => InviteCode {
            expires_at: Some(now - EXPIRED_CODE_AGE_MS),
            ..base
        },
        E2EInviteCodeKind::Exhausted => InviteCode {
            uses: FIXTURE_CODE_MAX_USES,
            ..base
        },
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
        if !is_well_formed_code(&normalized, INVITE_CODE_LENGTH) {
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
