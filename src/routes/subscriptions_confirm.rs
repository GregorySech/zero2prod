use actix_web::{web::Query, HttpResponse};

#[derive(serde::Deserialize)]
pub struct ConfirmationParameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(_parameters)
)]
pub async fn confirm(_parameters: Query<ConfirmationParameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}