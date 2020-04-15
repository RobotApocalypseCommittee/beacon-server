use crate::database::{Pool, extract_connection};
use uuid::Uuid;
use crate::utils::{HandlerError, InternalError};
use crate::schema::devices;
use diesel::prelude::*;

pub fn create_device(pool: &Pool, public_key: &Vec<u8>) -> Result<Uuid, HandlerError> {
    let conn = extract_connection(pool)?;
    diesel::insert_into(devices::table)
        .values(devices::public_key.eq(public_key))
        .returning(devices::id)
        .get_result::<Uuid>(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))
}