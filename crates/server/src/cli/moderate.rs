//! Operator levers onto the server's abuse controls (SPEC §9.4): ban an account, revoke an
//! invite code, pause redemption.
//!
//! ```text
//! jackioh-server moderate ban <profile-id>
//! jackioh-server moderate unban <profile-id>
//! jackioh-server moderate revoke-code <CODE>
//! jackioh-server moderate redemption <on|off>
//! ```
//!
//! The server enforces every abuse control and, until this tool, nothing invoked any of them:
//! `ProfileStatus::Banned` refuses every `Active` route and the `/ws/match` upgrade
//! (`api::http::assert_active`), `invite_codes.revoked_at` gates step 5's lookup, and
//! `app.settings.redemption_enabled` is §9.4's database-side circuit breaker. The `invite-clusters`
//! watch and the rejection-rate alert produce the signal; these are the levers that act on it.
//!
//! Each lever is idempotent and prints what changed; an unknown profile or code is refused
//! without writing anything. A ban applies at once even mid-match: new requests and upgrades are
//! refused through `assert_active`, while the running actor is untouched and the reaper owns the
//! in-flight game (the account-deletion posture). Unbanning refuses a pending account without
//! writing: activating it would skip the invite gate and R111's launch grant, which rides the
//! redemption's own `pending → active` transition.
//!
//! Why this reads the whole environment through `load_env()` when it only needs two variables:
//! the code hash revoked here has to be byte-identical to the one the running server computes
//! when it redeems, and `load_env` plus the `{CODE_PEPPER}:code` derivation is the single
//! contract that guarantees it (`cli/mint_code.rs`). Stdout is the audit trail: the CLI has no
//! log pipeline, so the line it prints is the record of the lever.
//!
//! Like `mint-code`, banning a never-activated account then unbanning it leaves it active without
//! R111's launch collection (the trigger only sees `pending → active`): operators ban pending
//! accounts only to stop abuse.

use anyhow::{anyhow, bail};
use indexmap::IndexMap;
use sqlx::postgres::PgPoolOptions;

use crate::api::codes::code_hash;
use crate::api::crypto::canonical_invite_code;
use crate::db::store::{Db, ProfileStatus};
use crate::env::{load_env, quoted};

const USAGE: &str = "Usage: jackioh-server moderate <command> [arguments]
       (from a checkout: cargo run --release -p jackioh-server -- moderate <command> [arguments])

  ban <profile-id>      Ban the account: its next Active request answers account_banned
                        and its /ws/match upgrade is refused. Idempotent.
  unban <profile-id>    Restore a banned account to active. Refuses a pending account.
  revoke-code <CODE>    Revoke the invite code, which then redeems exactly as an
                        exhausted one. Idempotent.
  redemption <on|off>   Pause (off) or resume (on) redemption through §9.4's
                        database-side circuit breaker. Pausing touches no account
                        or session. Idempotent.

Prints what changed. An unknown profile or code is refused without writing anything.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerateCommand {
    Ban { profile_id: String },
    Unban { profile_id: String },
    RevokeCode { code: String },
    Redemption { enabled: bool },
}

/// The verb and its arguments after `moderate` (`main.rs`), refusing anything else with the usage.
pub fn parse_moderate_args(argv: &[String]) -> anyhow::Result<ModerateCommand> {
    let Some((verb, rest)) = argv.split_first() else {
        bail!("Moderate needs a command.\n\n{USAGE}");
    };
    match verb.as_str() {
        "ban" | "unban" => {
            let [profile_id] = rest else {
                bail!("{verb} takes exactly one profile id.\n\n{USAGE}");
            };
            if profile_id.is_empty() {
                bail!(
                    "{verb} takes exactly one profile id (got {}).\n\n{USAGE}",
                    quoted(profile_id)
                );
            }
            if verb == "ban" {
                Ok(ModerateCommand::Ban {
                    profile_id: profile_id.clone(),
                })
            } else {
                Ok(ModerateCommand::Unban {
                    profile_id: profile_id.clone(),
                })
            }
        }
        "revoke-code" => {
            let [code] = rest else {
                bail!("revoke-code takes exactly one code.\n\n{USAGE}");
            };
            if code.is_empty() {
                bail!(
                    "revoke-code takes exactly one code (got {}).\n\n{USAGE}",
                    quoted(code)
                );
            }
            Ok(ModerateCommand::RevokeCode { code: code.clone() })
        }
        "redemption" => {
            let [state] = rest else {
                bail!("redemption takes exactly one of on, off.\n\n{USAGE}");
            };
            match state.as_str() {
                "on" => Ok(ModerateCommand::Redemption { enabled: true }),
                "off" => Ok(ModerateCommand::Redemption { enabled: false }),
                _ => bail!(
                    "redemption takes exactly one of on, off (got {}).\n\n{USAGE}",
                    quoted(state)
                ),
            }
        }
        _ => bail!("Unrecognised moderate command {}.\n\n{USAGE}", quoted(verb)),
    }
}

/// Bans the profile through `profiles_set_status`: its next `Active` request answers
/// `account_banned` and its `/ws/match` upgrade is refused. Answers the stdout line.
pub async fn ban_profile(db: &Db, profile_id: &str) -> anyhow::Result<String> {
    let mut tx = db.begin(Some(profile_id)).await?;
    let profile = tx.profiles_get_by_id(profile_id).await?;
    let Some(profile) = profile else {
        bail!("no such profile {profile_id}");
    };
    if profile.status == ProfileStatus::Banned {
        tx.commit().await?;
        return Ok(format!("already banned {profile_id}"));
    }
    tx.profiles_set_status(profile_id, ProfileStatus::Banned)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    tx.commit().await?;
    Ok(format!("banned {profile_id}"))
}

/// Restores a banned profile to active. A pending account is refused without writing: unbanning
/// it would activate an account that never redeemed, skipping the gate and R111's launch grant.
/// Answers the stdout line.
pub async fn unban_profile(db: &Db, profile_id: &str) -> anyhow::Result<String> {
    let mut tx = db.begin(Some(profile_id)).await?;
    let profile = tx.profiles_get_by_id(profile_id).await?;
    let Some(profile) = profile else {
        bail!("no such profile {profile_id}");
    };
    if profile.status == ProfileStatus::Pending {
        bail!("cannot unban {profile_id}: it never redeemed an invite code (still pending)");
    }
    if profile.status == ProfileStatus::Active {
        tx.commit().await?;
        return Ok(format!("already active {profile_id}"));
    }
    tx.profiles_set_status(profile_id, ProfileStatus::Active)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    tx.commit().await?;
    Ok(format!("unbanned {profile_id}"))
}

/// Revokes the invite code by its hash, so §9.4 step 5 rejects it exactly as an exhausted one
/// (R145's identical error, R107's timing floor: one `identical_code_error` plus the shared
/// `pad_to`). Answers the stdout line, naming the canonical code.
pub async fn revoke_code(db: &Db, pepper: &str, plain: &str) -> anyhow::Result<String> {
    let Some(canonical) = canonical_invite_code(plain) else {
        bail!("not an invite code {}", quoted(plain));
    };
    let hash = code_hash(pepper, &canonical);
    let mut tx = db.begin(None).await?;
    if tx.codes_find_by_hash(&hash).await?.is_none() {
        bail!("unknown code {}", quoted(&canonical));
    }
    let revoked = tx.codes_revoke_by_hash(&hash).await?;
    tx.commit().await?;
    if revoked {
        Ok(format!("revoked {canonical}"))
    } else {
        Ok(format!("already revoked {canonical}"))
    }
}

/// Flips §9.4's database-side kill switch. Pausing refuses new redemptions with the breaker's
/// 503 while touching no account or session; resuming restores them. Answers the stdout line,
/// naming the previous state.
pub async fn set_redemption(db: &Db, enabled: bool) -> anyhow::Result<String> {
    let mut tx = db.begin(None).await?;
    let previous = tx.redemption_set_enabled(enabled).await?;
    tx.commit().await?;
    Ok(match (enabled, previous) {
        (false, true) => "redemption paused".to_string(),
        (false, false) => "redemption already paused".to_string(),
        (true, false) => "redemption resumed".to_string(),
        (true, true) => "redemption already resumed".to_string(),
    })
}

/// `moderate`'s arguments after the subcommand (`main.rs`): parse, read the environment, run one
/// lever over one connection, print what changed, close.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let command = parse_moderate_args(&args)?;
    let source: IndexMap<String, String> = std::env::vars().collect();
    let env = load_env(&source).map_err(|error| anyhow!("{error}"))?;

    // One connection: this process runs a single lever and exits.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&env.database_url)
        .await?;
    let db = Db::Pg(pool.clone());
    let outcome = match &command {
        ModerateCommand::Ban { profile_id } => ban_profile(&db, profile_id).await,
        ModerateCommand::Unban { profile_id } => unban_profile(&db, profile_id).await,
        ModerateCommand::RevokeCode { code } => revoke_code(&db, &env.code_pepper, code).await,
        ModerateCommand::Redemption { enabled } => set_redemption(&db, *enabled).await,
    };
    pool.close().await;
    let line = outcome?;
    println!("{line}");
    Ok(())
}
