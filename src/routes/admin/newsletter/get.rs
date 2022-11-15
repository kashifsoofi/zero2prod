use actix_web::HttpResponse;

pub async fn publish_newsletter_form() -> HttpResponse {
    HttpResponse::Ok().finish()
}
