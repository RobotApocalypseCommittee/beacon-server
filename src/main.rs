#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use crate::database::Pool;

mod base64enc;
mod schema;
mod database;
mod session;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn test(dat: web::Json<session::SessionRequest>, app_data: web::Data<ApplicationData>) -> impl Responder {
    session::check_session(dat.into_inner(), app_data.pool.clone()).await;
    HttpResponse::Ok().body("Hello world!")
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
            .route("/test", web::post().to(test))
            .service(
                web::scope("/messages")
                    .route("/send", web::post().to(session::new_session_request))
            )
    })
        .bind("127.0.0.1:8088")?
        .run()
        .await
}