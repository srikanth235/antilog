//! The GitHub-style webhook signature: `sha256=` and the hex of HMAC-SHA256 over the raw body.

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const BLOCK: usize = 64;

/// HMAC-SHA256 (RFC 2104), written out because the workspace has `sha2` and no `hmac`.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let hashed;
    let key = if key.len() > BLOCK {
        hashed = Sha256::digest(key);
        hashed.as_slice()
    } else {
        key
    };
    let mut block = [0_u8; BLOCK];
    for (slot, byte) in block.iter_mut().zip(key) {
        *slot = *byte;
    }
    let pad = |byte: u8| block.map(|k| k ^ byte);
    let mut inner = Sha256::new();
    inner.update(pad(0x36));
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(pad(0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut digits = text.bytes().map(nibble);
    let mut out = [0_u8; 32];
    for slot in &mut out {
        let high = digits.next()??;
        let low = digits.next()??;
        *slot = high << 4 | low;
    }
    Some(out)
}

/// Whether `header` (the value of `X-Hub-Signature-256`) is the signature of `body` under
/// `secret`. The comparison takes the same time wherever the two differ; an empty secret never
/// verifies anything.
pub fn verify(secret: &[u8], body: &[u8], header: &str) -> bool {
    if secret.is_empty() {
        return false;
    }
    let expected = hmac_sha256(secret, body);
    let given = header.strip_prefix("sha256=").and_then(unhex);
    // A header that cannot be read is compared against zeros and always refused, so its
    // malformed shape costs the same time as a wrong signature.
    let (given, readable) = match given {
        Some(given) => (given, true),
        None => ([0_u8; 32], false),
    };
    bool::from(expected.ct_eq(&given)) & readable
}

/// The header value for `body`, as a code host (or a test) sends it.
pub fn sign(secret: &[u8], body: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("sha256=");
    for byte in hmac_sha256(secret, body) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn matches_rfc_4231() {
        // Test case 1, and case 6 whose key is longer than the block.
        assert_eq!(
            hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        assert_eq!(
            hex(&hmac_sha256(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn verifies_only_the_right_signature() {
        let signed = sign(b"s3cret", b"{}");
        assert!(verify(b"s3cret", b"{}", &signed));
        assert!(!verify(b"s3cret", b"{ }", &signed));
        assert!(!verify(b"other", b"{}", &signed));
        assert!(!verify(
            b"s3cret",
            b"{}",
            &signed.replace("sha256=", "sha1=")
        ));
        assert!(!verify(b"s3cret", b"{}", "sha256=zz"));
        assert!(!verify(b"s3cret", b"{}", ""));
        assert!(!verify(b"", b"{}", &sign(b"", b"{}")));
    }
}
