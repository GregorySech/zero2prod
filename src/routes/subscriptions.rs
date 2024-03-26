use actix_web::{web, HttpResponse, Responder};
use sqlx::{types::Uuid, PgPool};
use chrono::Utc;

#[derive(serde::Deserialize)]
pub struct SubscribeFormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Saving new subscriber details to db",
    skip(pool, form)

)]
async fn insert_subscriber(
    pool: &PgPool,
    form: &SubscribeFormData
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
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

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<SubscribeFormData>,
    pool: web::Data<PgPool>) -> impl Responder {
    match insert_subscriber(
        &pool, &form,
    ).await {
        Ok(_) => {
            tracing::info!("New subscriber saved!");
            HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!("Failed to execute insert query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}