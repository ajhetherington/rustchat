use crate::server::app_state::AppState;
use actix_web::{
    error, get, http::StatusCode, post, put, web, HttpRequest, HttpResponse, Responder,
};
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, Salt, SaltString,
    },
    Argon2,
};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct LoginRequest {
    username: String,
    password: String,
}

fn resp(message: &str, status: Option<StatusCode>) -> HttpResponse {
    match status {
        Some(stat) => HttpResponse::build(stat).json(message),
        None => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(message),
    }
}

#[post("/login")]
pub async fn login_handle(
    app: web::Data<AppState>,
    somereq: web::Json<LoginRequest>,
) -> HttpResponse {
    let (user_id, password) = match sqlx::query!(
        r#"select id, password from users where username=$1"#,
        somereq.username
    )
    .fetch_one(&app.pool)
    .await
    {
        Ok(ret) => (ret.id, ret.password),
        Err(_e) => return resp("user not found", None),
    };
    match check_password(&somereq.password, &password) {
        true => return resp(generate_token().as_str(), Some(StatusCode::OK)),
        false => return resp("password doesn't match", Some(StatusCode::UNAUTHORIZED)),
    }
}

fn generate_token() -> String {
    let mut rng = thread_rng();
    let s: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
    // need to whack this into redis
    s
}

// Register
#[derive(Serialize, Deserialize, Debug)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

async fn check_user_duplicate(
    pool: &sqlx::Pool<sqlx::Postgres>,
    email: &String,
    username: &String,
) -> Result<i32, HttpResponse> {
    let value = sqlx::query!(
        r#"select 1 as "exists" from users where email = $1 or username = $2"#,
        email,
        username
    )
    .fetch_optional(pool)
    .await;
    let unpacked = match value {
        Ok(val) => val,
        Err(thing) => {
            return Err(
                HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(thing.to_string())
            )
        }
    };
    match unpacked {
        Some(_val) => Err(HttpResponse::build(StatusCode::CONFLICT)
            .json("found user with that email or username")),
        _ => Ok(1),
    }
}

fn check_password(plaintext: &String, ciphertext: &String) -> bool {
    let argon2 = Argon2::default();
    // let salt_str = SaltString::from_b64(salt).unwrap();
    // let password_hash = argon2.hash_password(plaintext.as_bytes(), &salt_str).unwrap();
    // let parsed_hash = PasswordHash::new(&ciphertext).unwrap();
    argon2
        .verify_password(
            plaintext.as_bytes(),
            &PasswordHash::new(ciphertext).unwrap(),
        )
        .is_ok()
}

async fn insert_user(
    pool: &sqlx::Pool<sqlx::Postgres>,
    data: web::Json<RegisterRequest>,
) -> Result<i32, HttpResponse> {
    let plaintext = data.password.as_bytes();
    // let salt = SaltString::generate(&mut OsRng).as_str().as_bytes();
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let ciphertext = match argon2.hash_password(plaintext, &salt) {
        Ok(val) => val.to_string(),
        Err(e) => {
            return Err(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(e.to_string()))
        }
    };
    let salt_string = salt.to_string();

    let check = check_password(&data.password, &ciphertext);
    println!("this is the password check (should be 1), {:?}", check);

    // whatever, turns out the salt is already stored in the ciphertext
    match sqlx::query!(
        r#"insert into users(username, password, display_name, email, salt) values ($1, $2, $3, $4, $5) returning id"#,
        data.username,
        ciphertext,
        data.username,
        data.email,
        salt_string

    ).fetch_one(pool).await {
        Ok(val) => Ok(val.id),
        Err(e) => Err(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(e.to_string()))
    }
}

#[post("/register")]
pub async fn register_handle(
    app: web::Data<AppState>,
    data: web::Json<RegisterRequest>,
) -> HttpResponse {
    // first check that username / email is not already entered
    match check_user_duplicate(&app.pool, &data.email, &data.username).await {
        Err(val) => return val,
        _ => {}
    }
    match insert_user(&app.pool, data).await {
        Ok(id) => println!("{:?} was id", id),
        Err(val) => return val,
    };

    HttpResponse::Ok().json("cool")
}
