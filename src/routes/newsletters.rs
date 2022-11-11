use actix_web::HttpResponse;

pub async fn publish_newsletter() -> HttpResponse {
    println!("here");
    HttpResponse::Ok().finish()
}
