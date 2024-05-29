use actix_web::{http::header::ContentType, web, HttpResponse};
use anyhow::Context;
use reqwest::header::LOCATION;
use sqlx::PgPool;
use tracing::field::display;
use uuid::Uuid;

use crate::session_state::TypedSession;

fn e500<ErrorType>(e: ErrorType) -> actix_web::Error
where
    ErrorType: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(
    name = "Admin dashboard", 
    skip(session, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        tracing::Span::current().record("user_id", &display(&user_id));
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };
    tracing::Span::current().record("username", &display(&username));
    let body = format!(
        r#"
    <!DOCTYPE html>
    <html lang="en">
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
        </head>

        <body>
            <p>Welcome {username}</p>
        </body>
    </html>
    "#
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed query to retrieve username.")?;

    Ok(row.username)
}
