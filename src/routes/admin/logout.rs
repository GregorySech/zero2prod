use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;
use tracing::field::display;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[tracing::instrument(
    name = "Log out",
    skip(session),
    fields(user_id=tracing::field::Empty)
)]
pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    let user_id_mb = session.get_user_id().map_err(e500)?;

    if let Some(user_id) = user_id_mb {
        tracing::Span::current().record("user_id", &display(user_id));
        session.log_out();
        FlashMessage::info("You have successfully logged out.").send()
    }

    Ok(see_other("/login"))
}
