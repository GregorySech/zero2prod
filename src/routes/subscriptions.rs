use std::os::unix::thread;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailAPIClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{types::Uuid, PgPool};

use super::ConfirmationParameters;

#[derive(serde::Deserialize)]
pub struct SubscribeFormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Saving new subscriber details to db",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now() // Is this web-server time? Kinda risky :/
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Saving subscription token to db", skip(pool))]
async fn store_subscription_token(
    pool: &PgPool,
    subscription_token: &str,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
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
            new_subscriber.email,
            "Welcome",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(
    name = "Generating subscription token"
)]
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

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
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Err(_) => return HttpResponse::BadRequest().finish(),
        Ok(subscriber) => subscriber,
    };

    let subscriber_id = match insert_subscriber(&pool, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let subscription_token = generate_subscription_token();
    if store_subscription_token(&pool, &subscription_token, subscriber_id)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        email_client.as_ref(),
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}
