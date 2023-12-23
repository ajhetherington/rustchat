use sqlx::postgres::PgPoolOptions;
use std::env::var;

pub async fn setup_database() -> Result<sqlx::PgPool, sqlx::Error> {
    let database_url = var("DATABASE_URL").expect("DATABASE_URL must be set in environment");
    PgPoolOptions::new().connect(&database_url).await
}