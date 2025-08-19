pub mod schema;
pub mod models;

pub use models::*;

use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use diesel_async::pooled_connection::bb8::Pool;
use anyhow::Result;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection = AsyncPgConnection;

/// Создание пула соединений с базой данных
pub async fn create_db_pool(database_url: &str) -> Result<DbPool> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(10)
        .build(config)
        .await?;
    
    Ok(pool)
}
