use std::sync::OnceLock;

use diesel::SqliteConnection;
use diesel::r2d2::{self, ConnectionManager, PooledConnection};
use dotenvy::dotenv;

type DbConn = ConnectionManager<SqliteConnection>;
type DbPool = r2d2::Pool<DbConn>;

fn establish_connection() -> DbPool {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool")
}

static POOL: OnceLock<DbPool> = OnceLock::new();

pub fn get_connection() -> Result<PooledConnection<DbConn>, String> {
    POOL.get_or_init(|| establish_connection())
        .get()
        .map_err(|e| e.to_string())
}
