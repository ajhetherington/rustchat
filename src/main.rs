use actix_session::storage::RedisActorSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::{put, web, App, HttpRequest, HttpServer, Responder};
use serde::{Deserialize, Serialize};
// use futures::future::FutureExt;
mod auth;
mod custom_types;
mod db;
mod login;
mod server;
use auth::Authentication;
use custom_types::GroupType;
use db::setup_database;
use login::login::{login_handle, register_handle};
use server::app_state::AppState;
use auth::AuthStruct;

#[derive(Serialize, Deserialize, Debug)]
struct MessageRequest {
    content: String,
    group_id: i32,
}

// should be authenticated
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

    // here i have to do "group_type!: GroupType" to override the built-in type infrence,
    // this is becuase i have a custom enum type in postgres that (for some reason)
    // sqlx cannot infer the type itself... (why?)
    let inferred_result =
        sqlx::query!(r#"select id, type as "group_type!: GroupType" from groups"#)
            .fetch_all(&app.pool)
            .await
            .unwrap();
    for ele in &inferred_result {
        let b = ele.group_type.clone();
        println!("b is {:?}", b)
    }

    println!("hello {:?}", result);

    format!("hi")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().expect("No .env file found");

    // std::env::set_var("RUST_LOG", "debug");
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
            .app_data(web::Data::new(AppState::new(&pool)))
            .service(
                // these are not protected
                web::scope("/auth")
                    .service(login_handle)
                    .service(register_handle),
            )
            .service(
                // these are protected by the AuthStruct middleware
                web::scope("/api")
                .wrap(AuthStruct)
                .service(some_thing),
            )
    })
    .bind(("localhost", 8080))?
    .workers(4)
    .run();

    println!("starting server");
    server.await
}
