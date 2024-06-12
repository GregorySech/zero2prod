use actix_web::{
    http::header::{self},
    web, HttpResponse, Responder, ResponseError,
};
use anyhow::Context;

use reqwest::{header::HeaderValue, StatusCode};
use sqlx::PgPool;

use crate::{
    authentication::UserId, domain::get_confirmed_subscribers, email_client::EmailAPIClient,
};

use super::error_chain_fmt;

#[tracing::instrument(name = "Publish a newsletter issue", skip(body, pool, email_client))]
pub async fn publish_newsletters(
    body: web::Json<PublishData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
    _user_id: web::Data<UserId>,
) -> Result<impl Responder, PublishError> {
    let body = body.0;
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to retrieve subscribers")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| format!("Failed to send newsletter to {}", subscriber.email))?,
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. Their stored contact details are invalid!");
            }
        }
    }

    Ok(HttpResponse::Ok())
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct PublishData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}
