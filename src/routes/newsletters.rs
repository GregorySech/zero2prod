use actix_web::{web, HttpResponse, Responder};

use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{publish_issue, IssueContent},
    email_client::EmailAPIClient,
    utils::e500,
};

#[tracing::instrument(name = "Publish a newsletter issue", skip(body, pool, email_client))]
pub async fn publish_newsletters(
    body: web::Json<IssueContent>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
    _user_id: web::ReqData<UserId>,
) -> Result<impl Responder, actix_web::Error> {
    let body = body.0;
    publish_issue(&body, &email_client, &pool)
        .await
        .map_err(e500)?;
    Ok(HttpResponse::Ok())
}
