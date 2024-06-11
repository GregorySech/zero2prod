use std::ops::Deref;

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::InternalError,
    http::header,
    web, FromRequest, HttpMessage, HttpResponse, ResponseError,
};
use actix_web_lab::middleware::Next;
use reqwest::{header::HeaderValue, StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    routes::error_chain_fmt,
    session_state::TypedSession,
    utils::{e500, see_other},
};

use super::{get_basic_authentication_credentials, validate_credentials, AuthError};

/// Tag type for Uuid that model UserIds.
#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

#[derive(thiserror::Error)]
pub enum BasicAuthError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
    #[error("Authentication failed")]
    Unauthorized(#[source] anyhow::Error),
}

impl std::fmt::Debug for BasicAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for BasicAuthError {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            BasicAuthError::Unexpected(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            BasicAuthError::Unauthorized(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

/// Rejects users that are not authenticated using session-based authentication.
pub async fn users_session_authentication(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_req, payload) = req.parts_mut();
        TypedSession::from_request(http_req, payload).await
    }?;
    let user_id_mb = session.get_user_id().map_err(e500)?;
    match user_id_mb {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            next.call(req).await
        }
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}

/// Rejects users that are not authenticated using basic authentication.
pub async fn users_basic_authentication(
    pool: web::Data<PgPool>,
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let headers = req.headers();
    let credentials =
        get_basic_authentication_credentials(headers).map_err(BasicAuthError::Unauthorized)?;

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => BasicAuthError::Unauthorized(e.into()),
            AuthError::UnexpectedError(_) => BasicAuthError::Unexpected(e.into()),
        })?;

    req.extensions_mut().insert(UserId(user_id));
    next.call(req).await
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
