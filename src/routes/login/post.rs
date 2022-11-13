use actix_web::http::header::LOCATION;
use actix_web::{web, HttpResponse};
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    username: String,
    password: Secret<String>,
}

pub async fn login(_login_request: web::Form<LoginRequest>) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
