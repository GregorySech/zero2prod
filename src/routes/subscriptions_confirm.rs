use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

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
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id
    FROM subscriptions_tokens
    WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute subscriber_id query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
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
    .await
    .map_err(|e| {
        tracing::error!("Failed query to update subscription status: {:?}", e);
        e
    })?;
    Ok(())
}

/// Endpoint for the subscription confirmation token. Checks if the subscription token is associated with a subscription and confirms is.
#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
pub async fn confirm(
    _parameters: Query<ConfirmationParameters>,
    pool: Data<PgPool>,
) -> HttpResponse {
    let subscription_token = _parameters.0.subscription_token;

    let subscriber_id = match subscriber_id_from_token(subscription_token, &pool).await {
        Ok(maybe_uuid) => match maybe_uuid {
            Some(uuid) => uuid,
            None => return HttpResponse::Unauthorized().finish(),
        },
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if confirm_subscriber(subscriber_id, &pool).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}
