use crate::{auth::TokenStore, server::app_state::AppState};
use actix_web::{
    error, get, http::StatusCode, post, put, web, HttpRequest, HttpResponse, Responder,
};
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, Salt, SaltString,
    },
    Argon2,
};
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

#[derive(Serialize, Deserialize)]
struct LoginResponse {
    user_id: i32,
    token: String,
}

#[post("/login")]
pub async fn login_handle(
    app: web::Data<AppState>,
    somereq: web::Json<LoginRequest>,
    tokenstore: web::Data<TokenStore>,
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
        true => {
            let token = tokenstore.validate_user(user_id);
            HttpResponse::build(StatusCode::OK).json(LoginResponse { token, user_id })
        }
        false => return resp("password doesn't match", Some(StatusCode::UNAUTHORIZED)),
    }
}

// Register
#[derive(Serialize, Deserialize, Debug)]
struct RegisterRequest {
    username: Option<String>,
    email: Option<String>,
    password: String,
}

async fn check_user_duplicate(
    pool: &sqlx::Pool<sqlx::Postgres>,
    email: Option<String>,
    username: Option<String>,
) -> Result<i32, HttpResponse> {
    
    // interesting that it works for Option<String>
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
            .json("already found user with that email or username")),
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
    tokenstore: web::Data<TokenStore>,
) -> HttpResponse {
    // first check that username / email is not already entered
    match check_user_duplicate(&app.pool, data.email.clone(), data.username.clone()).await {
        Err(val) => return val,
        _ => {}
    }

    let user_id = match insert_user(&app.pool, data).await {
        Ok(id) => id,
        Err(val) => return val,
    };

    let token = tokenstore.validate_user(user_id);
    HttpResponse::Ok().json(LoginResponse { token, user_id })
}

#[put("/logout")]
pub async fn logout_handle(tokenstore: web::Data<TokenStore>, req: HttpRequest) -> HttpResponse {
    // by being here, the user should already be logged in & verified
    // so just need to remove the user from the mutex
    let user_id = req
        .headers()
        .get("user_id")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<i32>()
        .unwrap();
    tokenstore.invalidate_user(user_id);
    HttpResponse::Ok().json(format!("logged out user {user_id}"))
}
