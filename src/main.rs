use actix_session::storage::RedisActorSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::{get, put, web, App, HttpRequest, HttpServer, Responder};
use serde::{Deserialize, Serialize};
// use futures::future::FutureExt;
mod db;
use db::setup_database;

struct AppState {
    pool: sqlx::PgPool,
}

#[derive(Serialize, Deserialize, Debug)]
struct MessageRequest {
    content: String,
    group_id: i32,
}

#[put("/groups/{group_id}")]
async fn some_thing(
    path: web::Path<u32>,
    app: web::Data<AppState>,
    thing: HttpRequest,
) -> impl Responder {
    let group_id = path.into_inner();
    let user_id = thing
        .headers()
        .get("user_id")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<i32>()
        .unwrap();

    let result = sqlx::query!("select id, parent_group_id from groups limit 1")
        .fetch_all(&app.pool)
        .await
        .unwrap();

    for ele in &result {
        match ele.parent_group_id {
            Some(val) => {
                println!("val {:?}", val)
            }
            _ => (),
        }
    }
    println!("hello {:?}", result);

    format!("hi")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().expect("argh");

    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let pool = setup_database().await.unwrap();
    let b = pool.num_idle();
    println!("number idle {:?}", b);
    let redis_connection_string = "localhost:6379";
    let secret_key = Key::generate();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(SessionMiddleware::new(
                RedisActorSessionStore::new(redis_connection_string),
                secret_key.clone(),
            ))
            .app_data(web::Data::new(AppState { pool: pool.clone() }))
            .service(some_thing)
    })
    .bind(("localhost", 8080))?
    .workers(4)
    .run();

    println!("starting server");
    server.await
}
