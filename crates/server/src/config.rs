//! The server half of BUILD §2's constants table (← `apps/server/src/config.ts`, added in M7; SURFACE
//! §11.1). It carries R79 (match lifecycle defaults) plus every other server-side number that SPEC
//! §9 requires but does not itself pin down. Nothing else in `crates/server` hard-codes these values
//! (CLAUDE.md rule 9).
//!
//! Engine constants such as `DECK_SIZE`, `MAX_MANA` etc. are NOT re-declared here — they live in
//! `crates/engine/src/config.rs` and are imported from `jackioh_engine::config` at the call site.
//!
//! `apps/web` reads these values through `apps/web/src/wire/serverConfig.ts`, which
//! `crates/server/tests/export_config.rs` generates from this file (SURFACE §5.1), so they ship to
//! browsers: only public values may ever live here, never a secret (secrets are environment
//! variables, read by `src/env.rs`).
//!
//! Every `SPEC §11 Rnnn` marker below is a value SPEC §9 does not pin down and SPEC §11 now records
//! as a ruling (R104-R112, added per CLAUDE.md rule 3). The comment is the cross-reference between
//! this file and that table.
//!
//! Types (part 18's choice, recorded in `78f131c^:.fullsend/notes/part-18-3.md`): durations, epoch-ms
//! quantities and counts compared with store counts are `i64`; lengths, caps and limits that bound a
//! collection are `usize`; game and ladder quantities are `i32`; ratings and Glicko numbers are
//! `f64`.

use jackioh_engine::wire::CodeFormat;
use serde::Serialize;

// ---------------------------------------------------------------------------------------------
// R79 — match lifecycle defaults (SPEC §9.5, BUILD §2). Exact names per BUILD §2's last
// paragraph; do not rename or re-derive these.
// ---------------------------------------------------------------------------------------------

/// R79: the turn clock belongs to the active player and ends a stalled turn.
pub const TURN_CLOCK_SECONDS: i64 = 75;
/// R79: a prompt held by the non-active player (e.g. a trap firing on the opponent's turn).
pub const PROMPT_CLOCK_SECONDS: i64 = 30;
/// SPEC §11 R268: the mulligan clock. Both seats' mulligans are open at once (R265), so one deadline
/// runs for both, armed when the window opens and never re-armed when one seat answers; on expiry
/// every seat still owing is timed out and keeps its whole hand. Longer than the prompt clock because
/// a mulligan reads a whole opening hand, shorter than the turn clock because nothing is played.
pub const MULLIGAN_CLOCK_SECONDS: i64 = 45;
/// R79: grace window after a disconnect before `disconnectExpired` ends the match as a loss.
pub const DISCONNECT_GRACE_SECONDS: i64 = 60;
/// R79, R389: hard wall-clock ceiling; reaching it ends the match as a draw via `ceilingReached`.
/// Patch v0.2.0 doubled the turn cap to 60 player-turns (B4.3), which at a full turn clock is 75
/// minutes, so the ceiling doubled with it and a slow game ends on the cap, not the clock.
pub const MATCH_CEILING_MINUTES: i64 = 120;
/// R79, §9.5: room codes are 6 characters from the invite-code alphabet.
pub const ROOM_CODE_LENGTH: usize = 6;
/// R79, §9.5: how long an unjoined room stays open, in seconds. Fifteen minutes. TS stated this as
/// the literal `15 * 60 * 1000` in `api/deps.ts`'s `defaultLimits()` (`roomCodeTtlMs`); v0.3.0 names
/// it here (CLAUDE.md rule 9, research C §10 #17), and `api/deps.ts` is not ported.
pub const ROOM_CODE_TTL_SECONDS: i64 = 15 * 60;

// ---------------------------------------------------------------------------------------------
// The code alphabet (§9.4, §9.5).
// ---------------------------------------------------------------------------------------------

// SPEC §9.4: invite codes are "16 characters (80 bits) from a 32-symbol alphabet without
// 0/O/1/I/l, formatted XXXX-XXXX-XXXX-XXXX". SPEC §9.5 (R79): room codes are "6 characters from
// the invite-code alphabet". 32 symbols means log2(32) = 5 bits per character, so 16 characters
// carry exactly 80 bits and 6 characters carry exactly 30 bits.
//
// SPEC §11 R104: the alphabet string itself is not written out in SPEC §9.4; this is the only
// 32-symbol set matching its exclusions. It is built from the 36 uppercase letters+digits minus
// the 4 excluded characters (0, O, 1, I) = 32 symbols: 23456789ABCDEFGHJKLMNPQRSTUVWXYZ.
// The alphabet is uppercase-only, so SPEC's exclusion of lowercase `l` is satisfied by
// normalising any user-entered code to upper case before comparison, rather than by omitting a
// lowercase `l` that could never appear here in the first place.
pub const CODE_ALPHABET: &str = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// §9.4: invite codes are 16 characters long (80 bits over `CODE_ALPHABET`).
pub const INVITE_CODE_LENGTH: usize = 16;
/// §9.4: invite codes are formatted in groups of 4, e.g. XXXX-XXXX-XXXX-XXXX.
pub const INVITE_CODE_GROUP_SIZE: usize = 4;
/// §9.4: the separator between groups in the formatted invite code.
pub const INVITE_CODE_SEPARATOR: &str = "-";

// ---------------------------------------------------------------------------------------------
// How a typed code is read (R191), and the client's sign-in numbers (R192, R194).
// ---------------------------------------------------------------------------------------------

// PUBLIC VALUES ONLY. `apps/web` reads this file's generated copy (the code field, the sign-in
// screens), so everything in it ships to browsers. It holds no secret and must never hold one: a
// secret added here later would be in every client bundle. Secrets live in the environment
// (`src/env.rs`), which the web never reads.

/// SPEC §11 R191: raw code input longer than this is malformed without being read, so config bounds
/// the work a redemption does, not the caller. Four times a formatted invite code, which leaves room
/// for any spacing a person or a mail client adds.
pub const CODE_INPUT_MAX_LENGTH: usize = 64;

/// R191: the shape `jackioh_engine::wire::codes`'s `read_code_input` reads an invite code with (TS:
/// `@jackioh/shared`'s `readCodeInput`; there a plain literal, structurally a `CodeFormat`).
pub const INVITE_CODE_FORMAT: CodeFormat = CodeFormat {
    alphabet: CODE_ALPHABET,
    length: INVITE_CODE_LENGTH,
    group_size: INVITE_CODE_GROUP_SIZE,
    separator: INVITE_CODE_SEPARATOR,
    max_input_length: CODE_INPUT_MAX_LENGTH,
};

/// R191, R79: a room code is one group of `ROOM_CODE_LENGTH` from the same alphabet.
pub const ROOM_CODE_FORMAT: CodeFormat = CodeFormat {
    alphabet: CODE_ALPHABET,
    length: ROOM_CODE_LENGTH,
    group_size: ROOM_CODE_LENGTH,
    separator: INVITE_CODE_SEPARATOR,
    max_input_length: CODE_INPUT_MAX_LENGTH,
};

/// SPEC §11 R190: how many `X-Forwarded-For` entries, counted from the right, the deployment's own
/// proxies wrote, when `TRUSTED_PROXY_HOPS` does not say. Zero: with no proxy in front (a local or
/// E2E server) the header is whatever the caller wrote, and trusting it would let a caller choose its
/// own per-IP bucket. A deployment behind a proxy says so explicitly (`render.yaml` sets 1).
pub const DEFAULT_TRUSTED_PROXY_HOPS: usize = 0;
/// R190: the most hops `TRUSTED_PROXY_HOPS` may name; anything above it is a misconfiguration.
pub const MAX_TRUSTED_PROXY_HOPS: usize = 5;
/// R190: an IPv6 client is counted by its first this-many bits. A /64 is only the least a line is
/// handed: many ISPs delegate a /56 to one home, and a tunnel broker routes a /48 to anyone who asks,
/// so keying on the /64 gave one host 256 (or 65,536) fresh buckets. A /56 is the usual compromise:
/// it closes the home delegation, and the price is that IPv6 neighbours sharing a /56 share a bucket.
pub const IPV6_RATE_LIMIT_PREFIX_BITS: u32 = 56;

/// The largest request body the API reads, in bytes. Every body the API accepts is a small JSON
/// object (a code, three decks of card ids), so 64 KiB is generous; past it the body is refused
/// before it is parsed.
pub const API_MAX_BODY_BYTES: usize = 65_536;

/// The client's sign-up check. Twelve characters, not Supabase Auth's default of six: a six-letter
/// password falls to guessing and credential stuffing. The auth provider's own minimum must be set
/// to the same number in the Supabase dashboard (Authentication > Providers > Email), or it lets a
/// shorter password through a client that does not run this check.
pub const AUTH_PASSWORD_MIN_LENGTH: usize = 12;
/// The auth provider's bcrypt limit, in UTF-8 BYTES (the provider measures a Go string, and bcrypt
/// reads bytes), not characters: a letter outside ASCII takes two to four of them.
pub const AUTH_PASSWORD_MAX_LENGTH: usize = 72;
/// SPEC §11 R192: the provider's per-address email interval, which the resend button waits out.
pub const AUTH_EMAIL_RESEND_COOLDOWN_SECONDS: i64 = 60;
/// SPEC §11 R193: how long this browser remembers the address it signed up with or asked to reset,
/// for comparing an emailed link against. The provider's email links last at most a day (its email
/// OTP expiry is capped there), so an older address could only ever be matched by someone else's.
pub const AUTH_PENDING_ADDRESS_TTL_SECONDS: i64 = 86_400;
/// SPEC §11 R194: a session this close to expiring is renewed before it is used.
pub const AUTH_SESSION_REFRESH_MARGIN_SECONDS: i64 = 60;
/// R194: an open screen's token is renewed ahead of its expiry at most once per this many seconds,
/// so a provider that issues tokens shorter-lived than the margin cannot set off a renewal loop.
pub const AUTH_SESSION_RENEWAL_FLOOR_SECONDS: i64 = 10;
/// R194: how long sign-out waits to renew an expired session so it can revoke it, before it loads
/// the landing page anyway. A live session is revoked without waiting at all.
pub const AUTH_SIGN_OUT_WAIT_SECONDS: i64 = 5;
/// SPEC §11 R194: how long the API remembers that an access token's session is still live at the
/// auth provider. A session the provider has ended (a sign-out, a link's session that was dropped, a
/// password reset that signed other devices out) stops being honoured here within this many seconds,
/// not when its access token expires.
pub const AUTH_SESSION_LIVE_CACHE_SECONDS: i64 = 30;
/// A request to the auth provider that has not answered in this long is a network failure.
pub const AUTH_PROVIDER_TIMEOUT_SECONDS: i64 = 30;
/// A request to this server that has not answered in this long is a network failure. Longer than the
/// provider's, because Render's free tier takes about 50 s to wake a sleeping instance.
pub const API_REQUEST_TIMEOUT_SECONDS: i64 = 75;
/// How long the gate's "Checking your account…" waits before it offers a way out and says why.
pub const GATE_SLOW_NOTICE_SECONDS: i64 = 5;
/// R192: the code screen reads `/api/codes/status` again when a wait it stated runs out, but never
/// sooner than this after the last read, so a wait that has just lapsed cannot make it poll.
pub const CODE_STATUS_RECHECK_FLOOR_SECONDS: i64 = 1;

// ---------------------------------------------------------------------------------------------
// Redemption rate limits (§9.4).
// ---------------------------------------------------------------------------------------------

// SPEC §9.4 step 2: "reject if this profile made more than 5 attempts in the last hour" — the
// limit is exceeded strictly *after* the 5th attempt, so the check must be `attempts > 5`, never
// `attempts >= 5`.
//
// Which attempt is the first refused: the SEVENTH, not the sixth as this comment used to say. The
// count is taken before the attempt is logged (`codes.rs` step 2 runs ahead of step 4), so attempt
// N sees N-1 rows. Attempt 6 sees 5, and 5 is not "more than 5", so it is allowed; attempt 7 sees
// 6 and is refused. That is the spec's sentence read literally, which is what rules here — a
// budget of "5 per hour" that admits 6 tries looks off by one until you notice the count excludes
// the attempt being made.
pub const CODE_ATTEMPTS_PER_PROFILE_PER_HOUR: i64 = 5;
// SPEC §9.4 step 3: "reject if this IP hash made more than 20" — same strictness: `> 20`, not
// `>= 20`.
pub const CODE_ATTEMPTS_PER_IP_PER_HOUR: i64 = 20;
/// §9.4: the rolling window both attempt limits above are counted over, in seconds.
pub const CODE_ATTEMPT_WINDOW_SECONDS: i64 = 3600;

// ---------------------------------------------------------------------------------------------
// Retention: how long the server keeps rows nothing reads any more. Not in SPEC, and no R-row
// (the play telemetry's period is R1442's). The privacy policy must state these same periods.
// ---------------------------------------------------------------------------------------------

/// Days a `code_attempts` row (profile id, peppered IP hash, time) is kept. The limits that read the
/// table look back at most `CODE_ATTEMPT_WINDOW_SECONDS`, so 30 days is far past any of them.
pub const CODE_ATTEMPT_RETENTION_DAYS: i64 = 30;
/// Days a finished match's action log is kept after the match ended. Only a live match is ever
/// replayed from its log; the result row, and so the rating history, is kept.
pub const MATCH_ACTION_RETENTION_DAYS: i64 = 90;
/// R1442: days a finished match's play telemetry (`action_timings`, `emote_events`,
/// `match_signals`, migration 0029) is kept after the match ended. Longer than the log, so the
/// think-time fits (`jackioh-server timing-fit`) have a year of games to draw on.
pub const PLAY_TELEMETRY_RETENTION_DAYS: i64 = 365;
/// How often the retention purge runs. It also runs once at boot, since a free instance sleeps.
pub const RETENTION_PURGE_INTERVAL_SECONDS: i64 = 3600;

// ---------------------------------------------------------------------------------------------
// Constant-time failure (§9.4, BUILD M6-T1).
// ---------------------------------------------------------------------------------------------

// SPEC §9.4: "Missing, expired and exhausted codes return an identical error in identical
// time." BUILD M6-T1's acceptance test wants the three failure responses within 5 ms of each
// other over 50 samples, which this floor is meant to comfortably clear: every redemption
// response (success or failure) is padded, if it finishes early, to take at least this long,
// so the wall-clock time never leaks which of the three checks failed.
//
// SPEC §11 R107: the floor duration itself is not fixed by SPEC §9.4. 250 ms is comfortably above a
// slow round trip to Postgres (steps 1-5 of the §9.4 transaction plus a couple of hash
// comparisons), while still being an unnoticeable delay for a human clicking "redeem".
pub const REDEMPTION_RESPONSE_FLOOR_MS: i64 = 250;

// SPEC §9.4: the single client-facing message for missing, expired and exhausted codes — never
// distinguish between the three in user-visible text or in timing.
pub const REDEMPTION_IDENTICAL_ERROR: &str = "This invite code is invalid.";

// ---------------------------------------------------------------------------------------------
// The circuit breaker (§9.4).
// ---------------------------------------------------------------------------------------------

// SPEC §9.4: "A global circuit breaker disables redemption and alerts when system-wide failures
// cross a threshold in a window" — SPEC fixes neither the threshold nor the window.
//
// SPEC §11 R106: 100 failed redemption attempts (across all profiles and IPs) within 600 s trips
// the breaker. High enough that normal typo/expired-code traffic never trips it, low enough to
// catch a scripted brute-force attempt quickly.
pub const REDEMPTION_CIRCUIT_FAILURE_THRESHOLD: i64 = 100;
/// SPEC §11 R106: the rolling window the circuit breaker counts failures over, in seconds.
pub const REDEMPTION_CIRCUIT_WINDOW_SECONDS: i64 = 600;

// ---------------------------------------------------------------------------------------------
// Matchmaking (§9.5, BUILD M7-T3).
// ---------------------------------------------------------------------------------------------

/// §9.5: the rating window's starting half-width, in rating points, at the moment a ticket enqueues.
pub const RATING_WINDOW_START: i64 = 100;
/// §9.5: the rating window widens by this many rating points every `RATING_WINDOW_WIDEN_EVERY_SECONDS`.
pub const RATING_WINDOW_WIDEN_BY: i64 = 50;
/// §9.5: the cadence, in seconds, at which the rating window widens.
pub const RATING_WINDOW_WIDEN_EVERY_SECONDS: i64 = 10;
/// §9.5: past this many seconds waited, the rating window is uncapped (any opponent qualifies).
pub const RATING_WINDOW_UNCAPPED_AFTER_SECONDS: i64 = 60;

// SPEC §9.5: "Pairing runs on enqueue plus a sweeper every few seconds" — the exact cadence is
// not fixed by SPEC.
//
// SPEC §11 R108: 3 s. Frequent enough that a ticket is rarely left waiting past its next window
// widen before being reconsidered, without hammering the ticket table.
pub const MATCHMAKER_SWEEP_INTERVAL_SECONDS: i64 = 3;

/// §9.5: the half-width of the acceptable rating gap for a ticket that has waited
/// `waited_seconds`. Shared by the pairing query and its tests so both read one implementation.
///
/// At `waited_seconds = 0` this is `RATING_WINDOW_START`; it widens by `RATING_WINDOW_WIDEN_BY`
/// every `RATING_WINDOW_WIDEN_EVERY_SECONDS`, and returns `f64::INFINITY` (TS
/// `Number.POSITIVE_INFINITY`) once `waited_seconds` passes `RATING_WINDOW_UNCAPPED_AFTER_SECONDS`
/// (any opponent qualifies).
pub fn rating_window(waited_seconds: f64) -> f64 {
    if waited_seconds >= RATING_WINDOW_UNCAPPED_AFTER_SECONDS as f64 {
        return f64::INFINITY;
    }
    let clamped_wait = waited_seconds.max(0.0);
    let widen_steps = (clamped_wait / RATING_WINDOW_WIDEN_EVERY_SECONDS as f64).floor();
    RATING_WINDOW_START as f64 + widen_steps * RATING_WINDOW_WIDEN_BY as f64
}

// ---------------------------------------------------------------------------------------------
// The hidden rating: Glicko-2 (SPEC §9.12, R603). The maths is `src/ranked/glicko2.rs`.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R603: the rating a new profile starts at. R79's Elo started at 1000, and Glicko-2's update
/// depends only on rating differences, so the ratings Elo left carry over unchanged as Glicko-2
/// ratings and only the deviation and volatility are new (migration 0022).
pub const RATING_START: f64 = 1000.0;
/// R603: a new profile's rating deviation: Glickman's starting value, the most uncertain rating.
pub const RATING_DEVIATION_START: f64 = 350.0;
/// R603: a new profile's rating volatility: Glickman's starting value.
pub const RATING_VOLATILITY_START: f64 = 0.06;
/// R603: Glicko-2's system constant τ, which bounds how fast volatility moves. Glickman's 0.3–1.2.
pub const GLICKO_TAU: f64 = 0.5;
/// R603: the factor between a displayed rating and Glicko-2's internal scale, 400 / ln 10. Glickman
/// writes it 173.7178; this is the same number to double precision (TS `400 / Math.LN10`).
pub const GLICKO_SCALE: f64 = 400.0 / std::f64::consts::LN_10;
/// R603: the volatility iteration stops once its bracket is narrower than this (Glickman's ε).
pub const GLICKO_CONVERGENCE: f64 = 0.000001;
/// R603: the most volatility iterations one update runs. The Illinois iteration converges in a
/// handful of steps on any real input; the cap only makes a non-finite input end rather than spin.
pub const GLICKO_MAX_ITERATIONS: u32 = 100;

// ---------------------------------------------------------------------------------------------
// The visible ladder (SPEC §9.12, R605–R608). The rules are `src/ranked/ladder.rs`. PUBLIC: the
// client reads the shape (divisions, pips, placements) to draw a rank it is handed.
// ---------------------------------------------------------------------------------------------

/// The shape of `RANK_TIER_PERCENTS` (SURFACE §4.2: a constant object becomes a const of a struct
/// named after it). Serialises as TS's object, keys in TS order.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankTierPercents {
    pub rotten: i32,
    pub normal: i32,
    pub large: i32,
    pub golden: i32,
    pub mythic: i32,
}

/// SPEC §11 R606: the share of active players each Grape tier is meant to hold, in whole percents,
/// lowest first: Rotten 12%, Normal 60%, Large 20%, Golden 7%, Mythic 1%. A player's hidden rating is
/// placed at its percentile among the season's placed players and read off these shares, so retuning
/// a boundary is a change here and nowhere else. Whole numbers, so a boundary is exact; they sum to 100.
pub const RANK_TIER_PERCENTS: RankTierPercents = RankTierPercents {
    rotten: 12,
    normal: 60,
    large: 20,
    golden: 7,
    mythic: 1,
};
/// R605: divisions per Grape tier, III up to I.
pub const RANK_DIVISIONS_PER_TIER: i32 = 3;
/// R605: pips per division. A division is climbed one pip at a time; a full division promotes.
pub const RANK_PIPS_PER_DIVISION: i32 = 3;
/// R605: rated games a season's placements take. Until they are played the player is a Raisin.
pub const RANK_PLACEMENT_GAMES: i32 = 5;
/// R606: pips a win gives before any bonus.
pub const RANK_WIN_PIPS: i32 = 1;
/// R606: pips a loss takes before any adjustment. A draw moves no pip.
pub const RANK_LOSS_PIPS: i32 = 1;
/// R606: the win that makes a streak this long, and every win after it, earns the streak bonus.
pub const RANK_STREAK_LENGTH: i32 = 3;
/// R606: the streak bonus, in pips, earned below Mythic Grape only.
pub const RANK_STREAK_BONUS_PIPS: i32 = 1;
/// R606: how far, in pips, the rank the hidden rating calls for must sit from the visible rank
/// before a game's pips lean toward it. One division: inside it a game is a plain win or loss.
pub const RANK_CONVERGENCE_GAP_PIPS: i32 = 3;
/// R606: how many pips the lean adds — to a win when the rating is above the visible rank, to a loss
/// when it is below. Gentle by design: never more than this, and a win never gives fewer pips than a
/// plain win, nor a loss take fewer than a plain loss.
pub const RANK_CONVERGENCE_PIPS: i32 = 1;
/// R608: Jlorious is the top this-many Mythic Grape players by hidden rating.
pub const JLORIOUS_SIZE: usize = 100;

// ---------------------------------------------------------------------------------------------
// Usernames (SPEC §9.4, R1432–R1435). The checks are `src/username/`.
// ---------------------------------------------------------------------------------------------

/// R1432: the fewest user-perceived characters (extended grapheme clusters) a base name may have.
/// Two, so that two-character Chinese, Japanese and Korean names fit. The `#n` tag never counts.
pub const USERNAME_MIN_LENGTH: usize = 2;
/// R1432: the most user-perceived characters a base name may have.
pub const USERNAME_MAX_LENGTH: usize = 16;
/// R1432: the most combining marks in a row after a letter, enough for the scripts that write
/// vowels and tones as marks (Devanagari, Thai) and too few for stacked "Zalgo" text.
pub const USERNAME_MAX_MARKS: usize = 3;
/// R1432: raw input longer than this many characters (code points) is refused as too long without
/// being normalised, so a huge query string costs nothing to refuse. Far above any real name
/// `USERNAME_MAX_LENGTH` allows: one user-perceived character can hold many code points, a letter
/// with its marks or an Indic conjunct (`स्त्री` is six), so sixteen of them can run past 64.
pub const USERNAME_INPUT_MAX_CHARS: usize = 256;
/// R1435: how long after a change of username the next one is allowed (24 hours).
pub const USERNAME_CHANGE_COOLDOWN_SECONDS: i64 = 86_400;
/// R1435: how long the client waits after the last keystroke before it asks for a preview.
pub const USERNAME_PREVIEW_DEBOUNCE_MS: i64 = 400;
/// R1434: the base name of every new account's default, `Player#n`, always tagged.
pub const USERNAME_DEFAULT_BASE: &str = "Player";

// ---------------------------------------------------------------------------------------------
// Seasons (SPEC §9.12, R609). The reset is `src/ranked/season.rs`.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R609: how far a season's soft reset pulls each rating toward the players' mean: 0 keeps
/// every rating, 1 puts everyone on the mean. Half way.
pub const SEASON_RESET_STRENGTH: f64 = 0.5;
/// R609: the deviation a season's reset adds, combined in quadrature as Glicko adds uncertainty for
/// time away: `min(√(RD² + this²), RATING_DEVIATION_START)`. A settled 60 becomes about 160.
pub const SEASON_RESET_DEVIATION_BOOST: f64 = 150.0;

// ---------------------------------------------------------------------------------------------
// Action flooding (§9.8).
// ---------------------------------------------------------------------------------------------

// SPEC §9.8: "Per-match rate limit in the actor, per-account rate limit at the API" — neither
// number is fixed by SPEC.
//
// SPEC §11 R109: 5 actions/second sustained per match comfortably covers legitimate play (even a
// fast combo turn resolves as a handful of actions) while blocking a scripted flood.
pub const MATCH_ACTIONS_PER_SECOND: usize = 5;
// SPEC §11 R109: 300 requests/minute per account across the API is generous for normal client
// polling and UI use while still bounding a runaway or malicious client.
pub const API_REQUESTS_PER_MINUTE: usize = 300;
/// SPEC §11 R738: the least time between two `aim` frames the actor relays for one seat. Aims that
/// arrive faster are coalesced, never queued: the newest waits out the interval and goes alone, so
/// the opponent's arrow always ends where the sender's aim ended, at most this long behind it.
/// Ten a second follows a hover from target to target and bounds what a scripted client can push.
pub const AIM_RELAY_INTERVAL_MS: i64 = 100;
/// Match sockets one client address may hold at once, counting handshakes still in progress. Not in
/// SPEC, and no R-row. A player needs one socket per match, plus one more for a moment while it
/// reconnects; ten leaves room for several players behind one home or campus address. Past it the
/// upgrade is refused with 429 before the handshake is read.
pub const WS_MAX_CONNECTIONS_PER_ADDRESS: usize = 10;
/// SPEC §11 R1441: how often the server pings a match socket that is attached. Fifteen seconds is well
/// under the 60-second idle limit common proxies hold a quiet connection to, so the ping also keeps
/// the path open. Pings run only while a socket is attached, so an idle server still sleeps.
pub const WS_PING_INTERVAL_SECONDS: i64 = 15;
/// R1441: how long a match socket may send no frame of any kind (a pong counts) before the server
/// drops it, which is a disconnect and starts the seat's grace (§9.5, R79). Three ping intervals, so
/// a slow mobile link that loses a pong or two is not dropped.
pub const WS_IDLE_TIMEOUT_SECONDS: i64 = 45;
/// R1441, §9.8: how many frames a match socket's outgoing queue holds before the socket is closed
/// for not reading. Views are full snapshots, so a reconnect loses nothing, and a reader that keeps
/// up holds only a few.
pub const WS_OUTBOX_MAX_FRAMES: usize = 256;

// ---------------------------------------------------------------------------------------------
// The reaper (§9.5).
// ---------------------------------------------------------------------------------------------

// SPEC §9.5: "a reaper resolves anything past the ceiling" — the polling cadence is not fixed
// by SPEC.
//
// SPEC §11 R108: 30 s. The ceiling itself is a 60-minute wall clock, so a half-minute reaper
// sweep resolves a stuck match promptly without meaningfully scanning `matches` too often.
pub const MATCH_REAPER_INTERVAL_SECONDS: i64 = 30;
/// §9.5's "every ending records a result", under two first writers at once — a live actor and the
/// reaper resolving the same match. The transaction's own `getByMatch` cannot see the other's
/// uncommitted write, so the loser only learns of the race when its insert hits `results_pkey`,
/// and then runs the whole write again: the rerun either returns the row the winner committed or,
/// if the winner rolled back, lands this one. Three losses in a row is a storm, not a race worth
/// waiting out (the same bound R263 gives a series transition's compare-and-set).
pub const RESULT_WRITE_ATTEMPTS: usize = 3;
/// SPEC §11 R1437: how many times the match actor tries to write a finished game's result
/// (`api::results::record_result`) before it lets go of the match. Each try is the whole
/// transaction, so one that fails leaves the match as it was: `live`, both players still in it.
/// Three tries outlast a blip in the store; the rebuild R1437 falls back on outlasts an outage.
pub const MATCH_RECORD_RESULT_ATTEMPTS: usize = 3;
/// SPEC §11 R1437: the wait before the match actor's second try at a result, doubled before each
/// try after it (250 ms, then 500 ms).
pub const MATCH_RECORD_RESULT_BACKOFF_MS: i64 = 250;

// ---------------------------------------------------------------------------------------------
// Glitch (issue #170; SPEC §7, R678, R679).
// ---------------------------------------------------------------------------------------------

/// R678: how many other players' last server boards a match samples as it is created, one per seat,
/// for a Glitch's boards outcome. Fewer when fewer other players have one.
pub const GLITCH_BOARDS_SAMPLED: usize = 2;
/// R679: the WebSocket close code both sockets of a voided match are closed with, beside the reason
/// `MATCH_VOIDED_CLOSE_REASON` (`actor/protocol.rs`). In the 4000–4999 application range, read as
/// HTTP's 410 Gone: the match no longer exists. The last `view` the clients got already shows the
/// game over with reason `voided`.
pub const MATCH_VOIDED_CLOSE_CODE: u16 = 4410;

// ---------------------------------------------------------------------------------------------
// Saved decks and trios (SPEC §9.4, R250–R256). PUBLIC: the deck builder imports these too.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R250: how many named decks a profile may save. Mirrored in `app.settings` (0007).
pub const MAX_SAVED_DECKS: usize = 10;
/// SPEC §11 R252: how many trios a profile may save. Mirrored in `app.settings` (0007).
pub const MAX_SAVED_TRIOS: usize = 5;
/// SPEC §11 R250, R252: the longest deck or trio name, in characters once trimmed. Long enough for
/// "Midrange Humans (anti-aggro)", short enough for a tab, a list row and a sentence in a message.
/// Mirrored in `app.settings` (0007).
pub const DECK_NAME_MAX_LENGTH: usize = 40;
/// SPEC §11 R250: the most draft issues one refused save lists in its `details`. A real deck of at
/// most `DECK_SIZE` cards can break D1–D4 about forty times; a request body of junk ids could break
/// D3 thousands of times and turn a 64 KiB request into a far larger answer. The first issue is
/// still the error's message, so nothing a player could fix is hidden.
pub const DRAFT_ISSUES_REPORTED_MAX: usize = 50;
/// SPEC §11 R255: the deck-code format's version. Version 2 (patch v0.2.0, docs/classic-sets.md B2.2)
/// writes each card's set with its number (`CATALOG_NUMBER_SET_OFFSETS`); a code naming a version
/// other than this one or `DECK_CODE_CORE_ONLY_VERSION` is refused.
pub const DECK_CODE_VERSION: i32 = 2;
/// SPEC §11 R255: the one older deck-code version still read. Every code minted before patch v0.2.0
/// is a version 1 code, and its numbers are Core's (the only set there was).
pub const DECK_CODE_CORE_ONLY_VERSION: i32 = 1;

/// The shape of `CATALOG_NUMBER_SET_OFFSETS` (SURFACE §4.2), keyed by set name exactly as TS's object
/// is (`"Core"`, `"Classic"`, `"Classic+"`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogNumberSetOffsets {
    #[serde(rename = "Core")]
    pub core: i32,
    #[serde(rename = "Classic")]
    pub classic: i32,
    #[serde(rename = "Classic+")]
    pub classic_plus: i32,
}

/// SPEC §11 R255, R339 (B2.2): a card's catalog number in a version 2 deck or trio code is its §5
/// index plus its set's offset — Core n, Classic 1000 + n, Classic+ 2000 + n — still written as
/// LEB128. No set holds 1000 cards, so a number names one set and one card.
pub const CATALOG_NUMBER_SET_OFFSETS: CatalogNumberSetOffsets = CatalogNumberSetOffsets {
    core: 0,
    classic: 1000,
    classic_plus: 2000,
};
/// SPEC §11 R255: raw deck-code input longer than this is refused before it is read. A v1 code for
/// a full deck with the longest name is under 200 characters, so this leaves room for whatever a
/// chat client wraps around a pasted code.
pub const DECK_CODE_MAX_INPUT_LENGTH: usize = 512;
/// SPEC §11 R256: how long the builder waits after the last edit before it saves.
pub const DECK_AUTOSAVE_DEBOUNCE_MS: i64 = 800;
/// SPEC §11 R256: how long the builder waits before it tries a failed save again.
pub const DECK_AUTOSAVE_RETRY_SECONDS: i64 = 5;
/// SPEC §11 R339: the trio-code format's version. Version 2 writes each deck as a version 2 deck code
/// does (`CATALOG_NUMBER_SET_OFFSETS`); a code naming a version other than this one or
/// `TRIO_CODE_CORE_ONLY_VERSION` is refused.
pub const TRIO_CODE_VERSION: i32 = 2;
/// SPEC §11 R339: the one older trio-code version still read, whose decks carry Core numbers.
pub const TRIO_CODE_CORE_ONLY_VERSION: i32 = 1;
/// SPEC §11 R339: raw trio-code input longer than this is refused before it is read. A trio code
/// carries three decks' names and cards and the trio's name: about 200 characters for ASCII names,
/// and under 1,100 at the very worst (four-byte characters filling every name to its limit), so this
/// leaves room for whatever a chat client wraps around a pasted code.
pub const TRIO_CODE_MAX_INPUT_LENGTH: usize = 2048;

// ---------------------------------------------------------------------------------------------
// Queue modes and the Conquest series (SPEC §9.5, R257–R264, R330–R338).
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R330: game wins that take a Conquest series — one with each deck of a trio, since a deck
/// that has won is locked. `tests/api/series_rules.rs` asserts it equals the validator's
/// `TRIO_DECKS`. (R259's Best of 3 needed 2.)
pub const SERIES_WINS_NEEDED: i32 = 3;
/// SPEC §11 R334: the most games a series plays, drawn games included. Without a draw a series is
/// decided by its fifth game at the latest (2 × `SERIES_WINS_NEEDED` − 1: a side is then at three
/// wins), so the cap leaves room for two drawn games in the longest series and stops a run of draws
/// from holding both players for ever. At the cap more wins takes the series and equal wins is a
/// series draw. (R259's Best of 3 played at most 3.)
pub const SERIES_MAX_GAMES: i32 = 7;
/// SPEC §11 R333: how long both players have to pick their deck for the next game of a series.
pub const SERIES_PICK_SECONDS: i64 = 60;
/// SPEC §11 R263: how often the series sweeper runs (pick clocks, and games a restart left unstarted).
pub const SERIES_SWEEP_INTERVAL_SECONDS: i64 = 5;
/// SPEC §11 R263: how long a series game may sit unstarted before the sweeper starts it. Longer
/// than any request that is starting it itself, so the sweeper never races a live start.
pub const SERIES_START_GRACE_SECONDS: i64 = 15;
/// SPEC §11 R263: how long a series game may sit with its picks in and no match before the series is
/// given up as abandoned (unrated). A game that has not started in this long cannot be started — its
/// frozen decks no longer build a game, say, after a catalog change — and without an end both players
/// would be held out of the queue by a series that cannot go on. Eight sweeps past the start grace.
pub const SERIES_START_GIVE_UP_SECONDS: i64 = 120;
/// SPEC §11 R263: how many times one request re-reads a series and re-applies its transition after
/// losing the compare-and-set, before it gives up with a 409. Each loss means another writer's
/// transition landed, and a series has at most a handful of writers, so three losses in a row is a
/// storm, not a race worth waiting out.
pub const SERIES_WRITE_ATTEMPTS: usize = 3;
/// How often the series screen and the match screen's series banner re-read the series.
pub const SERIES_POLL_SECONDS: i64 = 2;
/// How long the lobby shows "Match found! Taking you to your game…" before navigating to the
/// paired game, so the pairing reads as an arrival instead of a silent vanish. A `setStatus`
/// followed by a synchronous `navigate` never paints: the lobby unmounts in the same render
/// (`main.tsx` keys the gate and the error boundary by path).
pub const MATCH_FOUND_NAV_DELAY_MS: i64 = 1200;
/// SPEC §11 R672: how long one seat's rematch offer stands after a non-series match ends. Ten
/// minutes: long enough to read the result screen and press the button, short enough that a stale
/// offer cannot summon a game long after both players moved on.
pub const REMATCH_OFFER_TTL_MS: i64 = 600_000;

// ---------------------------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320). The lessons themselves are the client's
// (`apps/web/src/tutorial/lessons.ts`); the server stores ids it does not interpret, so these only
// bound what one account can hold.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R320: the most completed-lesson ids one account keeps. The path has four lessons; this
/// leaves room for the path to grow without a server change, and bounds a body of junk ids.
/// Mirrored in `app.settings` (0011).
pub const TUTORIAL_LESSONS_MAX: usize = 32;
/// SPEC §11 R320: the longest lesson id the server accepts, in characters. An id is a lower-case
/// slug (`basics`, `spells`); forty is room to spare. Mirrored in `app.settings` (0011).
pub const TUTORIAL_LESSON_ID_MAX_LENGTH: usize = 40;

// ---------------------------------------------------------------------------------------------
// Player settings on the account (SPEC §9.1, R633, R634). The settings themselves are the
// client's (`apps/web/src/settings/`); the server stores groups of flat values it does not
// interpret, so these only bound what one account can hold.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R633: the most setting groups one account keeps (the client has four: gameplay, audio,
/// effects and card display). Mirrored in `app.settings` (0018).
pub const PLAYER_SETTINGS_GROUPS_MAX: usize = 8;
/// SPEC §11 R633: the most settings in one group.
pub const PLAYER_SETTINGS_KEYS_MAX: usize = 32;
/// SPEC §11 R633: the longest group id or setting name, in characters. Both are camelCase or kebab-case words.
pub const PLAYER_SETTINGS_NAME_MAX_LENGTH: usize = 40;
/// SPEC §11 R633: the longest text value, in characters (a station or an intensity name).
pub const PLAYER_SETTINGS_TEXT_MAX_LENGTH: usize = 40;
/// SPEC §11 R633: the most text the stored groups of one account may come to, in bytes. Four
/// groups of a dozen settings are well under a kilobyte; this bounds a body of junk. Mirrored in
/// `app.settings` (0018).
pub const PLAYER_SETTINGS_BYTES_MAX: usize = 4096;

// ---------------------------------------------------------------------------------------------
// Public card and player statistics (SPEC §9.11, R654).
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R654: live ranked and unranked games (tutorial excluded) logged per patch before public
/// stats flip from provisional (padded with AI development runs) to live-only figures.
pub const PUBLIC_STATS_MIN_LIVE_GAMES: i64 = 1000;

/// SPEC §11 R654: minimum sample of games for a card row to display a win-rate percentage. Below it,
/// the row displays "not enough games".
pub const CARD_STATS_MIN_SAMPLE: i64 = 20;

/// SPEC §11 R654: cost bucket boundary for card statistics filters.
/// Cost 6 represents the "6+" bucket (cards costing 6 or more), matching the deckbuilder's CURVE_TOP.
pub const CARD_STATS_CURVE_TOP: i32 = 6;

/// Cache-Control max-age in seconds for public card statistics endpoints.
pub const CARD_STATS_CACHE_TTL_SECONDS: i64 = 300;

/// Cache-Control max-age in seconds for public player statistics endpoints.
pub const PLAYER_STATS_CACHE_TTL_SECONDS: i64 = 60;

/// Maximum players per page returned by GET /api/stats/players.
pub const PLAYER_STATS_PAGE_LIMIT: usize = 50;

/// The most text the stored player statistics JSON of one account may come to, in bytes.
/// Bounds request body and stored payload.
pub const PLAYER_STATS_BYTES_MAX: usize = 16384;

// ---------------------------------------------------------------------------------------------
// Play telemetry (SPEC §9.11, R1442): how humans pace their moves, for the ladder bots (#636).
// ---------------------------------------------------------------------------------------------

/// R1442: the fewest human think times a bucket needs before `jackioh-server timing-fit` fits it.
pub const TIMING_FIT_MIN_SAMPLES: usize = 30;

/// R1442: one bucket's log-normal fit of human think times: `ln(think_ms)` has mean `mu` and
/// standard deviation `sigma` over `samples` moves of `action_kind` (an `ActionType` literal), made
/// first or later in the turn, by players in `rank_bucket` (a `RankTier` literal; `None` for the
/// moves no tier was read for).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinkTimeFit {
    pub rank_bucket: Option<&'static str>,
    pub action_kind: &'static str,
    pub first_in_turn: bool,
    pub samples: usize,
    pub mu: f64,
    pub sigma: f64,
}

/// R1442: the think-time fits, pasted by hand from `jackioh-server timing-fit` (CLAUDE.md rule 9);
/// the ladder bots (#636) read it. Empty until enough human games are recorded.
pub const THINK_TIME_FITS: &[ThinkTimeFit] = &[];

// ---------------------------------------------------------------------------------------------
// Derived millisecond helpers, since timers (`tokio::time`) take milliseconds.
// ---------------------------------------------------------------------------------------------

/// R79: `TURN_CLOCK_SECONDS` in milliseconds.
pub const TURN_CLOCK_MS: i64 = TURN_CLOCK_SECONDS * 1000;
/// R79: `PROMPT_CLOCK_SECONDS` in milliseconds.
pub const PROMPT_CLOCK_MS: i64 = PROMPT_CLOCK_SECONDS * 1000;
/// R268: `MULLIGAN_CLOCK_SECONDS` in milliseconds.
pub const MULLIGAN_CLOCK_MS: i64 = MULLIGAN_CLOCK_SECONDS * 1000;
/// R79: `DISCONNECT_GRACE_SECONDS` in milliseconds.
pub const DISCONNECT_GRACE_MS: i64 = DISCONNECT_GRACE_SECONDS * 1000;
/// R79: `MATCH_CEILING_MINUTES` in milliseconds.
pub const MATCH_CEILING_MS: i64 = MATCH_CEILING_MINUTES * 60 * 1000;
/// R1435: `USERNAME_CHANGE_COOLDOWN_SECONDS` in milliseconds.
pub const USERNAME_CHANGE_COOLDOWN_MS: i64 = USERNAME_CHANGE_COOLDOWN_SECONDS * 1000;
