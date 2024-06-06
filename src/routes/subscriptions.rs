use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriberStatus},
    email_client::EmailAPIClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::StatusCode;
use sqlx::{types::Uuid, Executor, PgPool, Postgres, Transaction};

use super::error_chain_fmt;

/// Route creates a new subscription and sends a confirmation email to the user.
///
/// Requests for subscriptions that are in pending confirmation status send
/// again confirmation emails.  
/// Requests for other existing subscriptions are unauthorized until a better
/// response kind is proposed.
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<SubscribeFormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<impl Responder, SubscribeError> {
    // Confirm well formed new subscriber form.
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    // Confirm that the status for the subscriber is correct for the endpoint.
    let subscriber_status = subscription_status(&new_subscriber, &pool)
        .await
        .context("Failed to retrieve subscriber status.")?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire database connection from the pool")?;

    // Create pending subscription or retrieve existing subscription id.
    let sub_id = match subscriber_status {
        SubscriberStatus::Unsubscribed => insert_subscriber(&mut transaction, &new_subscriber)
            .await
            .context("Failed to insert subscriber.")?,
        SubscriberStatus::PendingConfirmation => uuid_for_subscriber(&new_subscriber, &pool)
            .await
            .context("Failed to retrieve subscriber information.")?,
        SubscriberStatus::Confirmed => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscription_token = generate_subscription_token();
    store_subscription_token(&mut transaction, &subscription_token, sub_id)
        .await
        .context("Failed to store confirmation token.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit subscription transaction.")?;

    send_confirmation_email(
        email_client.as_ref(),
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

pub struct StoreTokenError(sqlx::Error);

impl From<sqlx::Error> for StoreTokenError {
    fn from(value: sqlx::Error) -> Self {
        Self(value)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token: {:?}",
            self.0
        )
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SubscribeFormData {
    email: String,
    name: String,
}

impl TryFrom<SubscribeFormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscribeFormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;

        Ok(NewSubscriber { email, name })
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details to db",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now() // Is this web-server time? Kinda risky :/
    );

    transaction.execute(query).await?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Saving subscription token to db", skip(transaction))]
async fn store_subscription_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscription_token: &str,
    subscriber_id: Uuid,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    );
    transaction.execute(query).await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
async fn send_confirmation_email(
    email_client: &EmailAPIClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    // TODO change the confirmation link logic:
    // link should get a domain from app configuration
    // link should get a registration token baked-in.
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_content = format!(
        "Welcome to our newsletter! <br/> Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let text_content = format!(
        "Welcome to our newsletter! Visit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(
            &new_subscriber.email,
            "Welcome",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(name = "Generating subscription token")]
pub fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name = "Getting subscription status", skip(subscriber, pool))]
async fn subscription_status(
    subscriber: &NewSubscriber,
    pool: &PgPool,
) -> Result<SubscriberStatus, sqlx::Error> {
    let subscription_record = sqlx::query!(
        "SELECT status 
        FROM subscriptions 
        WHERE email = $1",
        subscriber.email.as_ref(),
    )
    .fetch_optional(pool)
    .await?;
    if subscription_record.is_none() {
        return Ok(SubscriberStatus::Unsubscribed);
    }
    let subscription_status = SubscriberStatus::parse(&subscription_record.unwrap().status)
        .map_err(|e| {
            // Keeping this tracing as the error is handled as a signal.
            tracing::error!(e);
            e
        });
    match subscription_status {
        Ok(status) => Ok(status),
        Err(_) => Ok(SubscriberStatus::Unsubscribed),
    }
}

#[tracing::instrument(name = "Getting subscriber uuid", skip(subscriber, pool))]
async fn uuid_for_subscriber(
    subscriber: &NewSubscriber,
    pool: &PgPool,
) -> Result<Uuid, sqlx::Error> {
    let uuid_record = sqlx::query!(
        "
        SELECT id
        FROM subscriptions
        WHERE email = $1",
        subscriber.email.as_ref()
    )
    .fetch_one(pool)
    .await?;

    Ok(uuid_record.id)
}
