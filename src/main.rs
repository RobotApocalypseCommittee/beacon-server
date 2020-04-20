#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use crate::database::Pool;
use uuid::Uuid;
use crate::utils::{HandlerError, InternalError, block};
use actix_web::error::BlockingError;
use crate::session::SessionInfo;
use ring::rand::SystemRandom;
use actix_web::web::JsonConfig;

mod utils;
mod base64enc;
mod schema;
mod database;
mod session;
mod user;
mod device;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn api_create_session(pool: web::Data<Pool>, rng: web::Data<SystemRandom>) -> Result<HttpResponse, HandlerError> {
    let nonce = block(move || session::new_session_request(&pool, &rng)).await?;
    Ok(HttpResponse::Ok().set_header("X-NEWNONCE", base64::encode(nonce)).finish())
}


#[derive(Serialize)]
struct CreateUserResponse {
    user_id: Uuid
}

async fn api_create_user(data: web::Json<user::UserCreation>, pool: web::Data<Pool>, session: SessionInfo) -> Result<HttpResponse, HandlerError> {
    let data = data.into_inner();
    let res = block(move || user::create_user(&pool, data, session.device_id)).await?;
    Ok(HttpResponse::Ok().json(CreateUserResponse { user_id: res }))
}

#[derive(Deserialize)]
struct RegisterDeviceRequest {
    #[serde(with = "base64enc")]
    public_key: Vec<u8>
}

#[derive(Serialize)]
struct RegisterDeviceResponse {
    device_id: Uuid
}

async fn api_register_device(data: web::Json<RegisterDeviceRequest>, pool: web::Data<Pool>) -> Result<HttpResponse, HandlerError> {
    let RegisterDeviceRequest { public_key } = data.into_inner();
    let res = web::block(move || device::create_device(&pool, &public_key)).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => InternalError::AsyncError.into()
    })?;
    Ok(HttpResponse::Ok().json(RegisterDeviceResponse { device_id: res }))
}

async fn api_new_signed_key(data: web::Json<user::PreKeyUpdate>, pool: web::Data<Pool>, session: SessionInfo) -> Result<HttpResponse, HandlerError> {
    block(move || user::update_prekey(&pool, data.into_inner(), session.user_id.ok_or(HandlerError::AuthenticationError)?)).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn api_new_otks(data: web::Json<user::OTKAdd>, pool: web::Data<Pool>, session: SessionInfo) -> Result<HttpResponse, HandlerError> {
    let user_id = session.user_id.ok_or(HandlerError::AuthenticationError)?;
    let data = data.into_inner().keys;
    let no_committed = block(move || user::add_otks(&pool, &data, user_id)).await?;
    Ok(HttpResponse::Ok().finish())
}


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let pool = database::obtain_pool();
    let rng = ring::rand::SystemRandom::new();

    HttpServer::new(move || {
        println!("Starting new App instance");
        App::new()
            .data(pool.clone())
            .data(rng.clone())
            .app_data(JsonConfig::default().error_handler(|e, _| {
                println!("Hello");
                HandlerError::MalformedBody { error_message: e.to_string() }.into()
            }))
            .route("/", web::get().to(index))
            .route("/session/new", web::post().to(api_create_session))
            .route("/devices/new", web::post().to(api_register_device))
            .service(
                web::scope("/users")
                    .wrap(session::CheckSession)
                    .route("/new", web::post().to(api_create_user))
            )
            .service(
                web::scope("/keys")
                    .wrap(session::CheckSession)
                    .route("/signed", web::post().to(api_new_signed_key))
                    .route("/onetime", web::post().to(api_new_otks))
            )
    })
        .bind("0.0.0.0:8088")?
        .run()
        .await
}