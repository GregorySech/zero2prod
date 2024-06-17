use anyhow::Context;
use sqlx::PgPool;

use crate::email_client::EmailAPIClient;

use super::get_confirmed_subscribers;

#[derive(serde::Deserialize)]
pub struct IssueContent {
    pub title: String,
    pub content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    pub html: String,
    pub text: String,
}

/// Publishes the issue content to the confirmed subscribers.
#[tracing::instrument(
    name = "Publish issue",
    skip(issue, email_client, pool),
    fields(num_subscribers)
)]
pub async fn publish_issue(
    issue: &IssueContent,
    email_client: &EmailAPIClient,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let subscribers = get_confirmed_subscribers(pool)
        .await
        .context("Failed to retrieve subscribers")?;

    tracing::Span::current().record("num_subscribers", subscribers.len());

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &issue.title,
                        &issue.content.html,
                        &issue.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. Their stored contact details are invalid!");
            }
        }
    }
    Ok(())
}
