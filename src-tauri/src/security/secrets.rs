//! Secret material storage — OS credential store in production, memory in tests.
//! SQLite holds **references only** (`secret_refs.store_key`), never plaintext.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Backend kind recorded for diagnostics (never includes secret values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretBackendKind {
    Keyring,
    Memory,
}

/// Core secret store API.
pub trait SecretStore: Send {
    fn backend_kind(&self) -> SecretBackendKind;
    fn set_secret(&mut self, store_key: &str, plaintext: &str) -> Result<(), String>;
    fn get_secret(&self, store_key: &str) -> Result<Option<String>, String>;
    fn delete_secret(&mut self, store_key: &str) -> Result<(), String>;
}

/// In-process store for tests and when OS keyring is unavailable.
#[derive(Debug, Default, Clone)]
pub struct MemorySecretStore {
    map: HashMap<String, String>,
}

impl SecretStore for MemorySecretStore {
    fn backend_kind(&self) -> SecretBackendKind {
        SecretBackendKind::Memory
    }

    fn set_secret(&mut self, store_key: &str, plaintext: &str) -> Result<(), String> {
        if store_key.is_empty() {
            return Err("empty store_key".into());
        }
        self.map.insert(store_key.to_string(), plaintext.to_string());
        Ok(())
    }

    fn get_secret(&self, store_key: &str) -> Result<Option<String>, String> {
        Ok(self.map.get(store_key).cloned())
    }

    fn delete_secret(&mut self, store_key: &str) -> Result<(), String> {
        self.map.remove(store_key);
        Ok(())
    }
}

/// OS-backed credential store. Keys are only identifiers; values never enter
/// ordinary app-data files or SQLite.
#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new() -> Result<Self, String> {
        // Construct a probe entry up front so unavailable platform backends fail
        // closed to the explicit in-memory fallback below.
        keyring::Entry::new("app.loopcode", "availability-probe")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            service: "app.loopcode".into(),
        })
    }

    fn entry(&self, store_key: &str) -> Result<keyring::Entry, String> {
        if store_key.is_empty() {
            return Err("empty store_key".into());
        }
        keyring::Entry::new(&self.service, store_key).map_err(|e| e.to_string())
    }
}

impl SecretStore for KeyringSecretStore {
    fn backend_kind(&self) -> SecretBackendKind {
        SecretBackendKind::Keyring
    }

    fn set_secret(&mut self, store_key: &str, plaintext: &str) -> Result<(), String> {
        self.entry(store_key)?
            .set_password(plaintext)
            .map_err(|e| e.to_string())
    }

    fn get_secret(&self, store_key: &str) -> Result<Option<String>, String> {
        match self.entry(store_key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn delete_secret(&mut self, store_key: &str) -> Result<(), String> {
        match self.entry(store_key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Shared secret store handle for Core.
#[derive(Clone)]
pub struct SharedSecretStore {
    inner: Arc<Mutex<Box<dyn SecretStore>>>,
    kind: SecretBackendKind,
}

impl SharedSecretStore {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Box::new(MemorySecretStore::default()))),
            kind: SecretBackendKind::Memory,
        }
    }

    pub fn from_box(store: Box<dyn SecretStore>) -> Self {
        let kind = store.backend_kind();
        Self {
            inner: Arc::new(Mutex::new(store)),
            kind,
        }
    }

    pub fn backend_kind(&self) -> SecretBackendKind {
        self.kind
    }

    pub fn set(&self, store_key: &str, plaintext: &str) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|_| "secret store lock poisoned".to_string())?
            .set_secret(store_key, plaintext)
    }

    pub fn get(&self, store_key: &str) -> Result<Option<String>, String> {
        self.inner
            .lock()
            .map_err(|_| "secret store lock poisoned".to_string())?
            .get_secret(store_key)
    }

    pub fn delete(&self, store_key: &str) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|_| "secret store lock poisoned".to_string())?
            .delete_secret(store_key)
    }
}

/// Use the OS credential manager in production. If the platform credential
/// backend is unavailable, retain secrets only in memory rather than writing a
/// plaintext fallback that survives restart.
pub fn default_secret_store(data_dir: &Path) -> SharedSecretStore {
    // A prior build wrote this file in plaintext. Do not import it into a new
    // store or leave it as a durable secret copy; affected users must re-enter
    // credentials into the OS credential manager.
    let legacy_fallback = data_dir.join("secrets-fallback.json");
    if legacy_fallback.is_file() {
        let _ = std::fs::remove_file(legacy_fallback);
    }
    match KeyringSecretStore::new() {
        Ok(store) => SharedSecretStore::from_box(Box::new(store)),
        Err(_) => SharedSecretStore::memory(),
    }
}

/// DTO for WebView: never includes plaintext.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRefDto {
    pub id: String,
    pub label: String,
    pub store_key: String,
    pub kind: String,
    pub provider: Option<String>,
}
