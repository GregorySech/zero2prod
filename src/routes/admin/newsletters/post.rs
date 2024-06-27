use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500},
};

#[derive(serde::Deserialize, Debug)]
pub struct IssueFormContent {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted and emails will go out shortly!")
}

#[tracing::instrument(name = "Publish issue form submission", 
skip(pool, body),
fields(idempotency_key = body.idempotency_key)
)]
pub async fn publish_issue_form_submission(
    body: web::Form<IssueFormContent>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let IssueFormContent {
        title,
        html_content,
        text_content,
        idempotency_key,
    } = body.0;

    let idempotency_key: IdempotencyKey = idempotency_key.clone().try_into().map_err(e400)?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = HttpResponse::Ok().content_type(ContentType::html()).body(
        r#"
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/htlm; charset=utf-8">
                <title>Newsletter published!</title>
            </head>
            <body>
                <h1>Your issue has been accepted and emails will go out shortly!</h1>
                <p>
                    <a href="/admin/dashboard">&lt;- Dashboard</a>
                    <a href="/admin/newsletters">&lt;- Send new Issue</a>
                </p>
            </body>
        </html>"#,
    );
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(&mut **transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO issue_delivery_queue (
        newsletter_issue_id,
        subscriber_email
    )
    SELECT $1, email
    FROM subscriptions
    WHERE status = 'confirmed'
    "#,
        newsletter_issue_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
