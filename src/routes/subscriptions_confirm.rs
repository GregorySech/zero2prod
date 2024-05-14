use actix_web::{
    web::{Data, Query},
    HttpResponse, Responder, ResponseError,
};
use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use super::error_chain_fmt;

/// Endpoint for the subscription confirmation token. Checks if the subscription token is associated with a subscription and confirms is.
#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
pub async fn confirm(
    _parameters: Query<ConfirmationParameters>,
    pool: Data<PgPool>,
) -> Result<impl Responder, ConfirmationError> {
    let subscription_token = _parameters.0.subscription_token;
    let subscriber_id = subscriber_id_from_token(subscription_token, &pool).await?;

    confirm_subscriber(subscriber_id, &pool)
        .await
        .context("Failed to change subscription status.")?;

    Ok(HttpResponse::Ok())
}

#[derive(serde::Deserialize)]
pub struct ConfirmationParameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Get subscriber_id from the confirmation token!",
    skip(pool, subscription_token)
)]
async fn subscriber_id_from_token(
    subscription_token: String,
    pool: &PgPool,
) -> Result<Uuid, ConfirmationError> {
    let result = sqlx::query!(
        "SELECT subscriber_id
    FROM subscriptions_tokens
    WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve subscription confirmation.")?;

    match result {
        Some(record) => Ok(record.subscriber_id),
        None => Err(ConfirmationError::ValidationError(
            "No subscription found!".into(),
        )),
    }
}

#[tracing::instrument(name = "Changing subscriber status", skip(subscriber_id, pool))]
async fn confirm_subscriber(subscriber_id: Uuid, pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            ConfirmationError::ValidationError(_) => StatusCode::UNAUTHORIZED,
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
