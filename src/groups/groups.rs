use crate::server::app_state::AppState;
use actix_web::{get, post};

// #[derive(Serialize, Deserialize, Debug)]
// struct GroupMember {
//     user_id: i32,
//     group_id: i32,
//     read: bool,
//     write: bool,
//     moderate: bool,
//     admin: bool
// }



#[get("/groups/{group_id}")]
pub fn group_members_handle(
    app: web::Data<AppState>,
)



