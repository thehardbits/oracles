mod cell_type;
pub mod decimal_scalar;
mod error;
pub mod pending_txn;
pub mod server;
pub mod token_type;
pub mod traits;
pub mod transaction;
pub mod txn_status;
mod uuid;

pub use cell_type::CellType;
pub use decimal_scalar::Mobile;
pub use error::{Error, Result};
pub use server::Server;
pub use uuid::Uuid;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{env, path::Path};

pub fn write_json<T: ?Sized + serde::Serialize>(
    fname_prefix: &str,
    after_ts: u64,
    before_ts: u64,
    data: &T,
) -> Result {
    let tmp_output_dir = env::var("TMP_OUTPUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let fname = format!("{}-{}-{}.json", fname_prefix, after_ts, before_ts);
    let fpath = Path::new(&tmp_output_dir).join(&fname);
    std::fs::write(Path::new(&fpath), serde_json::to_string_pretty(data)?)?;
    Ok(())
}

pub async fn mk_db_pool(size: u32) -> Result<Pool<Postgres>> {
    let db_connection_str = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(size)
        .connect(&db_connection_str)
        .await?;
    Ok(pool)
}
