pub mod groups;
use actix_web::web;
use groups::{get_group_members_handle, write_to_group, handle_get_groups};

pub fn group_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/groups")
            .service(write_to_group)
            .service(get_group_members_handle)
            .service(handle_get_groups),
    );
}
