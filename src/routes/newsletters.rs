use actix_web::{web, HttpResponse, Responder};

use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{publish_issue, Content, IssueContent},
    email_client::EmailAPIClient,
    idempotency::IdempotencyKey,
    utils::{e400, e500},
};

#[derive(serde::Deserialize)]
pub struct SendIssueContent {
    title: String,
    content: Content,
    idempotency_key: String,
}

impl From<SendIssueContent> for IssueContent {
    fn from(val: SendIssueContent) -> Self {
        IssueContent {
            title: val.title,
            content: val.content,
        }
    }
}

#[tracing::instrument(name = "Publish a newsletter issue", skip(body, pool, email_client))]
pub async fn publish_newsletters(
    body: web::Json<SendIssueContent>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
    _user_id: web::ReqData<UserId>,
) -> Result<impl Responder, actix_web::Error> {
    let _idempotency_key: IdempotencyKey =
        body.0.idempotency_key.clone().try_into().map_err(e400)?;

    publish_issue(&body.0.into(), &email_client, &pool)
        .await
        .map_err(e500)?;
    Ok(HttpResponse::Ok())
}
