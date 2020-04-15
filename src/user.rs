use crate::database::{Pool, extract_connection};
use uuid::Uuid;
use crate::schema::{users, devices};
use diesel::prelude::*;
use crate::utils::{HandlerError, InternalError};
use serde::Deserialize;


#[derive(Deserialize, Insertable)]
#[table_name = "users"]
pub struct UserCreation<'a> {
    email: &'a str,
    pubkey: Vec<u8>,
    nickname: Option<&'a str>,
    bio: Option<&'a str>
}

pub fn create_user(pool: Pool, user: UserCreation, device_id: Uuid) -> Result<Uuid, HandlerError> {
    let conn = extract_connection(&pool)?;
    // Assumes the device does not already have a user.
    let user_id = diesel::insert_into(users::table).values(&user)
        .returning(users::id)
        .get_result::<Uuid>(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))?;

    // Now update the device
    diesel::update(devices::table.find(&device_id))
        .set(devices::owner.eq(&user_id))
        .execute(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))?;

    // TODO: Get user email verification
    Ok(user_id)
}