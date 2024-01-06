use actix_utils::future::{err, ok, Ready};
use actix_web::{Error, FromRequest, error::ErrorUnauthorized};
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
            _ => err(ErrorUnauthorized("not authorized"))
        }
    }
}
