use argon2::PasswordHasher;
use once_cell::sync::Lazy;
use reqwest::Response;
use sqlx::{Connection, Executor, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry;
use zero2prod::telemetry::init_subscriber;

pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> Response {
        self.api_client
            .post(format!("{}/subscription", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn post_newsletter(&self, body: serde_json::Value) -> Response {
        self.api_client
            .post(format!("{}/newsletters", self.address))
            .json(&body)
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn post_login<T: serde::Serialize>(&self, form: T) -> Response {
        self.api_client
            .post(format!("{}/login", self.address))
            .form(&form)
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Failed to execute Request")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> Response {
        self.api_client
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/change_password", self.address))
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<T: serde::Serialize>(&self, form: T) -> Response {
        self.api_client
            .post(format!("{}/admin/change_password", self.address))
            .form(&form)
            .send()
            .await
            .expect("Failed to execute Request")
    }

    pub async fn post_logout(&self) -> Response {
        self.api_client
            .post(format!("{}/admin/logout", self.address))
            .send()
            .await
            .expect("Failed to execute Request")
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");

        c.database.database_name = uuid::Uuid::new_v4().to_string();

        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application.port());

    _ = tokio::spawn(application.run_until_stopped());

    let test_user = TestUser::generate();

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let app = TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port,
        test_user,
        api_client,
    };

    app.test_user.store(&app.db_pool).await;

    app
}

async fn configure_database(config: &DatabaseSettings) -> sqlx::PgPool {
    let mut connection = sqlx::PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to postgres instance");

    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, config.database_name,).as_str())
        .await
        .expect("Failed to create new database");

    let db_pool = sqlx::PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    db_pool
}

pub struct TestUser {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    fn generate() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, db_pool: &PgPool) {
        let argon2 = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(19 * 1024, 2, 1, None).unwrap(),
        );

        let salt = argon2::password_hash::SaltString::generate(rand::thread_rng());

        let hash = argon2
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap();

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1,$2,$3)",
            self.uuid,
            self.username,
            hash.to_string()
        )
        .execute(db_pool)
        .await
        .expect("Failed to create test user");
    }
}

pub struct ConfirmationLinks {
    pub html_link: reqwest::Url,
    pub plain_link: reqwest::Url,
}

impl ConfirmationLinks {
    pub fn get_confirmation_link(request: &wiremock::Request, port: u16) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let link = links[0].as_str().to_owned();
            let mut link = reqwest::Url::parse(&link).unwrap();
            link.set_port(Some(port)).unwrap();
            //Please do not send stuff outside localhost
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");
            link
        };

        ConfirmationLinks {
            html_link: get_link(body["HtmlBody"].as_str().unwrap()),
            plain_link: get_link(body["TextBody"].as_str().unwrap()),
        }
    }
}

pub fn assert_is_redirect_to(response: &Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers()["Location"], location);
}
