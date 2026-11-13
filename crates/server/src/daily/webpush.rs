//! Web Push for the daily games' "your move" (SPEC §9.13, #3727): one message, encrypted for one
//! browser's subscription as RFC 8291 says (`aes128gcm`, RFC 8188), signed for its push service as RFC
//! 8292's VAPID says, and posted. A push service answering 404 or 410 says the subscription is gone,
//! and the caller deletes it.
//!
//! The keys are the server's VAPID pair, raw base64url as the browser's `applicationServerKey` wants
//! them: the private key the P-256 scalar (32 bytes), the public key the uncompressed point (65). With
//! either unset the server sends no push (`env.rs`).

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ring::rand::{SecureRandom, SystemRandom};
use ring::{aead, agreement, hkdf, signature};
use serde_json::{Value, json};

use crate::config::{
    PUSH_SERVICE_HOSTS, PUSH_TIMEOUT_MS, PUSH_TTL_SECONDS, VAPID_SUBJECT, VAPID_TOKEN_SECONDS,
};

/// RFC 8188 §2: the record size this server writes, the one RFC 8291 §4 recommends. A "your move"
/// message fits one record.
const RECORD_SIZE: u32 = 4096;
/// RFC 8188 §2.1: the salt's length.
const SALT_LEN: usize = 16;
/// RFC 8291 §3.2: the authentication secret's length.
const AUTH_LEN: usize = 16;
/// An uncompressed P-256 point (SEC 1 §2.3.3): a browser's `p256dh` and the sender's key id.
const POINT_LEN: usize = 65;
/// RFC 8188 §2.2: the content-encryption key's length (AES-128).
const KEY_LEN: usize = 16;
/// RFC 8188 §2.3: the nonce's length.
const NONCE_LEN: usize = 12;
/// RFC 8291 §3.4's PRK and IKM, an HKDF-SHA256 output's length.
const IKM_LEN: usize = 32;
/// RFC 8188 §2: the padding delimiter of the last record.
const LAST_RECORD: u8 = 0x02;
/// AES-GCM's tag (RFC 8188 §2).
const TAG_LEN: usize = 16;

/// The server's VAPID key pair (RFC 8292).
pub struct PushKeys {
    signer: signature::EcdsaKeyPair,
    public_key: String,
}

impl std::fmt::Debug for PushKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushKeys")
            .field("public_key", &self.public_key)
            .finish_non_exhaustive()
    }
}

impl PushKeys {
    /// The pair from its raw base64url halves, or why they are no pair.
    pub fn from_base64url(private_key: &str, public_key: &str) -> Result<PushKeys, String> {
        let private = URL_SAFE_NO_PAD
            .decode(private_key.trim())
            .map_err(|error| format!("the VAPID private key is not base64url: {error}"))?;
        let public = URL_SAFE_NO_PAD
            .decode(public_key.trim())
            .map_err(|error| format!("the VAPID public key is not base64url: {error}"))?;
        let signer = signature::EcdsaKeyPair::from_private_key_and_public_key(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &private,
            &public,
            &SystemRandom::new(),
        )
        .map_err(|error| format!("the VAPID keys are no P-256 pair: {error}"))?;
        Ok(PushKeys {
            signer,
            public_key: public_key.trim().to_string(),
        })
    }

    /// The public key, base64url, as the browser subscribes with it.
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}

/// One browser's subscription, as `PushSubscription.toJSON()` gives it.
pub struct Subscription<'a> {
    pub endpoint: &'a str,
    /// The browser's P-256 public key, base64url.
    pub p256dh: &'a str,
    /// The browser's authentication secret, base64url.
    pub auth: &'a str,
}

/// HKDF's output length, as ring asks for it.
struct Len(usize);

impl hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}

fn expand(prk: &hkdf::Prk, info: &[&[u8]], len: usize) -> Result<Vec<u8>, String> {
    let mut out = vec![0; len];
    prk.expand(info, Len(len))
        .and_then(|okm| okm.fill(&mut out))
        .map_err(|_| "HKDF refused its length".to_string())?;
    Ok(out)
}

/// RFC 8291 §3.3 and §3.4: the content-encryption key and the nonce, from the ECDH secret, the
/// browser's authentication secret and both public keys, and the salt.
fn derive(
    ecdh_secret: &[u8],
    auth: &[u8],
    ua_public: &[u8],
    as_public: &[u8],
    salt: &[u8],
) -> Result<([u8; KEY_LEN], [u8; NONCE_LEN]), String> {
    let prk_key = hkdf::Salt::new(hkdf::HKDF_SHA256, auth).extract(ecdh_secret);
    let ikm = expand(&prk_key, &[b"WebPush: info\0", ua_public, as_public], IKM_LEN)?;
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(&ikm);
    let cek = expand(&prk, &[b"Content-Encoding: aes128gcm\0"], KEY_LEN)?;
    let nonce = expand(&prk, &[b"Content-Encoding: nonce\0"], NONCE_LEN)?;
    let mut key = [0; KEY_LEN];
    key.copy_from_slice(&cek);
    let mut iv = [0; NONCE_LEN];
    iv.copy_from_slice(&nonce);
    Ok((key, iv))
}

/// RFC 8188 §2: `plaintext` sealed as the one, last record, under `key` and `nonce`.
fn seal(key: &[u8; KEY_LEN], nonce: [u8; NONCE_LEN], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let unbound =
        aead::UnboundKey::new(&aead::AES_128_GCM, key).map_err(|_| "AES-128-GCM refused its key")?;
    let mut record = plaintext.to_vec();
    record.push(LAST_RECORD);
    aead::LessSafeKey::new(unbound)
        .seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::empty(),
            &mut record,
        )
        .map_err(|_| "AES-128-GCM refused to seal".to_string())?;
    Ok(record)
}

/// RFC 8188 §2.1's header: the salt, the record size, the key id's length and the key id, the
/// sender's public key (RFC 8291 §4).
fn header(salt: &[u8], as_public: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SALT_LEN + 4 + 1 + POINT_LEN);
    out.extend_from_slice(salt);
    out.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    out.push(u8::try_from(as_public.len()).unwrap_or(u8::MAX));
    out.extend_from_slice(as_public);
    out
}

/// RFC 8291: `plaintext` encrypted for `subscription` under a fresh ECDH key pair and salt, as the
/// `aes128gcm` body a push service takes: the header, then the one record.
pub fn encrypt(subscription: &Subscription, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let ua_public = URL_SAFE_NO_PAD
        .decode(subscription.p256dh)
        .map_err(|_| "the subscription's p256dh is not base64url")?;
    let auth = URL_SAFE_NO_PAD
        .decode(subscription.auth)
        .map_err(|_| "the subscription's auth is not base64url")?;
    if ua_public.len() != POINT_LEN || auth.len() != AUTH_LEN {
        return Err("the subscription's keys are the wrong length".to_string());
    }
    if plaintext.len() + 1 + TAG_LEN > RECORD_SIZE as usize {
        return Err("the message does not fit one record".to_string());
    }
    let rng = SystemRandom::new();
    let mut salt = [0; SALT_LEN];
    rng.fill(&mut salt).map_err(|_| "no randomness for the salt")?;
    let private = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
        .map_err(|_| "no ECDH key pair")?;
    let as_public = private
        .compute_public_key()
        .map_err(|_| "no ECDH public key")?
        .as_ref()
        .to_vec();
    let peer = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, &ua_public);
    let (key, nonce) = agreement::agree_ephemeral(private, &peer, |secret| {
        derive(secret, &auth, &ua_public, &as_public, &salt)
    })
    .map_err(|_| "the subscription's p256dh is no P-256 point")??;
    let mut body = header(&salt, &as_public);
    body.extend(seal(&key, nonce, plaintext)?);
    Ok(body)
}

/// §9.8: whether `endpoint` is at a browser's push service: an https URL on the default port, with no
/// user name or password, whose host is one of `PUSH_SERVICE_HOSTS` or a subdomain of one. Nothing
/// else is kept as a subscription, so a player cannot aim the server's pushes at an address, a local
/// or private host or a name of their own (#3762's review).
pub fn is_push_service(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    let plain = url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none();
    // `domain` is `None` for an IP address; the parser has lowercased a name.
    plain
        && url.domain().is_some_and(|host| {
            PUSH_SERVICE_HOSTS.iter().any(|service| {
                host == *service || host.strip_suffix(service).is_some_and(|sub| sub.ends_with('.'))
            })
        })
}

/// The origin (`scheme://host[:port]`) of `endpoint`, VAPID's audience (RFC 8292 §2).
fn origin_of(endpoint: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(endpoint).map_err(|_| "the push endpoint is no URL")?;
    let origin = url.origin();
    if !origin.is_tuple() {
        return Err("the push endpoint has no origin".to_string());
    }
    Ok(origin.ascii_serialization())
}

/// RFC 8292 §3: the `Authorization` header for a push to `endpoint` at `now_secs`: an ES256 JWT for
/// the endpoint's origin, good for `VAPID_TOKEN_SECONDS`, and the public key.
pub fn vapid_header(keys: &PushKeys, endpoint: &str, now_secs: i64) -> Result<String, String> {
    let header = URL_SAFE_NO_PAD.encode(json!({ "typ": "JWT", "alg": "ES256" }).to_string());
    let claims = URL_SAFE_NO_PAD.encode(
        json!({
            "aud": origin_of(endpoint)?,
            "exp": now_secs + VAPID_TOKEN_SECONDS,
            "sub": VAPID_SUBJECT,
        })
        .to_string(),
    );
    let signing_input = format!("{header}.{claims}");
    let signed = keys
        .signer
        .sign(&SystemRandom::new(), signing_input.as_bytes())
        .map_err(|_| "ES256 refused to sign")?;
    let token = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signed.as_ref()));
    Ok(format!("vapid t={token}, k={}", keys.public_key))
}

/// What one push came to.
#[derive(Debug, PartialEq, Eq)]
pub enum PushOutcome {
    Sent,
    /// 404 or 410: the subscription is gone, and its row goes too (§9.13).
    Gone,
    Failed(String),
}

/// Posts `payload` to one subscription.
pub async fn send(
    client: &reqwest::Client,
    keys: &PushKeys,
    subscription: &Subscription<'_>,
    payload: &Value,
    now_secs: i64,
) -> PushOutcome {
    let prepared = encrypt(subscription, payload.to_string().as_bytes())
        .and_then(|body| Ok((body, vapid_header(keys, subscription.endpoint, now_secs)?)));
    let (body, authorization) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return PushOutcome::Failed(reason),
    };
    let sent = client
        .post(subscription.endpoint)
        .header("ttl", PUSH_TTL_SECONDS.to_string())
        .header("content-encoding", "aes128gcm")
        .header("content-type", "application/octet-stream")
        .header("urgency", "normal")
        .header("authorization", authorization)
        .timeout(Duration::from_millis(PUSH_TIMEOUT_MS))
        .body(body)
        .send()
        .await;
    match sent {
        Ok(response) if response.status().is_success() => PushOutcome::Sent,
        Ok(response) if matches!(response.status().as_u16(), 404 | 410) => PushOutcome::Gone,
        Ok(response) => PushOutcome::Failed(format!("the push service answered {}", response.status())),
        Err(error) => PushOutcome::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(text: &str) -> Vec<u8> {
        URL_SAFE_NO_PAD.decode(text).expect("base64url")
    }

    /// A user agent's side of RFC 8291 §3: the header read, the shared secret agreed with its own
    /// private key and the sender's key id, the same keys derived, the record opened and its
    /// delimiter taken off.
    fn decrypt(
        body: &[u8],
        ua_private: agreement::EphemeralPrivateKey,
        ua_public: &[u8],
        auth: &[u8],
    ) -> Vec<u8> {
        let salt = &body[..SALT_LEN];
        let rs = u32::from_be_bytes(body[SALT_LEN..SALT_LEN + 4].try_into().expect("four bytes"));
        assert_eq!(rs, RECORD_SIZE);
        let idlen = usize::from(body[SALT_LEN + 4]);
        assert_eq!(idlen, POINT_LEN);
        let key_id = &body[SALT_LEN + 5..SALT_LEN + 5 + idlen];
        let record = &body[SALT_LEN + 5 + idlen..];
        let peer = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, key_id);
        let (key, nonce) = agreement::agree_ephemeral(ua_private, &peer, |secret| {
            derive(secret, auth, ua_public, key_id, salt)
        })
        .expect("agreed")
        .expect("derived");
        let opener = aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_128_GCM, &key).expect("a key"));
        let mut in_out = record.to_vec();
        let plain = opener
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::empty(),
                &mut in_out,
            )
            .expect("the record opens");
        assert_eq!(plain.last(), Some(&LAST_RECORD));
        plain[..plain.len() - 1].to_vec()
    }

    #[test]
    fn webpush_encrypts_what_the_user_agent_decrypts() {
        let rng = SystemRandom::new();
        let ua_private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng).expect("a key");
        let ua_public = ua_private
            .compute_public_key()
            .expect("its point")
            .as_ref()
            .to_vec();
        let mut auth = [0; AUTH_LEN];
        rng.fill(&mut auth).expect("random");
        let (p256dh, auth_text) = (URL_SAFE_NO_PAD.encode(&ua_public), URL_SAFE_NO_PAD.encode(auth));
        let subscription = Subscription {
            endpoint: "https://push.example.net/send/abc",
            p256dh: &p256dh,
            auth: &auth_text,
        };
        let message = br#"{"title":"Your move","url":"/daily/m1"}"#;
        let body = encrypt(&subscription, message).expect("encrypted");
        assert_eq!(decrypt(&body, ua_private, &ua_public, &auth), message.to_vec());

        // Wrong-length keys are refused, not sent.
        let short = Subscription {
            auth: "AAAA",
            ..subscription
        };
        assert!(encrypt(&short, message).is_err());
    }

    /// RFC 8291 §5's worked example, from its shared secret on (ring makes no ECDH key from a given
    /// scalar): the keys derived and the record sealed are the RFC's, byte for byte.
    #[test]
    fn webpush_derives_rfc_8291s_example() {
        let ecdh_secret = b64("kyrL1jIIOHEzg3sM2ZWRHDRB62YACZhhSlknJ672kSs");
        let auth = b64("BTBZMqHH6r4Tts7J_aSIgg");
        let ua_public =
            b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4");
        let as_public =
            b64("BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8");
        let salt = b64("DGv6ra1nlYgDCS1FRnbzlw");
        let (key, nonce) = derive(&ecdh_secret, &auth, &ua_public, &as_public, &salt).expect("derived");
        assert_eq!(key.to_vec(), b64("oIhVW04MRdy2XN9CiKLxTg"));
        assert_eq!(nonce.to_vec(), b64("4h_95klXJ5E_qnoN"));
        assert_eq!(
            URL_SAFE_NO_PAD.encode(header(&salt, &as_public)),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8"
        );
        let plaintext = b64("V2hlbiBJIGdyb3cgdXAsIEkgd2FudCB0byBiZSBhIHdhdGVybWVsb24");
        assert_eq!(
            URL_SAFE_NO_PAD.encode(seal(&key, nonce, &plaintext).expect("sealed")),
            "8pfeW0KbunFT06SuDKoJH9Ql87S1QUrdirN6GcG7sFz1y1sqLgVi1VhjVkHsUoEsbI_0LpXMuGvnzQ"
        );
    }

    /// A raw P-256 pair, base64url, cut from a PKCS#8 document ring made: its v1 template keeps the
    /// private scalar at bytes 36..68 and the public point in the last 65.
    fn vapid_pair() -> (String, String) {
        let pkcs8 = signature::EcdsaKeyPair::generate_pkcs8(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &SystemRandom::new(),
        )
        .expect("a document");
        let bytes = pkcs8.as_ref();
        (
            URL_SAFE_NO_PAD.encode(&bytes[36..68]),
            URL_SAFE_NO_PAD.encode(&bytes[bytes.len() - POINT_LEN..]),
        )
    }

    #[test]
    fn webpush_signs_a_vapid_token_for_the_endpoints_origin() {
        let (private, public) = vapid_pair();
        let keys = PushKeys::from_base64url(&private, &public).expect("a pair");
        assert_eq!(keys.public_key(), public);
        let now = 1_700_000_000;
        let value = vapid_header(&keys, "https://push.example.net:8443/send/abc?x=1", now).expect("signed");
        let (token, key) = value
            .strip_prefix("vapid t=")
            .and_then(|rest| rest.split_once(", k="))
            .expect("t= and k=");
        assert_eq!(key, public);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: Value = serde_json::from_slice(&b64(parts[0])).expect("json");
        assert_eq!(header, json!({ "typ": "JWT", "alg": "ES256" }));
        let claims: Value = serde_json::from_slice(&b64(parts[1])).expect("json");
        assert_eq!(claims["aud"], "https://push.example.net:8443");
        assert_eq!(claims["exp"], now + VAPID_TOKEN_SECONDS);
        assert_eq!(claims["sub"], VAPID_SUBJECT);
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, b64(&public))
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &b64(parts[2]))
            .expect("the signature verifies with the public key");

        // Halves of two pairs are no pair.
        let (_, other) = vapid_pair();
        assert!(PushKeys::from_base64url(&private, &other).is_err());
        assert!(vapid_header(&keys, "not a url", now).is_err());
    }

    #[test]
    fn webpush_keeps_only_an_endpoint_at_a_browsers_push_service() {
        for endpoint in [
            "https://fcm.googleapis.com/fcm/send/abc:APA91b",
            "https://android.googleapis.com/gcm/send/abc",
            "https://updates.push.services.mozilla.com/wpush/v2/gAAAA",
            "https://web.push.apple.com/QGuQyavXutnMx",
            "https://wns2-par02p.notify.windows.com/w/?token=BQYAAA",
            "https://FCM.googleapis.com:443/fcm/send/abc",
        ] {
            assert!(is_push_service(endpoint), "{endpoint}");
        }
        for endpoint in [
            "http://fcm.googleapis.com/fcm/send/abc",
            "https://fcm.googleapis.com:8443/fcm/send/abc",
            "https://user:pass@fcm.googleapis.com/fcm/send/abc",
            "https://fcm.googleapis.com@169.254.169.254/latest/meta-data",
            "https://evilfcm.googleapis.com.example.net/send",
            "https://notfcm.googleapis.com/send",
            "https://fcm.googleapis.com.example.net/send",
            "https://googleapis.com/send",
            "https://push.example.net/send/abc",
            "https://localhost/send",
            "https://127.0.0.1/send",
            "https://[::1]/send",
            "https://10.0.0.7/send",
            "https://2130706433/send",
            "not a url",
        ] {
            assert!(!is_push_service(endpoint), "{endpoint}");
        }
    }
}
