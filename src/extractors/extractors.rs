use actix_utils::future::{err, ok, ready, Ready};
use actix_web::{error, Error, FromRequest, HttpResponse};

pub struct User {
    pub user_id: i32,
}

impl FromRequest for User {
    type Error = Error;
    type Future = Ready<Result<User, Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let user_id = req.headers().get("user_id").unwrap();

        let str_user_id = match user_id.to_str() {
            Ok(val) => val,
            _ => return err(error::ErrorInternalServerError("User id is not found")),
        };

        let int_user_id = match str_user_id.parse::<i32>() {
            Ok(val) => val,
            _ => {
                return err(error::ErrorInternalServerError(
                    "User id not parsable to int",
                ))
            }
        };
        ok(User {
            user_id: int_user_id,
        })
    }
}
