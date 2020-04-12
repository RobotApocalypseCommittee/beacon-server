#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use crate::database::Pool;

mod utils;
mod base64enc;
mod schema;
mod database;
mod session;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
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
    let (device_id, sess_resp) = session::check_session(session, &*app_data.into_inner()).await?;
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