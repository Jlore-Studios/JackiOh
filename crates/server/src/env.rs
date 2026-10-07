//! SERVER-ONLY (← `apps/server/src/env.ts`, SURFACE §11.1): every environment variable, its default
//! and its refusal. It reads secrets — the Supabase secret key, the raw Postgres connection string,
//! the invite-code/IP-hash pepper — that bypass every RLS policy. SPEC §9.1 makes the server the sole
//! writer of collection, loadouts and matches; that guarantee only holds if these values never reach
//! a browser bundle, and nothing in `apps/web` reads this module (its generated constants come from
//! `config.rs` only, SURFACE §5.1).
//!
//! It reads only `./config` (for R190's two proxy-hop bounds), the compiled-in catalog version
//! (SURFACE §11.3) and the map handed to it; `server_env()` alone reads the process environment.

use std::sync::OnceLock;

use indexmap::IndexMap;

use crate::config::{DEFAULT_TRUSTED_PROXY_HOPS, MAX_TRUSTED_PROXY_HOPS};

/// The deployment environment (TS `"development" | "test" | "production"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeEnv {
    Development,
    Test,
    Production,
}

impl NodeEnv {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeEnv::Development => "development",
            NodeEnv::Test => "test",
            NodeEnv::Production => "production",
        }
    }
}

impl std::fmt::Display for NodeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The server's fully validated, typed environment (SPEC §9.1, §9.4, §9.5); TS `ServerEnv`, whose
/// SCREAMING field names are snake_cased here (`SUPABASE_URL` → `supabase_url`).
#[derive(Clone, Debug, PartialEq)]
pub struct Env {
    /// Supabase project URL, e.g. https://<ref>.supabase.co. Server-only by convention (no VITE_ prefix).
    pub supabase_url: String,
    /// Supabase secret key (or legacy service_role JWT). Server-only: bypasses every RLS policy.
    pub supabase_secret_key: String,
    /// Direct Postgres connection string for transactional work (§9.4, §9.5). Server-only.
    pub database_url: String,
    /// JWKS endpoint used to verify browser Supabase Auth JWTs. Defaults from SUPABASE_URL.
    pub supabase_jwks_url: String,
    /// Legacy HS256 shared-secret fallback for JWT verification, if configured. Discouraged.
    pub supabase_jwt_secret: Option<String>,
    /// Server-side pepper for the invite-code / IP-hash HMAC (§9.4). Server-only, secret.
    pub code_pepper: String,
    /// Port the server listens on.
    pub port: u16,
    /// Allowed browser origins for CORS and WebSocket `Origin` checks.
    pub public_origins: Vec<String>,
    /// Deployment environment.
    pub node_env: NodeEnv,
    /// BUILD M8's E2E=1 test-server mode (fixture accounts, seeded games). Must be false in prod.
    pub e2e: bool,
    /// Catalog version this server accepts; must match `cards.catalog_version`, the client's (§9.4)
    /// and, since v0.3.0, the version compiled into this binary (`jackioh_cards::catalog_version()`,
    /// SURFACE §11.3).
    pub catalog_version: String,
    /// SPEC §11 R190: how many `X-Forwarded-For` entries, counted from the right, this deployment's
    /// own proxies wrote. 0 to `MAX_TRUSTED_PROXY_HOPS`; defaults to `DEFAULT_TRUSTED_PROXY_HOPS`; 0
    /// ignores the header and keys every request on the socket's peer address.
    pub trusted_proxy_hops: usize,
    /// The git commit this build is running: Render's `RENDER_GIT_COMMIT`, which it sets on every
    /// deploy and which is unset anywhere else (`None` here). `GET /api/catalog` reports it in the
    /// `x-deployed-commit` response header so `deploy-watch.yml` can tell a server that is on the
    /// pushed commit from one Render never redeployed, which the catalog version alone cannot show
    /// when a push leaves the catalog as it was. Only a hex string is kept, since it is echoed into a
    /// header.
    pub deployed_commit: Option<String>,
}

/// TS's name for [`Env`].
pub type ServerEnv = Env;

/// TS `loadEnv`'s throw: every problem found, not just the first. `Display` is TS's message, one
/// problem per line, each naming the variable, what it is for, and where to obtain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvError {
    pub problems: Vec<String>,
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lines: Vec<String> = self.problems.iter().map(|problem| format!("  - {problem}")).collect();
        write!(
            f,
            "Invalid server environment — {} problem(s):\n{}",
            self.problems.len(),
            lines.join("\n")
        )
    }
}

impl std::error::Error for EnvError {}

/// The client-visible half of the environment contract (`apps/web`, via Vite's `VITE_` prefix
/// convention — Vite only exposes prefixed variables to the browser bundle). Listed here for
/// documentation only: this module never reads these, and none of them may carry a secret.
/// `SUPABASE_SECRET_KEY` in particular must never be given a `VITE_` alias.
pub const PUBLIC_ENV_VARS: &[&str] = &[
    "VITE_SUPABASE_URL",
    "VITE_SUPABASE_PUBLISHABLE_KEY",
    "VITE_SERVER_HTTP_URL",
    "VITE_SERVER_WS_URL",
    "VITE_CATALOG_VERSION",
    // R666: which OAuth providers the sign-in screen offers, by name only (no client id, no secret).
    "VITE_AUTH_OAUTH_PROVIDERS",
];

/// The server-only half: every variable `load_env` below reads. A lint or test can assert this
/// list and `PUBLIC_ENV_VARS` are disjoint and that none of these ever gain a `VITE_` prefix.
pub const SERVER_ONLY_ENV_VARS: &[&str] = &[
    "SUPABASE_URL",
    "SUPABASE_SECRET_KEY",
    "DATABASE_URL",
    "SUPABASE_JWKS_URL",
    "SUPABASE_JWT_SECRET",
    "CODE_PEPPER",
    "PORT",
    "PUBLIC_ORIGINS",
    "NODE_ENV",
    "E2E",
    "CATALOG_VERSION",
    "TRUSTED_PROXY_HOPS",
    "RENDER_GIT_COMMIT",
];

const MIN_CODE_PEPPER_LENGTH: usize = 32;
const DEFAULT_PORT: u16 = 8787;
const DEFAULT_NODE_ENV: NodeEnv = NodeEnv::Development;

/// Where every secret in this contract comes from, for error messages only — never logged with a value.
const SUPABASE_DASHBOARD_HINT: &str =
    "in the Supabase dashboard under Project Settings > API (or > Data API for newer projects)";

/// JS `String.prototype.trim`: strips JS's WhiteSpace and LineTerminator code points (which differ
/// from Rust's `char::is_whitespace` at U+0085 and U+FEFF).
fn js_trim(value: &str) -> &str {
    fn is_js_space(c: char) -> bool {
        matches!(
            c,
            '\u{0009}'
                | '\u{000B}'
                | '\u{000C}'
                | '\u{0020}'
                | '\u{00A0}'
                | '\u{FEFF}'
                | '\u{000A}'
                | '\u{000D}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200A}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
        )
    }
    value.trim_matches(is_js_space)
}

/// JS `JSON.stringify` of a string, for the `(got …)` part of a message.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// JS `value.length`: UTF-16 code units.
fn js_length(value: &str) -> usize {
    value.encode_utf16().count()
}

/// JS `Number(text)` for the strings an environment variable can hold: whitespace-trimmed, empty is
/// 0, `0x`/`0o`/`0b` integer literals, `Infinity` with an optional sign, otherwise a decimal literal;
/// anything else is NaN.
fn js_number(text: &str) -> f64 {
    let trimmed = js_trim(text);
    if trimmed.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(digits) = trimmed.strip_prefix(prefix) {
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return digits
                .chars()
                .fold(0.0, |total, c| total * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0)));
        }
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let decimal = trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'));
    if !decimal {
        return f64::NAN;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

fn is_non_empty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !js_trim(text).is_empty())
}

/// TS `new URL(value)` with `protocol === "https:"` (the WHATWG parser, which `reqwest::Url` is).
fn parse_https_url(value: &str) -> Option<reqwest::Url> {
    reqwest::Url::parse(value).ok().filter(|url| url.scheme() == "https")
}

fn parse_port(value: Option<&str>, problems: &mut Vec<String>) -> u16 {
    let Some(raw) = value.filter(|text| is_non_empty(Some(text))) else {
        return DEFAULT_PORT;
    };
    let parsed = js_number(raw);
    if parsed.fract() != 0.0 || !parsed.is_finite() || parsed <= 0.0 || parsed > 65535.0 {
        problems.push(format!(
            "PORT: the TCP port the server listens on. Must be an integer between 1 and 65535 (got {}). Optional; defaults to {DEFAULT_PORT} if unset.",
            quoted(raw)
        ));
        return DEFAULT_PORT;
    }
    parsed as u16
}

fn parse_node_env(value: Option<&str>, problems: &mut Vec<String>) -> NodeEnv {
    let Some(raw) = value.filter(|text| is_non_empty(Some(text))) else {
        return DEFAULT_NODE_ENV;
    };
    match raw {
        "development" => NodeEnv::Development,
        "test" => NodeEnv::Test,
        "production" => NodeEnv::Production,
        _ => {
            problems.push(format!(
                "NODE_ENV: the deployment environment. Must be one of 'development', 'test' or 'production' (got {}). Optional; defaults to '{DEFAULT_NODE_ENV}'.",
                quoted(raw)
            ));
            DEFAULT_NODE_ENV
        }
    }
}

fn parse_e2e(value: Option<&str>, problems: &mut Vec<String>) -> bool {
    let Some(raw) = value.filter(|text| is_non_empty(Some(text))) else {
        return false;
    };
    match raw {
        "1" | "true" => true,
        "0" | "false" => false,
        _ => {
            problems.push(format!(
                "E2E: BUILD M8's test-server flag (fixture accounts, seeded games). Must be one of '1', 'true', '0' or 'false' (got {}). Optional; defaults to false. Set by the e2e test runner, never by a production deploy.",
                quoted(raw)
            ));
            false
        }
    }
}

fn parse_trusted_proxy_hops(value: Option<&str>, problems: &mut Vec<String>) -> usize {
    let Some(raw) = value.filter(|text| is_non_empty(Some(text))) else {
        return DEFAULT_TRUSTED_PROXY_HOPS;
    };
    let trimmed = js_trim(raw);
    // TS: `/^\d+$/u.test(trimmed)` and `Number(trimmed) > MAX_TRUSTED_PROXY_HOPS`. A digit string too
    // long for a u64 is far above the bound, as `Number` would read it.
    let digits = !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit());
    let parsed = if digits { trimmed.parse::<u64>().ok() } else { None };
    match parsed {
        Some(hops) if hops <= MAX_TRUSTED_PROXY_HOPS as u64 => hops as usize,
        _ => {
            problems.push(format!(
                "TRUSTED_PROXY_HOPS: how many X-Forwarded-For entries, counted from the right, this deployment's own proxies write (R190; Render's edge is 1). Must be an integer from 0 to {MAX_TRUSTED_PROXY_HOPS} (got {}). Optional; defaults to {DEFAULT_TRUSTED_PROXY_HOPS}. 0 ignores the header and keys every request on its peer address. The api.forwarded_for log lines report the fewest entries any request carried; this must not be more than that.",
                quoted(raw)
            ));
            DEFAULT_TRUSTED_PROXY_HOPS
        }
    }
}

/// Optional, and never a problem: anything that is not a git SHA is simply not reported.
fn parse_deployed_commit(value: Option<&str>) -> Option<String> {
    let trimmed = js_trim(value?).to_lowercase();
    let length = trimmed.len();
    let hex = trimmed.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    ((7..=64).contains(&length) && hex).then_some(trimmed)
}

fn parse_public_origins(value: Option<&str>, problems: &mut Vec<String>) -> Vec<String> {
    let Some(raw) = value.filter(|text| is_non_empty(Some(text))) else {
        problems.push(
            "PUBLIC_ORIGINS: comma-separated browser origins allowed for CORS and the WebSocket Origin check (e.g. https://app.example.com,https://staging.example.com). Set by you for this deployment; there is no dashboard value to copy."
                .to_string(),
        );
        return Vec::new();
    };
    raw.split(',')
        .map(js_trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect()
}

/// Validates and types the process environment (SPEC §9.1, §9.4, §9.5). Pure with respect to
/// `source` so it is testable without touching the process environment. On any problem it returns
/// one `EnvError` listing every problem found — not just the first — one per line, each naming the
/// variable, what it is for, and where to obtain it.
pub fn load_env(source: &IndexMap<String, String>) -> Result<Env, EnvError> {
    let read = |name: &str| source.get(name).map(String::as_str);
    let mut problems: Vec<String> = Vec::new();

    let supabase_url_raw = read("SUPABASE_URL");
    let mut supabase_url = String::new();
    match supabase_url_raw {
        Some(raw) if is_non_empty(Some(raw)) => {
            if parse_https_url(raw).is_none() {
                problems.push(format!(
                    "SUPABASE_URL: must be a valid https URL (e.g. https://<ref>.supabase.co) (got {}). Find it {SUPABASE_DASHBOARD_HINT}.",
                    quoted(raw)
                ));
            } else {
                supabase_url = raw.to_string();
            }
        }
        _ => problems.push(format!(
            "SUPABASE_URL: the project URL (e.g. https://<ref>.supabase.co), used to reach the Data API and to derive the default JWKS URL. Find it {SUPABASE_DASHBOARD_HINT}."
        )),
    }

    let secret_key_raw = read("SUPABASE_SECRET_KEY");
    if !is_non_empty(secret_key_raw) {
        problems.push(format!(
            "SUPABASE_SECRET_KEY: the secret API key (sb_secret_...), which maps to the Postgres service_role and bypasses every RLS policy — SPEC §9.1 relies on this so the server is the only writer of collection, loadouts and matches. A legacy service_role JWT is accepted as a deprecated fallback. Find it {SUPABASE_DASHBOARD_HINT}, in the 'Secret keys' (or legacy 'service_role') section. Never expose this to a client bundle."
        ));
    }

    let database_url_raw = read("DATABASE_URL");
    if !is_non_empty(database_url_raw) {
        problems.push(format!(
            "DATABASE_URL: a direct Postgres connection string, used for the transactional work the Data API cannot express — invite redemption (§9.4), saveLoadout's all-or-nothing write (§9.4) and the atomic matchmaking ticket claim (§9.5). Find it {SUPABASE_DASHBOARD_HINT}, under Project Settings > Database > Connection string (use the pooled/transaction connection string for a serverless deployment)."
        ));
    }

    let jwks_url_raw = read("SUPABASE_JWKS_URL");
    let mut supabase_jwks_url = String::new();
    match jwks_url_raw {
        Some(raw) if is_non_empty(Some(raw)) => {
            if parse_https_url(raw).is_none() {
                problems.push(format!(
                    "SUPABASE_JWKS_URL: optional, but if set must be a valid https URL (got {}). Defaults to '${{SUPABASE_URL}}/auth/v1/.well-known/jwks.json' when unset.",
                    quoted(raw)
                ));
            } else {
                supabase_jwks_url = raw.to_string();
            }
        }
        _ => {
            if !supabase_url.is_empty() {
                // Asymmetric verification (RS256/ES256) against this JWKS is the supported path for
                // checking a browser's Supabase Auth JWT; only the verified `sub` is trusted as the
                // profile id. A JWKS response must never be cached longer than Supabase's own
                // 10-minute edge cache, or a just-rotated key would still verify old, revoked tokens.
                supabase_jwks_url = format!("{supabase_url}/auth/v1/.well-known/jwks.json");
            }
            // else: SUPABASE_URL itself already failed above; that problem is reported once, not twice.
        }
    }

    let jwt_secret_raw = read("SUPABASE_JWT_SECRET");
    // SUPABASE_JWT_SECRET is optional and its presence enables a legacy HS256 (shared-secret)
    // verification path, which is discouraged in favor of the JWKS/RS256/ES256 path above:
    // a shared secret that leaks lets an attacker mint arbitrary profile ids, whereas the JWKS
    // path only ever needs to trust Supabase's published public keys.
    let supabase_jwt_secret = jwt_secret_raw.filter(|raw| is_non_empty(Some(raw))).map(str::to_string);

    let code_pepper_raw = read("CODE_PEPPER");
    let mut code_pepper = String::new();
    match code_pepper_raw {
        Some(raw) if is_non_empty(Some(raw)) => {
            if js_length(raw) < MIN_CODE_PEPPER_LENGTH {
                problems.push(format!(
                    "CODE_PEPPER: must be at least {MIN_CODE_PEPPER_LENGTH} characters long so a weak pepper fails at boot, not in production (got a value of length {}). Generate one with e.g. `openssl rand -base64 48`.",
                    js_length(raw)
                ));
            } else {
                code_pepper = raw.to_string();
            }
        }
        _ => problems.push(format!(
            "CODE_PEPPER: the server-side pepper for the HMAC that turns an invite code into invite_codes.code_hash and an IP address into code_attempts.ip_hash (§9.4: codes are stored hashed). Must be at least {MIN_CODE_PEPPER_LENGTH} characters of random data (e.g. `openssl rand -base64 48`). This is not a Supabase dashboard value — generate and store it yourself as a deployment secret."
        )),
    }

    let port = parse_port(read("PORT"), &mut problems);
    let node_env = parse_node_env(read("NODE_ENV"), &mut problems);
    let e2e = parse_e2e(read("E2E"), &mut problems);
    let public_origins = parse_public_origins(read("PUBLIC_ORIGINS"), &mut problems);
    let trusted_proxy_hops = parse_trusted_proxy_hops(read("TRUSTED_PROXY_HOPS"), &mut problems);

    if e2e && node_env == NodeEnv::Production {
        problems.push(
            "E2E: must not be set together with NODE_ENV=production — BUILD M8's fixture accounts and seeded games must never be reachable in a production deployment."
                .to_string(),
        );
    }

    let catalog_version_raw = read("CATALOG_VERSION");
    let mut catalog_version = String::new();
    match catalog_version_raw {
        Some(raw) if is_non_empty(Some(raw)) => {
            // SURFACE §11.3: the catalog is compiled in, so the environment's version must name it.
            // One honest check instead of render.yaml's start-command derivation: a stale dashboard
            // value refuses to boot rather than being served.
            let compiled = jackioh_cards::catalog_version();
            if raw != compiled {
                problems.push(format!(
                    "CATALOG_VERSION: must equal the catalog version this build was compiled with ({}, the newest patch in crates/cards/patches/patches.json) (got {}). A server cannot accept saves and queues for a catalog it does not carry.",
                    quoted(compiled),
                    quoted(raw)
                ));
            } else {
                catalog_version = raw.to_string();
            }
        }
        _ => problems.push(
            "CATALOG_VERSION: the catalog version this server accepts (§9.4: a stale catalog version is rejected at save and queue). Must match the catalog_version the seed loader stamps on public.cards and the version the client sends. Not a Supabase dashboard value — set it to the version of crates/cards/catalog.json this deploy ships."
                .to_string(),
        ),
    }

    if !problems.is_empty() {
        return Err(EnvError { problems });
    }

    Ok(Env {
        supabase_url,
        supabase_secret_key: secret_key_raw.unwrap_or("").to_string(),
        database_url: database_url_raw.unwrap_or("").to_string(),
        supabase_jwks_url,
        supabase_jwt_secret,
        code_pepper,
        port,
        public_origins,
        node_env,
        e2e,
        catalog_version,
        trusted_proxy_hops,
        deployed_commit: parse_deployed_commit(read("RENDER_GIT_COMMIT")),
    })
}

static CACHED_ENV: OnceLock<Env> = OnceLock::new();

/// Memoised accessor for the process's own environment. Reads the process environment until one
/// read succeeds, then never again (TS caches only a successful `loadEnv`).
pub fn server_env() -> Result<&'static Env, EnvError> {
    if let Some(env) = CACHED_ENV.get() {
        return Ok(env);
    }
    let source: IndexMap<String, String> = std::env::vars().collect();
    let env = load_env(&source)?;
    Ok(CACHED_ENV.get_or_init(|| env))
}
