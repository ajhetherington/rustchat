use crate::{auth::TokenStore, server::app_state::AppState};
use actix_web::{http::StatusCode, post, put, web, HttpRequest, HttpResponse};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
struct LoginRequest {
    username: String,
    password: String,
}


/// A generic http response builder, giving INTERNAL_SERVER_ERROR 
/// or a response of given status with message attached
/// ```
/// use login::resp;
/// fn function() -> HttpResponse {
///   resp("Fine", 200)
/// }
/// function();
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
    tokenstore: web::Data<Arc<TokenStore>>,
    form_data: web::Form<LoginRequest>,
) -> HttpResponse {
    println!("{:?}", tokenstore);
    let (user_id, password) = match sqlx::query!(
        r#"select id, password from users where username=$1"#,
        form_data.username
    )
    .fetch_one(&app.pool)
    .await
    {
        Ok(ret) => (ret.id, ret.password),
        Err(_e) => return resp("user not found", None),
    };

    match check_password(&form_data.password, &password) {
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

fn create_ciphertext(plaintext: &String) -> Result<(String, String), argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    match argon2.hash_password(plaintext.as_bytes(), &salt) {
        Ok(val) => Ok((val.to_string(), salt.to_string())),
        Err(e) => Err(e),
    }
}

async fn insert_user(
    pool: &sqlx::Pool<sqlx::Postgres>,
    data: web::Form<RegisterRequest>,
) -> Result<i32, HttpResponse> {
    let (ciphertext, salt_string) = match create_ciphertext(&data.password) {
        Ok(val) => val,
        Err(e) => {
            return Err(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(e.to_string()))
        }
    };

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
    tokenstore: web::Data<Arc<TokenStore>>,
    // data: web::Json<RegisterRequest>,
    form_data: web::Form<RegisterRequest>,
) -> HttpResponse {
    // first check that username / email is not already entered
    match check_user_duplicate(
        &app.pool,
        form_data.email.clone(),
        form_data.username.clone(),
    )
    .await
    {
        Err(val) => return val,
        _ => {}
    }

    let user_id = match insert_user(&app.pool, form_data).await {
        Ok(id) => id,
        Err(val) => return val,
    };

    let token = tokenstore.validate_user(user_id);
    HttpResponse::Ok().json(LoginResponse { token, user_id })
}

#[put("/logout")]
pub async fn logout_handle(
    tokenstore: web::Data<Arc<TokenStore>>,
    req: HttpRequest,
) -> HttpResponse {
    // by being here, the user should already be logged in & verified
    // so just need to remove the user from the mutex
    let token = req
        .headers()
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap();
    // .parse::<String>()
    // .unwrap();
    let user_id = tokenstore.invalidate_token(token.to_owned());
    HttpResponse::Ok().json(format!("logged out user {user_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_create_password() {
        let password = "hunter2".to_owned();
        let (_value, _salt) = create_ciphertext(&password).unwrap();
    }

    #[test]
    fn test_password_hashing() {
        let password = "hunter2".to_owned();
        let (ciphertext, _salt) = create_ciphertext(&password).unwrap();
        assert!(check_password(&password, &ciphertext));
    }
}
