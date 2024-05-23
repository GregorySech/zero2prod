use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailAPIClient,
    routes::{confirm, health_check, home, login, login_form, publish_newsletters, subscribe},
};
use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use secrecy::Secret;
use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres};
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email!");

        let timeout = configuration.email_client.timeout();
        let email_client = EmailAPIClient::new(
            configuration.email_client.api_base_url,
            sender_email,
            Secret::new(configuration.email_client.authorization_token),
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr()?.port();

        let server = run(
            listener,
            connection,
            email_client,
            configuration.application.base_url,
        )?;

        Ok(Self { port, server })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> Pool<Postgres> {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub struct ApplicationBaseUrl(pub String);

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailAPIClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/health_check", web::get().to(health_check))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/newsletters", web::post().to(publish_newsletters))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
