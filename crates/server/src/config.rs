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

use jackioh_engine::wire::{CATALOG_SETS, CodeFormat, SetName};

// ---------------------------------------------------------------------------------------------
// Time units: conversions, not configuration. Rows and clocks are stamped in epoch milliseconds
// and most durations here are stated in seconds or days; every module that converts between them
// takes these (#2160).
// ---------------------------------------------------------------------------------------------

/// Milliseconds in a second: `jackioh-shared`'s (#3098), where the series' pick clock converts.
pub use jackioh_shared::config::MS_PER_SECOND;
/// Nanoseconds in a millisecond: `time::OffsetDateTime` counts the one, the port the other.
pub const NANOS_PER_MS: i128 = 1_000_000;

// ---------------------------------------------------------------------------------------------
// R79 — match lifecycle defaults (SPEC §9.5, BUILD §2). Exact names per BUILD §2's last
// paragraph; do not rename or re-derive these.
// ---------------------------------------------------------------------------------------------

// The match host's numbers (the clocks, the aim relay's interval, the retry paces, the rematch window and
// its stakes) are `jackioh_shared::config`'s since #3101: the pure machine decides with them, and the
// server names them here as it always has.
pub use jackioh_shared::config::{
    AIM_RELAY_INTERVAL_MS, BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES,
    MATCH_SERVER_ACTION_RETRY_MS, MULLIGAN_CLOCK_SECONDS, PROMPT_CLOCK_SECONDS, REMATCH_OFFER_TTL_MS,
    TURN_CLOCK_SECONDS,
};

// The ladder bots' numbers (#3103) are `jackioh_shared::config`'s since the bots became a seat of the
// match host's machine (`crates/shared/src/bots/`, `host/bot.rs`): the machine decides with them, and the
// server names them here as it always has, so `serverConfig.ts` and the server's code read them unchanged.
pub use jackioh_shared::config::{
    AI_EMOTE,
    AI_PERSONAS,
    AiEmoteConfig,
    BOT_ACCOUNT_AGE_MAX_DAYS,
    BOT_ACCOUNT_AGE_MIN_DAYS,
    BOT_ACTIVE_HOURS_MAX,
    BOT_ACTIVE_HOURS_MIN,
    BOT_AIM_LEAD_MAX_MS,
    BOT_AIM_LEAD_MIN_MS,
    BOT_CALIBRATION_GENERATION,
    BOT_CLOCK_MARGIN_MS,
    BOT_CONCEDE_FIT,
    // The bots' cosmetics (R1834005, R1834006).
    BOT_COSMETIC_AGE_BUCKETS_DAYS,
    BOT_COSMETIC_FIT_MIN_SAMPLES,
    BOT_COSMETIC_FITS,
    BOT_COSMETIC_MIN_AGE_DAYS,
    BOT_COSMETIC_MIN_OWNERS,
    BOT_COSMETIC_PRIOR,
    BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
    BOT_COSMETIC_PRIOR_DEFAULT_SHARE,
    BOT_COSMETIC_PRIOR_FULL_ART_WEAR,
    BOT_COSMETIC_PRIOR_OWNERSHIP,
    BOT_COSMETIC_PRIOR_PER_CARD_SHARE,
    BOT_COSMETIC_REDRAW_DAYS,
    BOT_COSMETIC_SLOT_CHANGE_SHARE,
    BOT_DECISION_DEADLINE_MS,
    BOT_DECK_DRAW_ATTEMPTS,
    BOT_DECKS_MAX,
    BOT_DECKS_MIN,
    BOT_DEFAULT_USERNAME_SHARE,
    BOT_DRAW_ANSWER_FIT,
    BOT_DRAW_OFFER,
    BOT_EMOTE_DELAY_FIT,
    BOT_EMOTE_REPLY_DELAY_FIT,
    BOT_FALLBACK_MAX_SECONDS,
    BOT_FALLBACK_MIN_SECONDS,
    BOT_GRIND_ACTIVE_HOURS,
    BOT_GRIND_GAP,
    BOT_GRIND_HYSTERESIS,
    BOT_GRIND_REFERENCE_RANK,
    BOT_GRINDERS,
    BOT_HOURS_PER_DAY,
    BOT_MID_STRENGTH_BUDGET,
    BOT_NAME_BIRTH_YEAR_MAX,
    BOT_NAME_BIRTH_YEAR_MIN,
    BOT_NAME_CHINESE_NUMBERS,
    BOT_NAME_CHINESE_YEAR_SHARE,
    BOT_NAME_COMPOUND_LETTERS,
    BOT_NAME_DRAW_ATTEMPTS,
    BOT_NAME_FILLER_NUMBER_SHARE,
    BOT_NAME_FILLER_NUMBERS,
    BOT_NAME_LUIS_SHARE,
    BOT_NAME_SHAPES_CHINESE_HAN,
    BOT_NAME_SHAPES_CHINESE_PINYIN,
    BOT_NAME_SHAPES_CRUDE,
    BOT_NAME_SHAPES_EDGY,
    BOT_NAME_SHAPES_GAMER,
    BOT_NAME_SHAPES_GIVEN,
    BOT_NAME_SHAPES_NONSENSE,
    BOT_NAME_SHAPES_PLAIN,
    BOT_NAME_SHAPES_POLITICS,
    BOT_NAME_SHAPES_REFERENCE,
    BOT_NAME_SHARE_ANIME,
    BOT_NAME_SHARE_CHINESE_HAN,
    BOT_NAME_SHARE_CHINESE_PINYIN,
    BOT_NAME_SHARE_CRUDE,
    BOT_NAME_SHARE_EDGY,
    BOT_NAME_SHARE_GAMER,
    BOT_NAME_SHARE_GIVEN,
    BOT_NAME_SHARE_NONSENSE,
    BOT_NAME_SHARE_OTHER_REFS,
    BOT_NAME_SHARE_PLAIN,
    BOT_NAME_SHARE_POLITICS,
    BOT_NAME_STRETCH_MAX,
    BOT_NAME_STRETCH_MIN,
    BOT_NAME_SYLLABLES_MAX,
    BOT_NAME_SYLLABLES_MIN,
    BOT_NAME_TWO_DIGIT_MAX,
    BOT_NAME_TWO_DIGIT_MIN,
    BOT_NAME_YEAR_SHARE,
    BOT_PERSONA_WEIGHTS,
    BOT_PICK_DELAY_MAX_MS,
    BOT_PICK_DELAY_MIN_MS,
    BOT_PICK_RULE_SHARE,
    BOT_POOL_ACTIVE_WINDOW_DAYS,
    BOT_POOL_MAX,
    BOT_POOL_MIN,
    BOT_POOL_PER_WEEKLY_ACTIVE,
    BOT_RECENT_OPPONENTS,
    BOT_REMATCH_DELAY_MAX_MS,
    BOT_REMATCH_DELAY_MIN_MS,
    BOT_REMATCH_FIT,
    BOT_REMATCH_LINGER_MAX_MS,
    BOT_REMATCH_LINGER_MIN_MS,
    BOT_SEED_RATINGS,
    BOT_SPEED_MAX,
    BOT_SPEED_MIN,
    BOT_STRENGTHS,
    BOT_THINK_MAX_MS,
    BOT_THINK_MIN_MS,
    BOT_THINK_PRIOR_FIRST_MU,
    BOT_THINK_PRIOR_FIRST_SIGMA,
    BOT_THINK_PRIOR_LATER_MU,
    BOT_THINK_PRIOR_LATER_SIGMA,
    BOT_THINK_PRIOR_LN_LEGAL_MEAN,
    BOT_THINK_PRIORS,
    BOT_TIMEOUT_FIT,
    BOT_VS_BOT_MIN_GAP_SECONDS,
    BotNameShape,
    BotStrength,
    CARD_STATS_MIN_SAMPLE,
    ConcedeFit,
    CosmeticFit,
    CosmeticOwnership,
    CosmeticWear,
    DrawAnswerFit,
    DrawOfferRule,
    EMOTE_REPLY_KEYS,
    EMOTE_TRIGGERS,
    EmoteDelayFit,
    EmoteReplyKey,
    EmoteTrigger,
    MAX_FAIR_GAP,
    MS_PER_DAY,
    PERSONA_DEAL_STEPS,
    PERSONA_NAMES,
    PersonaCaps,
    PersonaName,
    PersonaRow,
    PersonaSpec,
    PersonaWeights,
    RematchFit,
    SKILL_ELO_ANCHOR,
    SKILL_ELO_TABLE,
    ScalarFit,
    THINK_COMPLEXITY_SLOPE,
    THINK_LETHAL_SHIFT,
    THINK_TIME_FITS,
    ThinkTimeFit,
    ThinkTimePrior,
    TimeoutFit,
    expected_elo_for_skill,
    persona_spec,
};

/// R79, §9.5: room codes are 6 characters from the invite-code alphabet.
pub const ROOM_CODE_LENGTH: usize = 6;
/// R79, §9.5: how long an unjoined room stays open, in seconds. Fifteen minutes. TS stated this as
/// the literal `15 * 60 * 1000` in `api/deps.ts`'s `defaultLimits()` (`roomCodeTtlMs`); v0.3.0 names
/// it here (CLAUDE.md rule 9, research C §10 #17), and `api/deps.ts` is not ported.
pub const ROOM_CODE_TTL_SECONDS: i64 = 15 * 60;
/// R890000, §9.5: how many unclaimed unexpired rooms one host may hold open at once. Three bounds
/// how far one host's rooms can amplify a seat race without stranding a host whose old room
/// lingers to expiry. Advisory: the seat compare-and-set is what stops a double seat, not this.
/// `create` refuses a fourth with 409.
pub const MAX_OPEN_ROOMS_PER_HOST: i64 = 3;

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
// normalizing any user-entered code to upper case before comparison, rather than by omitting a
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
/// own per-IP bucket. A deployment behind a proxy says so explicitly (wrangler.jsonc sets 1).
pub const DEFAULT_TRUSTED_PROXY_HOPS: usize = 0;
/// R190: the most hops `TRUSTED_PROXY_HOPS` may name; anything above it is a misconfiguration.
pub const MAX_TRUSTED_PROXY_HOPS: usize = 5;
/// R190: an IPv6 client is counted by its first this-many bits. A /64 is only the least a line is
/// handed: many ISPs delegate a /56 to one home, and a tunnel broker routes a /48 to anyone who asks,
/// so keying on the /64 gave one host 256 (or 65,536) fresh buckets. A /56 is the usual compromise:
/// it closes the home delegation, and the price is that IPv6 neighbours sharing a /56 share a bucket.
pub const IPV6_RATE_LIMIT_PREFIX_BITS: u32 = 56;

/// The largest request body the API reads, in bytes. Every body the API accepts is a small JSON
/// object (a code, three decks of card ids) but the account's statistics copy, which may reach
/// `PLAYER_STATS_BYTES_MAX` (R1142000), so the cap is that; past it the body is refused before it is
/// parsed.
pub const API_MAX_BODY_BYTES: usize = PLAYER_STATS_BYTES_MAX;

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
/// password reset that signed other devices out) stops being honored here within this many seconds,
/// not when its access token expires.
pub const AUTH_SESSION_LIVE_CACHE_SECONDS: i64 = 30;
/// SPEC §11 R159: §9.4 step 1's "verified email" is read from the auth provider and only the
/// positive answer is remembered, briefly and per user id (`auth.rs`'s `confirmed`, pruned on each
/// insert). An unreachable provider still fails closed: the email counts as unverified. Not in
/// SPEC: how brief, 30 seconds.
pub const EMAIL_CONFIRMED_CACHE_TTL_MS: i64 = 30_000;
/// A request to the auth provider that has not answered in this long is a network failure.
pub const AUTH_PROVIDER_TIMEOUT_SECONDS: i64 = 30;
/// R3209000: how long the API remembers that the provider did not answer about one session (R194's
/// check), so an outage costs that session one wait per window, not one per request. Never longer
/// than `AUTH_SESSION_LIVE_CACHE_SECONDS` (`auth.rs` asserts it). Server-only.
pub const AUTH_SESSION_OUTAGE_CACHE_SECONDS: i64 = 15;
/// R3209000: how long R194's in-request session check (and tier 3's lookup by token, which shares
/// the call) waits before the provider counts as unreachable. The sign-in grant keeps
/// `AUTH_PROVIDER_TIMEOUT_SECONDS`. Server-only.
pub const AUTH_SESSION_CHECK_TIMEOUT_SECONDS: i64 = 5;
// ---------------------------------------------------------------------------------------------
// The sign-in key set (SPEC §9.2, R1144000). The server verifies Supabase's asymmetric tokens
// against the project's published JWKS, fetched on first use. A fetched set is kept this long
// before it is fetched again; a refetch that fails keeps serving the last good set for at most
// `JWKS_STALE_MAX_MS` past that age, retried at most once per `JWKS_COOLDOWN_MS`, so a Supabase
// blip signs nobody out. A successful fetch replaces the set whole, so a dropped key is refused
// at once. Server-only: the client imports none of these (`tests/export_config.rs`).
// ---------------------------------------------------------------------------------------------

/// R1144000: how long a fetched key set is trusted before it is fetched again (jose's default).
pub const JWKS_CACHE_MAX_AGE_MS: i64 = 10 * 60 * 1000;
/// R1144000: a failed refetch, and a key miss on a recently fetched set, is retried at most this
/// often (jose's `cooldownDuration`), so requests share one fetch instead of queuing fetches.
pub const JWKS_COOLDOWN_MS: i64 = 30_000;
/// R1144000: how long the key-set request may take (jose's `timeoutDuration`).
pub const JWKS_FETCH_TIMEOUT_MS: i64 = 5_000;
/// R1144000: how long past `JWKS_CACHE_MAX_AGE_MS` a failed refetch keeps serving the last good
/// set. About one Supabase access-token lifetime, so no token outlives its own `exp` through the
/// stale set; past it the token is refused until the key endpoint answers again.
pub const JWKS_STALE_MAX_MS: i64 = 3_600_000;
/// A request to this server that has not answered in this long is a network failure. Longer than the
/// provider's, because the first request after the server has slept starts its container, which
/// migrates and reseeds before it listens (`release`).
pub const API_REQUEST_TIMEOUT_SECONDS: i64 = 75;
/// How long the gate's "Checking your account…" waits before it offers a way out and says why.
pub const GATE_SLOW_NOTICE_SECONDS: i64 = 5;
/// R192: the code screen reads `/api/codes/status` again when a wait it stated runs out, but never
/// sooner than this after the last read, so a wait that has just lapsed cannot make it poll.
pub const CODE_STATUS_RECHECK_FLOOR_SECONDS: i64 = 1;

// ---------------------------------------------------------------------------------------------
// The age screen and the terms a player agrees to (SPEC §9.4, R757000, R757001). Public: the
// sign-up form, the consent screen and the privacy policy read them.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R757000: the youngest age, in whole years, at which a person may hold an account. The
/// owner's to choose: 13 meets COPPA; 16 if players in the EU are expected (GDPR art. 8 lets each
/// member state pick between 13 and 16, and 16 is the default). Changing it asks no existing
/// account again: each account's age check records the minimum it was checked against.
pub const MIN_ACCOUNT_AGE_YEARS: i32 = 13;
/// R757000: the earliest birth year the age screen offers and the server accepts.
pub const BIRTH_YEAR_EARLIEST: i32 = 1900;
/// SPEC §11 R757001: the version of the Terms and the Privacy Policy a player agrees to: the terms'
/// "Last updated" date, then the privacy policy's (`TERMS_LAST_UPDATED` in
/// `apps/web/src/routes/terms.tsx` and `PRIVACY_LAST_UPDATED` in `privacy.tsx`, which a web test
/// holds it to). Changing either page's text changes its date and so this, and every account is
/// asked to agree again. R970001: a second change on a day the page already changed keeps its date
/// and gives its half a letter (`2026-10-15b`, then `c`); `privacy.test.tsx` holds each version to
/// the words of both pages, so a change to them fails until this moves.
pub const TERMS_VERSION: &str = "2026-11-05/2026-11-05";

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
/// R1044001: days a game record's detail (both decklists, hands, draws and plays) is kept after it
/// was written. Every card figure reads the totals the record was folded into as it was written
/// (R1044000), so the detail serves only `stats-export` and the analysis notebooks; the record's id,
/// source, mode, patch and time stay, so a second write of the same game is still refused.
pub const GAME_RECORD_RAW_RETENTION_DAYS: i64 = 7;
/// How often the retention purge runs. It also runs once at boot, since the container sleeps when idle.
pub const RETENTION_PURGE_INTERVAL_SECONDS: i64 = 3600;
/// R1596000: seconds after a match ended before the retention loop files its missing game record
/// again, so the filing that follows the result has long finished and is never raced.
pub const GAME_RECORD_REFILE_GRACE_SECONDS: i64 = 10 * 60;
/// R1596000: seconds after a match ended during which each retention pass files its missing game
/// record again. A write that failed once lands within the hour; past this, the match gets one try
/// more as the purge is about to take its log.
pub const GAME_RECORD_REFILE_WINDOW_SECONDS: i64 = 24 * 60 * 60;

// ---------------------------------------------------------------------------------------------
// The background loops' supervisor (`loops.rs`). Not in SPEC, and no R-row (#842).
// ---------------------------------------------------------------------------------------------

/// How long `loops::spawn_supervised` waits after a loop panics or returns before it starts that
/// loop again. The retention purge and the bot pool tick as soon as they start, so without a wait a
/// panic that recurs on every tick, or a loop that returns at once, would restart them in a hot
/// loop. Five seconds holds such a loop to one alert line per five seconds and leaves a pairing or
/// a sweep late by seconds only.
pub const LOOP_RESTART_DELAY_SECONDS: i64 = 5;

// ---------------------------------------------------------------------------------------------
// The probes and the shutdown (`api/health.rs`, `app::serve_until`). Not in SPEC, and no R-row (#861).
// ---------------------------------------------------------------------------------------------

/// How long `GET /readyz` waits for the store to answer `select 1` before it answers 503. The
/// pool retries a refused connection until its 10 s checkout timeout (`POOL_ACQUIRE_TIMEOUT_MS`,
/// `db/pg.rs`). Without this bound, a dead database would hold the probe instead of failing it.
pub const READINESS_PROBE_TIMEOUT_MS: i64 = 2_000;
/// How long `app::serve_until`'s drain may run after SIGTERM or Ctrl-C before it stops waiting
/// and still exits 0. Under the 30 s that the Worker gives a container on an older image (apps/edge's
/// `RETIRE_WAIT_MS`) and `docker stop -t 30` in `tests/deploy/rehearse.sh` gives, between SIGTERM and
/// the kill.
pub const SHUTDOWN_DRAIN_SECONDS: i64 = 20;
/// How often the drain checks whether every match socket has finished.
pub const SHUTDOWN_SOCKETS_POLL_MS: i64 = 50;

// ---------------------------------------------------------------------------------------------
// The Postgres pool (`db/pg.rs`, #3208). Not in SPEC, and no R-row.
// ---------------------------------------------------------------------------------------------

/// A pooled connection idle this long is pinged before a request gets it (`pg::pool_options`'s
/// `before_acquire`); one checked out sooner is not, which saves a round trip on most requests. Half
/// `pg.rs`'s 10 s idle timeout (`POOL_IDLE_TIMEOUT_MS`), which already sits under Supavisor's cut.
pub const POOL_PING_AFTER_IDLE_MS: i64 = 5_000;
/// The connections the pool keeps open while the server is quiet, so a request after a quiet minute
/// does not pay for TCP, TLS and the Postgres login. The pool still connects lazily: it keeps warm the
/// connections requests opened, up to this many, and refills to it when a burst's are retired. A u32,
/// as sqlx takes it.
pub const POOL_MIN_CONNECTIONS: u32 = 2;
/// How often `pg::heartbeat` pings the warm connections: under `POOL_PING_AFTER_IDLE_MS`, so a warm
/// connection is never pinged at checkout, and so also under the idle timeout, which never retires one.
pub const POOL_HEARTBEAT_MS: i64 = 4_000;

// ---------------------------------------------------------------------------------------------
// Online replays (R768, issue #510). A page of steps is the engine's `REPLAY_PAGE_STEPS`. PUBLIC:
// the client reads the list's page and the routes' allowance.
// ---------------------------------------------------------------------------------------------

/// How many finished matches one page of `GET /api/replays` lists, newest first.
pub const REPLAY_LIST_PAGE: usize = 10;
/// How many opened replays (each its folded log's checkpoints, R768) the server keeps in memory at
/// once; opening another drops the one used longest ago.
pub const REPLAY_CACHE_MATCHES: usize = 16;
/// How long an opened replay stays in memory after it was opened, in seconds.
pub const REPLAY_CACHE_TTL_SECONDS: i64 = 600;
/// Requests per minute per account to the two replay routes together, on top of R109's
/// API-wide allowance: opening a replay folds a whole game.
pub const REPLAY_REQUESTS_PER_MINUTE: usize = 60;

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
// The invite-cluster watch (§9.8).
// ---------------------------------------------------------------------------------------------

// SPEC §9.8 (abuse surface): "Multi-accounting to farm rating: invite codes; watch for clusters of
// accounts sharing an IP hash." `jackioh-server invite-clusters` is that watch: an offline report a
// person reads, never a ban, because many people can share one address (a campus, a household, a
// carrier's NAT). SPEC fixes no threshold or window, so these three are advisory defaults an
// operator may override with the command's flags; none of them is a game rule, and the client reads
// none of them.

/// §9.8: how far back the watch counts attempts by default, in seconds. A week: farming rating
/// takes accounts made over days, and a window of the rate limit's own hour (`CODE_ATTEMPT_WINDOW_SECONDS`)
/// would never see a slow cluster. The same table also bounds the window: `code_attempts` rows are
/// kept `CODE_ATTEMPT_RETENTION_DAYS`, so the flag takes at most that many days.
pub const INVITE_CLUSTER_WINDOW_SECONDS: i64 = 7 * 24 * 3600;
/// §9.8: how many distinct accounts one IP hash must be behind, by default, for the watch to list
/// it. A household or a dorm room stays under it; a farm built to be worth the trouble does not.
pub const INVITE_CLUSTER_MIN_ACCOUNTS: i64 = 6;
/// §9.8: how many characters of an IP hash the report prints. Enough for an operator to tell two
/// hashes apart and to find one in the table, too few to stand in for the hash itself.
pub const INVITE_CLUSTER_HASH_PREFIX_LEN: usize = 8;

// ---------------------------------------------------------------------------------------------
// Matchmaking (§9.5, BUILD M7-T3).
// ---------------------------------------------------------------------------------------------

/// §9.5: the rating window's starting half-width, in rating points, at the moment a ticket enqueues.
pub const RATING_WINDOW_START: i64 = 100;
/// §9.5: the rating window widens by this many rating points every `RATING_WINDOW_WIDEN_EVERY_SECONDS`.
pub const RATING_WINDOW_WIDEN_BY: i64 = 50;
/// §9.5: the cadence, in seconds, at which the rating window widens.
pub const RATING_WINDOW_WIDEN_EVERY_SECONDS: i64 = 10;
/// R640001: the seconds a ticket waits before its window reaches `MAX_FAIR_GAP`. The gap is a whole
/// number of widenings past `RATING_WINDOW_START`, so the window reaches it exactly at a widening.
/// PUBLIC: e2e spec 19 reads it.
pub const RATING_WINDOW_FULL_AFTER_SECONDS: i64 =
    (MAX_FAIR_GAP - RATING_WINDOW_START) / RATING_WINDOW_WIDEN_BY * RATING_WINDOW_WIDEN_EVERY_SECONDS;

// SPEC §9.5: "Pairing runs on enqueue plus a sweeper every few seconds" — the exact cadence is
// not fixed by SPEC.
//
// SPEC §11 R108: 3 s. Frequent enough that a ticket is rarely left waiting past its next window
// widen before being reconsidered, without hammering the ticket table.
pub const MATCHMAKER_SWEEP_INTERVAL_SECONDS: i64 = 3;

/// SPEC §11 R897000: how long a queued ticket may go unheard from before the pairing sweep
/// cancels it, in milliseconds. Six missed `SERIES_POLL_SECONDS` polls, so a flaky connection
/// does not eject a real player, and under `BOT_FALLBACK_MIN_SECONDS`, so expiry always runs
/// before the empty-queue fallback could seat the absent player.
pub const QUEUE_TICKET_STALE_MS: i64 = 12_000;
/// SPEC §11 R897000: how many `GET /api/auth/me` reads in a row must say the ticket is gone
/// before the client stops waiting with a rejoin message. Two, so one failed or slow read cannot
/// end a live queue.
pub const QUEUE_ENDED_AFTER_READS: usize = 2;

/// SPEC §11 R640002: the window the queue readout counts over, in seconds: the waits of the tickets
/// claimed in it and the games that ended in it.
pub const QUEUE_READOUT_WINDOW_SECONDS: i64 = 3600;
/// R640002: the share of a mode's counted waits the expected wait is at least as long as: the upper
/// quartile, so "usually under" holds for three waits in four.
pub const QUEUE_WAIT_QUANTILE: f64 = 0.75;
/// R640002: the fewest counted waits a mode needs before the readout states an expected wait.
pub const QUEUE_WAIT_MIN_SAMPLES: usize = 5;
/// R640002: the expected wait is rounded up to the next multiple of this many seconds.
pub const QUEUE_WAIT_STEP_SECONDS: i64 = 10;
/// SPEC §11 R1996000: how long the server keeps the queue readout it counted, in seconds, so the
/// polls of every waiting client inside it share one count. Short beside the
/// `QUEUE_READOUT_WINDOW_SECONDS` it counts over, so a kept answer is never visibly late.
pub const QUEUE_READOUT_CACHE_TTL_SECONDS: i64 = 5;
/// SPEC §11 R3207000: how often the lobby's wait re-reads the queue readout, in seconds, after the
/// read it makes when it opens. Beyond `QUEUE_READOUT_CACHE_TTL_SECONDS`, so every read counts anew,
/// and well inside the `QUEUE_READOUT_WINDOW_SECONDS` it counts over, so the readout is never visibly
/// late. The wait's `GET /api/auth/me` read keeps `SERIES_POLL_SECONDS`: it keeps the ticket alive
/// (R897000), and the readout does not.
pub const QUEUE_POPULATION_POLL_SECONDS: i64 = 30;

/// §9.5: the half-width of the acceptable rating gap for a ticket that has waited
/// `waited_seconds`. Shared by the pairing query and its tests so both read one implementation.
///
/// At `waited_seconds = 0` this is `RATING_WINDOW_START`; it widens by `RATING_WINDOW_WIDEN_BY`
/// every `RATING_WINDOW_WIDEN_EVERY_SECONDS` and stops at `MAX_FAIR_GAP` (R640001), which it
/// reaches at `RATING_WINDOW_FULL_AFTER_SECONDS`.
pub fn rating_window(waited_seconds: f64) -> f64 {
    let clamped_wait = waited_seconds.max(0.0);
    let widen_steps = (clamped_wait / RATING_WINDOW_WIDEN_EVERY_SECONDS as f64).floor();
    (RATING_WINDOW_START as f64 + widen_steps * RATING_WINDOW_WIDEN_BY as f64).min(MAX_FAIR_GAP as f64)
}

// ---------------------------------------------------------------------------------------------
// The hidden rating: Glicko-2 (SPEC §9.12, R603). The maths is `jackioh-shared`'s `src/ranked/glicko2.rs`.
// ---------------------------------------------------------------------------------------------

/// The hidden rating's numbers are `jackioh-shared`'s (#3105), where its maths now is.
pub use jackioh_shared::config::{
    GLICKO_CONVERGENCE, GLICKO_MAX_ITERATIONS, GLICKO_SCALE, GLICKO_TAU, RATING_DEVIATION_START,
    RATING_START, RATING_VOLATILITY_START,
};

// ---------------------------------------------------------------------------------------------
// The visible ladder (SPEC §9.12, R605–R608). The rules are `jackioh-shared`'s `src/ranked/ladder.rs`. PUBLIC: the
// client reads the shape (divisions, pips, placements) to draw a rank it is handed.
// ---------------------------------------------------------------------------------------------

/// The visible ladder's numbers are `jackioh-shared`'s (#3105), where its rules now are.
pub use jackioh_shared::config::{
    JLORIOUS_SIZE, RANK_CONVERGENCE_GAP_PIPS, RANK_CONVERGENCE_PIPS, RANK_DIVISIONS_PER_TIER, RANK_LOSS_PIPS,
    RANK_PIPS_PER_DIVISION, RANK_PLACEMENT_GAMES, RANK_STREAK_BONUS_PIPS, RANK_STREAK_LENGTH,
    RANK_TIER_PERCENTS, RANK_WIN_PIPS, RankTierPercents,
};

/// R2101000: the ranked win streak (R606's `streak`) from which the profile and a ranked match's
/// line show a flame with its count. Only the client reads it. Three, where R606's bonus starts,
/// but apart from `RANK_STREAK_LENGTH` so each tunes alone.
pub const RANK_STREAK_FLAME_LENGTH: i32 = 3;

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
/// being normalized, so a huge query string costs nothing to refuse. Far above any real name
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
// Username sign-in (SPEC §9.4, R1443). The route is `src/api/auth.rs`'s `username_sign_in`.
// ---------------------------------------------------------------------------------------------

/// R1443: how many username sign-ins one address may try in a window, and, counted apart, how many
/// may be tried at one username.
pub const USERNAME_SIGN_IN_ATTEMPTS_PER_WINDOW: usize = 10;
/// R1443: the window `USERNAME_SIGN_IN_ATTEMPTS_PER_WINDOW` counts in (5 minutes).
pub const USERNAME_SIGN_IN_WINDOW_SECONDS: i64 = 300;

// ---------------------------------------------------------------------------------------------
// Seasons (SPEC §9.12, R609). The reset is `jackioh-shared`'s `src/ranked/season.rs`.
// ---------------------------------------------------------------------------------------------

/// The season reset's numbers are `jackioh-shared`'s (#3105), where the reset now is.
pub use jackioh_shared::config::{SEASON_RESET_DEVIATION_BOOST, SEASON_RESET_STRENGTH};
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
/// SPEC §11 R835000: how many refused actions from one seat, inside `REJECTION_ALERT_WINDOW_MS`,
/// raise `match.rejections.unusual` (§9.8). Twenty: a lost player mashing End turn out of turn
/// stays under it, while a scripted flooder crosses it in seconds. Counted strictly over the
/// threshold, so exactly this many refusals stay silent.
pub const REJECTION_ALERT_THRESHOLD: usize = 20;
/// SPEC §11 R835000, R835002: the rolling window the rejection count and the malformed-frame count
/// are kept over, in milliseconds. A minute: abuse inside a live match (§9.8) is bursty, and a
/// window this wide keeps a rapid test's sends inside one of them.
pub const REJECTION_ALERT_WINDOW_MS: i64 = 60_000;
/// SPEC §11 R1717000: how many requests the API's rate limit (`API_REQUESTS_PER_MINUTE`) refuses one
/// account or address, inside `API_REJECTION_ALERT_WINDOW_MS`, before `api.rejections.unusual` is
/// raised (§9.8). Twenty, as a seat's (`REJECTION_ALERT_THRESHOLD`): a client that waits out a
/// refusal's `retryAfterMs` (R192) is refused about once per wait, while a script that ignores it
/// crosses twenty in a second. Counted strictly over the threshold, so exactly this many stay silent.
pub const API_REJECTION_ALERT_THRESHOLD: usize = 20;
/// SPEC §11 R1717000: the rolling window `API_REJECTION_ALERT_THRESHOLD` counts in, in milliseconds,
/// and the least time between two alerts for one key. A minute, the API limit's own window, so a
/// sustained flood alerts once a minute.
pub const API_REJECTION_ALERT_WINDOW_MS: i64 = 60_000;
/// SPEC §11 R2478000: how many verified auth users the router remembers the profile id of
/// (`api::http::KnownProfiles`), so a request from an account whose `API_REQUESTS_PER_MINUTE` is
/// spent is refused before the store is read. Ten thousand: far more accounts than one server hears
/// from in the minute a budget lasts, in about two megabytes; past it the user remembered longest
/// ago is dropped and reads the store once more.
pub const KNOWN_PROFILES_MAX: usize = 10_000;
/// SPEC §11 R835002: how many malformed frames from one seat, inside `REJECTION_ALERT_WINDOW_MS`,
/// the actor answers before it closes that seat's socket. Ten: a stale client sends a few, never
/// this many, so past it the peer is a flooder, not a bug.
pub const MALFORMED_FRAMES_PER_WINDOW: usize = 10;
/// SPEC §11 R2609000: the least time between two later hellos of one watcher's socket that the actor
/// answers (R1103000); a hello sooner after the last answered one is dropped unanswered. R2686000
/// holds a player's socket to the same interval, on `/ws/match` and `/ws/duos`: a later hello of one
/// socket generation, sooner than this after the last answered one, is dropped unanswered, while the
/// first hello of a new generation (a reattach) is always answered. A second: a page probes only once
/// its socket has been quiet past `WS_PING_INTERVAL_SECONDS` and any answer ends the quiet (R1441),
/// so a real page's probe never falls inside it, while a scripted flood costs one view a second.
/// R3214000 holds a `/ws/social` socket's hellos to it too: one social read a second at most.
pub const WATCH_HELLO_INTERVAL_MS: i64 = 1_000;
/// Match sockets one client address may hold at once, counting handshakes still in progress. Not in
/// SPEC, and no R-row. A player needs one socket per match, plus one more for a moment while it
/// reconnects; ten leaves room for several players behind one home or campus address. Past it the
/// upgrade is refused with 429 before the handshake is read. `/ws/social`'s sockets are counted apart,
/// against a cap of the same size, so idle tabs never take a match socket's room (R3214000).
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
/// R1450: how long past its `ceiling_at` an `open` matches row stays before the reaper deletes it
/// (`api::results::reap_open_matches`). A room's `ceiling_at` is its expiry and a queue reservation's
/// its claim, so this is the time a start may still take after either. Five minutes outlasts a
/// Conquest series' first game, which starts or is given up within `SERIES_PICK_SECONDS +
/// SERIES_START_GIVE_UP_SECONDS` (three minutes) of its claim.
pub const OPEN_MATCH_REAP_GRACE_SECONDS: i64 = 5 * 60;
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
/// #1169: the fewest logged actions a match must hold for the restart drain to store a checkpoint
/// of it (`actor::checkpoint`). A shorter log is cheap to fold whole, so the drain spends no write on
/// it: as #1169's plan measured a release build, a fold costs about 0.39 ms an action and the
/// telemetry's replay about 0.50 ms, while a checkpoint (some 75 KB) takes about 0.3 ms to serialize
/// and hash, plus its write.
pub const MATCH_CHECKPOINT_MIN_ACTIONS: i64 = 32;
/// #1169: the shape of a checkpoint's body and of what the server folds a log to. A checkpoint
/// stamped with another is refused and the whole log folded. Bump it when a server change alters
/// either: the body (`actor::checkpoint::CheckpointBody`), the setup a rebuild folds with
/// (`Registry::rebuild`'s arguments) or the bot turns it reads (`bot_moved_turns`). A change to the
/// engine, the cards or `actor/engine.rs` and `actor/telemetry.rs` needs none: the rules fingerprint
/// (`build.rs`) refuses those.
pub const MATCH_CHECKPOINT_VERSION: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Bug reports (`bug_reports.rs`): a match its own log does not reproduce. Not in SPEC, and no
// R-row: operational, as #861's probes are.
// ---------------------------------------------------------------------------------------------

/// The repository a report is filed in when `CREATE_ISSUES_TOKEN` is set and `GITHUB_ISSUES_REPO`
/// is not.
pub const BUG_REPORT_REPO_DEFAULT: &str = "Jlore-Studios/JackiOh";
/// GitHub's REST API, which a report is filed through.
pub const BUG_REPORT_API_URL: &str = "https://api.github.com";
/// The labels a report carries: a micro patch's type, the night bot's queue (`bot:build`,
/// bot/README.md), the strong tier (CLAUDE.md rule 11: the server and the engine are hard) and the
/// front of the queue, since a match that does not rebuild is a broken game.
pub const BUG_REPORT_LABELS: &[&str] = &["patch", "bot:build", "difficulty:hard", "priority:high"];
/// How many issues one process opens in `BUG_REPORT_WINDOW_MS`, one per failure signature
/// (`bug_reports::Signature`). A determinism break shows on every match a deploy rebuilds, and
/// those matches share a signature and so an issue; a signature beyond this is only logged.
pub const BUG_REPORTS_PER_WINDOW: usize = 5;
/// The window `BUG_REPORTS_PER_WINDOW` counts in: an hour.
pub const BUG_REPORT_WINDOW_MS: i64 = 60 * 60 * 1000;
/// How long one request to GitHub may take before the report is given up (it is logged either way).
pub const BUG_REPORT_TIMEOUT_MS: u64 = 10_000;
/// How many refused actions a report lists one by one; the rest are counted.
pub const BUG_REPORT_LISTED_ACTIONS: usize = 10;
/// How many matches one signature's issue lists. Each one edits the issue's body, so this bounds the
/// edits as `BUG_REPORTS_PER_WINDOW` bounds the issues; a later match is only logged (`bug.found`).
pub const BUG_REPORT_LISTED_MATCHES: usize = 20;

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
/// SPEC §9.4, R3212001: the most items one `POST /api/decks/sync` carries, its saves and its
/// deletions together: every deck and trio a profile can hold saved and deleted in one request, so
/// one autosave pass always fits. A longer list is 400, running none of it.
pub const DECK_SYNC_ITEMS_MAX: usize = 2 * (MAX_SAVED_DECKS + MAX_SAVED_TRIOS);
/// SPEC §11 R250: the most draft issues one refused save lists in its `details`. A real deck of at
/// most `DECK_SIZE` cards can break D1–D4 about fifty times; a request body of junk ids could break
/// D3 thousands of times and turn a 64 KiB request into a far larger answer. The first issue is
/// still the error's message, so nothing a player could fix is hidden.
pub const DRAFT_ISSUES_REPORTED_MAX: usize = 50;
/// SPEC §11 R255: the deck-code format's version. Version 2 (patch v0.2.0, docs/classic-sets.md B2.2)
/// writes each card's set with its number (`catalog_number_set_offset`); a code naming a version
/// other than this one or `DECK_CODE_CORE_ONLY_VERSION` is refused. A set that ships later takes an
/// offset of its own and leaves the version alone (R1410), so every code minted before still reads.
pub const DECK_CODE_VERSION: i32 = 2;
/// SPEC §11 R255: the one older deck-code version still read. Every code minted before patch v0.2.0
/// is a version 1 code, and its numbers are Core's (the only set there was).
pub const DECK_CODE_CORE_ONLY_VERSION: i32 = 1;

/// SPEC §11 R255, R339 (B2.2), R1410: a card's catalog number in a version 2 deck or trio code is
/// its §5 index plus its set's offset, a stride for each place its set holds in the catalog order —
/// Core n, Classic 1000 + n, Classic+ 2000 + n, Meditative 3000 + n, Meditative+ 4000 + n,
/// Stinkyscape 5000 + n, Training Grounds 6000 + n — still written as LEB128. No set holds 1000
/// cards, so a number names one set and one card.
pub const CATALOG_NUMBER_SET_STRIDE: i32 = 1000;

/// SPEC §11 R255, R339, R1410, R3114000: a set's offset in a version 2 deck or trio code, its place in
/// the catalog order (`CATALOG_SETS`, the set table's order) times `CATALOG_NUMBER_SET_STRIDE`, or
/// `None` for a set the catalog does not order (Boss, Boss-X). Meditative's offset was the
/// next thousand in catalog order (R1420), and so was Meditative+'s after it (v0.3.15, issue #1187,
/// R1178000), Stinkyscape's after that (issue #1884, R1857000) and Training Grounds' after
/// Stinkyscape's (issue #2204); a set's row is appended to the table and never moved, so none moves
/// another set's, and a code minted before a set shipped reads as it always did. The client reads
/// the shipped sets' offsets (`CATALOG_NUMBER_SET_OFFSETS` in `serverConfig.ts`).
pub fn catalog_number_set_offset(set: SetName) -> Option<i32> {
    let place = CATALOG_SETS.iter().position(|listed| *listed == set)?;
    i32::try_from(place)
        .ok()
        .map(|place| place * CATALOG_NUMBER_SET_STRIDE)
}
/// SPEC §11 R255: raw deck-code input longer than this is refused before it is read. A v1 code for
/// a full deck with the longest name is under 200 characters, so this leaves room for whatever a
/// chat client wraps around a pasted code.
pub const DECK_CODE_MAX_INPUT_LENGTH: usize = 512;
/// SPEC §11 R256: how long the builder waits after the last edit before it saves.
pub const DECK_AUTOSAVE_DEBOUNCE_MS: i64 = 800;
/// SPEC §11 R256: how long the builder waits before it tries a failed save again.
pub const DECK_AUTOSAVE_RETRY_SECONDS: i64 = 5;
/// SPEC §11 R2100000: how many earlier versions the account keeps of one deck. A save past it drops
/// the oldest. Twenty covers an evening's tinkering many times over and keeps a deck's history
/// under a few kilobytes per deck.
pub const DECK_VERSIONS_KEPT: usize = 20;
/// SPEC §11 R2100000: a save this soon after the deck's newest version replaces that version rather
/// than keeping another, so a burst of edits (the builder saves `DECK_AUTOSAVE_DEBOUNCE_MS` after
/// each pause) is one version. The window runs from the newest version's own save, which each
/// replacing save moves on.
pub const DECK_VERSION_DEBOUNCE_MS: i64 = 60_000;
/// SPEC §11 R339: the trio-code format's version. Version 2 writes each deck as a version 2 deck code
/// does (`catalog_number_set_offset`); a code naming a version other than this one or
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

/// R330, R333, R334: the series' wins needed, its most games and its pick clock are `jackioh-shared`'s
/// since #3098, which holds the rules that read them.
pub use jackioh_shared::config::{SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED};
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
/// How often the series screen and the match screen's series banner re-read the series, the banner
/// (and the rematch buttons, R3207002) only while no open match socket pushes it (R3213000).
pub const SERIES_POLL_SECONDS: i64 = 2;
/// SPEC §11 R3213000: the version of the match socket's post-game frames (`jackioh_shared::host::post_game`);
/// a client reads only the one it knows and polls otherwise, so a changed shape takes the next number.
pub const POST_GAME_FRAME_VERSION: u32 = 1;
/// SPEC §11 R3207001: how often the live-game banner (the landing page's and the practice page's) re-reads
/// the account for the game it names, in seconds. A game outlasts it by minutes, and a missed tick only
/// shows the banner a few seconds late; the queue follow keeps `SERIES_POLL_SECONDS` for the ticket's
/// sake (R897000).
pub const LIVE_GAME_POLL_SECONDS: i64 = 10;
/// How long the lobby shows "Match found! Taking you to your game…" before navigating to the
/// paired game, so the pairing reads as an arrival instead of a silent vanish. A `setStatus`
/// followed by a synchronous `navigate` never paints: the lobby unmounts in the same render
/// (`main.tsx` keys the gate and the error boundary by path).
pub const MATCH_FOUND_NAV_DELAY_MS: i64 = 1200;

// ---------------------------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320). The lessons themselves are the client's
// (`apps/web/src/tutorial/lessons.ts`); the server stores ids it does not interpret, so these only
// bound what one account can hold.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R320: the most completed-lesson ids one account keeps. The path has five lessons (R2205003); this
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

/// SPEC §11 R633: the most setting groups one account keeps (the client has five: gameplay, audio,
/// effects, card display and card cosmetics, R1295000). Mirrored in `app.settings` (0018).
pub const PLAYER_SETTINGS_GROUPS_MAX: usize = 8;
/// SPEC §11 R633: the most settings in one group.
pub const PLAYER_SETTINGS_KEYS_MAX: usize = 32;
/// SPEC §11 R633: the longest group id or setting name, in characters. Both are camelCase or kebab-case words.
pub const PLAYER_SETTINGS_NAME_MAX_LENGTH: usize = 40;
/// SPEC §11 R633: the most text the stored groups of one account may come to, in bytes. Four
/// groups of a dozen settings are well under a kilobyte; this bounds a body of junk. Mirrored in
/// `app.settings` (0018).
pub const PLAYER_SETTINGS_BYTES_MAX: usize = 4096;

// ---------------------------------------------------------------------------------------------
// Public card and player statistics (SPEC §9.11, R654).
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R654, amended by R1367000: the live games a card's stats window logs before its public
/// figures are live-only. The engine holds it, since the window function that applies it is the
/// engine's (`jackioh_engine::wire::stats::window`), shared with `cargo jackioh stats` and `winrates`.
pub use jackioh_engine::config::PUBLIC_STATS_MIN_LIVE_GAMES;

/// SPEC §9.11 (R377): the fewest in-deck games a card needs before `jackioh-server balance-watch`
/// may flag it. Sits above `CARD_STATS_MIN_SAMPLE` (20): a card under 20 games counts toward the
/// report's not-enough-games list and is never flagged, and a card with 20–39 games shows its
/// figures as watched but is never flagged either — the same 55% is noise at 20 games and a signal
/// at hundreds, so the flag reads a z-score against 50% (draws count as not won), not a fixed
/// win-rate cut-off.
pub const BALANCE_WATCH_MIN_GAMES: i64 = 40;
/// SPEC §9.11 (R377): the |z-score| against 50% at and above which `jackioh-server balance-watch`
/// flags a card with at least `BALANCE_WATCH_MIN_GAMES` in-deck games as too strong (high) or too
/// weak (low). Advisory only: the report files nothing and changes no card.
pub const BALANCE_WATCH_Z_THRESHOLD: f64 = 2.5;
/// SPEC §9.11 (R377): the |played delta| (played win rate minus drawn-but-not-played win rate, in
/// fraction points) at and above which `jackioh-server balance-watch` flags a card with at least
/// `BALANCE_WATCH_MIN_GAMES` in-deck games, whatever its in-deck win rate. A big gap here is the
/// sign of a card the AI pilot cannot use (trap timing, hand-dump spells), so the flag asks for
/// live human games, not a change.
pub const BALANCE_WATCH_PLAYED_DELTA_THRESHOLD: f64 = 0.15;

/// SPEC §11 R654: cost bucket boundary for card statistics filters.
/// Cost 6 represents the "6+" bucket (cards costing 6 or more), matching the deckbuilder's CURVE_TOP.
pub const CARD_STATS_CURVE_TOP: i32 = 6;

/// Cache-Control max-age in seconds for public card statistics endpoints.
pub const CARD_STATS_CACHE_TTL_SECONDS: i64 = 300;

/// SPEC §11 R978000, R2491000: how many computed reports each of the card statistics' kept stores
/// (`api::stats::KeptReports`) holds at once, each for `CARD_STATS_CACHE_TTL_SECONDS`; keeping
/// another drops the one kept longest ago. The card drill-down keeps at most 24 however many patches
/// ship, a patch history and the current patch's report for each of three pilots and four modes, so
/// the bound holds down the list route's windowed reports, one per patch, `?source=`, `?pilot=` and
/// `?mode=` asked for.
pub const CARD_STATS_KEPT_REPORTS: usize = 256;

/// SPEC §11 R654: how many co-played cards a card's drill-down (`GET /api/stats/cards/:id`) lists,
/// most games first.
pub const CARD_STATS_CO_PLAYED_TOP: usize = 10;

/// SPEC §11 R1583000: how long the server keeps the play rates (`GET /api/stats/play-rates`) before
/// reading them off the totals again, and the `Cache-Control` max-age it answers with, in seconds.
pub const PLAY_RATES_REFRESH_SECONDS: i64 = 3600;

/// SPEC §11 R1583000: the human live seats that must have held a card before its play rate is listed.
pub const PLAY_RATES_MIN_GAMES: i64 = 20;

/// SPEC §11 R1935000: the workflow whose runs `/stats`'s Daily runs tab lists (`GET /api/stats/ci`),
/// the daily super run, read in `GITHUB_ISSUES_REPO`'s repository.
pub const CI_RUNS_WORKFLOW: &str = "super.yml";

/// SPEC §11 R1935000: the branch whose runs of `CI_RUNS_WORKFLOW` the tab lists.
pub const CI_RUNS_BRANCH: &str = "main";

/// SPEC §11 R1935000: how many runs the tab lists, newest first: two weeks of daily runs.
pub const CI_RUNS_LISTED: usize = 14;

/// SPEC §11 R1935000: how long the server keeps what GitHub answered before asking again, the
/// `Cache-Control` max-age it answers with and how often the open tab asks again, in seconds. Without
/// a token GitHub allows one address 60 requests an hour, and this reads the runs at most 30 times.
pub const CI_RUNS_CACHE_TTL_SECONDS: i64 = 120;

/// SPEC §11 R1935000: the most jobs one read of a run attempt takes, GitHub's largest page.
pub const CI_RUNS_JOBS_PAGE: usize = 100;

/// GitHub's REST API, which the daily runs are read through.
pub const CI_RUNS_API_URL: &str = BUG_REPORT_API_URL;

/// How long one request to GitHub for the daily runs may take before the read fails.
pub const CI_RUNS_TIMEOUT_MS: u64 = 10_000;

/// R1044000: how many game records one transaction folds into the totals, both when `stats-import`
/// writes a run's file and when `stats-fold` (or the retention loop) folds the records written
/// before the totals. A batch rewrites each card row it touches once, so a bigger one rewrites
/// fewer rows per record; 500 keeps a transaction to a few seconds of the box's day of games.
pub const STATS_FOLD_BATCH_RECORDS: usize = 500;

/// Cache-Control max-age in seconds for public player statistics endpoints.
pub const PLAYER_STATS_CACHE_TTL_SECONDS: i64 = 60;

/// Maximum players per page returned by GET /api/stats/players.
pub const PLAYER_STATS_PAGE_LIMIT: usize = 50;

/// The most text the stored player statistics JSON of one account may come to, in bytes.
/// Bounds request body and stored payload (R1142000: raised from 16384 to 65536 so pruned catalog fits,
/// then to 131072 by the Training Grounds release, issue #2204, when 771 entries at four-digit
/// counters came to 67,445 bytes).
pub const PLAYER_STATS_BYTES_MAX: usize = 131_072;

// ---------------------------------------------------------------------------------------------
// Play telemetry (SPEC §9.11, R1442): how humans pace their moves, for the ladder bots (#636).
// ---------------------------------------------------------------------------------------------

/// R1442: the fewest human think times a bucket needs before `jackioh-server timing-fit` fits it.
pub const TIMING_FIT_MIN_SAMPLES: usize = 30;

// R639000: the rest of a person's habits a ladder bot copies, each one row that
// `jackioh-server timing-fit` prints from the telemetry and a person pastes over the row here by
// hand. A row with `samples: 0` is a hand-set prior, kept until a fit with
// `TIMING_FIT_MIN_SAMPLES` behind it replaces it.

/// R639000: the quantiles of the observed delays an emote delay window runs between.
pub const TIMING_FIT_DELAY_QUANTILES: (f64, f64) = (0.1, 0.9);

/// R639000: the quantile of the turns people conceded on that a concession's `min_turn` is.
pub const TIMING_FIT_CONCEDE_TURN_QUANTILE: f64 = 0.1;

// ---------------------------------------------------------------------------------------------
// Ladder bots (SPEC §9.5, §9.12, R1453, R1454, R1451, R1452; #638). What a bot decides, and so its numbers
// (the strengths and their seed ratings, the think times and habits, the personas, the pool, the
// fallback and the picks), is `crates/shared/src/config.rs`'s since #3103, re-exported at the top of this
// file; what is left here is the server's own: the calibration's, the pool's refresh and the stale start.
// ---------------------------------------------------------------------------------------------

/// R748000: games each pair of strengths plays in a calibration. At 40 the five strengths' offsets
/// carried errors of about 28 Elo, wider than the gaps between three of them.
pub const BOT_CALIBRATION_GAMES_PER_PAIR: usize = 100;
/// R748000: the fewest games a pair plays, and the fewest rated games against people the live anchor
/// read accepts.
pub const BOT_CALIBRATION_MIN_GAMES: usize = 30;
/// R748000: drawn games added to each pair's tally and to the live read, so a sweep (every game won
/// by one side) fits to a finite offset.
pub const BOT_CALIBRATION_PRIOR_GAMES: f64 = 1.0;
/// R748000: minorization-maximization steps of the Bradley-Terry fit.
pub const BOT_CALIBRATION_FIT_ITERATIONS: usize = 1000;
/// R748000: halvings of the live anchor's search for the rating that matches the score.
pub const BOT_CALIBRATION_BISECTIONS: usize = 100;
/// R748000: how far past the opponents' lowest and highest rating that search looks, in rating points.
pub const BOT_CALIBRATION_SEARCH_SPAN: f64 = 2000.0;
/// R748000: moves a calibration game plays before it is left out as unfinished.
pub const BOT_CALIBRATION_MAX_ACTIONS: usize = 3000;
/// R748000: actions one turn may take before its game is left out as unfinished. A calibration game
/// has no acting clock (R79) to end a turn, and a turn that goes on this long is a loop whose every
/// step costs more than the last: a University (Meditative 97.6) that Buffs itself raises its own
/// "Activate N" with each use, so the AI went on using it. Of 100 games sampled, the longest turn took
/// 20 actions.
pub const BOT_CALIBRATION_MAX_TURN_ACTIONS: usize = 30;
/// R1560000: the deviation each AI promotion adds to a ladder bot's rating (`jackioh-shared`'s, #3105).
pub use jackioh_shared::config::BOT_PROMOTION_DEVIATION_BOOST;

/// R1454: how often the server sizes the pool again (and once as it boots).
pub const BOT_POOL_REFRESH_INTERVAL_SECONDS: i64 = 3600;
/// R1555000: the rated wins a grinder needs while grinding to step up one strength.
pub const BOT_GRIND_STEP_GAMES: i32 = 10;
/// R1555000: the most matches between two bots that are live at once.
pub const BOT_VS_BOT_MAX_LIVE: usize = 1;
/// SPEC §11 R1989000: the stream label the seat coin is drawn on. Every 1v1 door seats its two
/// profiles by one draw of the match or series seed on `{seed}:seats` at cursor 0, beside the
/// dealt-deck streams (`{seed}:p1-deck`, R258) and the emote stream (`{seed}:emotes:{seat}`,
/// R1341), so a fixed seed always seats the same way and a replay folds unchanged.
pub const SEAT_COIN_LABEL: &str = "seats";
/// R1454: how old a match a bot was claimed for may be and still be `open` (claimed, never
/// started) before the queue's sweeper lets the bot and its player go and discards the match: a
/// start that a crash or a restart cut off. A start that goes on takes milliseconds.
pub const BOT_STALE_START_SECONDS: i64 = 60;

// ---------------------------------------------------------------------------------------------
// Ladder bots' cosmetics (SPEC §14.2, R1834005, R1834006; #1834). A bot wears a loadout drawn as a
// person's from a wardrobe drawn from the pool's seed, so its seat looks like a person's (R1452)
// while it owns nothing (R1454). That draw, its fits and its priors are `jackioh-shared`'s
// (`bots::cosmetics`, #3103), which the server re-exports from its `use` list above; the fits are
// pasted by hand from `jackioh-server cosmetics-fit` as `THINK_TIME_FITS` is from `timing-fit`, into
// `crates/shared/src/config.rs`. The one number only the server reads is the window that command reads.
// ---------------------------------------------------------------------------------------------

/// R1834006: how far back `cosmetics-fit` reads people's frozen loadouts, in days.
pub const BOT_COSMETIC_FIT_WINDOW_DAYS: i64 = 90;

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
/// R1443: `USERNAME_SIGN_IN_WINDOW_SECONDS` in milliseconds.
pub const USERNAME_SIGN_IN_WINDOW_MS: i64 = USERNAME_SIGN_IN_WINDOW_SECONDS * 1000;
/// R1450: `OPEN_MATCH_REAP_GRACE_SECONDS` in milliseconds.
pub const OPEN_MATCH_REAP_GRACE_MS: i64 = OPEN_MATCH_REAP_GRACE_SECONDS * 1000;

// ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----
// SPEC S7 (#936, Play together): friends, blocks, presence, challenges, watching and chat. PUBLIC:
// the client reads the heartbeat, the poll, the chat's caps and a challenge's life.
// ---------------------------------------------------------------------------------------------

/// B43: friend requests one account may send in an hour; the next is 429.
pub const FRIEND_REQUESTS_PER_HOUR: usize = 20;
/// B43: friends one account may hold; a request or an accept past it is 409 `friend_cap`.
pub const MAX_FRIENDS: i64 = 200;
/// B47: a friend is online while their last heartbeat is at most this old.
pub const PRESENCE_TTL_MS: i64 = 90_000;
/// B47, R3210000: how long a signed-in client goes with no signed-in request before it sends
/// `POST /api/presence`: half the TTL, so one lost beat never shows a friend offline.
pub const PRESENCE_HEARTBEAT_MS: i64 = 45_000;
/// R3210000: the least time between two writes of one account's presence row by the router's touch,
/// which every signed-in request makes. A client sends something at least every PRESENCE_HEARTBEAT_MS,
/// so two writes are at most this plus a heartbeat apart, under PRESENCE_TTL_MS: a friend never
/// flickers offline.
pub const PRESENCE_TOUCH_EVERY_MS: i64 = 30_000;
const _: () = assert!(PRESENCE_TOUCH_EVERY_MS + PRESENCE_HEARTBEAT_MS < PRESENCE_TTL_MS);
/// B49: how long a challenge, and the room it opened, stays open unanswered.
pub const CHALLENGE_TTL_MS: i64 = 120_000;
/// B50: how far behind the live game a watcher's view runs. Five seconds keeps each view
/// clearly behind live play, so a watcher cannot coach the moment, while feeling live.
pub const SPECTATE_DELAY_MS: i64 = 5_000;
/// B50: watchers one live match may hold at once.
pub const SPECTATORS_PER_MATCH: usize = 8;
/// R2990001: the emoji in a watcher's emote hand, dealt from the match seed and the watcher's own
/// account, with no voice line in it (a voice line would sound like a player speaking).
pub const SPECTATOR_EMOTE_HAND_SIZE: usize = 5;
/// R2990002: the least time between two of one watcher's emotes, twice a player's
/// (`EMOTE_COOLDOWN_MS`), so the sidelines never drown out the players.
pub const SPECTATOR_EMOTE_COOLDOWN_MS: i64 = 3_000;
/// R2990002: the rolling window `SPECTATOR_EMOTE_WINDOW_MAX` counts a watcher's emotes in.
pub const SPECTATOR_EMOTE_WINDOW_MS: i64 = 30_000;
/// R2990002: the most emotes one watcher sends in `SPECTATOR_EMOTE_WINDOW_MS`.
pub const SPECTATOR_EMOTE_WINDOW_MAX: usize = 3;
/// B53: a chat message's longest body, in characters after trimming (`chat_messages`' check).
pub const CHAT_MAX_CHARS: usize = 500;
/// B53: chat messages one account may send in a minute; the next is 429.
pub const CHAT_MESSAGES_PER_MINUTE: usize = 20;
/// B53: how long a chat message is kept before the purge deletes it, and its reports with it.
pub const CHAT_RETENTION_DAYS: i64 = 30;
/// B53: the messages one page of `GET /api/chat/:friendId` holds, newest last.
pub const CHAT_PAGE: i64 = 50;
/// B52 (#1825): the most shares one `GET /api/replays/shared` lists, newest first, so that list is
/// bounded like the friends (`MAX_FRIENDS`) and a chat page (`CHAT_PAGE`); fifty is a chat page's.
pub const SHARED_REPLAYS_LIST_MAX: i64 = 50;
/// B58: how often an open chat panel reads its conversation again, while the tab's `/ws/social` is
/// down (R3214000).
pub const CHAT_POLL_MS: i64 = 4_000;
/// §9.4: how often a signed-in client reads its unread counts and its challenges again (the challenge
/// toast, on every screen, and Play together). A challenge lives CHALLENGE_TTL_MS, so ten seconds
/// shows one within a twelfth of its life; the friends list itself is read once a heartbeat
/// (PRESENCE_HEARTBEAT_MS), the pace a status changes at. It runs only while the tab's `/ws/social`
/// is down (R3214000).
pub const SOCIAL_POLL_MS: i64 = 10_000;
/// R3214000: how often an open `/ws/social` checks its session again, as the upgrade checked it (the
/// token, the profile, §9.4's gate), so a session the provider ended (R194) or an account banned
/// while the socket stays open is cut off within one R194 window, as a polling tab is refused. No
/// longer than `AUTH_SESSION_LIVE_CACHE_SECONDS`. Server-only.
pub const SOCIAL_SESSION_CHECK_SECONDS: i64 = AUTH_SESSION_LIVE_CACHE_SECONDS;
const _: () = assert!(SOCIAL_SESSION_CHECK_SECONDS <= AUTH_SESSION_LIVE_CACHE_SECONDS);
/// R3214000: the `/ws/social` sockets one account may hold at once, one per signed-in tab; the next
/// is refused with 4403 "too many open tabs". Five: a person seldom keeps more tabs of the site open,
/// and a tab past it still reads by its polls. Server-only.
pub const SOCIAL_SOCKETS_PER_ACCOUNT: usize = 5;
/// §9.5, §9.8: challenges one account may send in an hour; the next is 429. Each opens a room and a
/// toast on the friend's screen, so a create-and-cancel loop is bounded; thirty is one every two
/// minutes, a challenge's whole life, kept up for an hour.
pub const CHALLENGES_PER_HOUR: usize = 30;
// NOT IN SPEC: how often the challenge sweeper deletes the challenges lapsed by now. A lapsed
// challenge is already shut (its room expired with it, the reaper takes the room's row, and no read
// lists the challenge), so the sweep only removes the rows; ten seconds keeps none much past its
// CHALLENGE_TTL_MS for one cheap delete.
pub const CHALLENGE_SWEEP_INTERVAL_SECONDS: i64 = 10;
// ---- end Social ----
// ---- Duos co-op (#946) ----
// Fullsend SPEC S6 (#936): the Duos co-op rooms and their four-seat actor (`src/duos/`). Every one
// but `DUOS_AI_BUDGET` is exported to `apps/web/src/wire/serverConfig.ts`, since the co-op board
// renders the same clocks and the ping wheel the same limits. The disconnect grace is the 1v1's `DISCONNECT_GRACE_SECONDS`.

// S6, B38, B2, B36: the co-op match's clocks and its AI's pace are the match host's (`jackioh_shared::host::duos`,
// #3101): the pure machine decides with them, and the server names them here as it always has.
pub use jackioh_shared::config::{
    DUOS_AI_STEP_DELAY_MS, DUOS_MULLIGAN_CLOCK_MS, DUOS_PROMPT_CLOCK_MS, DUOS_ROUND_CLOCK_MS,
};
/// R2575000: the search budget of each AI-played Duos seat (`duos::actor`, cut short at
/// `BOT_DECISION_DEADLINE_MS`): the practice AI's before issue #2575 doubled it, so the server's work
/// per AI step does not double. Raising it is a person's call. Only the server reads it.
pub const DUOS_AI_BUDGET: jackioh_ai::SearchBudget = jackioh_ai::SearchBudget {
    nodes: 600,
    lethal_nodes: 150,
    determinizations: 3,
    beam_width: 4,
    root_branching: 20,
    branching: 6,
    max_depth: 8,
    finalists: 3,
};
/// S6, B34: how long an unjoined co-op room stays open.
pub const DUOS_ROOM_TTL_MS: i64 = 600_000;
/// S6, B39: the pings one seat may send inside `PING_RATE_WINDOW_MS`; the rest are dropped silently.
pub const PING_RATE_LIMIT: usize = 5;
/// S6, B39: the sliding window `PING_RATE_LIMIT` is counted over.
pub const PING_RATE_WINDOW_MS: i64 = 3_000;
/// S6, B64: how long the board marks a pinged card.
pub const PING_SHOW_MS: i64 = 2_500;
// ---- end Duos co-op (#946) ----

// ---------------------------------------------------------------------------------------------
// The collection economy (SPEC §14, #1294; built by #1298). PUBLIC: the client reads prices,
// pity counters, bars, tracks and quests from these, so every one is exported to
// `apps/web/src/wire/serverConfig.ts` by `tests/export_config.rs` (CLAUDE.md rules 7, 9).
// The table in SPEC §14 lists every constant; the values are #1333's table, the owner's numbers
// brought in at the end of #1294, and #1339's drop tables, and where they leave a value to be
// derived the note below says how. No drop rate is tuned: every drop is a tiered roll on one of the
// weight tables below (R1339000), and the seeded simulation in `crates/shared/src/economy/math.rs` only reports what
// pity adds on top (R1339001) and derives both bars' points-per-duplicate from their target packs.
// ---------------------------------------------------------------------------------------------

// The rolls' numbers (the drop tables, pity, the token bars, the cosmetic packs and the track curve) are
// `jackioh_shared::config`'s since #3104: the pure economy decides with them, and the server names them
// here as it always has, so `serverConfig.ts` is unchanged.
pub use jackioh_shared::config::{
    CARD_TIER_WEIGHTS, CARD_TOKEN_BAR_SIZE, CARD_TOKEN_BAR_TARGET_PACKS, CHASE_SHARD_PACKS,
    COSMETIC_PACK_CARD_WEIGHTS, COSMETIC_PACK_FLOOR_RARITY, COSMETIC_PACK_SLOTS, COSMETIC_TIER_WEIGHTS,
    COSMETIC_TOKEN_BAR_SIZE_EPIC, COSMETIC_TOKEN_BAR_SIZE_LEGENDARY, COSMETIC_TOKEN_BAR_SIZE_PACK,
    COSMETIC_TOKEN_BAR_SIZE_RARE, COSMETIC_TOKEN_BAR_TARGET_PACKS_EPIC,
    COSMETIC_TOKEN_BAR_TARGET_PACKS_LEGENDARY, COSMETIC_TOKEN_BAR_TARGET_PACKS_PACK,
    COSMETIC_TOKEN_BAR_TARGET_PACKS_RARE, COSMETIC_TOKEN_CYCLE, CosmeticTokenStep,
    GLOBAL_COSMETIC_FLOOR_RARITY, GLOBAL_COSMETIC_ROLL_WEIGHT, GLOBAL_PACK_NAME, LEVEL_SKIP_WEIGHT,
    OFFER_SAMPLE_PERCENT_SCALE, PACK_FLOOR_RARITY, PACK_FLOORED_SLOT, PACK_SLOTS, PACK_UNOWNED_WEIGHTS,
    PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, PLAYER_SETTINGS_TEXT_MAX_LENGTH,
    PackUnownedWeights, RarityPoints, SET_TRACK_XP_PER_LEVEL, TIER_WEIGHT_TOTAL,
    TOKEN_BAR_POINTS_BY_CARD_RARITY, TOKEN_BAR_POINTS_BY_COSMETIC_RARITY, TierWeights,
    VERSION_OFFER_MIN_CARDS, VERSION_OFFER_MIN_PER_SET,
};
// What a finished game pays and how a quest moves (the Jlore and XP rates, the daily cap, the track levels, the
// quest rhythm and the day) are `jackioh_shared::config`'s since #3104: the pure economy decides with them, and
// the server names them here as it always has.
pub use jackioh_shared::config::{
    FLEET_TRACK_DECK_SHARE, FLEET_TRACK_LEVELS, FLEET_TRACK_NEXT_REWARDS_SHOWN, FLEET_TRACK_XP_DAILY_MAX,
    FLEET_TRACK_XP_PER_LAUNCH, FLEET_TRACK_XP_PER_LEVEL, FLEET_TRACK_XP_PER_PIECE_FUSED,
    FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY, FLEET_TRACK_XP_STINKYSCAPE_WIN, JLORE_CONQUEST_LOSS,
    JLORE_CONQUEST_WIN, JLORE_COOP_LOSS, JLORE_COOP_WIN, JLORE_GAMES_DAILY_CAP, JLORE_LADDER_LOSS,
    JLORE_LADDER_WIN, JLORE_PRACTICE_LOSS, JLORE_PRACTICE_WIN, JLORE_UNRANKED_LOSS, JLORE_UNRANKED_WIN,
    QUEST_INTERVAL_HOURS, QUEST_JLORE_MAX, QUEST_JLORE_MIN, QUEST_MAX_HELD, QUEST_REPLACES_PER_DAY,
    QUEST_XP_PER_JLORE, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL, XP_COOP_SCALE,
    XP_GAME_CAP, XP_LADDER_TARGET_MEAN, XP_PER_ACTION, XP_PRACTICE_SCALE, XP_WIN_BONUS,
};
/// SPEC §14.3: the Jlore a card pack costs (#1333's table: 100), five ladder wins.
pub const PACK_PRICE_JLORE: i32 = 100;
/// SPEC §14.3, R3212000: the most packs one Open all (`POST /api/packs/open-all`) opens. Each pack
/// writes about seven ledger rows, each carrying its opening's log (tens of KB) as its `ref`, and
/// the batch binds them all in one statement per ledger, so twenty keeps a statement to a few MB;
/// a player holding more presses again.
pub const PACKS_OPEN_ALL_MAX: usize = 20;
/// SPEC §14.14, R3013000: the total plays of one card from hand, in games that pay rewards, that
/// reach each of its proficiency levels, lowest first (#3013: the first comes in a game or two, the
/// last after 200 plays, a season's commitment to one card). Level n unlocks stamp n (R3013001).
pub const PROFICIENCY_LEVEL_PLAYS: [i64; 5] = [3, 15, 40, 100, 200];
/// SPEC §14.14, R3013000: a card's proficiency levels, one stamp each (R3013001).
pub const PROFICIENCY_LEVELS: usize = PROFICIENCY_LEVEL_PLAYS.len();
/// SPEC §14.8: how long a practice seed waits to be claimed, in seconds. #1294's table gives no
/// value; an hour outlasts any game without holding seeds.
pub const PRACTICE_SEED_TTL_SECONDS: i64 = 3600;
/// SPEC §14.11, R2124000: hours a spend's idempotency key and its answer are kept before the
/// retention loop sweeps them. A day outlasts any retry a player makes after a slow answer
/// (`API_REQUEST_TIMEOUT_SECONDS`) while keeping the table to a day of spends.
pub const IDEMPOTENCY_KEY_TTL_HOURS: i64 = 24;
/// SPEC §14.11, R2378000: how many of one account's economy requests run at once: its spends (a
/// pack bought or opened, a token redeemed, a cosmetic pack opened, a chase bought) and the other
/// routes that write under its economy lock (a card's picks saved or uploaded, a practice seed
/// asked for or given back, a quest replaced; issue #2610). One sent while they run waits its turn
/// in this process, holding no database connection, so a burst of presses cannot hold the pool's
/// connections waiting on the account's economy lock (issue #2378).
pub const ECONOMY_SPENDS_RUNNING_MAX: usize = 1;
/// SPEC §14.11, R2378000: how many of one account's economy requests may wait their turn behind
/// the ones running. One sent past it is refused at once with 429 and writes nothing. Four cover a
/// person's tabs and devices pressing together; a script's burst is refused past them.
pub const ECONOMY_SPENDS_WAITING_MAX: usize = 4;
/// SPEC §14.11, R2378000: the wait the refusal of a spend sent past `ECONOMY_SPENDS_WAITING_MAX`
/// states (`retryAfterMs`, R192).
pub const SPEND_BUSY_RETRY_AFTER_MS: i64 = 1_000;
/// SPEC §14.8, R1483000: how many unclaimed, unexpired practice seeds an account holds at once; a
/// request past it is refused and issues nothing. Bounds one account's place in the claim queue
/// (`PRACTICE_VERIFY_CONCURRENCY`). The approved proposal's number (#1483).
pub const PRACTICE_SEEDS_OPEN_MAX: i64 = 3;
/// SPEC §14.8, #1303: how many practice claims the server verifies at once, each a fold of the
/// game's log off the runtime (`economy::practice`, R2560000). A claim past it
/// waits for a verification to finish, and none is dropped. #1294's table gives no value; two keep
/// the blocking pool free for the live matches' folds.
pub const PRACTICE_VERIFY_CONCURRENCY: usize = 2;
/// SPEC §14.8, R1688000: how many times the server tries a passed practice claim's pay (its
/// rewards and its last board, one transaction) before it answers an error. Each try is the whole
/// transaction, so one that fails pays nothing and the try after it pays once; the seed stays
/// burned whatever happens (R1303001). Three tries outlast a blip in the store, as R1437's do.
pub const PRACTICE_PAY_ATTEMPTS: usize = 3;
/// SPEC §14.8, R1688000: the wait before the second try at a practice claim's pay, doubled before
/// each try after it (250 ms, then 500 ms), as `MATCH_RECORD_RESULT_BACKOFF_MS` waits.
pub const PRACTICE_PAY_BACKOFF_MS: i64 = 250;
/// SPEC §14.8, R2422000: how long a practice claim may stay unsettled (neither paid nor refused)
/// after it burned its seed before the retention loop settles it: a claim whose pay every try
/// failed, or whose server stopped or crashed under it. Well past what a live settle takes (a wait
/// for a verification permit, the check and the pay's tries), so the sweep never takes a claim a
/// live settle is still working on; ten minutes, as `GAME_RECORD_REFILE_GRACE_SECONDS` waits.
pub const PRACTICE_SETTLE_GRACE_SECONDS: i64 = 10 * 60;

// ---- Economy (SPEC §14, #1299) ----

/// §14.12: the server's switch for the economy, on since the going-live sub-issue (#1310,
/// R1310000): finished games pay (R1302006), quests arrive (R1304002) and the client prints a full
/// art only on its owner's own copies (R1313000). Exported to `serverConfig.ts`.
pub const ECONOMY_LIVE: bool = true;

// ---- News (SPEC §14.13, #2875) ----

/// §14.13, R2875001: how many days a news entry lives after its `postedAt`, on the server's clock:
/// past them it leaves `GET /api/news` and its reward can no longer be claimed. Exported to
/// `serverConfig.ts`.
pub const NEWS_TTL_DAYS: i64 = 14;
/// §14.13: the most sentences a news entry's summary holds (`economy::news::parse` refuses more).
pub const NEWS_SUMMARY_MAX_SENTENCES: usize = 2;
/// §14.13, R2875003: the Jlore a news entry's Jlore reward pays, a ladder win's worth
/// (`JLORE_LADDER_WIN`), since patches ship several a day. Exported to `serverConfig.ts`.
pub const NEWS_REWARD_JLORE: i64 = 20;
/// §14.13, R2875003: the unopened card packs a news entry's card pack reward adds to the
/// inventory. Exported to `serverConfig.ts`.
pub const NEWS_REWARD_CARD_PACKS: i64 = 1;
/// §14.13, R2875003: the card tokens a news entry's card token reward adds. Exported to
/// `serverConfig.ts`.
pub const NEWS_REWARD_CARD_TOKENS: i64 = 1;
/// §14.13, R2875003: the cosmetic tokens of its tier a news entry's cosmetic token reward adds.
/// Exported to `serverConfig.ts`.
pub const NEWS_REWARD_COSMETIC_TOKENS: i64 = 1;
/// §14.13, R2875003: the unopened cosmetic packs a news entry's cosmetic pack reward adds.
/// Exported to `serverConfig.ts`.
pub const NEWS_REWARD_COSMETIC_PACKS: i64 = 1;

/// §14.12, R1310000, R1705002: the starter cards, what a new account owns on becoming active
/// (R111): the 50 cards of the three practice presets (`apps/web/src/practice/decks.ts`), so a new
/// account can queue any of them. `seed-catalog` writes them to `app.settings.starter_cards`, which
/// migration 0050's launch grant reads, and the in-memory store reads them here; migration 0055
/// adds the eleven the presets gained at 25 cards to the stored list and grants them to every
/// active account. Exported to `serverConfig.ts`.
pub const STARTER_CARDS: &[&str] = &[
    "core-001", "core-002", "core-003", "core-004", "core-005", "core-008", "core-009", "core-010",
    "core-011", "core-012", "core-013", "core-014", "core-015", "core-016", "core-019", "core-020",
    "core-025", "core-030", "core-031", "core-032", "core-034", "core-035", "core-036", "core-037",
    "core-041", "core-043", "core-044", "core-045", "core-047", "core-049", "core-053", "core-054",
    "core-056", "core-058", "core-061", "core-063", "core-066", "core-067", "core-068", "core-069",
    "core-070", "core-072", "core-073", "core-074", "core-077", "core-080", "core-081", "core-088",
    "core-089", "core-091",
];

// ---- Dev accounts (SPEC §14.10, #1300) ----

/// §14.10, R1300000: the environment variable that lists the dev accounts, comma-separated profile
/// ids (`env.rs` reads it at boot, `seed-catalog` for R1300001's exemption). A name, not a number,
/// and never exported: the client learns only whether its own caller is one (`isDev`).
pub const DEV_PROFILE_IDS: &str = "DEV_PROFILE_IDS";
/// §14.10, R3074000: the environment variable that sets the dev dashboard's strict mode, `on` or
/// `off`, unset meaning `on`. Strict, every `/api/admin/*` route needs the two-step code (`aal2`)
/// and a hand-set rating is off the public ladder until it is reset (R3074001). Production leaves it
/// unset; staging's `wrangler.jsonc` vars and end-to-end mode set `off`. A name, never exported.
pub const DEV_STRICT: &str = "DEV_STRICT";
/// §14.10, R3074000: the most Jlore one `POST /api/dev/jlore` gives. Exported to `serverConfig.ts`.
pub const DEV_JLORE_PER_REQUEST_MAX: i64 = 1_000_000;
/// §14.10, R3074000: the most XP one `POST /api/dev/xp` gives a track. Exported to
/// `serverConfig.ts`.
pub const DEV_XP_PER_REQUEST_MAX: i64 = 1_000_000;
/// §14.10, R3074001: the lowest rating a dev account may hand-set itself on a ladder. Exported to
/// `serverConfig.ts`.
pub const DEV_RATING_MIN: f64 = 100.0;
/// §14.10, R3074001: the highest rating a dev account may hand-set itself on a ladder, past every
/// bot's (`SKILL_ELO_TABLE`). Exported to `serverConfig.ts`.
pub const DEV_RATING_MAX: f64 = 3000.0;
/// §14.10, R3074000: the newest audit rows `GET /api/admin/audit` answers.
pub const ADMIN_AUDIT_PAGE: i64 = 200;
/// §14.13, R3074003: the newest posts `GET /api/admin/news` answers, taken down or not.
pub const NEWS_POSTS_PAGE: i64 = 100;
/// §14.13, R3074003: the most characters a post's title holds. Exported to `serverConfig.ts`.
pub const NEWS_POST_TITLE_MAX_CHARS: usize = 80;
/// §14.13, R3074003: the most characters a post's summary holds. Exported to `serverConfig.ts`.
pub const NEWS_POST_SUMMARY_MAX_CHARS: usize = 400;

// ---- The match actor's speed (#3429) ----
// `tests/actor/perf.rs`, daily: bot-against-bot matches played at once through the match actor on
// the in-memory store. Not in SPEC, and no R-row. Never exported to `serverConfig.ts`.

/// #3429: the matches the daily perf test plays at once.
pub const SERVER_PERF_MATCHES: usize = 16;
/// #3429: the matches the per-pull-request smoke test plays at once, so the harness keeps working.
pub const SERVER_PERF_SMOKE_MATCHES: usize = 2;
/// #3429: the worker threads of the runtime the perf tests play on: a GitHub `ubuntu-latest`
/// runner's four vCPUs.
pub const SERVER_PERF_WORKER_THREADS: usize = 4;
/// #3429: the most actions a perf match takes before its test fails it as a stall (golden's
/// `STEP_CAP`, §13.1).
pub const SERVER_PERF_ACTION_CAP: usize = 3_000;
/// #3429: the most the median action may take from `submit` to its answer, which comes only after
/// the actor has pushed both seats' views, in milliseconds, at `cargo test`'s profile. Measured on
/// 2026-11-09 on a 4-vCPU GitHub Actions ubuntu runner (AMD EPYC 7763), `SERVER_PERF_MATCHES` at once,
/// in eleven runs: medians from 88.6 to 98.75 ms (91.77, 92.24 and 92.43 in three in a row). 5x the
/// most, rounded up to two significant figures. The in-memory store's transactions take one lock and
/// copy every table (`db/fake.rs`), so the matches queue on it: a run kept about 1.3 of the 4 cores
/// busy (22.7 s of CPU in 17.8 s).
pub const SERVER_PERF_BROADCAST_MS_BUDGET: f64 = 500.0;
/// #3429: the fewest matches per second the actor must finish, `SERVER_PERF_MATCHES` at once, at
/// `cargo test`'s profile. Measured in the same eleven runs: 16 matches of 1,887 actions in all, in
/// 16.5 to 18.68 s, so 0.86 to 0.97 a second. The least divided by 5, rounded down.
pub const SERVER_PERF_MATCHES_PER_SECOND_FLOOR: f64 = 0.17;
