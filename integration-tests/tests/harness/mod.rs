//! Test harness for integration tests.
//!
//! Provides TestContext that sets up:
//! - Embedded PostgreSQL instance
//! - Database migrations
//! - HTTP server running in background
//! - HTTP client for making requests

use postgresql_embedded::{PostgreSQL, Settings};
use reqwest::Client;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;

static PORT_COUNTER: AtomicU16 = AtomicU16::new(19000);
static PG_INSTANCE: OnceCell<PostgreSQL> = OnceCell::const_new();

/// RAII guard that restores an environment variable to its previous value on drop.
struct EnvGuard {
    key: String,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Tests run single-threaded via RUST_TEST_THREADS=1 in .cargo/config.toml
        unsafe { std::env::set_var(key, value) };
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: Tests run single-threaded via RUST_TEST_THREADS=1 in .cargo/config.toml
        unsafe {
            if let Some(v) = &self.prev {
                std::env::set_var(&self.key, v);
            } else {
                std::env::remove_var(&self.key);
            }
        }
    }
}

/// Test context containing HTTP client and server URL.
pub struct TestContext {
    pub client: Client,
    pub base_url: String,
    _server_handle: JoinHandle<()>,
    _env_guards: Vec<EnvGuard>,
}

impl TestContext {
    /// Set up a new test context with isolated database.
    pub async fn setup() -> Self {
        // Get or initialize shared PostgreSQL instance
        let pg = PG_INSTANCE
            .get_or_init(|| async {
                let settings = Settings::default();
                let mut pg = PostgreSQL::new(settings);
                pg.setup().await.expect("Failed to setup PostgreSQL");
                pg.start().await.expect("Failed to start PostgreSQL");
                pg
            })
            .await;

        // Create a unique database for this test
        let db_name = format!("test_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        pg.create_database(&db_name)
            .await
            .expect("Failed to create test database");

        // Get connection URL
        let pg_settings = pg.settings();
        let db_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            pg_settings.username, pg_settings.password, pg_settings.host, pg_settings.port, db_name
        );

        // Run migrations
        run_migrations(&db_url).await;

        // Pick a unique port for this test's HTTP server
        let http_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let base_url = format!("http://127.0.0.1:{}", http_port);

        // Set environment variables for Gearbox config (restored on TestContext drop)
        let env_guards = vec![
            EnvGuard::set("GEARBOX_POSTGRES__DATABASE_URL", &db_url),
            EnvGuard::set("GEARBOX_GEARBOX_APP__HTTP_PORT", &http_port.to_string()),
            EnvGuard::set("CONFIG_LOCATION", "/nonexistent/config.toml"),
        ];

        // Start server in background
        let server_handle = tokio::spawn(async move {
            // Import and run the app
            // Note: In a real setup, you'd import from the example crate
            if let Err(e) = run_server(http_port, &db_url).await {
                eprintln!("Server error: {}", e);
            }
        });

        // Wait for server to be ready
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        wait_for_server(&client, &base_url).await;

        Self {
            client,
            base_url,
            _server_handle: server_handle,
            _env_guards: env_guards,
        }
    }

    /// Build full URL for a path.
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

async fn run_migrations(db_url: &str) {
    let pool = sqlx::PgPool::connect(db_url)
        .await
        .expect("Failed to connect to database");

    // Create tables
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            role VARCHAR(50) NOT NULL DEFAULT 'user',
            active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS posts (
            id UUID PRIMARY KEY,
            author_id UUID NOT NULL REFERENCES users(id),
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            published BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
        CREATE INDEX IF NOT EXISTS idx_posts_author ON posts(author_id);
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to run migrations");

    pool.close().await;
}

async fn run_server(
    _port: u16,
    _db_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use gearbox_rs::Gearbox;

    // The Gearbox::crank() will read from env vars we set
    let gearbox = Gearbox::crank().await?;
    gearbox.ignite().await?;
    Ok(())
}

async fn wait_for_server(client: &Client, base_url: &str) {
    let health_url = format!("{}/users", base_url);

    for _ in 0..50 {
        match client.get(&health_url).send().await {
            Ok(_) => return,
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }

    panic!("Server failed to start within 5 seconds");
}
