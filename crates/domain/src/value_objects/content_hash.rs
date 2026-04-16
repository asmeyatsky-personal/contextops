use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// SHA-256 content hash for content-addressable artifact storage.
///
/// Every context artifact is pinned to an immutable content hash.
/// Agents always reference an immutable snapshot via this digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Compute a SHA-256 hash from raw content bytes.
    pub fn from_content(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Create from a pre-computed hex digest string.
    /// Returns None if the string is not a valid SHA-256 hex digest.
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        if hex_str.len() == 64 && hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
            Some(Self(hex_str.to_lowercase()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sha256:{}", &self.0[..12])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_produces_same_hash() {
        let h1 = ContentHash::from_content(b"hello world");
        let h2 = ContentHash::from_content(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_content_produces_different_hash() {
        let h1 = ContentHash::from_content(b"hello");
        let h2 = ContentHash::from_content(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_is_64_hex_characters() {
        let h = ContentHash::from_content(b"test");
        assert_eq!(h.as_str().len(), 64);
        assert!(h.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn from_hex_rejects_invalid() {
        assert!(ContentHash::from_hex("not-a-hash").is_none());
        assert!(ContentHash::from_hex("abc").is_none());
    }

    #[test]
    fn from_hex_accepts_valid() {
        let valid = "a".repeat(64);
        assert!(ContentHash::from_hex(&valid).is_some());
    }

    #[test]
    fn display_shows_truncated_hash() {
        let h = ContentHash::from_content(b"test");
        let display = format!("{h}");
        assert!(display.starts_with("sha256:"));
        assert_eq!(display.len(), 7 + 12); // "sha256:" + 12 hex chars
    }
}
