use actix_web::{web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailAPIClient};

use super::error_chain_fmt;

pub async fn publish_newsletters(
    _body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
) -> Result<impl Responder, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to retrieve subscribers")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &_body.title,
                    &_body.content.html,
                    &_body.content.text,
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
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subs = sqlx::query!(
        r#"
    SELECT email
    FROM subscriptions
    WHERE status = 'confirmed'
    "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subs)
}
