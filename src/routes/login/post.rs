use actix_web::{web, HttpResponse};
use reqwest::header::LOCATION;
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct LoginFormData {
    username: String,
    password: Secret<String>,
}

pub async fn login(_form: web::Form<LoginFormData>) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
