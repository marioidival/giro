use anyhow::Result;
use bcrypt::{DEFAULT_COST, hash, verify};

/// Hashes a password using bcrypt with the default cost factor.
///
/// # Arguments
/// * `password` - The password to hash
///
/// # Returns
/// A Result containing the hashed password string
///
/// # Example
/// ```
/// let hashed = hash_password("my_password")?;
/// ```
pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

/// Verifies a password against a bcrypt hash.
///
/// # Arguments
/// * `password` - The password to verify
/// * `hashed` - The bcrypt hash to compare against
///
/// # Returns
/// A Result containing true if the password matches, false otherwise
///
/// # Example
/// ```
/// let valid = verify_password("my_password", &hashed)?;
/// ```
pub fn verify_password(password: &str, hashed: &str) -> Result<bool> {
    let is_valid = verify(password, hashed)?;
    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_same_password() {
        let password = "test_password_123";
        let hashed = hash_password(password).expect("Failed to hash password");

        assert_ne!(hashed, password);
        let is_valid = verify_password(password, &hashed).expect("Failed to verify password");
        assert!(is_valid);
    }

    #[test]
    fn test_verify_fails_with_wrong_password() {
        let password = "test_password_123";
        let wrong_password = "wrong_password_456";
        let hashed = hash_password(password).expect("Failed to hash password");

        let is_valid = verify_password(wrong_password, &hashed).expect("Failed to verify password");
        assert!(!is_valid);
    }
}
