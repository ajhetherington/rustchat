use actix_utils::future::{err, ok, ready, Ready};
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::Data;
use actix_web::HttpResponse;
use actix_web::{error::ErrorUnauthorized, Error, FromRequest};
use chrono::prelude::*;
use futures_util::future::LocalBoxFuture;
use log;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
struct UserLoginValues {
    user_id: i32,
    login_time: DateTime<Utc>,
    expiry_time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct TokenStore {
    tokens: Mutex<HashMap<String, UserLoginValues>>, // Token as key, User ID as value
}

impl TokenStore {
    pub fn new() -> Self {
        TokenStore {
            tokens: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_expiry(&self) {
        let mut values = self.tokens.lock().unwrap();
        let now = Utc::now();
        let expired_tokens: Vec<String> = values
            .iter()
            .filter(|val| val.1.expiry_time < now)
            .map(|val| val.0.to_string())
            .collect();
        println!("{:?} tokens to expire", expired_tokens.len());
        for token in expired_tokens {
            values.remove(&token);
        }
    }

    pub fn validate_user(&self, user_id: i32) -> String {
        let mut tokens = self.tokens.lock().unwrap();
        let token = generate_token();
        tokens.insert(
            token.clone(),
            UserLoginValues {
                user_id,
                login_time: Utc::now(),
                expiry_time: Utc::now() + chrono::Duration::seconds(30),
            },
        );
        token
    }

    pub fn invalidate_token(&self, token: String) -> i32 {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.remove(&token).unwrap().user_id
    }

    fn check_token(&self, token: &str) -> Option<i32> {
        let tokens = self.tokens.lock().unwrap();
        match tokens.get(token) {
            Some(login) => Some(login.user_id),
            _ => None,
        }
    }
}

fn generate_token() -> String {
    let mut rng = thread_rng();
    let s: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    s
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Authentication {
    pub token: String,
}

impl FromRequest for Authentication {
    type Error = Error;
    type Future = Ready<Result<Authentication, Error>>;

    #[inline]
    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let token_store = req.app_data::<Data<Arc<TokenStore>>>().unwrap();
        let headers = req.headers();
        let header_user_id = match headers.get("user_id") {
            Some(header) => header.to_str().unwrap(),
            _ => return err(ErrorUnauthorized("No user_id specified in request")),
        };

        match headers.get("Authorization") {
            // todo, check things
            Some(header) => {
                let header_auth_token = header.to_str().unwrap();
                let cached_user_id = (*token_store).check_token(header_auth_token);
                match cached_user_id {
                    Some(id) => {
                        if header_user_id.parse::<i32>().unwrap() == id {
                            return ok(Authentication {
                                token: header_auth_token.to_owned(),
                            });
                        } else {
                            // here don't be specific in error as that would give away
                            // that the token is valid but user_id invalid
                            return err(ErrorUnauthorized("Token invalid"));
                        }
                    }
                    None => return err(ErrorUnauthorized("Token invalid")),
                }
            }
            _ => err(ErrorUnauthorized(
                "not authorized, no token found in Authorization",
            )),
        }
    }
}

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct AuthStruct;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for AuthStruct
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    // using EitherBody allows for an
    // early return from unauthorized requests
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthorizationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthorizationMiddleware { service }))
    }
}

pub struct AuthorizationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let b = futures::executor::block_on(req.extract::<Authentication>());
        match b {
            Ok(_) => {}
            Err(t) => {
                log::error!("Authentication error: {t}");
                return Box::pin(async {
                    Ok(req
                        .into_response(HttpResponse::Unauthorized())
                        .map_into_right_body())
                });
            }
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            Ok(res.map_into_left_body())
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_create_token() {
        let a = generate_token();
        assert!(a.len() > 0);

    }
    #[test]
    fn tokens_are_different() {
        let a = generate_token();
        let b = generate_token();
        let c = generate_token(); 
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn create_token_store() {
        let user_id = 2;
        let token_store = TokenStore::new();
        let user_token = token_store.validate_user(user_id);
        assert_eq!(token_store.check_token(&user_token).unwrap(), user_id);
    }

    #[test]
    fn user_token_no_crossover() {
        let user_id_1 = 1;
        let user_id_2 = 2;
        let token_store = TokenStore::new();
        let user_token_1 = token_store.validate_user(user_id_1);
        let user_token_2 = token_store.validate_user(user_id_2);

        assert_ne!(token_store.check_token(&user_token_1).unwrap(), user_id_2);
        assert_ne!(token_store.check_token(&user_token_2).unwrap(), user_id_1);
        assert_eq!(token_store.check_token(&user_token_1).unwrap(), user_id_1);
        assert_eq!(token_store.check_token(&user_token_2).unwrap(), user_id_2);
    }

    #[test]
    fn invalidate_token() {
        let token_store = TokenStore::new();
        let user_token = token_store.validate_user(2);

        assert!(token_store.check_token("badToken").is_none());
        token_store.invalidate_token(user_token.clone());
        assert!(token_store.check_token(&user_token).is_none());
        
    }
}