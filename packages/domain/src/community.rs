use serde::{Deserialize, Serialize};

pub const COMMUNITY_NAME_MAX: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub is_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommunityNameError {
    Empty,
    TooLong,
    InvalidCharacter,
}

pub fn validate_community_name(raw: &str) -> Result<String, CommunityNameError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CommunityNameError::Empty);
    }
    if trimmed.chars().count() > COMMUNITY_NAME_MAX {
        return Err(CommunityNameError::TooLong);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(CommunityNameError::InvalidCharacter);
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn community_name_is_trimmed() {
        assert_eq!(
            validate_community_name("  Lunch group  ").unwrap(),
            "Lunch group"
        );
    }

    #[test]
    fn community_name_rejects_empty() {
        assert_eq!(
            validate_community_name(" \n "),
            Err(CommunityNameError::Empty)
        );
    }

    #[test]
    fn community_name_rejects_long_values() {
        let too_long = "a".repeat(COMMUNITY_NAME_MAX + 1);
        assert_eq!(
            validate_community_name(&too_long),
            Err(CommunityNameError::TooLong)
        );
    }

    #[test]
    fn community_name_rejects_control_chars() {
        assert_eq!(
            validate_community_name("Lunch\u{0000}group"),
            Err(CommunityNameError::InvalidCharacter)
        );
    }
}
