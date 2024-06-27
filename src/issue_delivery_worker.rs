use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::{
    configuration, domain::SubscriberEmail, email_client::EmailAPIClient,
    startup::get_connection_pool,
};

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(skip_all, fields(
    newsletter_issue_id=tracing::field::Empty,
    subscriber_email=tracing::field::Empty
), err)]
pub async fn try_execute_delivery(
    pool: &PgPool,
    email_client: &EmailAPIClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(pool).await?;

    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (transaction, issue_id, email) = task.unwrap();

    Span::current().record("newsletter_issue_id", &display(issue_id));
    Span::current().record("subscriber_email", &display(&email));

    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_newsletter_issue(pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(error.cause_chain = ?e, error.message = %e, "Failed to deliver issue to a confirmed subscriber. Skipping.");
            }
        }
        Err(e) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "Skipping a confirmed subscriber. Stored contact details invalid.");
        }
    }
    delete_task(transaction, issue_id, &email).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

struct NewsletterIssueRecord {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_newsletter_issue(
    pool: &PgPool,
    issue_id: Uuid,
) -> Result<NewsletterIssueRecord, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssueRecord,
        r#"
    SELECT title, text_content, html_content
    FROM newsletter_issues
    WHERE newsletter_issue_id = $1
    "#,
        issue_id
    )
    .fetch_one(pool)
    .await?;
    Ok(issue)
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip(pool))]
async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = pool.begin().await?;

    let r = sqlx::query!(
        r#"
    SELECT newsletter_issue_id, subscriber_email
    FROM issue_delivery_queue
    FOR UPDATE
    SKIP LOCKED
    LIMIT 1
    "#
    )
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id,
            r.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
    "#,
        issue_id,
        email
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn worker_loop(pool: PgPool, email_client: EmailAPIClient) -> Result<(), anyhow::Error> {
    loop {
        let task_outcome = try_execute_delivery(&pool, &email_client).await;
        match task_outcome {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn run_worker_until_stopped(
    configuration: configuration::Settings,
) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);

    let email_client = configuration.email_client.client();

    worker_loop(connection_pool, email_client).await
}
