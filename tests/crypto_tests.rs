//! Cryptographic security tests
//!
//! Tests for:
//! - Nonce uniqueness across encryptions
//! - Same plaintext produces different ciphertext
//! - Ciphertext format validation
//! - Encryption/decryption roundtrip

use aiy_core::security::{CredentialBackend, CredentialManager, MasterKey, SecurityError};
use std::collections::HashSet;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test credential manager
fn create_test_manager() -> (CredentialManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cred_path = temp_dir.path().join("credentials.enc");
    let manager = CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })
        .expect("Failed to create credential manager");
    (manager, temp_dir)
}

/// Test that 1000 encryptions produce 1000 unique nonces
#[test]
fn test_nonce_uniqueness_across_encryptions() {
    let (manager, _temp_dir) = create_test_manager();
    let key = MasterKey::new([42u8; 32]);
    let plaintext = b"test data for nonce uniqueness";

    let mut nonces: HashSet<[u8; 12]> = HashSet::new();

    for _ in 0..1000 {
        let encrypted = manager.encrypt(plaintext, &key).expect("Encryption should succeed");

        // Extract nonce (first 12 bytes)
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&encrypted[..12]);

        // Each nonce should be unique
        let is_new = nonces.insert(nonce);
        assert!(is_new, "Duplicate nonce detected! Nonces must be unique.");
    }

    assert_eq!(nonces.len(), 1000, "Should have 1000 unique nonces");
}

/// Test that encrypting the same plaintext produces different ciphertext each time
#[test]
fn test_same_plaintext_different_ciphertext() {
    let (manager, _temp_dir) = create_test_manager();
    let key = MasterKey::new([42u8; 32]);
    let plaintext = b"identical plaintext for multiple encryptions";

    let encrypted1 = manager.encrypt(plaintext, &key).expect("First encryption should succeed");
    let encrypted2 = manager.encrypt(plaintext, &key).expect("Second encryption should succeed");

    // The ciphertexts should be different due to random nonces
    assert_ne!(
        encrypted1, encrypted2,
        "Same plaintext should produce different ciphertext due to random nonces"
    );

    // But both should decrypt to the same plaintext
    let decrypted1 = manager.decrypt(&encrypted1, &key).expect("First decryption should succeed");
    let decrypted2 = manager.decrypt(&encrypted2, &key).expect("Second decryption should succeed");

    assert_eq!(decrypted1, decrypted2, "Both should decrypt to same plaintext");
    assert_eq!(
        decrypted1.as_slice(),
        plaintext,
        "Decrypted data should match original"
    );
}

/// Test that ciphertext shorter than 28 bytes is rejected
/// (12 bytes nonce + 16 bytes authentication tag minimum)
#[test]
fn test_ciphertext_format_validation() {
    let (manager, _temp_dir) = create_test_manager();
    let key = MasterKey::new([42u8; 32]);

    // Test various invalid lengths
    let test_cases = vec![
        (vec![], "empty"),
        (vec![0u8; 1], "1 byte"),
        (vec![0u8; 11], "11 bytes"),
        (vec![0u8; 12], "12 bytes (nonce only)"),
        (vec![0u8; 20], "20 bytes"),
        (vec![0u8; 27], "27 bytes (just under minimum)"),
    ];

    for (invalid_ciphertext, description) in test_cases {
        let result = manager.decrypt(&invalid_ciphertext, &key);
        assert!(
            matches!(result, Err(SecurityError::InvalidCiphertext(_))),
            "Ciphertext of {} should be rejected as InvalidCiphertext, got: {:?}",
            description,
            result
        );
    }

    // 28 bytes should be accepted (though decryption will fail due to invalid tag)
    let exactly_28 = vec![0u8; 28];
    let result = manager.decrypt(&exactly_28, &key);
    assert!(
        !matches!(result, Err(SecurityError::InvalidCiphertext(_))),
        "28 bytes should pass format validation (may fail decryption for other reasons)"
    );
}

/// Test that encryption followed by decryption returns the original plaintext
#[test]
fn test_encrypt_decrypt_roundtrip() {
    let (manager, _temp_dir) = create_test_manager();
    let key = MasterKey::new([42u8; 32]);

    // Test various plaintext sizes
    let test_cases = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"short".to_vec(),
        b"medium length plaintext for testing".to_vec(),
        vec![0u8; 1000],  // 1KB of zeros
        vec![0xFFu8; 5000], // 5KB of 0xFF
        (0..256).cycle().take(10000).collect::<Vec<u8>>(), // 10KB pattern
    ];

    for plaintext in test_cases {
        let encrypted = manager.encrypt(&plaintext, &key)
            .expect("Encryption should succeed for all plaintext sizes");

        let decrypted = manager.decrypt(&encrypted, &key)
            .expect("Decryption should succeed for valid ciphertext");

        assert_eq!(
            decrypted, plaintext,
            "Roundtrip should preserve plaintext of length {}",
            plaintext.len()
        );
    }
}

/// Test that decryption with wrong key fails
#[test]
fn test_wrong_key_fails_decryption() {
    let (manager, _temp_dir) = create_test_manager();
    let key1 = MasterKey::new([1u8; 32]);
    let key2 = MasterKey::new([2u8; 32]);
    let plaintext = b"secret message";

    let encrypted = manager.encrypt(plaintext, &key1).expect("Encryption should succeed");
    let result = manager.decrypt(&encrypted, &key2);

    assert!(
        result.is_err(),
        "Decryption with wrong key should fail"
    );
}

/// Test that tampered ciphertext is rejected
#[test]
fn test_tampered_ciphertext_rejected() {
    let (manager, _temp_dir) = create_test_manager();
    let key = MasterKey::new([42u8; 32]);
    let plaintext = b"message to be tampered with";

    let mut encrypted = manager.encrypt(plaintext, &key).expect("Encryption should succeed");

    // Tamper with the ciphertext (flip a bit in the middle)
    let middle = encrypted.len() / 2;
    encrypted[middle] ^= 0xFF;

    let result = manager.decrypt(&encrypted, &key);
    assert!(
        result.is_err(),
        "Tampered ciphertext should be rejected"
    );
}
