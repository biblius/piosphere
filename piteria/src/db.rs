use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

/// Establish a connection pool at the specified sqlite file
pub async fn db_pool(file: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(file)
        .create_if_missing(true);

    SqlitePool::connect_with(options).await
}

pub trait PiteriaDatabase {}
