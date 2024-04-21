use std::fmt::Debug;

use crate::custom_types::{GroupType, UserRole};
use crate::extractors::extractors::User;
use crate::server::app_state::AppState;
use actix_utils::future::{ok, Ready};
use actix_web::{get, post, put, web, Error, FromRequest, HttpRequest, HttpResponse, Responder};
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::any::AnyConnectionBackend;
use sqlx::prelude::FromRow;
use sqlx::{query, query_as, Execute, Executor, Pool, Postgres, Statement};

#[derive(Serialize, Deserialize)]
struct AllGroups {
    id: i32,
    group_name: String,
    parent_group_id: Option<i32>,
    created_by: Option<i32>,
    #[serde(skip_serializing, skip_deserializing)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    r#type: GroupType,
    read: bool,
    write: bool,
    moderate: bool,
    admin: bool,
}

#[get("")]
async fn handle_get_groups(app: web::Data<AppState>, user: User) -> HttpResponse {
    let val = get_groups(Some(user.user_id), &app.pool).await.unwrap();
    HttpResponse::Ok().json(val)
}

#[derive(Serialize)]
struct GetGroupsReturn {
    r#type: GroupType,
    group_name: String,
    parent_group_id: Option<i32>,
}

async fn get_groups(
    user_id: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<Vec<GetGroupsReturn>, sqlx::Error> {
    let val = match user_id {
        Some(user_id) => {
            sqlx::query_as!(
                GetGroupsReturn,
                r#"select
            type as "type!: GroupType",
            group_name, parent_group_id from groups g 
            join group_permissions gp on gp.group_id = g.id where  gp.user_id = $1
            and gp.read
            "#,
                user_id
            )
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as!(
                GetGroupsReturn,
                r#"select
            type as "type!: GroupType",
            group_name, parent_group_id from groups g"#
            )
            .fetch_all(pool)
            .await
        }
    };
    val
}

#[derive(Serialize, Deserialize)]
struct CreateGroupRequest {
    group_name: String,
    parent_group_id: Option<i32>,
    group_type: GroupType,
}

#[post("/create")]
async fn handle_create_group(
    app: web::Data<AppState>,
    group_req: web::Json<CreateGroupRequest>,
    user: User,
) -> HttpResponse {
    create_group(
        &app.pool,
        &group_req.group_name,
        group_req.group_type,
        group_req.parent_group_id,
        user,
    )
    .await
}

#[derive(Serialize, Deserialize)]
struct CreateGroupResponse {
    group_id: i32,
}

async fn create_group(
    pool: &Pool<Postgres>,
    group_name: &String,
    group_type: GroupType,
    parent_group_id: Option<i32>,
    user: User,
) -> HttpResponse {
    let group = sqlx::query!(
        r#"insert into groups(group_name, parent_group_id, created_by, type)
    values ( $1, $2, $3, $4 ) returning id"#,
        group_name,
        parent_group_id,
        user.user_id,
        group_type as GroupType
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let result = sqlx::query!(
        r"insert into group_permissions (user_id, group_id,
        read, write, moderate, admin)
        values (
            $1, $2, $3, $4, $5, $6
        )
        returning id
         ",
        user.user_id,
        group.id,
        true,
        true,
        true,
        false
    )
    .fetch_one(pool)
    .await;

    HttpResponse::Ok().json(CreateGroupResponse { group_id: group.id })
}

#[get("{group_id}/members")]
async fn get_group_members_handle(
    group_id: web::Path<i32>,
    app: web::Data<AppState>,
) -> impl Responder {
    let group_id = group_id.into_inner();
    let ret = get_group_members(group_id, &app.pool).await;
    return ret;
}

#[derive(Serialize)]
struct GroupMembersResponse {
    user_id: i32,
    username: String,
    user_role: UserRole,
    email: String,
}

async fn get_group_members(group_id: i32, pool: &Pool<Postgres>) -> impl Responder {
    let rows = match sqlx::query_as!(
        GroupMembersResponse,
        r#"select u.username, u.id as user_id,
    u.role as "user_role!: UserRole",
    u.email
    from group_permissions gp
    join users u on gp.user_id = u.id
    where gp.group_id = $1"#,
        group_id
    )
    .fetch_all(pool)
    .await
    {
        Ok(val) => val,
        _ => return HttpResponse::InternalServerError().into(),
        Err(_) => todo!(),
    };

    HttpResponse::Ok().json(rows)
}

#[put("{group_id}/message")]
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

#[post("{group_id}/add_user/{user_id}")]
async fn handle_add_to_group(
    group_id: web::Path<i32>,
    user_id: web::Path<i32>,
    user: User,
    app: web::Data<AppState>,
) -> impl Responder {
    // check user_id adding is allowed to add
    if !is_user_admin(user.user_id, group_id.to_owned(), &app.pool).await {
        return HttpResponse::Unauthorized().body(format!(
            "user {} is not permitted to add users to group {}",
            user.user_id,
            group_id.to_owned()
        ));
    }
    match add_to_group(group_id.to_owned(), user_id.to_owned(), user.user_id, &app.pool).await {
        Ok(_val) => HttpResponse::Ok().body(""),
        Err(_e) => HttpResponse::InternalServerError().body("Unexpected error occured when trying to add user to group")
    }
}

async fn is_user_admin(user_id: i32, group_id: i32, pool: &Pool<Postgres>) -> bool {
    match sqlx::query!(
        r#"select moderate from group_permissions where group_id = $1 and user_id = $2"#,
        group_id,
        user_id
    )
    .fetch_one(pool)
    .await
    {
        Ok(rec) => return rec.moderate,
        Err(_e) => return false,
    }
}

async fn add_to_group(group_id: i32, user_id_added: i32, user_id_adding: i32, pool: &Pool<Postgres>) -> Result<i32, sqlx::Error> {
    match sqlx::query!(r#"insert into group_permissions (user_id, group_id, created_by, write) values
    ( $1, $2, $3, $4) returning id
    "#, user_id_added, group_id, user_id_adding, true).fetch_one(pool).await {
        Ok(val) => return Ok(val.id),
        Err(e) => return Err(e)
    }
}
