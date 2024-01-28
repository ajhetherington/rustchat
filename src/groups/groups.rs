use std::fmt::Debug;

use crate::custom_types::{GroupType, UserRole};
use crate::extractors::extractors::User;
use crate::server::app_state::AppState;
use actix_utils::future::{err, ok, Ready};
use actix_web::web::block;
use actix_web::{get, post, put, web, Error, FromRequest, HttpRequest, HttpResponse, Responder};
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::any::AnyConnectionBackend;
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

struct VecAllGroups {
    rows: Vec<AllGroups>,
}

impl FromRequest for VecAllGroups {
    type Error = Error;
    type Future = Ready<Result<VecAllGroups, Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let state = req.app_data::<AppState>().unwrap();
        let user_id = req
            .headers()
            .get("user_id")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<i32>()
            .unwrap();

        let out = block_on(async {
            sqlx::query_as!(
                AllGroups,
                r#"select g.id, g.group_name, g.parent_group_id, g.created_by, g.created_at,
            g.type as "type!: GroupType",
            read, write, moderate, admin
            from groups g join group_permissions gp on gp.group_id = g.id
            where user_id = $1"#,
                user_id
            )
            .fetch_all(&state.pool)
            .await
            .unwrap()
        });
        ok(VecAllGroups { rows: out })
    }
}

#[get("/")]
async fn get_groups(groups: VecAllGroups) -> HttpResponse {
    let ret: Vec<AllGroups> = groups.rows.into_iter().filter(|val| val.read).collect();
    HttpResponse::Ok().json(ret)
}

#[derive(Serialize, Deserialize)]
struct CreateGroupRequest {
    group_name: String,
    parent_group_id: Option<i32>,
    group_type: GroupType,
}

#[derive(Serialize, Deserialize)]
struct CreateGroupResponse {
    group_id: i32,
}


async fn create_group(
    app: web::Data<AppState>,
    group_req: web::Json<CreateGroupRequest>,
    user: User,
) -> HttpResponse {
    // let mut tran = app.pool.begin().await.unwrap();
    let con = app.pool.acquire().await.unwrap();
    let mut comit = con.begin();

    let this = sqlx::query!(
        r#"insert into groups(group_name, parent_group_id, type)
        values ( $1, $2, $3 ) returning id"#,
            group_req.group_name,
            group_req.parent_group_id,
            group_req.group_type as GroupType
    ).statement().unwrap();
    let row = con.execute(this.query()).await.unwrap();
    let row = sqlx::query!(
        r#"insert into groups(group_name, parent_group_id, type)
    values ( $1, $2, $3 ) returning id"#,
        group_req.group_name,
        group_req.parent_group_id,
        group_req.group_type as GroupType
    ).fetch_one(comit).await.unwrap();
    sqlx::query!(
        r"insert into group_permissions (user_id, group_id,
        read, write, moderate, admin)
        values (
            $1, $2, $3, $4, $5, $6
        )
         ", user.user_id, row.id, true, true, true, false
    ).execute(tran).await;


    HttpResponse::Ok().json(CreateGroupResponse{group_id: row.id})

}

#[get("{group_id}/members")]
async fn get_group_members_handle(
    group_id: web::Path<i32>,
    app: web::Data<AppState>,
    user: User,
) -> impl Responder {
    let group_id = group_id.into_inner();
    let ret = get_group_members(group_id, &app.pool).await;
    return ret;
}

async fn get_group_members(group_id: i32, pool: &Pool<Postgres>) -> impl Responder {
    let rows = match sqlx::query!(
        r#"select gp.*, u.username,
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
        _ => return HttpResponse::InternalServerError(),
    };

    let resp = HttpResponse::Ok();
    resp
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
