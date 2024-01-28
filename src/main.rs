use actix_session::storage::RedisActorSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::{put, web, App, HttpRequest, HttpServer, Responder};
use serde::{Deserialize, Serialize};
// use futures::future::FutureExt;
mod auth;
mod custom_types;
mod db;
mod extractors;
mod groups;
mod login;
mod server;
use auth::AuthStruct;
use custom_types::GroupType;
use db::setup_database;
use groups::group_routes;
use login::login::{login_handle, logout_handle, register_handle};
use server::app_state::AppState;

use crate::auth::TokenStore;

#[derive(Serialize, Deserialize, Debug)]
struct MessageRequest {
    content: String,
    group_id: i32,
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
    let token_storage = web::Data::new(TokenStore::new());

    let server = HttpServer::new(move || {
        App::new()
            .app_data(token_storage.clone())
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
                web::scope("/noauth")
                    .wrap(AuthStruct)
                    .service(logout_handle),
            )
            .service(
                // these are protected by the AuthStruct middleware
                web::scope("/api").wrap(AuthStruct).configure(group_routes),
            )
    })
    .bind(("localhost", 8080))?
    .workers(4)
    .run();

    println!("starting server");
    server.await
}
