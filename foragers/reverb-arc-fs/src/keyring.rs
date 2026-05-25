//! Per-forager private key. After the forager-as-library refactor, every forager process
//! holds exactly one EOA private key: the bee whose key it holds IS the forager. No
//! multi-tenant keyring; no `as_bee` per-call auth; the process boundary is the identity
//! boundary.
//!
//! Load from a 0600-mode keyfile at `~/.config/hum/{persona-set}/{bee-name}.key`; the file
//! contains the hex private key (with or without `0x` prefix) and nothing else.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::ForagerError;

/// A 0x-prefixed hex EOA private key.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct PrivateKey(String);

impl PrivateKey {
    pub fn new(hex_with_0x: impl Into<String>) -> Result<Self, ForagerError> {
        let s = hex_with_0x.into();
        let normalized = if let Some(stripped) = s.strip_prefix("0x") {
            format!("0x{stripped}")
        } else {
            format!("0x{s}")
        };
        if normalized.len() != 66 {
            return Err(ForagerError::InvalidKeyFormat);
        }
        if !normalized[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ForagerError::InvalidKeyFormat);
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Load from a keyfile path. Enforces 0600 mode on unix; refuses to load looser permissions.
    pub fn load(path: &Path) -> Result<Self, ForagerError> {
        Self::verify_permissions(path)?;
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ForagerError::KeyIo(e.to_string()))?;
        Self::new(raw.trim())
    }

    #[cfg(unix)]
    fn verify_permissions(path: &Path) -> Result<(), ForagerError> {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path)
            .map_err(|e| ForagerError::KeyIo(e.to_string()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(ForagerError::KeyPermissions { mode });
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn verify_permissions(_path: &Path) -> Result<(), ForagerError> {
        Ok(())
    }
}

impl std::fmt::Debug for PrivateKey {
    /// Never print the key material in logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrivateKey(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn accepts_well_formed_key_with_prefix() {
        let k = PrivateKey::new("0x".to_string() + &"a".repeat(64));
        assert!(k.is_ok());
    }

    #[test]
    fn accepts_well_formed_key_without_prefix() {
        let k = PrivateKey::new("a".repeat(64));
        assert!(k.is_ok());
        assert!(k.unwrap().as_str().starts_with("0x"));
    }

    #[test]
    fn rejects_short_key() {
        assert!(matches!(PrivateKey::new("0xdeadbeef"), Err(ForagerError::InvalidKeyFormat)));
    }

    #[test]
    fn rejects_non_hex() {
        assert!(matches!(
            PrivateKey::new("0x".to_string() + &"z".repeat(64)),
            Err(ForagerError::InvalidKeyFormat)
        ));
    }

    #[test]
    fn debug_redacts_material() {
        let k = PrivateKey::new("0x".to_string() + &"f".repeat(64)).unwrap();
        let s = format!("{k:?}");
        assert!(s.contains("redacted"));
        assert!(!s.contains("fff"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_group_or_other_readable_file() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = std::env::temp_dir().join(format!("rafs-pk-test-{}.key", std::process::id()));
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "{}", "f".repeat(64)).unwrap();
        let mut perms = f.metadata().unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&tmp, perms).unwrap();
        assert!(matches!(PrivateKey::load(&tmp), Err(ForagerError::KeyPermissions { .. })));
        std::fs::remove_file(&tmp).ok();
    }
}
