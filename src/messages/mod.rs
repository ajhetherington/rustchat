pub mod messages;
use actix_web::web;
use messages::{handle_get_messages, handle_write_message, message_ws};

pub fn message_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/messages")
            .service(handle_write_message)
            .service(handle_get_messages)
            .service(message_ws),
            
    );
}
