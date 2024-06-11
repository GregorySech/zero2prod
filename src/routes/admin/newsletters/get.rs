use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

#[tracing::instrument(name = "Change password form", skip(flash_messages))]
pub async fn send_newsletter_form(
    flash_messages: IncomingFlashMessages,
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
        <title>Send newsletter</title>
    </head>
    <body>
        <h1>Send Newsletter</h1>
        {error_html}
        <form action="/admin/newsletters" method="post">
            <label for="html_content">
                HTML content
            </label> <br>
            <textarea 
                    id="html_content"
                    name="html_content">
                </textarea>
            <br>
            <label for="text_content">
                Text content
            </label> <br>
            <textarea 
                id="text_content"
                name="text_content">
            </textarea>
            <br>
            <button type="submit">Send Newsletter</button>
        </form>
        <p>
            <a href="/admin/dashboard">&lt;- Back</a>
        </p>
    </body>
</html>"#,
        )))
}
