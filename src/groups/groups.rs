use std::fmt::Debug;

use crate::extractors::extractors::User;
use crate::server::app_state::AppState;
use crate::custom_types::UserRole;
use actix_web::{get, post, put, web, HttpRequest, Responder, HttpResponse};
use serde::Serialize;
use sqlx::{Postgres, Pool};
use serde_json;

// #[derive(Serialize, Deserialize, Debug)]
// struct GroupMember {
//     user_id: i32,
//     group_id: i32,
//     read: bool,
//     write: bool,
//     moderate: bool,
//     admin: bool
// }

#[get("groups/{group_id}/members")]
async fn get_group_members_handle(
    group_id: web::Path<i32>,
    app: web::Data<AppState>,
    user: User
) -> impl Responder {
    let group_id = group_id.into_inner();
    let ret = get_group_members(group_id, &app.pool).await;
    return ret
}

async fn get_group_members(group_id: i32, pool: &Pool<Postgres>) -> impl Responder {
    let rows = match sqlx::query!(
    r#"select gp.*, u.username,
    u.role as "user_role!: UserRole",
    u.email
    from group_permissions gp
    join users u on gp.user_id = u.id
    where gp.group_id = $1"#, group_id).fetch_all(pool).await {
        Ok(val) => val,
        _ => return HttpResponse::InternalServerError()
    };
    
    let mut resp = HttpResponse::Ok();
    resp
}

#[put("/groups/{group_id}/message")]
async fn write_to_group(
    group_id: web::Path<u32>,
    app: web::Data<AppState>,
    user: User,
    req: HttpRequest,
) -> impl Responder {
    let group_id = group_id.into_inner();
    let pool = &app.pool;
    let user_id = user.user_id;


    format!("hi")
}
