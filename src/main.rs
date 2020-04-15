#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use crate::database::Pool;
use uuid::Uuid;
use crate::utils::{HandlerError, InternalError};
use actix_web::error::BlockingError;

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

#[derive(Deserialize)]
struct RegisterDeviceRequest {
    #[serde(with = "base64enc")]
    public_key: Vec<u8>
}

#[derive(Serialize)]
struct RegisterDeviceResponse {
    device_id: Uuid
}

async fn api_register_device(data: web::Json<RegisterDeviceRequest>, app_data: web::Data<ApplicationData>) -> Result<HttpResponse, HandlerError> {
    let pool = app_data.into_inner().pool.clone();
    let RegisterDeviceRequest{ public_key} = data.into_inner();
    let res = web::block(move || device::create_device(&pool, &public_key)).await.map_err(|e| match e {
        BlockingError::Error(he) => he,
        BlockingError::Canceled => HandlerError::InternalError(InternalError::AsyncError)
    })?;
    Ok(HttpResponse::Ok().json(RegisterDeviceResponse{device_id: res}))
}

#[derive(Deserialize)]
struct TestRequest {
    #[serde(flatten)]
    session: session::SessionRequest,

    number: i32
}

#[derive(Serialize)]
struct TestResponse {

    #[serde(flatten)]
    session: session::SessionResponse,

    number: i32
}

async fn test(dat: web::Json<TestRequest>, app_data: web::Data<ApplicationData>) -> impl Responder {
    // Decompose
    let TestRequest{number, session} = dat.into_inner();
    let (_device_id, sess_resp) = session::check_session(session, &*app_data.into_inner()).await?;
    let new_number = number + 1;
    // Type hint needed (Rust can't figure it out properly)
    Ok::<HttpResponse, utils::HandlerError>(HttpResponse::Ok().json(TestResponse{ session: sess_resp, number: new_number }))
}

#[derive(Clone)]
pub struct ApplicationData {
    pool: Pool,
    rng: ring::rand::SystemRandom
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let data = ApplicationData{
        pool: database::obtain_pool(),
        rng: ring::rand::SystemRandom::new()
    };

    HttpServer::new(move || {
        println!("Starting new App instance");
        App::new()
            .data(data.clone())
            .route("/", web::get().to(index))
            .route("/increment", web::post().to(test))
            .route("/newsession", web::post().to(session::new_session_request))
    })
        .bind("127.0.0.1:8088")?
        .run()
        .await
}