use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

use crate::authentication::UserId;

#[tracing::instrument(name = "Change password form", skip(flash_messages))]
pub async fn change_password_form(
    flash_messages: IncomingFlashMessages,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut error_html = String::new();
    for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/htlm; charset=utf-8">
        <title>Change password</title>
    </head>
    <body>
        {error_html}
        <form action="/admin/password" method="post">
            <label>
                Current password
                <input 
                    type="password" 
                    placeholder="Enter current password" 
                    name="current_password">
            </label>
            <br>
            <label>
                New password
                <input 
                    type="password" 
                    placeholder="Enter new password" 
                    name="new_password">
            </label>
            <br>
            <label>
                Confirm new password
                <input 
                    type="password" 
                    placeholder="Again the new password" 
                    name="new_password_check">
            </label>
            <button type="submit">Change password</button>
        </form>
        <p>
            <a href="/admin/dashboard">&lt;- Back</a>
        </p>
    </body>
</html>"#,
        )))
}
