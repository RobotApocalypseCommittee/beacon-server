use actix_web::{Responder, web, HttpResponse};
use actix_web::error::BlockingError;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use ring::rand::SecureRandom;
use ring::signature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApplicationData, base64enc};
use crate::utils::{HandlerError, InternalError};

use super::schema::{devices, sessions};

#[derive(Deserialize)]
pub struct SessionRequest {
    device_id: Uuid,

    #[serde(with = "base64enc")]
    nonce: Vec<u8>,

    #[serde(with = "base64enc")]
    signed_nonce: Vec<u8>
}

#[derive(Serialize)]
pub struct SessionResponse {
    #[serde(with = "base64enc")]
    nonce: Vec<u8>
}

#[derive(Serialize)]
struct NewSessionResponse {
    #[serde(with = "base64enc")]
    nonce: Vec<u8>,
    expiry: NaiveDateTime
}

// In seconds
const SESSION_DURATION: i64 = 60*60;

pub async fn new_session_request(data: web::Data<ApplicationData>) -> impl Responder {
    let res: Result<NewSessionResponse, HandlerError> = web::block(move || {
        let conn = data.pool.get()
            .map_err(|e| HandlerError::InternalError(InternalError::PoolError(e)))?;

        // Generate new nonce
        let mut nonce = vec![0u8; 16];
        data.rng.fill(&mut nonce)
            .map_err(|_e| HandlerError::InternalError(InternalError::RNGError))?;

        // Expiry
        let expiry = (Utc::now() + Duration::seconds(SESSION_DURATION)).naive_utc();
        diesel::insert_into(sessions::table)
            .values((
                sessions::nonce.eq(&nonce),
                sessions::expires.eq(expiry)
            )).execute(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))?;
        // Return new nonce
        Ok::<NewSessionResponse, HandlerError>(NewSessionResponse { nonce, expiry })
    }).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => HandlerError::InternalError(InternalError::AsyncError)
    });
    match res {
        Ok(obj) => Ok(HttpResponse::Ok().json(obj)),
        Err(e) => Err(e),
    }
}

pub async fn check_session(req: SessionRequest, data: &ApplicationData) -> Result<(Uuid, SessionResponse), HandlerError> {
    let data = data.clone();
    web::block(move || {
        let conn = data.pool.get()
            .map_err(|e| HandlerError::InternalError(InternalError::PoolError(e)))?;

        // Look for active session
        let (session_id, session_expires) = sessions::table.filter(sessions::nonce.eq(&req.nonce))
            .select((sessions::id, sessions::expires))
            .first::<(i32, NaiveDateTime)>(&conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => HandlerError::SessionInvalid,
                _ => HandlerError::InternalError(InternalError::DatabaseError(e))
            })?;

        // Still valid
        return if session_expires > Utc::now().naive_utc() {
            // Query devices
            let pub_key = devices::table.find(req.device_id).select(devices::public_key).first::<Vec<u8>>(&conn)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => HandlerError::DeviceUnknown,
                    _ => HandlerError::InternalError(InternalError::DatabaseError(e))
                })?;
            let device_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
            device_public_key.verify(&req.nonce[..], &req.signed_nonce[..])
                .map_err(|_e| HandlerError::AuthenticationError)?;

            // Now generate new nonce
            let mut new_nonce =  vec![0u8; 16];
            data.rng.fill(&mut new_nonce)
                .map_err(|_e| HandlerError::InternalError(InternalError::RNGError))?;

            // And expiry
            let expiry = (Utc::now() + Duration::seconds(SESSION_DURATION)).naive_utc();
            diesel::update(sessions::table.find(session_id))
                .set((
                    sessions::nonce.eq(&new_nonce),
                    sessions::expires.eq(expiry)
                )).execute(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))?;
            // Return new nonce, and device id

            Ok((req.device_id, SessionResponse{ nonce: new_nonce}))
        } else {
            diesel::delete(sessions::table.find(session_id))
                .execute(&conn).map_err(|e| HandlerError::InternalError(InternalError::DatabaseError(e)))?;
            Err(HandlerError::SessionInvalid)
        }
    }).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => HandlerError::InternalError(InternalError::AsyncError)
    })
}