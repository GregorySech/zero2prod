use actix_web::{http::header::ContentType, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use tracing::field::display;
use uuid::Uuid;

use crate::{authentication::UserId, utils::e500};

#[tracing::instrument(
    name = "Admin dashboard", 
    skip(pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    tracing::Span::current().record("user_id", &display(&user_id));
    let username = get_username(*user_id, &pool).await.map_err(e500)?;
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
            <p>Available action:</p>
            <ol>
                <li>
                    <a href="/admin/password">Change password</a>
                </li>
                <li>
                    <form name="logoutForm" action="/admin/logout" method="post">
                        <input type="submit" value="logout">
                    </form>
                </li>
            </ol>
        </body>
    </html>
    "#
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
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
