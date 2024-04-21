use actix_session::storage::RedisActorSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::middleware;
use actix_web::{get, put, web, App, HttpRequest, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::process;
use tokio;
// use futures::future::FutureExt;
use std::sync::{Arc, Mutex};
mod auth;
mod custom_types;
mod db;
mod extractors;
mod groups;
mod login;
mod messages;
mod server;
use crate::auth::TokenStore;
use auth::AuthStruct;
use custom_types::GroupType;
use db::setup_database;
use groups::group_routes;
use login::login::{login_handle, logout_handle, register_handle};
use messages::message_routes;
use server::app_state::AppState;

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
    log::info!("process id is {}", process::id());
    let pool = setup_database().await.unwrap();
    let b = pool.num_idle();
    println!("number idle {:?}", b);
    let redis_connection_string = "localhost:6379";
    let secret_key = Key::generate();
    let token_store = Arc::new(TokenStore::new());

    let thread_token_store = Arc::clone(&token_store);
    // spawns a thread that checks expiry every 60 seconds
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            log::info!("checking expiry, process_id is {}", process::id());
            thread_token_store.check_expiry();
        }
    });
    let token_storage = web::Data::new(token_store);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .wrap(SessionMiddleware::new(
                RedisActorSessionStore::new(redis_connection_string),
                secret_key.clone(),
            ))
            .app_data(token_storage.clone())
            .app_data(web::Data::new(AppState::new(&pool)))
            .service(
                // these are not protected
                web::scope("/auth")
                    .service(login_handle)
                    .service(register_handle),
            )
            .service(
                // protected
                web::scope("/auth").wrap(AuthStruct).service(logout_handle),
            )
            .service(
                // these are protected by the AuthStruct middleware
                web::scope("/api")
                    .wrap(AuthStruct)
                    .configure(group_routes)
                    .configure(message_routes),
            )
    })
    .bind(("localhost", 8080))?
    .workers(4)
    .run();

    println!("starting server");
    server.await
}
