//! Stable ed25519 identity for a bee, derived per the hum hives contract.
//!
//! The hid is mandatory and is what humd uses to deduplicate a bee across reconnects.
//! Without a valid hid, every reconnect leaks a fresh manifest and the bee's tool count
//! multiplies until humd restarts. Format: `fbee_<hex>` where hex is `sha256(ed25519 pubkey)`,
//! derived from a 32-byte seed persisted at `$XDG_STATE_HOME/hum/bees/<kind>.key`.
//!
//! This must be byte-identical with the reference implementations:
//! - Rust: `nest_common::load_or_mint_bee_key`
//! - TypeScript: `hives/openai-server/src/identity.ts`
//! - Go: `beeHid` in `hives/twilio-sms/main.go`
//!
//! Spec: <https://adiled.github.io/hum/hives/> "Build a hive / 2. Handshake".

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

use crate::errors::ForagerError;

/// Bee role prefix on the hid. Foragers are `fbee_`, workers are `wbee_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeRole {
    Forager,
    Worker,
}

impl BeeRole {
    fn prefix(self) -> &'static str {
        match self {
            BeeRole::Forager => "fbee_",
            BeeRole::Worker => "wbee_",
        }
    }
}

/// A stable bee identity. Holds the 32-byte ed25519 seed and the role used to format the hid.
///
/// The seed is loaded from `$XDG_STATE_HOME/hum/bees/<kind>.key` on second and subsequent
/// boots; on first boot it is minted via `getrandom` and persisted at 0600.
#[derive(Clone)]
pub struct BeeIdentity {
    kind: String,
    role: BeeRole,
    seed: [u8; 32],
}

impl std::fmt::Debug for BeeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BeeIdentity")
            .field("kind", &self.kind)
            .field("role", &self.role)
            .field("hid", &self.hid_string())
            .field("seed", &"<redacted>")
            .finish()
    }
}

impl BeeIdentity {
    /// Load the seed at `$XDG_STATE_HOME/hum/bees/<kind>.key`, or mint and persist a new one.
    /// Mints with `BeeRole::Forager` by default; use [`BeeIdentity::load_or_mint_with_role`]
    /// for workers.
    pub fn load_or_mint(kind: &str) -> Result<Self, ForagerError> {
        Self::load_or_mint_with_role(kind, BeeRole::Forager)
    }

    pub fn load_or_mint_with_role(kind: &str, role: BeeRole) -> Result<Self, ForagerError> {
        let path = key_path(kind)?;
        let seed = if path.exists() {
            load_seed(&path)?
        } else {
            mint_and_persist(&path)?
        };
        Ok(Self { kind: kind.to_string(), role, seed })
    }

    /// Construct from an explicit seed. Test-only ergonomics; production callers go through
    /// `load_or_mint`.
    pub fn from_seed(kind: impl Into<String>, role: BeeRole, seed: [u8; 32]) -> Self {
        Self { kind: kind.into(), role, seed }
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn role(&self) -> BeeRole {
        self.role
    }

    /// The canonical hid string: `fbee_<hex>` for a forager, `wbee_<hex>` for a worker, where
    /// hex is `sha256(ed25519 pubkey)`. This is what goes into the `hid` field of the hello.
    pub fn hid_string(&self) -> String {
        let signing = SigningKey::from_bytes(&self.seed);
        let pubkey = signing.verifying_key();
        let digest = Sha256::digest(pubkey.as_bytes());
        format!("{}{}", self.role.prefix(), hex::encode(digest))
    }
}

fn key_path(kind: &str) -> Result<PathBuf, ForagerError> {
    let invalid = kind.is_empty()
        || kind == "."
        || kind == ".."
        || kind.contains('/')
        || kind.contains('\\')
        || kind.contains('\0');
    if invalid {
        return Err(ForagerError::BuilderIncomplete(format!(
            "invalid bee kind for hid path: {kind:?}"
        )));
    }
    let base = state_home()?;
    Ok(base.join("hum").join("bees").join(format!("{kind}.key")))
}

fn state_home() -> Result<PathBuf, ForagerError> {
    if let Ok(custom) = std::env::var("XDG_STATE_HOME") {
        if !custom.is_empty() {
            return Ok(PathBuf::from(custom));
        }
    }
    dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))
        .ok_or_else(|| {
            ForagerError::KeyIo("could not resolve XDG_STATE_HOME or home directory".into())
        })
}

fn load_seed(path: &Path) -> Result<[u8; 32], ForagerError> {
    enforce_0600(path)?;
    let bytes = fs::read(path).map_err(|e| ForagerError::KeyIo(format!("read {path:?}: {e}")))?;
    if bytes.len() != 32 {
        return Err(ForagerError::KeyIo(format!(
            "bee key at {path:?} is {} bytes, expected 32",
            bytes.len()
        )));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}

fn mint_and_persist(path: &Path) -> Result<[u8; 32], ForagerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ForagerError::KeyIo(format!("create_dir_all {parent:?}: {e}"))
        })?;
    }
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed)
        .map_err(|e| ForagerError::KeyIo(format!("getrandom: {e}")))?;
    fs::write(path, seed).map_err(|e| ForagerError::KeyIo(format!("write {path:?}: {e}")))?;
    set_0600(path)?;
    Ok(seed)
}

#[cfg(unix)]
fn enforce_0600(path: &Path) -> Result<(), ForagerError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)
        .map_err(|e| ForagerError::KeyIo(format!("stat {path:?}: {e}")))?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(ForagerError::KeyPermissions { mode });
    }
    Ok(())
}

#[cfg(not(unix))]
fn enforce_0600(_path: &Path) -> Result<(), ForagerError> {
    Ok(())
}

#[cfg(unix)]
fn set_0600(path: &Path) -> Result<(), ForagerError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perms)
        .map_err(|e| ForagerError::KeyIo(format!("chmod 0600 {path:?}: {e}")))
}

#[cfg(not(unix))]
fn set_0600(_path: &Path) -> Result<(), ForagerError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_state_home(test_id: &str) -> tempfile::TempDir {
        let dir = tempfile::Builder::new()
            .prefix(&format!("reverb-arc-fs-{test_id}-"))
            .tempdir()
            .unwrap();
        std::env::set_var("XDG_STATE_HOME", dir.path());
        dir
    }

    #[test]
    fn hid_string_has_fbee_prefix_for_forager() {
        let seed = [0x42u8; 32];
        let id = BeeIdentity::from_seed("test", BeeRole::Forager, seed);
        let hid = id.hid_string();
        assert!(hid.starts_with("fbee_"), "got {hid}");
        assert_eq!(hid.len(), "fbee_".len() + 64, "hex must be 64 chars");
    }

    #[test]
    fn hid_string_has_wbee_prefix_for_worker() {
        let seed = [0x42u8; 32];
        let id = BeeIdentity::from_seed("test", BeeRole::Worker, seed);
        assert!(id.hid_string().starts_with("wbee_"));
    }

    #[test]
    fn hid_string_is_deterministic_from_seed() {
        let seed = [0x07u8; 32];
        let a = BeeIdentity::from_seed("a", BeeRole::Forager, seed);
        let b = BeeIdentity::from_seed("b", BeeRole::Forager, seed);
        // Different kinds, same seed: same hid (hid is keyed off pubkey, not name).
        assert_eq!(a.hid_string(), b.hid_string());
    }

    #[test]
    fn load_or_mint_creates_persists_and_reloads() {
        // Serialize on a shared XDG var to keep tests independent.
        let _guard = TEST_LOCK.lock().unwrap();
        let _tmp = isolated_state_home("mint_persist");

        let first = BeeIdentity::load_or_mint("markets-auto-create").unwrap();
        let second = BeeIdentity::load_or_mint("markets-auto-create").unwrap();
        assert_eq!(first.hid_string(), second.hid_string());

        let path = key_path("markets-auto-create").unwrap();
        assert!(path.exists());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn load_or_mint_distinguishes_kinds() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _tmp = isolated_state_home("distinguish");

        let a = BeeIdentity::load_or_mint("markets-auto-create").unwrap();
        let b = BeeIdentity::load_or_mint("markets-arbiter").unwrap();
        assert_ne!(a.hid_string(), b.hid_string(), "different kinds, different seeds");
    }

    #[test]
    fn load_or_mint_rejects_path_traversal_in_kind() {
        let err = BeeIdentity::load_or_mint("..").unwrap_err();
        assert!(matches!(err, ForagerError::BuilderIncomplete(_)));
        let err = BeeIdentity::load_or_mint("a/b").unwrap_err();
        assert!(matches!(err, ForagerError::BuilderIncomplete(_)));
    }

    use std::sync::Mutex;
    static TEST_LOCK: Mutex<()> = Mutex::new(());
}
