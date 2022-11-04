use std::net::TcpListener;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web::dev::Server;

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

// We need to mark `run` as public.
// It is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation.
pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .listen(listener)?
        .run();

    Ok(server)
}