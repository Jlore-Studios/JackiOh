//! Daily games (SPEC §9.13, #3727): a 1v1 game by correspondence, a move every 1, 3, 7 or 14 days, over
//! plain HTTPS so a client needs no socket, no in-browser engine and no cookie to play one.
//!
//! - [`games`]: a game's read and its actions, its start, and the list of the caller's games.
//! - [`invites`]: a challenge to a friend and the daily queue's seeks.
//! - [`notify`]: "your move" by email and Web Push, the sweep and the settings.
//! - [`webpush`]: RFC 8291's encryption and RFC 8292's VAPID.
//!
//! Its clock (who holds the move, until when) and what to send are `jackioh_shared`'s, pure
//! (`host::daily`, `notify`); nothing here keeps a game in memory between requests.

pub mod games;
pub mod invites;
pub mod notify;
pub mod webpush;

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use crate::app::now_ms;
use crate::config::PLAYER_EMAIL_API_URL;
use crate::env::Env;

/// The player mail's Resend endpoint and key.
pub struct Mailer {
    pub http: reqwest::Client,
    pub api: String,
    pub key: String,
}

impl std::fmt::Debug for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mailer")
            .field("api", &self.api)
            .finish_non_exhaustive()
    }
}

/// What the daily games keep beside the app: the channels the server was given keys for, and the
/// e2e clock's offset.
pub struct DailyState {
    /// `None` without `PLAYER_RESEND_API_KEY`: no email is sent.
    pub mailer: Option<Mailer>,
    /// `None` without both VAPID keys: no push is sent.
    pub push: Option<webpush::PushKeys>,
    /// The client the pushes go out on. It follows no redirect: a push goes to the push service its
    /// subscription names (`webpush::is_push_service`) and nowhere else (§9.8).
    pub http: reqwest::Client,
    /// BUILD M8's e2e mode only (`POST /api/e2e/daily-clock`): how far the daily clock reads ahead of
    /// the server's, so a spec can watch a move run out.
    offset_ms: AtomicI64,
    /// Whether a sweep runs now ([`DailyState::begin_sweep`]).
    sweeping: AtomicBool,
}

/// A running sweep's hold ([`DailyState::begin_sweep`]), let go when it ends, failed or not.
pub struct SweepHold<'a>(&'a AtomicBool);

impl Drop for SweepHold<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl DailyState {
    pub fn new(mailer: Option<Mailer>, push: Option<webpush::PushKeys>) -> DailyState {
        DailyState {
            mailer,
            push,
            // As `reqwest::Client::new()` builds one, which fails only when TLS cannot start.
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("a client"),
            offset_ms: AtomicI64::new(0),
            sweeping: AtomicBool::new(false),
        }
    }

    /// §9.13: one sweep at a time. The Worker's hourly cron and the loop of a server that stays awake
    /// can land together, and two sweeps would each read the same "last sent" and send each notice
    /// twice. `None` while another runs, whose hold this is not.
    pub fn begin_sweep(&self) -> Option<SweepHold<'_>> {
        self.sweeping
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| SweepHold(&self.sweeping))
    }

    /// The channels `env` gives keys for. A pair of VAPID keys that is no pair is logged and leaves
    /// push off rather than failing the boot. The e2e fixture server sends nothing.
    pub fn from_env(env: &Env) -> DailyState {
        if env.e2e {
            return DailyState::new(None, None);
        }
        let mailer = env.player_email_key.as_ref().map(|key| Mailer {
            http: reqwest::Client::new(),
            api: PLAYER_EMAIL_API_URL.to_string(),
            key: key.clone(),
        });
        let push = match (&env.vapid_private_key, &env.vapid_public_key) {
            (Some(private), Some(public)) => match webpush::PushKeys::from_base64url(private, public) {
                Ok(keys) => Some(keys),
                Err(reason) => {
                    tracing::error!(event = "daily.push.keys_refused", reason = %reason);
                    None
                }
            },
            _ => None,
        };
        DailyState::new(mailer, push)
    }

    /// The daily games' clock: the server's, plus the e2e offset (zero outside e2e).
    pub fn now(&self) -> i64 {
        now_ms() + self.offset_ms.load(Ordering::SeqCst)
    }

    /// e2e only: the daily clock reads `offset_ms` ahead from now on.
    pub fn set_offset(&self, offset_ms: i64) {
        self.offset_ms.store(offset_ms, Ordering::SeqCst);
    }
}
