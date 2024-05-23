use actix_web::{web, HttpResponse, ResponseError};
use reqwest::{header::LOCATION, StatusCode};
use secrecy::Secret;
use sqlx::{encode, PgPool};

use crate::{
    authentication::{validate_credentials, Credentials},
    routes::error_chain_fmt,
};

#[derive(serde::Deserialize)]
pub struct LoginFormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<LoginFormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            crate::authentication::AuthError::InvalidCredentials(_) => {
                LoginError::AuthError(e.into())
            }
            crate::authentication::AuthError::UnexpectedError(_) => {
                LoginError::UnexpectedError(e.into())
            }
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={}", encoded_error)))
            .finish()
    }

    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
