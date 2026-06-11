use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Member,
}

impl Role {
    pub fn is_admin(self) -> bool {
        matches!(self, Role::Admin)
    }
}

#[derive(Debug, Clone)]
pub struct Membership {
    pub id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: Role,
    pub display_name: String,
    pub is_active: bool, // false when removed_at is set
}
