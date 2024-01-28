use serde::{Deserialize, Serialize};
use sqlx;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    ADMIN,
    SUPER,
    NORMAL,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy)]
#[sqlx(type_name = "group_type", rename_all = "lowercase")]
pub enum GroupType {
    CHANNEL,
    ROOM,
    TEAM,
}
