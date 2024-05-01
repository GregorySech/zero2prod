use actix_web::HttpResponse;

#[tracing::instrument(
    name = "Confirm a pending subscriber"
)]
pub async fn confirm() -> HttpResponse {
    HttpResponse::Ok().finish()
}