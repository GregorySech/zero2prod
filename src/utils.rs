use actix_web::HttpResponse;
use reqwest::header::LOCATION;

pub fn e500<ErrorType>(e: ErrorType) -> actix_web::Error
where
    ErrorType: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(route: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, route))
        .finish()
}
