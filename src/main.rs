use std::net::TcpListener;

use secrecy::Secret;
use sqlx::postgres::PgPoolOptions;

use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailAPIClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Setting up telemetry!
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

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

    run(listener, connection, email_client)?.await
}
