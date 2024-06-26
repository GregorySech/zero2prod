use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{publish_issue, Content, IssueContent},
    email_client::EmailAPIClient,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
};

#[derive(serde::Deserialize, Debug)]
pub struct IssueFormContent {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
}

#[tracing::instrument(name = "Publish issue form submission", 
skip(pool, email_client, body),
fields(idempotency_key = body.idempotency_key)
)]
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

    let idempotency_key: IdempotencyKey = body.idempotency_key.clone().try_into().map_err(e400)?;

    let transaction = match try_processing(&pool, &idempotency_key, **user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let publish_result = publish_issue(&issue_content, &email_client, &pool).await;

    match publish_result {
        Err(err) => {
            FlashMessage::error("Error while publishing the issue!").send();
            tracing::error!("Error while publishing the issue! {:?}", err);
            Ok(see_other("/admin/newsletters"))
        }
        Ok(_) => {
            let response = HttpResponse::Ok().content_type(ContentType::html()).body(
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
            );
            let response = save_response(transaction, &idempotency_key, **user_id, response)
                .await
                .map_err(e500)?;

            Ok(response)
        }
    }
}
