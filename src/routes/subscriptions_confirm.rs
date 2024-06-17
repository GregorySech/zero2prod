use actix_web::{
    web::{Data, Query},
    HttpResponse, Responder, ResponseError,
};
use anyhow::{anyhow, Context};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::SubscriberStatus;

use super::error_chain_fmt;

/// Endpoint for the subscription confirmation token. Checks if the subscription token is associated with a subscription and confirms is.
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: Query<ConfirmationParameters>,
    pool: Data<PgPool>,
) -> Result<impl Responder, ConfirmationError> {
    let subscription_token = parameters.0.subscription_token;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire database transaction.")?;

    let subscriber_id = subscriber_id_from_token(subscription_token, &mut transaction).await?;

    let status = subscriber_status_from_id(subscriber_id, &mut transaction).await?;

    match status {
        SubscriberStatus::Confirmed => Err(ConfirmationError::AlreadySubscribed(
            "Subscription already confirmed".to_string(),
        )),
        _ => Ok(()),
    }?;

    confirm_subscriber(subscriber_id, &mut transaction)
        .await
        .context("Failed to change subscription status.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction.")?;

    Ok(HttpResponse::Ok())
}

#[derive(serde::Deserialize)]
pub struct ConfirmationParameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Get the subscriber status given the subscription id!",
    skip(transaction, subscription_id)
)]
async fn subscriber_status_from_id(
    subscription_id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<SubscriberStatus, ConfirmationError> {
    let result = sqlx::query!(
        "SELECT status
         FROM subscriptions
         WHERE id = $1
         FOR UPDATE
         ",
        subscription_id
    )
    .fetch_one(&mut **transaction)
    .await
    .context("Failed to retrieve subscription status.")?;

    SubscriberStatus::parse(&result.status)
        .map_err(|e| ConfirmationError::UnexpectedError(anyhow!(e)))
}

#[tracing::instrument(
    name = "Get subscriber_id from the confirmation token!",
    skip(transaction, subscription_token)
)]
async fn subscriber_id_from_token(
    subscription_token: String,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, ConfirmationError> {
    let result = sqlx::query!(
        "SELECT subscriber_id
    FROM subscriptions_tokens
    WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(&mut **transaction)
    .await
    .context("Failed to retrieve subscription confirmation.")?;

    match result {
        Some(record) => Ok(record.subscriber_id),
        None => Err(ConfirmationError::ValidationError(
            "No subscription found!".into(),
        )),
    }
}

#[tracing::instrument(name = "Changing subscriber status", skip(subscriber_id, transaction))]
async fn confirm_subscriber(
    subscriber_id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{0}")]
    AlreadySubscribed(String),
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
            ConfirmationError::AlreadySubscribed(_) => StatusCode::GONE,
        }
    }
}
