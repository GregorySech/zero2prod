use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials}, routes::get_username, session_state::TypedSession, utils::{e500, see_other}
};

#[derive(serde::Deserialize)]
pub struct ChangePasswordFormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[tracing::instrument(
    name = "Submit change password",
    skip(session, form, pool),
    fields(user_id=tracing::field::Empty)
)]
pub async fn change_password(
    form: web::Form<ChangePasswordFormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id_mb = session.get_user_id().map_err(e500)?;
    let user_id = match user_id_mb {
        None => return Ok(see_other("/login")),
        Some(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            user_id
        }
    };

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            },
            AuthError::UnexpectedError(_) => Err(e500(e)),
        }
    }

    todo!()
}
