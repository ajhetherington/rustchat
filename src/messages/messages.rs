use std::time::{Duration, Instant};

use crate::custom_types;
use crate::extractors::extractors::User;
use crate::server::app_state::AppState;
use actix::{AsyncContext, Running};
use actix::{Actor, StreamHandler};
use actix_web::web::Payload;
use actix_web::{get, put, web, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};
use actix_web_actors::ws;
use log::log;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use sqlx::types::chrono;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[put("{group_id}")]
async fn handle_write_message(
    group_id: web::Path<i32>,
    message: web::Json<Message>,
    user: User,
    app: web::Data<AppState>,
) -> impl Responder {
    let state = write_message(group_id.abs(), user.user_id, message.0, &app.pool).await;
    match state {
        0 => HttpResponse::Unauthorized().body(format!(
            "User {} does not have write permissions for group {}",
            user.user_id,
            group_id.abs()
        )),
        _ => HttpResponse::Ok().body(state.to_string()),
    }
}

async fn write_message(
    group_id: i32,
    user_id: i32,
    message: Message,
    pool: &Pool<Postgres>,
) -> i32 {
    // first need to check that user has write access
    let count = sqlx::query!(r#"select count(*) from group_permissions where user_id = $1 and group_id = $2 and write limit 1"#, user_id, group_id).fetch_one(pool).await.unwrap().count.unwrap();
    if count < 1 {
        return 0;
    }

    sqlx::query!(r#"insert into messages (sender_user_id, group_id, content) values ($1, $2, $3) returning id"#,
        user_id, group_id, message.content).fetch_one(pool).await.unwrap().id
}

#[get("{group_id}")]
async fn handle_get_messages(
    group_id: web::Path<i32>,
    user: User,
    app: web::Data<AppState>,
) -> impl Responder {
    match get_messages(group_id.abs(), user.user_id, &app.pool).await {
        Some(val) => HttpResponse::Ok().json(val),
        None => HttpResponse::Unauthorized().body("nah"),
    }
}

#[derive(Serialize, FromRow)]
struct MessageResponse {
    sender_user_id: i32,
    group_id: i32,
    content: String,
    sent_at: Option<chrono::DateTime<chrono::Utc>>,
}
async fn get_messages(
    group_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Option<Vec<MessageResponse>> {
    let count = sqlx::query!(
        r#"select count(*) from group_permissions where user_id = $1 and group_id = $2 and write"#,
        user_id,
        group_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap();
    if count < 1 {
        return None;
    }

    let offset = 0;
    let resp = sqlx::query_as!(MessageResponse, r#"select sender_user_id, group_id, content, sent_at from messages where group_id = $1 order by sent_at asc limit 100 offset $2"#, group_id, offset).fetch_all(pool).await.unwrap();
    Some(resp)
}

const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsChatSession {
    pub session_id: Uuid,
    pub heartbeat: Instant,
    pub user_id: i32,
    pub group_id: i32,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_heartbeat(ctx);
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::prelude::Running {
        Running::Stop
    }

}

impl WsChatSession {
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(CLIENT_TIMEOUT, |act, _ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                log::error!("Heartbeat duration timeout");
                // todo: disconnect
                return;
            }
            // send ping to client
            _ctx.ping("".as_bytes());
        });
    }
}

struct MyWs;

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[get("ws")]
async fn message_ws(req: HttpRequest, stream: Payload) -> impl Responder {
    let ws_actor = MyWs {};
    let value = ws::start(ws_actor, &req, stream);
    println!("value {:?}", value);
    format!("hi")
}
