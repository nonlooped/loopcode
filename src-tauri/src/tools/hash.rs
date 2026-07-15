//! Content hashing for patch pre-image conflict detection.

use sha2::{Digest, Sha256};

/// Collision-resistant fingerprint of file bytes used for patch pre-images and
/// checkpoint blob identity.
pub fn content_hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub fn content_hash_str(s: &str) -> String {
    content_hash_bytes(s.as_bytes())
}
