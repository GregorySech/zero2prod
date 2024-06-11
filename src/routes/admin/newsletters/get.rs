use actix_web::HttpResponse;

pub async fn send_newsletter_form() -> HttpResponse {
    HttpResponse::Ok().finish()
}
