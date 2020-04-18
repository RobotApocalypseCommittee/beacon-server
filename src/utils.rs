use actix_web::{ResponseError, HttpResponse};
use actix_web::http::StatusCode;
use std::fmt;
use uuid::Uuid;
use actix_web::error::BlockingError;

#[derive(Debug)]
pub enum HandlerError {
    SessionInvalid,
    DeviceUnknown,
    UserUnknown(Uuid),
    AuthenticationError,
    MalformedData(&'static str),
    InternalError(InternalError)
}
impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            HandlerError::SessionInvalid => String::from("SessionInvalid"),
            HandlerError::DeviceUnknown => String::from("DeviceUnknown"),
            HandlerError::AuthenticationError => String::from("AuthenticationError"),
            HandlerError::InternalError(_) => String::from("InternalError"),
            HandlerError::MalformedData(s) => format!("MalformedData: {}", s),
            HandlerError::UserUnknown(u) => format!("UserUnknown: {}", u)
        })
    }
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        match self {
            HandlerError::SessionInvalid => StatusCode::UNAUTHORIZED,
            HandlerError::DeviceUnknown => StatusCode::UNAUTHORIZED,
            HandlerError::AuthenticationError => StatusCode::UNAUTHORIZED,
            HandlerError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::MalformedData(_) => StatusCode::BAD_REQUEST,
            HandlerError::UserUnknown(_) => StatusCode::UNAUTHORIZED
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("plain/text")
            .body(self.to_string())
    }
}

#[derive(Debug)]
pub enum InternalError {
    DatabaseError(diesel::result::Error),
    PoolError(r2d2::Error),
    AsyncError,
    RNGError,
    ServerDataError
}

pub fn malformed_data(s: &'static str) -> HandlerError {
    HandlerError::MalformedData(s)
}


// Ludicrous syntactic sugar
pub async fn block<F, I>(f: F) -> Result<I, HandlerError>
    where
        F: FnOnce() -> Result<I, HandlerError> + Send + 'static,
        I: Send + 'static,
{
    return actix_web::web::block(f).await
        .map_err(|e| match e {
            BlockingError::Error(e) => e,
            BlockingError::Canceled => HandlerError::InternalError(InternalError::AsyncError),
        })
}