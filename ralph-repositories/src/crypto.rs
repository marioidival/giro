use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::RngCore;

/// Encryption key for token encryption
/// In production, this should come from environment variable or secure key management service
const ENCRYPTION_KEY: &[u8; 32] = b"ralph-loop-manager-enc-key-32b!!";

/// Encrypt a token using AES-256-GCM encryption
///
/// # Arguments
/// * `plaintext` - The token to encrypt (e.g., Git personal access token)
///
/// # Returns
/// Base64-encoded encrypted token (nonce + ciphertext)
///
/// # Errors
/// Returns error if encryption fails
///
/// # Example
/// ```no_run
/// # use ralph_repositories::crypto::encrypt_token;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let token = "ghp_my_secret_token";
/// let encrypted = encrypt_token(token)?;
/// println!("Encrypted: {}", encrypted);
/// # Ok(())
/// # }
/// ```
pub fn encrypt_token(plaintext: &str) -> Result<String> {
    let cipher = Aes256Gcm::new(ENCRYPTION_KEY.into());

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);

    Ok(STANDARD.encode(&combined))
}

/// Decrypt a token using AES-256-GCM decryption
///
/// # Arguments
/// * `ciphertext` - Base64-encoded encrypted token (nonce + ciphertext)
///
/// # Returns
/// Decrypted plaintext token
///
/// # Errors
/// Returns error if decryption fails or ciphertext is invalid
///
/// # Example
/// ```no_run
/// # use ralph_repositories::crypto::{encrypt_token, decrypt_token};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let token = "ghp_my_secret_token";
/// let encrypted = encrypt_token(token)?;
/// let decrypted = decrypt_token(&encrypted)?;
/// assert_eq!(token, decrypted);
/// # Ok(())
/// # }
/// ```
pub fn decrypt_token(ciphertext: &str) -> Result<String> {
    let combined = STANDARD
        .decode(ciphertext)
        .map_err(|e| anyhow!("Base64 decoding failed: {}", e))?;

    if combined.len() < 12 {
        return Err(anyhow!("Invalid ciphertext: too short (nonce is 12 bytes)"));
    }

    let (nonce_bytes, ciphertext_bytes) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(ENCRYPTION_KEY.into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow!("UTF-8 conversion failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt_token() {
        let plaintext = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let encrypted = encrypt_token(plaintext).expect("Encryption should succeed");
        let decrypted = decrypt_token(&encrypted).expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encryption_produces_different_output() {
        let plaintext = "ghp_test_token";

        let encrypted1 = encrypt_token(plaintext).expect("Encryption should succeed");
        let encrypted2 = encrypt_token(plaintext).expect("Encryption should succeed");

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let invalid_ciphertext = "not-valid-base64!!!";
        let result = decrypt_token(invalid_ciphertext);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Base64 decoding failed")
        );
    }

    #[test]
    fn test_decrypt_too_short_ciphertext_fails() {
        let short_ciphertext = "YWJj";
        let result = decrypt_token(short_ciphertext);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_decrypt_tampered_ciphertext_fails() {
        let plaintext = "ghp_test_token";
        let encrypted = encrypt_token(plaintext).expect("Encryption should succeed");

        let mut encrypted_bytes = encrypted.into_bytes();
        if let Some(last_byte) = encrypted_bytes.last_mut() {
            *last_byte = last_byte.wrapping_add(1);
        }
        let tampered = String::from_utf8(encrypted_bytes).expect("Valid UTF-8");

        let result = decrypt_token(&tampered);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Decryption failed")
        );
    }

    #[test]
    fn test_encryption_unicode_token() {
        let plaintext = "ghp_😀测试🚀token";
        let encrypted = encrypt_token(plaintext).expect("Encryption should succeed");
        let decrypted = decrypt_token(&encrypted).expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encryption_empty_token() {
        let plaintext = "";
        let encrypted = encrypt_token(plaintext).expect("Encryption should succeed");
        let decrypted = decrypt_token(&encrypted).expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encryption_long_token() {
        let plaintext = "a".repeat(10000);
        let encrypted = encrypt_token(&plaintext).expect("Encryption should succeed");
        let decrypted = decrypt_token(&encrypted).expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }
}
