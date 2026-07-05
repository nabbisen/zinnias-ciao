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
mod tests;
