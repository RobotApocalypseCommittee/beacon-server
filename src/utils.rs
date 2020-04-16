use actix_web::{ResponseError, HttpResponse};
use actix_web::http::StatusCode;
use std::fmt;

#[derive(Debug)]
pub enum HandlerError {
    SessionInvalid,
    DeviceUnknown,
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
            HandlerError::MalformedData(s) => format!("MalformedData: {}", s)
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
            HandlerError::MalformedData(_) => StatusCode::BAD_REQUEST
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