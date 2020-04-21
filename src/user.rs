use crate::database::{Pool, extract_connection};
use uuid::Uuid;
use crate::schema::{users, devices, onetimekeys};
use diesel::prelude::*;
use crate::utils::{HandlerError, InternalError, Entity};
use crate::base64enc;
use serde::{Deserialize, Serialize};
use diesel::result::Error;
use diesel::pg::expression::array_comparison::any;
use actix_web::http::header::q;

fn check_signed_prekey(identity_key: &Vec<u8>, signed_key: &Vec<u8>, signature: &Vec<u8>) -> Result<(), HandlerError>{
    let key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, identity_key);
    key.verify(signed_key, signature).map_err(|_| HandlerError::SignatureMismatch)
}

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
    bio: Option<String>,
}

pub fn create_user(pool: &Pool, user: UserCreation, device_id: Uuid) -> Result<Uuid, HandlerError> {
    check_signed_prekey(&user.identity_key, &user.signed_prekey, &user.prekey_signature)?;

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
    }).map_err(|e| match e {
        diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, err_dat) =>
            HandlerError::RecordMustBeUnique { name: err_dat.column_name().unwrap_or("").to_string() },
        _ => InternalError::DatabaseError(e).into()
    })
    // TODO: Send verification email
}

#[derive(Deserialize)]
pub struct PreKeyUpdate {
    #[serde(with = "base64enc")]
    signed_prekey: Vec<u8>,
    #[serde(with = "base64enc")]
    prekey_signature: Vec<u8>,
}

pub fn update_prekey(pool: &Pool, update: PreKeyUpdate, user_id: Uuid) -> Result<(), HandlerError> {
    let conn = extract_connection(pool)?;

    let identity_key = users::table.find(user_id).select(users::identity_key)
        .first::<Vec<u8>>(&conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => HandlerError::UnknownEntity {entity: Entity::User { uuid: user_id}},
            _ => InternalError::DatabaseError(e).into()
        })?;

    check_signed_prekey(&identity_key, &update.signed_prekey, &update.prekey_signature)?;

    match diesel::update(users::table.find(user_id))
        .set((users::signed_prekey.eq(&update.signed_prekey),
              users::prekey_signature.eq(&update.prekey_signature)))
        .execute(&conn)
        .map_err(|e| InternalError::DatabaseError(e))?
    {
        0 => Err(HandlerError::UnknownEntity { entity: Entity::User { uuid: user_id } }),
        _ => Ok(())
    }
}

#[derive(Deserialize)]
pub struct OTKAdd {
    pub keys: Vec<String>
}

pub fn add_otks(pool: &Pool, keys: &Vec<String>, user_id: Uuid) -> Result<usize, HandlerError> {
    if keys.len() == 0 { return Ok(0) };
    let conn = extract_connection(pool)?;

    let values = keys.iter().map(|s| -> Result<_, HandlerError> {Ok((
        onetimekeys::prekey.eq(base64::decode(s)
        .map_err(|_e|HandlerError::MalformedBody { error_message: "base64 error".to_string()})?),
    onetimekeys::user_id.eq(&user_id)))}).collect::<Result<Vec<_>, HandlerError>>()?;

    diesel::insert_into(onetimekeys::table)
        .values(&values)
        .execute(&conn).map_err(|e| InternalError::DatabaseError(e).into())
}

#[derive(Serialize)]
pub struct ChatPackage {
    #[serde(with = "base64enc")]
    identity_key: Vec<u8>,
    #[serde(with = "base64enc")]
    signed_prekey: Vec<u8>,
    #[serde(with = "base64enc")]
    prekey_signature: Vec<u8>,
    #[serde(with = "base64enc")]
    onetime_key: Vec<u8>
}

pub fn retrieve_package(pool: &Pool, user_id: Uuid) -> Result<ChatPackage, HandlerError> {
    let conn = extract_connection(pool)?;
    let (identity_key, signed_prekey, prekey_signature) = users::table.find(user_id).select((users::identity_key, users::signed_prekey, users::prekey_signature))
        .first::<(Vec<u8>, Vec<u8>, Vec<u8>)>(&conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => HandlerError::UnknownEntity { entity: Entity::User {uuid: user_id}},
            _ => InternalError::DatabaseError(e).into()
        })?;
    let mut query = diesel::delete(onetimekeys::table.filter(
        onetimekeys::id.eq(any(onetimekeys::table.select(onetimekeys::id)
            .filter(onetimekeys::user_id.eq(user_id))
            .limit(1).into_boxed()))))
        .returning((onetimekeys::prekey))
        .load::<Vec<u8>>(&conn).map_err(|e| HandlerError::InternalError {error: InternalError::DatabaseError(e)})?;

    if query.len() == 0 {
        return Err(HandlerError::InsufficientPrekeys)
    }

    Ok(ChatPackage {
        identity_key,
        signed_prekey,
        prekey_signature,
        onetime_key: query.remove(0)
    })
}