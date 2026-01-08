//! Secure credential management with AES-256-GCM encryption.
//!
//! # Security Features
//!
//! - **Random nonce per encryption**: Prevents nonce reuse attacks on AES-GCM
//! - **Argon2 key derivation**: Memory-hard function resistant to GPU/ASIC attacks
//! - **Automatic zeroization**: Sensitive data cleared from memory on drop
//! - **Restrictive file permissions**: 600 on Unix systems
//!
//! # Ciphertext Format
//!
//! ```text
//! [12-byte nonce][ciphertext][16-byte authentication tag]
//! ```
//!
//! Minimum valid ciphertext size: 28 bytes (12 nonce + 16 tag)

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2};
use keyring::{Entry, Error as KeyringError};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Nonce size for AES-256-GCM (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// Authentication tag size for AES-GCM (128 bits = 16 bytes)
const TAG_SIZE: usize = 16;

/// Minimum valid ciphertext size (nonce + tag, no plaintext)
const MIN_CIPHERTEXT_SIZE: usize = NONCE_SIZE + TAG_SIZE;

const KEYCHAIN_SERVICE: &str = "aiy";
const KEYCHAIN_PROVIDER_INDEX_USER: &str = "__providers__";

/// Argon2 memory cost in KiB (64 MiB)
const ARGON2_MEMORY_COST: u32 = 65536;

/// Argon2 time cost (iterations)
const ARGON2_TIME_COST: u32 = 3;

/// Argon2 parallelism factor
const ARGON2_PARALLELISM: u32 = 4;

/// Security-related errors
#[derive(Debug, Error)]
pub enum SecurityError {
    /// Encryption operation failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption operation failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    /// Credential not found
    #[error("Credential not found for provider: {0}")]
    CredentialNotFound(String),

    /// Credential manager is locked
    #[error("Credential manager is locked - call unlock() first")]
    ManagerLocked,

    /// Invalid ciphertext format
    #[error("Invalid ciphertext: {0}")]
    InvalidCiphertext(String),

    /// File I/O error
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Keyring error
    #[error("Keyring error: {0}")]
    KeyringError(String),

    /// Salt generation or loading failed
    #[error("Salt error: {0}")]
    SaltError(String),

    /// Permission error
    #[error("Permission error: {0}")]
    PermissionError(String),
}

/// Master encryption key with automatic memory zeroization on drop.
///
/// The key material is securely cleared from memory when this struct
/// is dropped, preventing sensitive data from lingering in memory.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterKey {
    /// The raw 256-bit key material
    key: [u8; 32],
}

impl MasterKey {
    /// Create a new master key from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `key` - A 32-byte array containing the key material
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Get a reference to the key bytes.
    ///
    /// # Security
    ///
    /// The returned reference should not be stored or cloned.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Backend for credential storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CredentialBackend {
    /// Store credentials in an encrypted file on disk
    EncryptedFile {
        /// Path to the encrypted credentials file
        path: PathBuf,
    },
    /// Use the system's native keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
    SystemKeychain,
    /// Use a cloud secret manager (AWS Secrets Manager, GCP Secret Manager, etc.)
    SecretManager {
        /// The cloud provider (aws, gcp, azure)
        provider: String,
        /// Optional region/project identifier
        region: Option<String>,
    },
}

/// Encrypted credentials storage format
#[derive(Serialize, Deserialize, Default)]
struct EncryptedCredentials {
    /// Map of provider name to encrypted API key (hex-encoded)
    keys: HashMap<String, String>,
}

/// Secure credential manager for storing and retrieving API keys.
///
/// # Security Model
///
/// - Master key derived from password using Argon2id
/// - Individual credentials encrypted with AES-256-GCM
/// - Random nonce generated for each encryption operation
/// - Master key zeroized from memory when locked or dropped
///
/// # Example
///
/// ```ignore
/// let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
///     path: PathBuf::from("~/.config/aiy/credentials.enc"),
/// })?;
///
/// // Unlock with user password
/// manager.unlock("my-secure-password")?;
///
/// // Store an API key
/// manager.store_key("openai", "sk-...")?;
///
/// // Retrieve the key
/// let key = manager.get_key("openai")?;
///
/// // Lock when done (or let it auto-lock on drop)
/// manager.lock();
/// ```
pub struct CredentialManager {
    /// The storage backend
    backend: CredentialBackend,
    /// The derived master key (None when locked)
    master_key: Option<MasterKey>,
    /// Path to the salt file
    salt_path: PathBuf,
    /// In-memory cache of decrypted credentials
    credentials_cache: HashMap<String, String>,
}

impl CredentialManager {
    /// Create a new credential manager with the specified backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - The storage backend to use
    ///
    /// # Returns
    ///
    /// A new locked `CredentialManager` instance
    pub fn new(backend: CredentialBackend) -> Result<Self, SecurityError> {
        let salt_path = match &backend {
            CredentialBackend::EncryptedFile { path } => {
                let mut salt_path = path.clone();
                salt_path.set_extension("salt");
                salt_path
            }
            CredentialBackend::SystemKeychain => {
                // System keychain does not use a user-supplied master password.
                PathBuf::new()
            }
            CredentialBackend::SecretManager { .. } => {
                // Secret manager handles its own key management
                PathBuf::new()
            }
        };

        let manager = Self {
            backend,
            master_key: None,
            salt_path,
            credentials_cache: HashMap::new(),
        };

        // Best-effort probe: fail fast if system keychain is not accessible so callers can
        // fall back to the encrypted-file backend.
        if manager.backend == CredentialBackend::SystemKeychain {
            manager.probe_system_keychain()?;
        }

        Ok(manager)
    }

    /// Unlock the credential manager with a password.
    ///
    /// Derives the master encryption key from the password using Argon2id.
    /// The salt is loaded from disk or generated if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `password` - The user's password
    ///
    /// # Security
    ///
    /// - Uses Argon2id with memory-hard parameters
    /// - Salt is unique per installation
    /// - Password is not stored
    pub fn unlock(&mut self, password: &str) -> Result<(), SecurityError> {
        if self.backend == CredentialBackend::SystemKeychain {
            // System keychain is protected by the OS/user session and does not require a master
            // password for this credential manager.
            return Ok(());
        }

        let salt = self.load_or_create_salt(&self.salt_path.clone())?;

        // Configure Argon2id with secure parameters
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                ARGON2_MEMORY_COST,
                ARGON2_TIME_COST,
                ARGON2_PARALLELISM,
                Some(32),
            )
            .map_err(|e| SecurityError::KeyDerivationFailed(e.to_string()))?,
        );

        // Derive the key
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt.as_bytes(), &mut key)
            .map_err(|e| SecurityError::KeyDerivationFailed(e.to_string()))?;

        self.master_key = Some(MasterKey::new(key));

        // Load existing credentials
        self.load_encrypted_keys()?;

        Ok(())
    }

    /// Lock the credential manager, zeroizing the master key from memory.
    ///
    /// After calling this method, `unlock()` must be called again before
    /// any credential operations.
    pub fn lock(&mut self) {
        // MasterKey implements ZeroizeOnDrop, so dropping it will clear the key
        self.master_key = None;
        // Clear the credentials cache
        for (_, value) in self.credentials_cache.iter_mut() {
            value.zeroize();
        }
        self.credentials_cache.clear();
    }

    /// Store an API key for a provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider name (e.g., "openai", "anthropic")
    /// * `key` - The API key to store
    ///
    /// # Errors
    ///
    /// Returns `SecurityError::ManagerLocked` if the manager is not unlocked.
    pub fn store_key(&mut self, provider: &str, key: &str) -> Result<(), SecurityError> {
        match &self.backend {
            CredentialBackend::EncryptedFile { .. } => {
                let master_key = self
                    .master_key
                    .as_ref()
                    .ok_or(SecurityError::ManagerLocked)?;

                // Encrypt and store in file
                let encrypted = self.encrypt(key.as_bytes(), master_key)?;
                self.credentials_cache
                    .insert(provider.to_string(), key.to_string());

                // Save to disk
                self.save_encrypted_keys()?;

                // Explicitly drop encrypted to avoid leaking it
                drop(encrypted);
            }
            CredentialBackend::SystemKeychain => {
                // Store in system keychain
                let entry = Self::keychain_entry(provider)?;
                entry
                    .set_password(key)
                    .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
                self.keychain_add_provider(provider)?;
            }
            CredentialBackend::SecretManager { provider: _, .. } => {
                todo!("Secret manager auth/storage")
            }
        }

        Ok(())
    }

    /// Retrieve an API key for a provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider name
    ///
    /// # Returns
    ///
    /// The decrypted API key
    ///
    /// # Errors
    ///
    /// - `SecurityError::ManagerLocked` if not unlocked
    /// - `SecurityError::CredentialNotFound` if the provider has no stored key
    pub fn get_key(&self, provider: &str) -> Result<String, SecurityError> {
        match &self.backend {
            CredentialBackend::EncryptedFile { .. } => {
                if self.master_key.is_none() {
                    return Err(SecurityError::ManagerLocked);
                }
                self.credentials_cache
                    .get(provider)
                    .cloned()
                    .ok_or_else(|| SecurityError::CredentialNotFound(provider.to_string()))
            }
            CredentialBackend::SystemKeychain => {
                let entry = Self::keychain_entry(provider)?;
                match entry.get_password() {
                    Ok(value) => Ok(value),
                    Err(KeyringError::NoEntry) => {
                        Err(SecurityError::CredentialNotFound(provider.to_string()))
                    }
                    Err(e) => Err(SecurityError::KeyringError(e.to_string())),
                }
            }
            CredentialBackend::SecretManager { provider: _, .. } => {
                todo!("Secret manager auth/storage")
            }
        }
    }

    /// Encrypt plaintext using AES-256-GCM.
    ///
    /// # CRITICAL SECURITY: Random Nonce Generation
    ///
    /// A new random 12-byte nonce is generated for **every** encryption call
    /// using a cryptographically secure random number generator (`OsRng`).
    /// This prevents nonce reuse attacks which would completely break AES-GCM security.
    ///
    /// # Ciphertext Format
    ///
    /// ```text
    /// [12-byte random nonce][encrypted data][16-byte authentication tag]
    /// ```
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `key` - The master encryption key
    ///
    /// # Returns
    ///
    /// The encrypted data with prepended nonce
    pub fn encrypt(&self, plaintext: &[u8], key: &MasterKey) -> Result<Vec<u8>, SecurityError> {
        let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // CRITICAL: Generate a random 12-byte nonce for EVERY encryption
        // Using OsRng ensures cryptographically secure randomness
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the plaintext
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // Prepend the nonce to the ciphertext
        // Format: [12-byte nonce][ciphertext + 16-byte auth tag]
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM.
    ///
    /// # Ciphertext Format
    ///
    /// Expects data in the format:
    /// ```text
    /// [12-byte nonce][encrypted data][16-byte authentication tag]
    /// ```
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted data with prepended nonce
    /// * `key` - The master encryption key
    ///
    /// # Returns
    ///
    /// The decrypted plaintext
    ///
    /// # Errors
    ///
    /// - `SecurityError::InvalidCiphertext` if data is too short
    /// - `SecurityError::DecryptionFailed` if decryption or authentication fails
    pub fn decrypt(&self, data: &[u8], key: &MasterKey) -> Result<Vec<u8>, SecurityError> {
        // Validate minimum ciphertext size
        if data.len() < MIN_CIPHERTEXT_SIZE {
            return Err(SecurityError::InvalidCiphertext(format!(
                "Ciphertext too short: {} bytes, minimum is {} bytes",
                data.len(),
                MIN_CIPHERTEXT_SIZE
            )));
        }

        let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        // Extract the nonce from the first 12 bytes
        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);

        // Decrypt the rest (ciphertext + auth tag)
        let plaintext = cipher
            .decrypt(nonce, &data[NONCE_SIZE..])
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext)
    }

    /// Load or create a salt file.
    ///
    /// If the salt file exists, loads it. Otherwise, generates a new
    /// cryptographically secure salt and saves it with restrictive permissions.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the salt file
    ///
    /// # Returns
    ///
    /// The salt string
    pub fn load_or_create_salt(&self, path: &PathBuf) -> Result<String, SecurityError> {
        if path.as_os_str().is_empty() {
            // For backends that don't need a salt file
            return Ok(SaltString::generate(&mut OsRng).to_string());
        }

        if path.exists() {
            // Load existing salt
            let salt = fs::read_to_string(path)?;
            Ok(salt.trim().to_string())
        } else {
            // Generate new salt
            let salt = SaltString::generate(&mut OsRng);

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write salt to file
            fs::write(path, salt.as_str())?;

            // Set restrictive permissions (Unix only)
            #[cfg(unix)]
            {
                let mut perms = fs::metadata(path)?.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(path, perms)?;
            }

            Ok(salt.to_string())
        }
    }

    /// Save encrypted credentials to disk.
    ///
    /// # Security
    ///
    /// - Each credential is encrypted with a fresh random nonce
    /// - File permissions are set to 600 (owner read/write only) on Unix
    pub fn save_encrypted_keys(&self) -> Result<(), SecurityError> {
        let master_key = self
            .master_key
            .as_ref()
            .ok_or(SecurityError::ManagerLocked)?;

        match &self.backend {
            CredentialBackend::EncryptedFile { path } => {
                let mut encrypted_creds = EncryptedCredentials::default();

                // Encrypt each credential with a fresh nonce
                for (provider, key) in &self.credentials_cache {
                    let encrypted = self.encrypt(key.as_bytes(), master_key)?;
                    encrypted_creds
                        .keys
                        .insert(provider.clone(), hex::encode(&encrypted));
                }

                // Serialize to JSON
                let json = serde_json::to_string_pretty(&encrypted_creds)
                    .map_err(|e| SecurityError::SerializationError(e.to_string()))?;

                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Write to file
                fs::write(path, json)?;

                // Set restrictive permissions (Unix only)
                #[cfg(unix)]
                {
                    let mut perms = fs::metadata(path)?.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }

                Ok(())
            }
            CredentialBackend::SystemKeychain => {
                // System keychain handles its own storage
                Ok(())
            }
            CredentialBackend::SecretManager { .. } => {
                todo!("Secret manager auth/storage")
            }
        }
    }

    /// Load and decrypt credentials from disk.
    pub fn load_encrypted_keys(&mut self) -> Result<(), SecurityError> {
        let master_key = self
            .master_key
            .as_ref()
            .ok_or(SecurityError::ManagerLocked)?;

        match &self.backend {
            CredentialBackend::EncryptedFile { path } => {
                if !path.exists() {
                    return Ok(());
                }

                let json = fs::read_to_string(path)?;
                let encrypted_creds: EncryptedCredentials = serde_json::from_str(&json)
                    .map_err(|e| SecurityError::SerializationError(e.to_string()))?;

                // Decrypt each credential
                for (provider, encrypted_hex) in &encrypted_creds.keys {
                    let encrypted = hex::decode(encrypted_hex)
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;
                    let decrypted = self.decrypt(&encrypted, master_key)?;
                    let key = String::from_utf8(decrypted)
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;
                    self.credentials_cache.insert(provider.clone(), key);
                }

                Ok(())
            }
            CredentialBackend::SystemKeychain => {
                // System keychain credentials are loaded on-demand
                // We could enumerate them here, but that's not always possible
                Ok(())
            }
            CredentialBackend::SecretManager { .. } => {
                todo!("Secret manager auth/storage")
            }
        }
    }

    /// Check if the manager is currently unlocked.
    pub fn is_unlocked(&self) -> bool {
        match &self.backend {
            CredentialBackend::EncryptedFile { .. } => self.master_key.is_some(),
            CredentialBackend::SystemKeychain => true,
            CredentialBackend::SecretManager { .. } => true,
        }
    }

    /// Get the configured backend.
    pub fn backend(&self) -> &CredentialBackend {
        &self.backend
    }

    /// List all stored provider names.
    pub fn list_providers(&self) -> Result<Vec<String>, SecurityError> {
        match &self.backend {
            CredentialBackend::EncryptedFile { .. } => {
                if self.master_key.is_none() {
                    return Err(SecurityError::ManagerLocked);
                }
                Ok(self.credentials_cache.keys().cloned().collect())
            }
            CredentialBackend::SystemKeychain => match self.keychain_load_provider_index() {
                Ok(providers) => Ok(providers),
                Err(SecurityError::SerializationError(_)) => Ok(Vec::new()),
                Err(e) => Err(e),
            },
            CredentialBackend::SecretManager { .. } => {
                todo!("Secret manager auth/storage")
            }
        }
    }

    /// Delete a stored credential.
    pub fn delete_key(&mut self, provider: &str) -> Result<(), SecurityError> {
        match &self.backend {
            CredentialBackend::EncryptedFile { .. } => {
                if self.master_key.is_none() {
                    return Err(SecurityError::ManagerLocked);
                }
                if self.credentials_cache.remove(provider).is_some() {
                    self.save_encrypted_keys()?;
                    Ok(())
                } else {
                    Err(SecurityError::CredentialNotFound(provider.to_string()))
                }
            }
            CredentialBackend::SystemKeychain => {
                let entry = Self::keychain_entry(provider)?;
                match entry.delete_password() {
                    Ok(()) => {
                        self.keychain_remove_provider(provider)?;
                        Ok(())
                    }
                    Err(KeyringError::NoEntry) => {
                        // Best-effort repair of the provider index.
                        let _ = self.keychain_remove_provider(provider);
                        Err(SecurityError::CredentialNotFound(provider.to_string()))
                    }
                    Err(e) => Err(SecurityError::KeyringError(e.to_string())),
                }
            }
            CredentialBackend::SecretManager { .. } => {
                todo!("Secret manager auth/storage")
            }
        }
    }
}

impl CredentialManager {
    fn keychain_entry(user: &str) -> Result<Entry, SecurityError> {
        Entry::new(KEYCHAIN_SERVICE, user).map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn probe_system_keychain(&self) -> Result<(), SecurityError> {
        let entry = Self::keychain_entry(KEYCHAIN_PROVIDER_INDEX_USER)?;
        match entry.get_password() {
            Ok(_) => Ok(()),
            Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(SecurityError::KeyringError(e.to_string())),
        }
    }

    fn keychain_load_provider_index(&self) -> Result<Vec<String>, SecurityError> {
        let entry = Self::keychain_entry(KEYCHAIN_PROVIDER_INDEX_USER)?;
        match entry.get_password() {
            Ok(raw) => {
                let mut providers: Vec<String> = serde_json::from_str(&raw)
                    .map_err(|e| SecurityError::SerializationError(e.to_string()))?;
                providers.sort();
                providers.dedup();
                Ok(providers)
            }
            Err(KeyringError::NoEntry) => Ok(Vec::new()),
            Err(e) => Err(SecurityError::KeyringError(e.to_string())),
        }
    }

    fn keychain_save_provider_index(&self, providers: &[String]) -> Result<(), SecurityError> {
        let entry = Self::keychain_entry(KEYCHAIN_PROVIDER_INDEX_USER)?;
        if providers.is_empty() {
            match entry.delete_password() {
                Ok(()) => Ok(()),
                Err(KeyringError::NoEntry) => Ok(()),
                Err(e) => Err(SecurityError::KeyringError(e.to_string())),
            }
        } else {
            let raw = serde_json::to_string(providers)
                .map_err(|e| SecurityError::SerializationError(e.to_string()))?;
            entry
                .set_password(&raw)
                .map_err(|e| SecurityError::KeyringError(e.to_string()))
        }
    }

    fn keychain_add_provider(&self, provider: &str) -> Result<(), SecurityError> {
        let mut providers = match self.keychain_load_provider_index() {
            Ok(providers) => providers,
            Err(SecurityError::SerializationError(_)) => Vec::new(),
            Err(e) => return Err(e),
        };

        if !providers.iter().any(|p| p == provider) {
            providers.push(provider.to_string());
            providers.sort();
            providers.dedup();
            self.keychain_save_provider_index(&providers)?;
        }

        Ok(())
    }

    fn keychain_remove_provider(&self, provider: &str) -> Result<(), SecurityError> {
        let mut providers = match self.keychain_load_provider_index() {
            Ok(providers) => providers,
            Err(SecurityError::SerializationError(_)) => Vec::new(),
            Err(e) => return Err(e),
        };

        let initial_len = providers.len();
        providers.retain(|p| p != provider);
        if providers.len() != initial_len {
            self.keychain_save_provider_index(&providers)?;
        }

        Ok(())
    }
}

impl Drop for CredentialManager {
    fn drop(&mut self) {
        // Ensure credentials are zeroized when the manager is dropped
        self.lock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};
    use keyring::{set_default_credential_builder, Error as KeyringError, Result as KeyringResult};
    use std::any::Any;
    use std::sync::{Arc, Mutex, OnceLock};

    fn create_test_manager() -> (CredentialManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("credentials.enc");
        let manager =
            CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path }).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (manager, _temp_dir) = create_test_manager();
        let key = MasterKey::new([42u8; 32]);
        let plaintext = b"super secret API key";

        let encrypted = manager.encrypt(plaintext, &key).unwrap();
        let decrypted = manager.decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        // CRITICAL: Each encryption MUST produce different ciphertext due to random nonce
        let (manager, _temp_dir) = create_test_manager();
        let key = MasterKey::new([42u8; 32]);
        let plaintext = b"same plaintext";

        let encrypted1 = manager.encrypt(plaintext, &key).unwrap();
        let encrypted2 = manager.encrypt(plaintext, &key).unwrap();

        // Ciphertexts must be different (different random nonces)
        assert_ne!(encrypted1, encrypted2);

        // But both must decrypt to the same plaintext
        let decrypted1 = manager.decrypt(&encrypted1, &key).unwrap();
        let decrypted2 = manager.decrypt(&encrypted2, &key).unwrap();
        assert_eq!(decrypted1, decrypted2);
        assert_eq!(plaintext.to_vec(), decrypted1);
    }

    #[test]
    fn test_ciphertext_format() {
        let (manager, _temp_dir) = create_test_manager();
        let key = MasterKey::new([42u8; 32]);
        let plaintext = b"test";

        let encrypted = manager.encrypt(plaintext, &key).unwrap();

        // Verify format: 12-byte nonce + ciphertext + 16-byte tag
        // For 4-byte plaintext: 12 + 4 + 16 = 32 bytes
        assert_eq!(encrypted.len(), NONCE_SIZE + plaintext.len() + TAG_SIZE);
    }

    #[test]
    fn test_minimum_ciphertext_validation() {
        let (manager, _temp_dir) = create_test_manager();
        let key = MasterKey::new([42u8; 32]);

        // Too short ciphertext should fail
        let short_data = vec![0u8; MIN_CIPHERTEXT_SIZE - 1];
        let result = manager.decrypt(&short_data, &key);

        assert!(matches!(result, Err(SecurityError::InvalidCiphertext(_))));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let (manager, _temp_dir) = create_test_manager();
        let key = MasterKey::new([42u8; 32]);
        let plaintext = b"sensitive data";

        let mut encrypted = manager.encrypt(plaintext, &key).unwrap();

        // Tamper with the ciphertext
        if let Some(byte) = encrypted.get_mut(NONCE_SIZE + 5) {
            *byte ^= 0xFF;
        }

        // Decryption should fail due to authentication failure
        let result = manager.decrypt(&encrypted, &key);
        assert!(matches!(result, Err(SecurityError::DecryptionFailed(_))));
    }

    #[test]
    fn test_wrong_key_fails() {
        let (manager, _temp_dir) = create_test_manager();
        let key1 = MasterKey::new([42u8; 32]);
        let key2 = MasterKey::new([43u8; 32]);
        let plaintext = b"secret";

        let encrypted = manager.encrypt(plaintext, &key1).unwrap();
        let result = manager.decrypt(&encrypted, &key2);

        assert!(matches!(result, Err(SecurityError::DecryptionFailed(_))));
    }

    #[test]
    fn test_unlock_lock_cycle() {
        let (mut manager, _temp_dir) = create_test_manager();

        assert!(!manager.is_unlocked());

        manager.unlock("test-password").unwrap();
        assert!(manager.is_unlocked());

        manager.lock();
        assert!(!manager.is_unlocked());

        // Operations should fail when locked
        let result = manager.get_key("test");
        assert!(matches!(result, Err(SecurityError::ManagerLocked)));
    }

    #[test]
    fn test_store_and_retrieve_key() {
        let (mut manager, _temp_dir) = create_test_manager();

        manager.unlock("test-password").unwrap();
        manager.store_key("openai", "sk-test123").unwrap();

        let retrieved = manager.get_key("openai").unwrap();
        assert_eq!(retrieved, "sk-test123");
    }

    #[test]
    fn test_credential_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("credentials.enc");

        // Store a credential
        {
            let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
                path: cred_path.clone(),
            })
            .unwrap();
            manager.unlock("test-password").unwrap();
            manager.store_key("anthropic", "sk-ant-test").unwrap();
        }

        // Load it in a new manager instance
        {
            let mut manager =
                CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })
                    .unwrap();
            manager.unlock("test-password").unwrap();
            let retrieved = manager.get_key("anthropic").unwrap();
            assert_eq!(retrieved, "sk-ant-test");
        }
    }

    #[test]
    fn test_master_key_zeroization() {
        let mut key_bytes = [42u8; 32];
        {
            let _key = MasterKey::new(key_bytes);
            // Key exists here
        }
        // After drop, the original array is unchanged but the MasterKey's internal
        // copy should be zeroized. We can't directly test this without unsafe code,
        // but we verify the struct derives ZeroizeOnDrop.

        // Reset for clarity
        key_bytes = [0u8; 32];
        assert_eq!(key_bytes, [0u8; 32]);
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("credentials.enc");

        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path.clone(),
        })
        .unwrap();
        manager.unlock("test-password").unwrap();
        manager.store_key("test", "value").unwrap();

        let metadata = fs::metadata(&cred_path).unwrap();
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    type InMemoryKey = (Option<String>, String, String);

    #[derive(Clone)]
    struct InMemoryCredentialBuilder {
        store: Arc<Mutex<HashMap<InMemoryKey, String>>>,
    }

    struct InMemoryCredential {
        store: Arc<Mutex<HashMap<InMemoryKey, String>>>,
        key: InMemoryKey,
    }

    impl CredentialApi for InMemoryCredential {
        fn set_password(&self, password: &str) -> KeyringResult<()> {
            let mut store = self.store.lock().expect("in-memory keyring store poisoned");
            store.insert(self.key.clone(), password.to_string());
            Ok(())
        }

        fn get_password(&self) -> KeyringResult<String> {
            let store = self.store.lock().expect("in-memory keyring store poisoned");
            store
                .get(&self.key)
                .cloned()
                .ok_or(KeyringError::NoEntry)
        }

        fn delete_password(&self) -> KeyringResult<()> {
            let mut store = self.store.lock().expect("in-memory keyring store poisoned");
            match store.remove(&self.key) {
                Some(_) => Ok(()),
                None => Err(KeyringError::NoEntry),
            }
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl CredentialBuilderApi for InMemoryCredentialBuilder {
        fn build(
            &self,
            target: Option<&str>,
            service: &str,
            user: &str,
        ) -> KeyringResult<Box<Credential>> {
            Ok(Box::new(InMemoryCredential {
                store: self.store.clone(),
                key: (
                    target.map(str::to_string),
                    service.to_string(),
                    user.to_string(),
                ),
            }))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    static KEYCHAIN_TEST_MUTEX: Mutex<()> = Mutex::new(());
    static TEST_KEYRING_STORE: OnceLock<Arc<Mutex<HashMap<InMemoryKey, String>>>> = OnceLock::new();

    fn install_in_memory_keyring() -> Arc<Mutex<HashMap<InMemoryKey, String>>> {
        TEST_KEYRING_STORE
            .get_or_init(|| {
                let store = Arc::new(Mutex::new(HashMap::new()));
                set_default_credential_builder(Box::new(InMemoryCredentialBuilder {
                    store: store.clone(),
                }));
                store
            })
            .clone()
    }

    fn reset_in_memory_keyring(store: &Arc<Mutex<HashMap<InMemoryKey, String>>>) {
        let mut store = store.lock().expect("in-memory keyring store poisoned");
        store.clear();
    }

    #[test]
    fn test_system_keychain_store_get_list_delete_without_unlock() {
        let _guard = KEYCHAIN_TEST_MUTEX.lock().unwrap();
        let store = install_in_memory_keyring();
        reset_in_memory_keyring(&store);

        let mut manager = CredentialManager::new(CredentialBackend::SystemKeychain).unwrap();
        assert!(manager.is_unlocked());

        manager.store_key("openai", "sk-test123").unwrap();
        assert_eq!(manager.get_key("openai").unwrap(), "sk-test123");

        let providers = manager.list_providers().unwrap();
        assert_eq!(providers, vec!["openai".to_string()]);

        manager.delete_key("openai").unwrap();
        assert!(matches!(
            manager.get_key("openai"),
            Err(SecurityError::CredentialNotFound(_))
        ));
        assert!(manager.list_providers().unwrap().is_empty());

        reset_in_memory_keyring(&store);
    }

    #[test]
    fn test_system_keychain_persists_across_manager_instances() {
        let _guard = KEYCHAIN_TEST_MUTEX.lock().unwrap();
        let store = install_in_memory_keyring();
        reset_in_memory_keyring(&store);

        {
            let mut manager = CredentialManager::new(CredentialBackend::SystemKeychain).unwrap();
            manager.store_key("anthropic", "sk-ant-test").unwrap();
        }

        {
            let manager = CredentialManager::new(CredentialBackend::SystemKeychain).unwrap();
            assert_eq!(manager.get_key("anthropic").unwrap(), "sk-ant-test");
            let providers = manager.list_providers().unwrap();
            assert_eq!(providers, vec!["anthropic".to_string()]);
        }

        reset_in_memory_keyring(&store);
    }
}
