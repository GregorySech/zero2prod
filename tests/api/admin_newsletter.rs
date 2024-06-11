use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn unauthenticated_request_to_send_newsletter_should_bounce() {
    let app = spawn_app().await;

    let response = app.get_admin_send_newsletters().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn send_newsletter_form_has_title_and_content_inputs() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    let html_body = app.get_admin_send_newsletters_html().await;

    let title_selector = scraper::Selector::parse(r#"form input[name="title"]"#).unwrap();

    let html_content_selector =
        scraper::Selector::parse(r#"form textarea[name="html_content"]"#).unwrap();

    let text_content_selector =
        scraper::Selector::parse(r#"form textarea[name="text_content"]"#).unwrap();

    let html_doc = scraper::Html::parse_document(&html_body);

    assert!(html_doc.select(&title_selector).count() == 1);
    assert!(html_doc.select(&html_content_selector).count() == 1);
    assert!(html_doc.select(&text_content_selector).count() == 1);
}
