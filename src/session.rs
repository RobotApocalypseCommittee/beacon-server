use actix_web::{web, Responder, HttpResponse, error};
use ring::rand::{SecureRandom};
use diesel::prelude::*;
use super::schema::{sessions, devices};
use crate::{ApplicationData, base64enc, database};
use chrono::{Utc, Duration, DateTime, NaiveDateTime};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::session::HandlerError::{SessionInvalid, AsyncError};
use ring::signature;
use actix_web::error::BlockingError;

#[derive(Debug)]
pub enum HandlerError {
    DatabaseError(diesel::result::Error),
    PoolError(r2d2::Error),
    AsyncError,
    SessionInvalid,
    DeviceUnknown,
    AuthenticationError
}

#[derive(Deserialize)]
pub struct SessionRequest {
    device_id: Uuid,

    #[serde(with = "base64enc")]
    nonce: Vec<u8>,

    #[serde(with = "base64enc")]
    signed_nonce: Vec<u8>
}

#[derive(Serialize)]
struct NewSessionResponse {
    #[serde(with = "base64enc")]
    nonce: Vec<u8>,
    expiry: NaiveDateTime
}

pub async fn new_session_request(app_data: web::Data<ApplicationData>) -> impl Responder {
    let expiry = (Utc::now() + Duration::days(1)).naive_utc();
    let mut r_nonce =  vec![0u8; 16];
    app_data.rng.fill(&mut r_nonce).expect("Random Broken AAAAA");
    let n_nonce = r_nonce.clone();
    let res = web::block( move || {
        let conn = app_data.pool.get()
            .map_err(|e| e.to_string())?;
        println!("Got pool");

        diesel::insert_into(sessions::table)
            .values((
                sessions::nonce.eq(n_nonce),
                sessions::expires.eq(expiry)
                ))
            .returning(sessions::id)
            .get_result::<i32>(&conn).map_err(|e| e.to_string())
    }).await;
    match res {
        Ok(_id) => Ok(HttpResponse::Ok().json(
            NewSessionResponse{nonce: r_nonce, expiry })),
        Err(e) => {
            eprintln!("{}", e);
            Err(error::ErrorInternalServerError("A mishap"))
        },
    }
}

pub async fn check_session(req: SessionRequest, pool: database::Pool) -> Result<Uuid, HandlerError>{
    web::block(move || {
        let conn = pool.get()
            .map_err(|e| HandlerError::PoolError(e))?;

        // Look for active session
        let (session_id, session_expires) = sessions::table.filter(sessions::nonce.eq(&req.nonce))
            .select((sessions::id, sessions::expires))
            .first::<(i32, NaiveDateTime)>(&conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => HandlerError::SessionInvalid,
                _ => HandlerError::DatabaseError(e)
            })?;

        // Still valid
        return if session_expires > Utc::now().naive_utc() {
            // Query devices
            let pub_key = devices::table.find(req.device_id).select(devices::public_key).first::<Vec<u8>>(&conn)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => HandlerError::DeviceUnknown,
                    _ => HandlerError::DatabaseError(e)
                })?;
            let device_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
            device_public_key.verify(&req.nonce[..], &req.signed_nonce[..])
                .map_err(|e| HandlerError::AuthenticationError)?;
            // TODO: Generate new nonce...
            Ok(req.device_id)
        } else {
            Err(HandlerError::SessionInvalid)
        }
    }).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => HandlerError::AsyncError
    })
}