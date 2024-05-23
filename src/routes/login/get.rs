use actix_web::{http::header::ContentType, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(query: web::Query<QueryParams>) -> HttpResponse {
    let error_html = match query.0.error {
        Some(error_message) => format!("<p><i>{error_message}</i></p>"),
        None => "".into(),
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/htlm; charset=utf-8">
        <title>Login</title>
    </head>
    <body>
        {error_html}
        <form action="/login" method="post">
            <label>
                Username
                <input 
                    type="text" 
                    placeholder="Enter Username" 
                    name="username">
            </label>
            <label>
                Password
                <input 
                    type="password" 
                    placeholder="Enter Password" 
                    name="password">
            </label>

            <button type="submit">Login</button>
        </form>
    </body>
</html>
            "#,
        ))
}
