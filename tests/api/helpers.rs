use argon2::{password_hash::SaltString, Argon2, Params, PasswordHasher};
use once_cell::sync::Lazy;

use reqwest::{Response, Url};
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub api_client: reqwest::Client,
    pub base_url: Url,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
}

impl TestApp {
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let email_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|link| *link.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_confirmation_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_confirmation_link).unwrap();
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(email_body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(email_body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_subscriptions(&self, body: String) -> Response {
        let expect_body = format!("Failed to execute subscriptions request for body {}.", body);

        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect(&expect_body)
    }

    pub async fn post_newsletters(&self, body: Value) -> Response {
        self.api_client
            .post(&format!("{}/newsletters", self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Request failed!")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute login request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }
}

pub struct TestUser {
    password: String,
    username: String,
    user_id: Uuid,
}

impl TestUser {
    fn generate() -> Self {
        TestUser {
            password: Uuid::new_v4().to_string(),
            username: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());

        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "
        INSERT INTO users (user_id, username, password_hash)
        VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user");
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

pub async fn spawn_app() -> TestApp {
    // Setting up telemetry
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    // Getting configuration.
    let configuration = {
        let mut c = get_configuration().expect("Failed to load configuration.");

        // Prepare db connection (setup + pool)
        // To ensure different dbs for each test!
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;

        c.email_client.api_base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let app =
        Application::build(configuration.clone()).expect("Failed to build the application server");
    let application_port = app.port();
    let address = format!("http://127.0.0.1:{}", application_port);

    // Spawn application.
    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address,
        api_client: client,
        base_url: Url::parse(&configuration.application.base_url).unwrap(),
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port,
        test_user: TestUser::generate(),
    };
    test_app.test_user.store(&test_app.db_pool).await;

    test_app
}

// Set's up telemetry once.
static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);
});

/// Creates a database according to the provided settings using the project's migrations.
async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres DB!");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
