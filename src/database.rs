use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use dotenv;
use crate::utils::{HandlerError, InternalError};
use r2d2::PooledConnection;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type Conn = PooledConnection<ConnectionManager<PgConnection>>;

pub fn obtain_pool() -> Pool {
    let database_url = dotenv::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder().build(manager).expect("Failed to create pool.")
}

pub fn extract_connection(pool: &Pool) -> Result<Conn, HandlerError> {
    pool.get().map_err(|e| InternalError::PoolError(e).into())
}