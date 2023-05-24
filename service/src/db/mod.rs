use anyhow::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::MysqlConnection;
use dotenv::dotenv;

pub mod models;
pub mod schema;

pub type MysqlPool = Pool<ConnectionManager<MysqlConnection>>;
pub type MysqlPooledConnection = PooledConnection<ConnectionManager<MysqlConnection>>;

use crate::DATABASE_URL;

// get a connection pool
pub fn get_connection_pool() -> Result<MysqlPool> {
    dotenv().ok();
    let manager = ConnectionManager::<MysqlConnection>::new(DATABASE_URL);
    Pool::builder()
        .max_size(50)
        .build(manager)
        .map_err(|e| anyhow!(format!("Build DB connection pool failed: {:?}", e)))
}
