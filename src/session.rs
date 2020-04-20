use actix_web::{web, FromRequest, HttpRequest, HttpMessage};
use actix_web::error::BlockingError;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{base64enc};
use crate::utils::{HandlerError, InternalError, Entity};

use super::schema::{devices, sessions};

use std::pin::Pin;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, Ready, ready};
use futures::Future;
use crate::database::{Pool, extract_connection};
use std::str::FromStr;
use actix_web::dev::{PayloadStream, Payload};
use std::rc::Rc;
use std::cell::RefCell;
use actix_web::http::{HeaderName, HeaderValue};
use std::convert::TryFrom;

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

pub fn new_session_request(pool: &Pool, rng: &SystemRandom) -> Result<Vec<u8>, HandlerError> {
        let conn = extract_connection(pool)?;

        // Generate new nonce
        let mut nonce = vec![0u8; 16];
        rng.fill(&mut nonce)
            .map_err(|_e| HandlerError::InternalError{ error: InternalError::RNGError})?;

        // Expiry
        let expiry = (Utc::now() + Duration::seconds(SESSION_DURATION)).naive_utc();
        diesel::insert_into(sessions::table)
            .values((
                sessions::nonce.eq(&nonce),
                sessions::expires.eq(expiry)
            )).execute(&conn).map_err(|e| -> HandlerError { InternalError::DatabaseError(e).into()})?;
        // Return new nonce
        Ok(nonce)
}
#[derive(Clone)]
pub struct SessionInfo {
    pub device_id: Uuid,
    pub user_id: Option<Uuid>
}

impl FromRequest for SessionInfo {
    type Error = HandlerError;
    type Future = Ready<Result<Self, HandlerError>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload<PayloadStream>) -> Self::Future {
        ready(match req.extensions().get::<SessionInfo>() {
            None => Err(HandlerError::InternalError{ error: InternalError::ServerDataError}),
            Some(s) => Ok(s.clone()),
        })
    }
}

fn check_session(req: SessionRequest, pool: &Pool, rng: &SystemRandom) -> Result<(Uuid, Option<Uuid>, Vec<u8>), HandlerError> {
    let conn = extract_connection(pool)?;

    // Look for active session
    let (session_id, session_expires) = sessions::table.filter(sessions::nonce.eq(&req.nonce))
        .select((sessions::id, sessions::expires))
        .first::<(i32, NaiveDateTime)>(&conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => HandlerError::SessionInvalid,
            _ => HandlerError::InternalError {error: InternalError::DatabaseError(e)}
        })?;

    // Still valid
    return if session_expires > Utc::now().naive_utc() {
        // Query devices
        let (pub_key, owner) = devices::table.find(req.device_id).select((devices::public_key, devices::user_id)).first::<(Vec<u8>, Option<Uuid>)>(&conn)
            .map_err(|e| -> HandlerError { match e {
                diesel::result::Error::NotFound => HandlerError::UnknownEntity {entity: Entity::Device {uuid: req.device_id}},
                _ => InternalError::DatabaseError(e).into()
            }})?;
        let device_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
        device_public_key.verify(&req.nonce[..], &req.signed_nonce[..])
            .map_err(|_e| HandlerError::AuthenticationError)?;

        // Now generate new nonce
        let mut new_nonce =  vec![0u8; 16];
        rng.fill(&mut new_nonce)
            .map_err(|_e| InternalError::RNGError)?;

        // And expiry
        let expiry = (Utc::now() + Duration::seconds(SESSION_DURATION)).naive_utc();
        diesel::update(sessions::table.find(session_id))
            .set((
                sessions::nonce.eq(&new_nonce),
                sessions::expires.eq(expiry)
            )).execute(&conn).map_err(|e| InternalError::DatabaseError(e))?;

        // Return new nonce, and device id
        Ok((req.device_id, owner, new_nonce))
    } else {
        diesel::delete(sessions::table.find(session_id))
            .execute(&conn).map_err(|e| InternalError::DatabaseError(e))?;
        Err(HandlerError::SessionInvalid)
    }
}

fn extract_header_data (req: &ServiceRequest) -> Result<SessionRequest, HandlerError> {
    let head_err = |n: &str| HandlerError::MalformedHeader { name: n.to_string()};
    let get_header = |n: &str| { req.headers().get(n)
        .ok_or(head_err(n))
        .and_then(|header| header.to_str().map_err(|_e| head_err(n)))};

    let device_id = uuid::Uuid::from_str(get_header("X-DEVICEID")?)
        .map_err(|_e| head_err("X-DEVICEID"))?;
    let nonce = base64::decode(get_header("X-NONCE")?)
        .map_err(|_e| head_err("X-NONCE"))?;
    let signed_nonce = base64::decode(get_header("X-SIGNEDNONCE")?)
        .map_err(|_e| head_err("X-SIGNEDNONCE"))?;

    Ok(SessionRequest {
        device_id,
        nonce,
        signed_nonce
    })

}

pub struct CheckSession;

impl<S: 'static, B> Transform<S> for CheckSession
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckSessionMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CheckSessionMiddleware { service: Rc::new(RefCell::new(service)) })
    }
}

pub struct CheckSessionMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for CheckSessionMiddleware<S>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let session_data = extract_header_data(&req);
        let pool = req.app_data::<Pool>().ok_or(InternalError::ServerDataError);
        let rng = req.app_data::<SystemRandom>().ok_or(InternalError::ServerDataError);
        let mut srv = self.service.clone();
        Box::pin(async move {
            let (device_id, user_id, nonce) = web::block(move || check_session(session_data?, &pool?.into_inner(), &rng?.into_inner()))
                .await.map_err(|e| match e {
                BlockingError::Error(he) => he,
                BlockingError::Canceled => InternalError::AsyncError.into()
            })?;
            req.extensions_mut().insert(SessionInfo{ device_id, user_id });
            let mut res: Self::Response = srv.call(req).await?;
            res.headers_mut().insert(HeaderName::try_from("x-newnonce").map_err(|_e| HandlerError::from(InternalError::JustAnError))?,
                                     HeaderValue::try_from(base64::encode(&nonce)).expect("NONCE BASE64 INVALID"));

            Ok(res)
        })
    }
}