use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use hex::FromHex;

/// Encryption key size for Aes256Gcm (32 bytes)
const KEY_SIZE: usize = 32;

/// Nonce size for Aes256Gcm (12 bytes)
const NONCE_SIZE: usize = 12;

/// Get encryption key from environment variable
///
/// The ENCRYPTION_KEY environment variable must be a 64-character hex string
/// representing a 32-byte key for Aes256Gcm encryption.
fn get_encryption_key() -> Result<[u8; KEY_SIZE]> {
    let hex_key = std::env::var("ENCRYPTION_KEY")
        .context("ENCRYPTION_KEY environment variable not set")?;

    if hex_key.len() != 64 {
        return Err(anyhow!("ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)"));
    }

    <[u8; KEY_SIZE]>::from_hex(hex_key)
        .map_err(|_| anyhow!("ENCRYPTION_KEY must be valid hexadecimal"))
}

/// Encrypt an API key using Aes256Gcm
///
/// # Arguments
/// * `plaintext` - The API key to encrypt
///
/// # Returns
/// Base64-encoded string containing nonce + ciphertext
///
/// # Process
/// 1. Get encryption key from environment
/// 2. Generate random nonce (12 bytes)
/// 3. Encrypt plaintext with key and nonce
/// 4. Combine nonce + ciphertext and encode as base64
pub fn encrypt_api_key(plaintext: &str) -> Result<String> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("Invalid encryption key length"))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Combine nonce + ciphertext and encode as base64
    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);

    Ok(STANDARD.encode(combined))
}

/// Decrypt an API key that was encrypted with `encrypt_api_key`
///
/// # Arguments
/// * `encrypted` - Base64-encoded string containing nonce + ciphertext
///
/// # Returns
/// Decrypted API key as a string
///
/// # Process
/// 1. Decode base64 to get nonce + ciphertext
/// 2. Extract nonce (first 12 bytes)
/// 3. Extract ciphertext (remaining bytes)
/// 4. Decrypt ciphertext with key and nonce
pub fn decrypt_api_key(encrypted: &str) -> Result<String> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("Invalid encryption key length"))?;

    let combined = STANDARD.decode(encrypted).context("Base64 decoding failed")?;

    if combined.len() < NONCE_SIZE {
        return Err(anyhow!("Encrypted data is too short"));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test encryption/decryption round-trip
    #[test]
    #[serial_test::serial]
    fn test_encrypt_decrypt_round_trip() {
        // Set a test encryption key (64 hex characters = 32 bytes)
        unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let plaintext = "sk-ant-api03-1234567890";
        let encrypted = encrypt_api_key(plaintext).unwrap();
        let decrypted = decrypt_api_key(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    // Test that encryption produces different output each time (due to random nonce)
    #[test]
    #[serial_test::serial]
    fn test_encryption_randomness() {
        unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let plaintext = "sk-ant-api03-1234567890";
        let encrypted1 = encrypt_api_key(plaintext).unwrap();
        let encrypted2 = encrypt_api_key(plaintext).unwrap();

        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(decrypt_api_key(&encrypted1).unwrap(), plaintext);
        assert_eq!(decrypt_api_key(&encrypted2).unwrap(), plaintext);
    }

    // Test with different inputs
    #[test]
    #[serial_test::serial]
    fn test_encrypt_different_inputs() {
        unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let key1 = "sk-ant-api03-key1";
        let key2 = "sk-ant-api03-key2";
        let key3 = "sk-openai-key3";

        let encrypted1 = encrypt_api_key(key1).unwrap();
        let encrypted2 = encrypt_api_key(key2).unwrap();
        let encrypted3 = encrypt_api_key(key3).unwrap();

        assert_eq!(decrypt_api_key(&encrypted1).unwrap(), key1);
        assert_eq!(decrypt_api_key(&encrypted2).unwrap(), key2);
        assert_eq!(decrypt_api_key(&encrypted3).unwrap(), key3);
    }

    // Test error handling for missing environment variable
    #[test]
    #[serial_test::serial]
    fn test_missing_encryption_key() {
        unsafe { std::env::remove_var("ENCRYPTION_KEY"); }

        let result = encrypt_api_key("test-key");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("ENCRYPTION_KEY environment variable not set"));
    }

    // Test error handling for invalid base64
    #[test]
    #[serial_test::serial]
    fn test_invalid_base64() {
        unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let result = decrypt_api_key("not-valid-base64!!!");
        assert!(result.is_err());
    }

    // Test error handling for corrupted data
    #[test]
    #[serial_test::serial]
    fn test_corrupted_data() {
        unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        // Valid base64 but will fail decryption
        let result = decrypt_api_key("dGVzdC1kYXRh");
        assert!(result.is_err());
    }
}
