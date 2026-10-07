//! Codes, hashes and ids (SPEC §9.4, §9.5, R79). Port of `apps/server/src/api/crypto.ts`.
//!
//! The alphabet, the invite-code length and the group size come from `crate::config`; nothing
//! here restates them. Invite codes and room codes share the alphabet, so both build on
//! `random_code`: 16 characters for an invite code (80 bits), `ROOM_CODE_LENGTH` for a room code.
//!
//! TS's `Hashes` and `Ids` ports become the `Hashes` struct and `system_ids` here (SURFACE §11.3:
//! `Ids` → `uuid` + `getrandom`, `Hashes` → functions in `api/crypto.rs`).

use hmac::{Hmac, KeyInit, Mac};
use jackioh_engine::wire::codes::{canonical_code, normalize_code_text};
use sha2::{Digest, Sha256};

use crate::config::{CODE_ALPHABET, INVITE_CODE_FORMAT, INVITE_CODE_GROUP_SIZE, INVITE_CODE_SEPARATOR, PLAYER_TAG_LENGTH};

const _: () = assert!(CODE_ALPHABET.len() == 32, "CODE_ALPHABET must hold exactly 32 symbols (§9.4)");

/// Uniform because 256 is a multiple of 32: a byte masked to 5 bits has no modulo bias.
pub fn code_from_bytes(bytes: &[u8]) -> String {
    let alphabet = CODE_ALPHABET.as_bytes();
    bytes.iter().map(|byte| alphabet[(byte & 31) as usize] as char).collect()
}

/// `length` bytes from the operating system's random source.
fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes).expect("the operating system's random source failed");
    bytes
}

pub fn random_code(length: usize) -> String {
    code_from_bytes(&random_bytes(length))
}

/// `XXXX-XXXX-XXXX-XXXX` (§9.4).
pub fn format_code(raw: &str) -> String {
    let characters: Vec<char> = raw.chars().collect();
    let size = (INVITE_CODE_GROUP_SIZE as usize).max(1);
    let groups: Vec<String> = characters.chunks(size).map(|group| group.iter().collect()).collect();
    groups.join(INVITE_CODE_SEPARATOR)
}

/// What the client typed, reduced to the form that gets hashed: SPEC §11 R191's reading, shared with
/// the client's code field through the wire module (NFKC, upper case one character at a time,
/// every space, dash and invisible separator removed). A string holding characters outside the
/// alphabet is still returned, so a malformed code takes the same path as a lookup miss and no
/// oracle distinguishes "well-formed but unknown" from "malformed" (§9.4).
pub fn normalize_code(input: &str) -> String {
    normalize_code_text(input)
}

/// R191: the invite code a raw input reads as, or none when it is not exactly one — too short, too
/// long (`CODE_INPUT_MAX_LENGTH` included, which is refused unread) or holding a character outside
/// R104's alphabet. Nothing is dropped or mapped on the way.
pub fn canonical_invite_code(raw: &str) -> Option<String> {
    canonical_code(raw, &INVITE_CODE_FORMAT)
}

pub fn is_well_formed_code(normalized: &str, length: usize) -> bool {
    // TS `normalized.length`: UTF-16 code units.
    if normalized.encode_utf16().count() != length {
        return false;
    }
    normalized.chars().all(|ch| CODE_ALPHABET.contains(ch))
}

/// TS `Hashes`: the keyed digests of an invite code and of a client address, each under its own
/// key (TS's `peppers: { code, ip }`).
#[derive(Clone, PartialEq, Eq)]
pub struct Hashes {
    code_pepper: String,
    ip_pepper: String,
}

/// Never prints the peppers.
impl std::fmt::Debug for Hashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Hashes { .. }")
    }
}

/// HMAC-SHA256 of `value` under `key`, as lower-case hex.
fn digest(key: &str, value: &str) -> String {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key.as_bytes()).expect("HMAC takes a key of any length");
    mac.update(value.as_bytes());
    hex(&mac.finalize().into_bytes())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl Hashes {
    /// §9.4: an invite code's hash, after `normalize_code`.
    pub fn code(&self, plain: &str) -> String {
        digest(&self.code_pepper, &normalize_code(plain))
    }

    /// §9.4, §9.8: a client address's hash, trimmed and lower-cased first.
    pub fn ip(&self, raw: &str) -> String {
        digest(&self.ip_pepper, &raw.trim().to_lowercase())
    }
}

/// §9.4: codes are stored hashed. A keyed SHA-256 rather than a password hash, because redemption
/// must find a code *by hash* and 80 bits of entropy needs no work factor; the pepper keeps a
/// stolen table from being brute-forced offline. The two arguments are TS's `peppers.code` and
/// `peppers.ip`, each already in its domain (`hashes_for_pepper` builds both from one pepper).
pub fn create_hashes(code_pepper: &str, ip_pepper: &str) -> Hashes {
    Hashes { code_pepper: code_pepper.to_string(), ip_pepper: ip_pepper.to_string() }
}

/// §9.4, §9.8: one pepper in the environment (`CODE_PEPPER`), two domains, `<pepper>:code` and
/// `<pepper>:ip`. Separating them means an invite-code hash and an IP hash can never collide, and
/// neither is reversible without the pepper. (TS's composition root built this; every caller here
/// builds it from `app.env.code_pepper`.)
pub fn hashes_for_pepper(pepper: &str) -> Hashes {
    create_hashes(&format!("{pepper}:code"), &format!("{pepper}:ip"))
}

/// SPEC §11 R612: a player's public tag on the leaderboard and the match screen, `PLAYER_TAG_LENGTH`
/// symbols of the code alphabet read off the SHA-256 of the profile id. Players have no public name,
/// and the profile id is never sent to anyone but its owner, so the tag stands in for both: stable,
/// the same on every screen, and no way back to the id or the account's email.
pub fn player_tag(profile_id: &str) -> String {
    let hash = Sha256::digest(profile_id.as_bytes());
    let length = (PLAYER_TAG_LENGTH as usize).min(hash.len());
    code_from_bytes(&hash[..length])
}

/// Constant-time compare, for a secret that is not looked up by hash.
pub fn safe_equal(a: &str, b: &str) -> bool {
    let left = a.as_bytes();
    let right = b.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right.iter()).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// TS `Ids`: every id and every seed comes from here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemIds;

impl SystemIds {
    /// A random (v4) UUID, lower case, as `crypto.randomUUID()` prints one.
    pub fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// A match seed: 16 random bytes as hex. The engine is seeded only from this (SPEC §9.3).
    pub fn seed(&self) -> String {
        hex(&random_bytes(16))
    }

    /// `length` characters from the invite-code alphabet.
    pub fn code(&self, length: usize) -> String {
        random_code(length)
    }
}

/// TS `systemIds`, under its TS name (SURFACE §4.2).
#[allow(non_upper_case_globals)]
pub const system_ids: SystemIds = SystemIds;
