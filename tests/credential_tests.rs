//! Credential storage tests
//!
//! Tests for:
//! - Basic store and retrieve operations
//! - Wrong password handling
//! - Key not found error
//! - Lock clears master key
//! - File permissions verification
//! - Multiple keys storage

use aiy_core::security::{CredentialBackend, CredentialManager, SecurityError};
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Helper to create a test credential manager with tempdir
fn create_test_manager() -> (CredentialManager, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cred_path = temp_dir.path().join("credentials.enc");
    let manager = CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })
        .expect("Failed to create credential manager");
    (manager, temp_dir)
}

/// Test basic store and retrieve round-trip
#[test]
fn test_store_and_retrieve_key() {
    let (mut manager, _temp_dir) = create_test_manager();

    // Unlock the manager
    manager.unlock("test-password").expect("Failed to unlock manager");

    // Store a key
    let provider = "openai";
    let api_key = "sk-test-1234567890abcdef";
    manager
        .store_key(provider, api_key)
        .expect("Failed to store key");

    // Retrieve the key
    let retrieved = manager.get_key(provider).expect("Failed to retrieve key");

    assert_eq!(
        retrieved, api_key,
        "Retrieved key should match stored key"
    );
}

/// Test that wrong password cannot decrypt stored credentials
#[test]
fn test_wrong_password_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cred_path = temp_dir.path().join("credentials.enc");

    // Store a credential with one password
    {
        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path.clone(),
        })
        .expect("Failed to create manager");

        manager.unlock("correct-password").expect("Failed to unlock");
        manager
            .store_key("provider", "secret-key")
            .expect("Failed to store key");
    }

    // Try to retrieve with wrong password
    {
        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path.clone(),
        })
        .expect("Failed to create manager");

        manager.unlock("wrong-password").expect("Unlock should succeed");

        // The key retrieval or decryption should fail because the wrong password
        // derives a different master key
        let result = manager.get_key("provider");

        // Either get_key fails due to decryption error, or if the cache was loaded
        // with wrong decryption, we may get corrupted data or a decryption error
        // during load_encrypted_keys
        match result {
            Ok(key) => {
                // If we got a key, it should NOT be the original
                // (extremely unlikely to get valid UTF-8 from wrong key)
                assert_ne!(
                    key, "secret-key",
                    "Wrong password should not retrieve original key"
                );
            }
            Err(_) => {
                // Expected - decryption failed with wrong key
            }
        }
    }
}

/// Test that requesting a non-existent key returns an error
#[test]
fn test_key_not_found() {
    let (mut manager, _temp_dir) = create_test_manager();

    manager.unlock("test-password").expect("Failed to unlock");

    let result = manager.get_key("nonexistent-provider");

    assert!(
        matches!(result, Err(SecurityError::CredentialNotFound(_))),
        "Should return CredentialNotFound for missing key, got: {:?}",
        result
    );
}

/// Test that lock clears the master key
#[test]
fn test_lock_clears_master_key() {
    let (mut manager, _temp_dir) = create_test_manager();

    // Initially locked
    assert!(
        !manager.is_unlocked(),
        "Manager should be locked initially"
    );

    // Unlock
    manager.unlock("test-password").expect("Failed to unlock");
    assert!(manager.is_unlocked(), "Manager should be unlocked after unlock()");

    // Store a key while unlocked
    manager.store_key("test", "secret").expect("Failed to store key");

    // Lock
    manager.lock();
    assert!(
        !manager.is_unlocked(),
        "Manager should be locked after lock()"
    );

    // Operations should fail when locked
    let store_result = manager.store_key("another", "value");
    assert!(
        matches!(store_result, Err(SecurityError::ManagerLocked)),
        "store_key should fail when locked"
    );

    let get_result = manager.get_key("test");
    assert!(
        matches!(get_result, Err(SecurityError::ManagerLocked)),
        "get_key should fail when locked"
    );
}

/// Test that credential files have 600 permissions on Unix
#[cfg(unix)]
#[test]
fn test_file_permissions_are_600() {
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cred_path = temp_dir.path().join("credentials.enc");

    let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
        path: cred_path.clone(),
    })
    .expect("Failed to create manager");

    manager.unlock("test-password").expect("Failed to unlock");
    manager
        .store_key("test-provider", "test-api-key")
        .expect("Failed to store key");

    // Check file permissions
    let metadata = fs::metadata(&cred_path).expect("Failed to get file metadata");
    let permissions = metadata.permissions();
    let mode = permissions.mode() & 0o777;

    assert_eq!(
        mode, 0o600,
        "Credential file should have 600 permissions (owner read/write only), got {:o}",
        mode
    );

    // Also check the salt file
    let salt_path = temp_dir.path().join("credentials.salt");
    if salt_path.exists() {
        let salt_metadata = fs::metadata(&salt_path).expect("Failed to get salt file metadata");
        let salt_permissions = salt_metadata.permissions();
        let salt_mode = salt_permissions.mode() & 0o777;

        assert_eq!(
            salt_mode, 0o600,
            "Salt file should have 600 permissions, got {:o}",
            salt_mode
        );
    }
}

/// Test storing and retrieving multiple keys for different providers
#[test]
fn test_multiple_keys_storage() {
    let (mut manager, _temp_dir) = create_test_manager();

    manager.unlock("test-password").expect("Failed to unlock");

    // Store multiple keys
    let providers = vec![
        ("openai", "sk-openai-1234567890abcdef"),
        ("anthropic", "sk-ant-anthropic-secret-key"),
        ("google", "AIzaSyGoogleCloudKey12345"),
        ("azure", "azure-key-abcdef123456"),
        ("cohere", "cohere-api-key-secret"),
    ];

    for (provider, key) in &providers {
        manager
            .store_key(provider, key)
            .expect(&format!("Failed to store key for {}", provider));
    }

    // Retrieve all keys and verify
    for (provider, expected_key) in &providers {
        let retrieved = manager
            .get_key(provider)
            .expect(&format!("Failed to retrieve key for {}", provider));

        assert_eq!(
            &retrieved, expected_key,
            "Key for {} should match",
            provider
        );
    }

    // Verify list of providers
    let stored_providers = manager
        .list_providers()
        .expect("Failed to list providers");

    for (provider, _) in &providers {
        assert!(
            stored_providers.contains(&provider.to_string()),
            "Provider {} should be in the list",
            provider
        );
    }

    assert_eq!(
        stored_providers.len(),
        providers.len(),
        "Should have exactly {} providers",
        providers.len()
    );
}

/// Test that credentials persist across manager instances
#[test]
fn test_credential_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cred_path = temp_dir.path().join("credentials.enc");
    let password = "persistence-test-password";

    let test_credentials = vec![
        ("provider1", "key1-secret"),
        ("provider2", "key2-secret"),
    ];

    // Store credentials in first manager instance
    {
        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path.clone(),
        })
        .expect("Failed to create manager");

        manager.unlock(password).expect("Failed to unlock");

        for (provider, key) in &test_credentials {
            manager.store_key(provider, key).expect("Failed to store key");
        }

        // Manager is dropped here, credentials should be persisted
    }

    // Create new manager instance and verify credentials
    {
        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path.clone(),
        })
        .expect("Failed to create manager");

        manager.unlock(password).expect("Failed to unlock");

        for (provider, expected_key) in &test_credentials {
            let retrieved = manager
                .get_key(provider)
                .expect(&format!("Failed to retrieve key for {}", provider));

            assert_eq!(
                &retrieved, expected_key,
                "Persisted key for {} should match",
                provider
            );
        }
    }
}

/// Test deleting a credential
#[test]
fn test_delete_credential() {
    let (mut manager, _temp_dir) = create_test_manager();

    manager.unlock("test-password").expect("Failed to unlock");

    // Store a key
    manager.store_key("to-delete", "secret").expect("Failed to store key");

    // Verify it exists
    assert!(manager.get_key("to-delete").is_ok(), "Key should exist before deletion");

    // Delete it
    manager.delete_key("to-delete").expect("Failed to delete key");

    // Verify it's gone
    let result = manager.get_key("to-delete");
    assert!(
        matches!(result, Err(SecurityError::CredentialNotFound(_))),
        "Key should not exist after deletion"
    );
}
