#![allow(dead_code)]
//! Community table access.

use worker::{D1Database, Result};

pub struct CommunityRow {
    pub id: String,
    pub name: String,
    pub timezone: String,
}

pub async fn find_active(db: &D1Database, community_id: &str) -> Result<Option<CommunityRow>> {
    let row = db
        .prepare(
            "SELECT id, name, timezone FROM communities \
             WHERE id = ?1 AND is_active = 1 LIMIT 1",
        )
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(CommunityRow {
            id: v.get("id")?.as_str()?.to_owned(),
            name: v.get("name")?.as_str()?.to_owned(),
            timezone: v.get("timezone")?.as_str()?.to_owned(),
        })
    }))
}
