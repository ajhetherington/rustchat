use std::rc::Rc;

use actix_utils::future::{err, ok, ready, Ready};
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::Data;
use actix_web::HttpResponse;
use actix_web::{error::ErrorUnauthorized, Error, FromRequest};
use futures_util::{future::LocalBoxFuture, FutureExt, TryFutureExt};
use log;
use serde::{Deserialize, Serialize};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use std::collections::HashMap;
use bimap::BiMap;
use std::sync::Mutex;

pub struct TokenStore {
    tokens: Mutex<BiMap<String, i32>>, // Token as key, User ID as value
}

impl TokenStore {
    pub fn new() -> Self {
        TokenStore {
            tokens: Mutex::new(BiMap::new()),
        }
    }

    pub fn get_token(&self, user_id: i32) -> Option<String> {
        let tokens = self.tokens.lock().unwrap();
        match tokens.get_by_right(&user_id) {
            Some(val) => Some((*val).clone()),
            _ => None
        }
    }

    pub fn validate_user(&self, user_id: i32) -> String { 
        let mut tokens = self.tokens.lock().unwrap();
        let token = generate_token();
        tokens.insert(token.clone(), user_id);
        token
    }

    pub fn invalidate_user(&self, user_id: i32) {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.remove_by_right(&user_id);
    }

    fn check_token(&self, token: &str) -> Option<i32> {
        let tokens = self.tokens.lock().unwrap();
        tokens.get_by_left(token).cloned()
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
        let token_store = req.app_data::<Data<TokenStore>>().unwrap();
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
                if cached_user_id.is_some() {
                    if header_user_id.parse::<i32>().unwrap() == cached_user_id.unwrap() {
                        return ok(Authentication {
                            token: header_auth_token.to_owned(),
                        });
                    }
                }
                return err(ErrorUnauthorized("Token invalid"));
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
            Ok(T) => println!("ok"),
            Err(T) => {
                log::info!("hope i see this");
                println!("Catching in middleware");
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
