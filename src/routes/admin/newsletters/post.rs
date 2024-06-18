use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{publish_issue, Content, IssueContent},
    email_client::EmailAPIClient,
    utils::see_other,
};

#[derive(serde::Deserialize)]
pub struct IssueFormContent {
    title: String,
    html_content: String,
    text_content: String,
}

#[tracing::instrument(name = "Publish issue form submission", skip(body, pool, email_client))]
pub async fn publish_issue_form_submission(
    body: web::Form<IssueFormContent>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailAPIClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let issue_content = IssueContent {
        title: body.title.clone(),
        content: Content {
            html: body.html_content.clone(),
            text: body.text_content.clone(),
        },
    };

    let publish_result = publish_issue(&issue_content, &email_client, &pool).await;

    match publish_result {
        Err(_) => {
            FlashMessage::error("Error while publishing the issue!").send();
            Ok(see_other("/admin/newsletters"))
        }
        Ok(_) => Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
            r#"
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/htlm; charset=utf-8">
                    <title>Newsletter published!</title>
                </head>
                <body>
                    <h1>Your issue has been sent!</h1>
                    <p>
                        <a href="/admin/dashboard">&lt;- Dashboard</a>
                        <a href="/admin/newsletters">&lt;- Send new Issue</a>
                    </p>
                </body>
            </html>"#,
        )),
    }
}
