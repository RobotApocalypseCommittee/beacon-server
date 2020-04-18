#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use crate::database::Pool;
use uuid::Uuid;
use crate::utils::{HandlerError, InternalError, block};
use actix_web::error::BlockingError;
use crate::session::{SessionInfo};
use ring::rand::SystemRandom;

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
    Ok(HttpResponse::Ok().json(CreateUserResponse{ user_id: res}))
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
    let RegisterDeviceRequest{ public_key} = data.into_inner();
    let res = web::block(move || device::create_device(&pool, &public_key)).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => HandlerError::InternalError(InternalError::AsyncError)
    })?;
    Ok(HttpResponse::Ok().json(RegisterDeviceResponse{device_id: res}))
}

#[derive(Deserialize)]
struct TestRequest {
    number: i32
}

#[derive(Serialize)]
struct TestResponse {
    number: i32
}

async fn test(data: web::Json<TestRequest>, session: SessionInfo) -> impl Responder {
    // Decompose
    let TestRequest{number} = data.into_inner();
    let SessionInfo{ device_id, user_id } = session;
    println!("{}, {:?}", device_id, user_id);
    let new_number = number + 1;
    // Type hint needed (Rust can't figure it out properly)
    Ok::<HttpResponse, utils::HandlerError>(HttpResponse::Ok().json(TestResponse{ number: new_number }))
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
            .route("/", web::get().to(index))
            .route("/session/new", web::post().to(api_create_session))
            .route("/devices/new", web::post().to(api_register_device))
            .service(
                web::scope("/users")
                    .wrap(session::CheckSession)
                    .route("/new", web::post().to(api_create_user))
            )
    })
        .bind("127.0.0.1:8088")?
        .run()
        .await
}