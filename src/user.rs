use crate::database::{Pool, extract_connection};
use uuid::Uuid;
use crate::schema::{users, devices};
use diesel::prelude::*;
use crate::utils::{HandlerError, InternalError, Entity};
use crate::base64enc;
use serde::Deserialize;



#[derive(Deserialize, Insertable)]
#[table_name = "users"]
pub struct UserCreation {
    email: String,
    #[serde(with = "base64enc")]
    identity_key: Vec<u8>,
    #[serde(with = "base64enc")]
    signed_prekey: Vec<u8>,
    #[serde(with = "base64enc")]
    prekey_signature: Vec<u8>,
    nickname: Option<String>,
    bio: Option<String>
}

pub fn create_user(pool: &Pool, user: UserCreation, device_id: Uuid) -> Result<Uuid, HandlerError> {
    let conn = extract_connection(pool)?;
    // Assumes the device does not already have a user.
    conn.transaction::<Uuid, _, _>(|| {
        let user_id = diesel::insert_into(users::table).values(&user)
            .returning(users::id)
            .get_result::<Uuid>(&conn)?;

        // Now update the device - assuming it exists
        diesel::update(devices::table.find(&device_id))
            .set(devices::user_id.eq(&user_id))
            .execute(&conn)?;
        Ok(user_id)
    }).map_err(|e| InternalError::DatabaseError(e).into())
    // TODO: Send verification email
}

#[derive(Deserialize)]
pub struct PreKeyUpdate {
    signed_prekey: Vec<u8>,
    prekey_signature: Vec<u8>
}

pub fn update_prekey(pool: &Pool, update: PreKeyUpdate, user_id: Uuid) -> Result<(), HandlerError> {
    let conn = extract_connection(pool)?;

    // Assuming user_id exists... (nothing happens if not)
    match diesel::update(users::table.find(user_id))
        .set(( users::signed_prekey.eq(&update.signed_prekey),
        users::prekey_signature.eq(&update.prekey_signature)))
        .execute(&conn)
        .map_err(|e| InternalError::DatabaseError(e))?
    {
        0 => Err(HandlerError::UnknownEntity { entity: Entity::User {uuid: user_id}}),
        _ => Ok(())
    }
}