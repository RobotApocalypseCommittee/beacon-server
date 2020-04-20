use actix_web::{ResponseError, HttpResponse};
use actix_web::http::StatusCode;
use std::fmt;
use uuid::Uuid;
use actix_web::error::BlockingError;
use serde::Serialize;
use serde::export::Formatter;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum HandlerError {
    SessionExpired,
    SessionInvalid,
    UnknownEntity { #[serde(flatten)] entity: Entity },
    AuthenticationError,
    MalformedHeader { name: String },
    MalformedBody { error_message: String },
    InternalError { #[serde(skip_serializing)] error: InternalError },
}

#[derive(Debug, Serialize)]
#[serde(tag = "entity")]
pub enum Entity {
    User{uuid: Uuid},
    Device{uuid: Uuid},
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        match self {
            HandlerError::SessionInvalid => StatusCode::UNAUTHORIZED,
            HandlerError::SessionExpired => StatusCode::UNAUTHORIZED,
            HandlerError::UnknownEntity {..} => StatusCode::BAD_REQUEST,
            HandlerError::AuthenticationError => StatusCode::UNAUTHORIZED,
            HandlerError::InternalError{ .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::MalformedHeader { .. } => StatusCode::BAD_REQUEST,
            HandlerError::MalformedBody { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        println!("Error serving request: {} {:?}", self.status_code(), self);
        HttpResponse::build(self.status_code())
            .json(self)
    }
}

#[derive(Debug)]
pub enum InternalError {
    DatabaseError(diesel::result::Error),
    PoolError(r2d2::Error),
    AsyncError,
    RNGError,
    ServerDataError,
    JustAnError,
}

impl From<InternalError> for HandlerError {
    fn from(e: InternalError) -> Self {
        HandlerError::InternalError {error: e}
    }
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
            BlockingError::Canceled => HandlerError::InternalError{ error: InternalError::AsyncError },
        });
}