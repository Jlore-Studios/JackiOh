//! A test-only ES256 key pair for tier-1 (JWKS) tests (R1144000).
//!
//! Generated fresh for this suite, never from a real project: it signs test tokens only, so every
//! name carries `TEST_ONLY`. The coordinates are the public half served in the scripted JWKS; the
//! private half signs the tokens `verify` checks.

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};

/// The key id the scripted JWKS serves this key under.
pub(crate) const TEST_ONLY_ES256_KID: &str = "es256-test-only";
/// A second key id for the same test-only key, served when a refetch must drop the first.
pub(crate) const TEST_ONLY_ES256_KID_B: &str = "es256-test-only-b";

/// The test-only P-256 public coordinates, base64url without padding.
pub(crate) const TEST_ONLY_ES256_JWK_X: &str = "jB3U_IHJY2YVmRvAKkZJnvt1RyNNohO0H6VASEY0CrM";
pub(crate) const TEST_ONLY_ES256_JWK_Y: &str = "5O_hz_nvh6l9s7daTyLfbWiAr6S5Q3DE9MKqMVMv9ls";

/// The test-only P-256 private key (PKCS#8), signing test tokens only.
const TEST_ONLY_ES256_PRIVATE_PEM: &str = "\
-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgxdo/etUErIa0KPP7\n\
OVCV/bAGwwuSxpMxmEUqnWZRCCWhRANCAASMHdT8gcljZhWZG8AqRkme+3VHI02i\n\
E7QfpUBIRjQKs+Tv4c/574epfbO3Wk8i321ogK+kuUNwxPTCqjFTL/Zb\n\
-----END PRIVATE KEY-----\n";

/// The scripted JWKS body serving the test-only key under `kid`.
fn jwks_body_for_kid(kid: &str) -> Value {
    json!({ "keys": [{
        "kty": "EC",
        "crv": "P-256",
        "x": TEST_ONLY_ES256_JWK_X,
        "y": TEST_ONLY_ES256_JWK_Y,
        "kid": kid,
        "alg": "ES256",
        "use": "sig",
    }] })
}

/// The scripted JWKS body serving the test-only key.
pub(crate) fn jwks_body() -> Value {
    jwks_body_for_kid(TEST_ONLY_ES256_KID)
}

/// The scripted JWKS body serving the test-only key under a new id, as a rotation that dropped
/// the old one reads.
pub(crate) fn jwks_body_b() -> Value {
    jwks_body_for_kid(TEST_ONLY_ES256_KID_B)
}

/// An ES256 token over `claims` for `user_id`, with §9.2's issuer, audience and a far-future
/// expiry, signed by the test-only key under `kid`.
pub(crate) fn sign_with_kid(kid: &str, issuer: &str, user_id: &str, mut claims: Value) -> String {
    claims["sub"] = json!(user_id);
    claims["iss"] = json!(issuer);
    claims["aud"] = json!("authenticated");
    claims["exp"] = json!(super::auth::FAR_FUTURE);
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(kid.to_string());
    let key = EncodingKey::from_ec_pem(TEST_ONLY_ES256_PRIVATE_PEM.as_bytes())
        .expect("the test-only private key parses");
    jsonwebtoken::encode(&header, &claims, &key).expect("a test-only ES256 token")
}

/// An ES256 token over `claims` for `user_id`, signed by the test-only key.
pub(crate) fn sign(issuer: &str, user_id: &str, claims: Value) -> String {
    sign_with_kid(TEST_ONLY_ES256_KID, issuer, user_id, claims)
}
