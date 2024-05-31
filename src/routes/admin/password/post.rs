use actix_web::{web, HttpResponse};
use secrecy::Secret;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct ChangePasswordFormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[tracing::instrument(
    name = "Submit change password",
    skip(session, _form),
    fields(user_id=tracing::field::Empty)
)]
pub async fn change_password(
    _form: web::Form<ChangePasswordFormData>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id_mb = session.get_user_id().map_err(e500)?;
    match user_id_mb {
        None => return Ok(see_other("/login")),
        Some(user_id) => tracing::Span::current().record("user_id", &tracing::field::display(&user_id)),
    };

    todo!()
}
