//! Per-bee wallet keyring loaded from disk.
//!
//! Each entry is `bee_name -> EOA private key`. Loaded at boot with restrictive filesystem
//! permissions (0600). The keyring is humd-local: in a multi-humd ensemble each humd's
//! forager instance holds only the EOAs of bees on that humd. No cross-humd key sharing.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::ForagerError;

/// A 0x-prefixed hex EOA private key for one bee.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct PrivateKey(String);

impl PrivateKey {
    pub fn new(hex_with_0x: impl Into<String>) -> Result<Self, ForagerError> {
        let s = hex_with_0x.into();
        if !s.starts_with("0x") || s.len() != 66 {
            return Err(ForagerError::InvalidKeyFormat);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Keyring {
    /// Map of `bee_name -> private key`.
    entries: HashMap<String, PrivateKey>,
}

impl Keyring {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load a keyring from a JSON file. Verifies the file's permission bits are 0600 on
    /// unix platforms; refuses to load if more permissive.
    pub fn load(path: &Path) -> Result<Self, ForagerError> {
        Self::verify_permissions(path)?;
        let content = std::fs::read_to_string(path)
            .map_err(|e| ForagerError::KeyringIo(e.to_string()))?;
        let raw: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| ForagerError::KeyringIo(e.to_string()))?;
        let mut entries = HashMap::with_capacity(raw.len());
        for (bee, key) in raw {
            entries.insert(bee, PrivateKey::new(key)?);
        }
        Ok(Self { entries })
    }

    pub fn insert(&mut self, bee: impl Into<String>, key: PrivateKey) {
        self.entries.insert(bee.into(), key);
    }

    /// Look up the private key registered for a bee.
    pub fn lookup(&self, bee: &str) -> Option<&PrivateKey> {
        self.entries.get(bee)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(unix)]
    fn verify_permissions(path: &Path) -> Result<(), ForagerError> {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path)
            .map_err(|e| ForagerError::KeyringIo(e.to_string()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(ForagerError::KeyringPermissions { mode });
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn verify_permissions(_path: &Path) -> Result<(), ForagerError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_short_key() {
        assert!(matches!(
            PrivateKey::new("0xdeadbeef"),
            Err(ForagerError::InvalidKeyFormat)
        ));
    }

    #[test]
    fn rejects_missing_prefix() {
        assert!(matches!(
            PrivateKey::new("deadbeef".repeat(8)),
            Err(ForagerError::InvalidKeyFormat)
        ));
    }

    #[test]
    fn accepts_well_formed_key() {
        let k = PrivateKey::new("0x".to_string() + &"a".repeat(64));
        assert!(k.is_ok());
    }

    #[test]
    fn lookup_returns_stored_key() {
        let mut k = Keyring::new();
        let pk = PrivateKey::new("0x".to_string() + &"f".repeat(64)).unwrap();
        k.insert("daman-watchdog-aggressive", pk.clone());
        assert_eq!(k.lookup("daman-watchdog-aggressive"), Some(&pk));
        assert_eq!(k.lookup("unknown"), None);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_group_or_other_readable_file() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = std::env::temp_dir().join(format!("reverb-arc-fs-test-{}.json", std::process::id()));
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "{{}}").unwrap();
        let mut perms = f.metadata().unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&tmp, perms).unwrap();
        let err = Keyring::load(&tmp).unwrap_err();
        assert!(matches!(err, ForagerError::KeyringPermissions { .. }));
        std::fs::remove_file(&tmp).ok();
    }
}
