//! #4006 and #4007 (#3897, parts 15 and 18): no crate the workspace locks encrypts with a bare block
//! mode such as CBC, whose padding errors let a caller decrypt a ciphertext byte by byte (a padding
//! oracle), and none parses XML, whose external entities can make a parser read a local file (XXE).
//! The server's one encryption is Web Push's `aes128gcm`, an AEAD through `ring`
//! (`src/daily/webpush.rs`); TLS is rustls's and Cloudflare's; and nothing reads XML. A change that
//! needs one of these crates says why here, and follows README.md's Conventions.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Crates that encrypt with a block mode outside an AEAD, or bind OpenSSL, which offers CBC beside the
/// rustls the server uses.
const BLOCK_MODE_CRATES: &[&str] = &[
    "block-modes",
    "block-padding",
    "cbc",
    "ecb",
    "native-tls",
    "openssl",
    "openssl-sys",
];

/// Crates that parse XML.
const XML_CRATES: &[&str] = &[
    "libxml",
    "minidom",
    "quick-xml",
    "roxmltree",
    "serde-xml-rs",
    "sxd-document",
    "xml-rs",
    "xml5ever",
    "xmltree",
    "yaserde",
];

/// Every package name in the workspace's `Cargo.lock`.
fn locked_crates() -> BTreeSet<String> {
    let lock = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let text = std::fs::read_to_string(&lock).expect("the workspace's Cargo.lock");
    let crates: BTreeSet<String> = text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("name = \"")?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .collect();
    // The AEAD the server does use, so a lock file this misreads cannot pass as one with nothing banned.
    assert!(
        crates.contains("ring"),
        "Cargo.lock read as {} names without ring",
        crates.len()
    );
    crates
}

fn locked_among(banned: &[&str]) -> Vec<String> {
    let crates = locked_crates();
    banned
        .iter()
        .filter(|name| crates.contains(**name))
        .map(|name| name.to_string())
        .collect()
}

#[test]
fn no_locked_crate_encrypts_with_a_bare_block_mode() {
    assert_eq!(
        locked_among(BLOCK_MODE_CRATES),
        Vec::<String>::new(),
        "use an AEAD: AES-GCM or ChaCha20-Poly1305 through ring"
    );
}

#[test]
fn no_locked_crate_parses_xml() {
    assert_eq!(
        locked_among(XML_CRATES),
        Vec::<String>::new(),
        "a parser of outside XML disables DTDs and external entities"
    );
}
