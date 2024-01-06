use sqlx::{Pool, Postgres};


pub struct AppState {
    pub pool: sqlx::PgPool,
}

impl AppState {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        AppState { pool: pool.clone() }
    }
}
