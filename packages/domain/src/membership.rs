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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_is_admin() {
        assert!(Role::Admin.is_admin());
    }

    #[test]
    fn member_is_not_admin() {
        assert!(!Role::Member.is_admin());
    }

    #[test]
    fn active_membership_is_accessible() {
        let m = Membership {
            id: "mem_1".into(),
            community_id: "com_1".into(),
            user_id: "usr_1".into(),
            role: Role::Member,
            display_name: "Aya".into(),
            is_active: true,
        };
        assert!(m.is_active);
        assert!(!m.role.is_admin());
    }

    #[test]
    fn removed_membership_is_inactive() {
        let m = Membership {
            id: "mem_1".into(),
            community_id: "com_1".into(),
            user_id: "usr_1".into(),
            role: Role::Member,
            display_name: "Aya".into(),
            is_active: false, // removed_at set
        };
        assert!(!m.is_active, "removed membership must not be active");
    }
}
