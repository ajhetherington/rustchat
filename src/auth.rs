use std::rc::Rc;

use actix_utils::future::{err, ok, ready, Ready};
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::HttpResponse;
use actix_web::{error::ErrorUnauthorized, Error, FromRequest};
use chrono::Local;
use futures_util::{future::LocalBoxFuture, FutureExt, TryFutureExt};
use log;
use serde::{Deserialize, Serialize};

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
        match req.headers().get("Authorization") {
            // todo, check things
            Some(header) => ok(Authentication {
                token: (header.to_str().unwrap().to_owned()),
            }),
            _ => err(ErrorUnauthorized("not authorized")),
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
