use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    // Login
    let response = app.login_with_test_user().await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Login is successful.
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Logout.
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Logout is comunicated as successful.
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("You have successfully logged out."));

    // Logout is indeed successfult.
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn dashboard_should_contain_link_to_send_newsletter() {
    use scraper::{Html, Selector};

    let app = spawn_app().await;

    // Login
    let response = app.login_with_test_user().await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Login is successful.
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Dashboard contains the send issue link.
    let send_newsletter_text = "Send a newsletter issue";
    assert!(html_page.contains(send_newsletter_text));

    let page_document = Html::parse_document(&html_page);
    let anchor_selector = Selector::parse("a").unwrap();

    let admin_newsletters_links: Vec<String> = page_document
        .select(&anchor_selector)
        .filter(|el| {
            el.attr("href")
                .is_some_and(|href| href == "/admin/newsletters")
        })
        .map(|el| el.inner_html())
        .collect();

    assert!(
        admin_newsletters_links.len() == 1,
        "One send newsletter link."
    );

    let value = admin_newsletters_links.first().unwrap();
    assert_eq!(*value, send_newsletter_text)
}
