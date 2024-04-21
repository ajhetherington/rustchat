pub mod groups;
use actix_web::web::{self, service};
use groups::{get_group_members_handle, write_to_group, handle_get_groups, handle_create_group, handle_add_to_group};

pub fn group_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/groups")
            .service(write_to_group)
            .service(get_group_members_handle)
            .service(handle_get_groups)
            .service(handle_create_group)
            .service(handle_add_to_group),
    );
}
