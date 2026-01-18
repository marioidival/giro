use validator::ValidationError;

/// Validate username format
/// - Must be 3-50 characters
/// - Must be alphanumeric
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.len() < 3 {
        return Err(ValidationError::new("username_too_short"));
    }

    if username.len() > 50 {
        return Err(ValidationError::new("username_too_long"));
    }

    if !username.chars().all(|c| c.is_alphanumeric()) {
        return Err(ValidationError::new("username_invalid_characters"));
    }

    Ok(())
}

/// Validate email format
/// - Must be valid email format
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    if !email.contains('@') {
        return Err(ValidationError::new("email_invalid_format"));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(ValidationError::new("email_invalid_format"));
    }

    let (local, domain) = (parts[0], parts[1]);

    if local.is_empty() || domain.is_empty() {
        return Err(ValidationError::new("email_invalid_format"));
    }

    if !domain.contains('.') {
        return Err(ValidationError::new("email_invalid_format"));
    }

    Ok(())
}

/// Validate password strength
/// - Minimum 8 characters
/// - Must contain at least one letter
/// - Must contain at least one number
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }

    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_letter {
        return Err(ValidationError::new("password_missing_letter"));
    }

    if !has_number {
        return Err(ValidationError::new("password_missing_number"));
    }

    Ok(())
}

/// Validate loop name
/// - Must be 1-100 characters
pub fn validate_loop_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::new("loop_name_empty"));
    }

    if name.len() > 100 {
        return Err(ValidationError::new("loop_name_too_long"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username_accepts_valid() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("bob123").is_ok());
        assert!(validate_username("ABC").is_ok());
    }

    #[test]
    fn test_validate_username_rejects_too_short() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("").is_err());
    }

    #[test]
    fn test_validate_username_rejects_too_long() {
        let long_name = "a".repeat(51);
        assert!(validate_username(&long_name).is_err());
    }

    #[test]
    fn test_validate_username_rejects_invalid_characters() {
        assert!(validate_username("alice@example.com").is_err());
        assert!(validate_username("alice_bob").is_err());
        assert!(validate_username("alice!").is_err());
    }

    #[test]
    fn test_validate_email_accepts_valid() {
        assert!(validate_email("alice@example.com").is_ok());
        assert!(validate_email("bob@example.co.uk").is_ok());
        assert!(validate_email("test+tag@example.com").is_ok());
    }

    #[test]
    fn test_validate_email_rejects_invalid() {
        assert!(validate_email("notanemail").is_err());
        assert!(validate_email("missing@domain").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn test_validate_password_accepts_valid() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("Secret12").is_ok());
        assert!(validate_password("abc123456").is_ok());
    }

    #[test]
    fn test_validate_password_rejects_too_short() {
        assert!(validate_password("pass1").is_err());
        assert!(validate_password("short").is_err());
    }

    #[test]
    fn test_validate_password_rejects_missing_letter() {
        assert!(validate_password("12345678").is_err());
    }

    #[test]
    fn test_validate_password_rejects_missing_number() {
        assert!(validate_password("password").is_err());
    }

    #[test]
    fn test_validate_loop_name_accepts_valid() {
        assert!(validate_loop_name("Test Loop").is_ok());
        assert!(validate_loop_name("A").is_ok());
        assert!(validate_loop_name(&"a".repeat(100)).is_ok());
    }

    #[test]
    fn test_validate_loop_name_rejects_empty() {
        assert!(validate_loop_name("").is_err());
    }

    #[test]
    fn test_validate_loop_name_rejects_too_long() {
        let long_name = "a".repeat(101);
        assert!(validate_loop_name(&long_name).is_err());
    }
}
