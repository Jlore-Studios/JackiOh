//! Codes, hashes and ids (SPEC §9.4, §9.5, R79, SURFACE §11.3).
//!
//! Invite codes and room codes share the alphabet from `crate::config` and build on `random_code`:
//! 16 characters for invite codes (80 bits), `ROOM_CODE_LENGTH` for room codes.

use hmac::{Hmac, KeyInit, Mac};
use jackioh_engine::wire::codes::{canonical_code, normalize_code_text};
use sha2::Sha256;

use crate::config::{CODE_ALPHABET, INVITE_CODE_FORMAT, INVITE_CODE_GROUP_SIZE, INVITE_CODE_SEPARATOR};

const _: () = assert!(
    CODE_ALPHABET.len() == 32,
    "CODE_ALPHABET must hold exactly 32 symbols (§9.4)"
);

/// Uniform because 256 is a multiple of 32: a byte masked to 5 bits has no modulo bias.
pub fn code_from_bytes(bytes: &[u8]) -> String {
    let alphabet = CODE_ALPHABET.as_bytes();
    bytes
        .iter()
        .map(|byte| alphabet[(byte & 31) as usize] as char)
        .collect()
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
    let size = INVITE_CODE_GROUP_SIZE.max(1);
    let groups: Vec<String> = characters
        .chunks(size)
        .map(|group| group.iter().collect())
        .collect();
    groups.join(INVITE_CODE_SEPARATOR)
}

/// Normalizes code input for hashing (SPEC §11 R191). Characters outside the alphabet are
/// preserved so malformed codes match lookup misses without leaking an oracle (§9.4).
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
    if normalized.encode_utf16().count() != length {
        return false;
    }
    normalized.chars().all(|ch| CODE_ALPHABET.contains(ch))
}

/// Keyed digests of an invite code and client address under separate pepper keys.
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
pub(crate) fn digest(key: &str, value: &str) -> String {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(key.as_bytes()).expect("HMAC takes a key of any length");
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

/// §9.4: codes are stored hashed with keyed SHA-256 HMAC (80 bits entropy, offline brute-force protection).
pub fn create_hashes(code_pepper: &str, ip_pepper: &str) -> Hashes {
    Hashes {
        code_pepper: code_pepper.to_string(),
        ip_pepper: ip_pepper.to_string(),
    }
}

/// §9.4, §9.8: derives domain-separated `<pepper>:code` and `<pepper>:ip` HMAC keys from `CODE_PEPPER`.
pub fn hashes_for_pepper(pepper: &str) -> Hashes {
    create_hashes(&format!("{pepper}:code"), &format!("{pepper}:ip"))
}

/// Generates UUIDs and match seeds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemIds;

impl SystemIds {
    /// A random (v4) UUID in lowercase.
    pub fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// A match seed: 16 random bytes as hex. The engine is seeded only from this (SPEC §9.3).
    pub fn seed(&self) -> String {
        hex(&random_bytes(16))
    }
}

/// Kept under lowercase name for SURFACE §4.2.
#[allow(non_upper_case_globals)]
pub const system_ids: SystemIds = SystemIds;
