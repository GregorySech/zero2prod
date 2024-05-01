use crate::{domain::{NewSubscriber, SubscriberEmail, SubscriberName}, email_client::EmailAPIClient};
use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::{types::Uuid, PgPool};

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
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::new_v4(),
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
    name = "Adding a new subscriber",
    skip(form, pool, email_client),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<SubscribeFormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Err(_) => return HttpResponse::BadRequest().finish(),
        Ok(subscriber) => subscriber,
    };

    if insert_subscriber(&pool, &new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if email_client.send_email(new_subscriber.email, "Welcome!", "Welcome to our newsletter!", "Welcome to our newsletter!").await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    
    HttpResponse::Ok().finish()

}

