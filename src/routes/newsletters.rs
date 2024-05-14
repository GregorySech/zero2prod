use actix_web::HttpResponse;

pub async fn publish_newsletters() -> HttpResponse {
    HttpResponse::Ok().finish()
}