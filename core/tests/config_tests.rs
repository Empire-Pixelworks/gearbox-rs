//! Integration tests for the Config module.

use gearbox_rs_core::{CogConfig, Config, ConfigMeta, deserialize_config, validate_config};
use serde::Deserialize;
use std::any::TypeId;
use std::env;
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use tempfile::TempDir;

static CONFIG_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard for environment variable cleanup.
struct EnvGuard {
    key: String,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let prev = env::var(key).ok();
        // SAFETY: Tests run single-threaded via --test-threads=1
        unsafe { env::set_var(key, value) };
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: Tests run single-threaded via --test-threads=1
        unsafe {
            if let Some(v) = &self.prev {
                env::set_var(&self.key, v);
            } else {
                env::remove_var(&self.key);
            }
        }
    }
}

fn setup_config_file(toml_content: &str) -> (TempDir, EnvGuard) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let path = dir.path().join("config.toml");
    let mut file = fs::File::create(&path).expect("Failed to create config file");
    file.write_all(toml_content.as_bytes())
        .expect("Failed to write config");
    drop(file);
    let guard = EnvGuard::set("CONFIG_LOCATION", path.to_str().unwrap());
    (dir, guard)
}

#[test]
fn test_load_from_toml_file() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[gearbox_app]
http_port = 9000
log_level = "debug"
app_name = "test-app"
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    assert_eq!(config.app().http_port, 9000);
    assert_eq!(config.app().log_level, "debug");
    assert_eq!(config.app().app_name, "test-app");
}

#[test]
fn test_snake_case_keys() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[gearbox_app]
http_port = 9002
log_level = "error"
app_name = "snake_app"
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    assert_eq!(config.app().http_port, 9002);
    assert_eq!(config.app().log_level, "error");
    assert_eq!(config.app().app_name, "snake_app");
}

#[test]
fn test_missing_section_uses_default() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = "# Empty config\n";

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    assert_eq!(config.app().http_port, 8080);
    assert_eq!(config.app().log_level, "info");
    assert_eq!(config.app().app_name, "gearbox-app");
}

#[test]
fn test_partial_section() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[gearbox_app]
http_port = 3000
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    assert_eq!(config.app().http_port, 3000);
    assert_eq!(config.app().log_level, "info");
    assert_eq!(config.app().app_name, "gearbox-app");
}

#[test]
fn test_raw_value_access() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[custom-section]
key = "value"
number = 42
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let raw = config.raw();

    assert!(raw.is_object());
    let section = raw.get("custom-section").expect("Section not found");
    assert_eq!(section.get("key").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(section.get("number").and_then(|v| v.as_i64()), Some(42));
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct TestDatabaseConfig {
    #[serde(default)]
    url: String,
    #[serde(default = "default_max_conn")]
    max_connections: u32,
}

fn default_max_conn() -> u32 {
    10
}

impl CogConfig for TestDatabaseConfig {
    const CONFIG_KEY: &'static str = "database";
}

fn test_db_type_id() -> TypeId {
    TypeId::of::<TestDatabaseConfig>()
}

inventory::submit! {
    ConfigMeta {
        key: "database",
        type_id_fn: test_db_type_id,
        type_name: "TestDatabaseConfig",
        deserialize_fn: deserialize_config::<TestDatabaseConfig>,
        validate_fn: validate_config::<TestDatabaseConfig>,
    }
}

#[test]
fn test_registered_config_via_inventory() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[database]
url = "postgres://localhost/testdb"
max_connections = 20
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let db_config: TestDatabaseConfig = config.get().unwrap();

    assert_eq!(db_config.url, "postgres://localhost/testdb");
    assert_eq!(db_config.max_connections, 20);
}

#[test]
fn test_registered_config_uses_defaults_when_missing() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[gearbox_app]
http_port = 8080
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let db_config: TestDatabaseConfig = config.get().unwrap();

    assert_eq!(db_config.url, "");
    assert_eq!(db_config.max_connections, 10);
}

#[test]
fn test_registered_config_partial_section() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[database]
url = "mysql://localhost/mydb"
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let db_config: TestDatabaseConfig = config.get().unwrap();

    assert_eq!(db_config.url, "mysql://localhost/mydb");
    assert_eq!(db_config.max_connections, 10);
}

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.app().http_port, 8080);
    assert_eq!(config.app().log_level, "info");
}

#[test]
fn test_defaulted_configs_tracked() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    // No [database] section → TestDatabaseConfig should appear in defaulted list
    let toml = r#"
[gearbox_app]
http_port = 8080
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let defaulted = config.defaulted_configs();
    assert!(
        defaulted.iter().any(|(key, _)| *key == "database"),
        "expected 'database' in defaulted configs, got: {:?}",
        defaulted,
    );
}

#[test]
fn test_no_defaulted_configs_when_section_present() {
    let _lock = CONFIG_TEST_MUTEX.lock().unwrap();
    let toml = r#"
[database]
url = "postgres://localhost/testdb"
"#;

    let (_dir, _guard) = setup_config_file(toml);
    let config = Config::load().expect("Failed to load config");
    let defaulted = config.defaulted_configs();
    assert!(
        !defaulted.iter().any(|(key, _)| *key == "database"),
        "'database' should not be in defaulted configs when section is present",
    );
}

// -- Validation tests --

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
struct ValidatedConfig {
    port: u16,
}

impl Default for ValidatedConfig {
    fn default() -> Self {
        // Default is valid so it doesn't break other tests when the section is missing
        Self { port: 8080 }
    }
}

impl CogConfig for ValidatedConfig {
    const CONFIG_KEY: &'static str = "validated";

    fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("port must not be zero".to_string());
        }
        Ok(())
    }
}

fn validated_type_id() -> TypeId {
    TypeId::of::<ValidatedConfig>()
}

inventory::submit! {
    ConfigMeta {
        key: "validated",
        type_id_fn: validated_type_id,
        type_name: "ValidatedConfig",
        deserialize_fn: deserialize_config::<ValidatedConfig>,
        validate_fn: validate_config::<ValidatedConfig>,
    }
}

#[test]
fn test_validation_fails_on_invalid_value() {
    // Explicitly set port = 0 which validate() rejects
    let raw = serde_json::json!({
        "database": { "url": "postgres://localhost" },
        "validated": { "port": 0 }
    });
    let result = Config::from_value(raw);
    assert!(result.is_err(), "expected validation error");
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Config validation failed"),
        "unexpected error: {}",
        err,
    );
    assert!(
        err.contains("port must not be zero"),
        "unexpected error: {}",
        err,
    );
}

#[test]
fn test_validation_passes_when_valid() {
    let raw = serde_json::json!({
        "database": { "url": "postgres://localhost" },
        "validated": { "port": 3000 }
    });
    let config = Config::from_value(raw).expect("validation should pass");
    let v: ValidatedConfig = config.get().unwrap();
    assert_eq!(v.port, 3000);
}

#[test]
fn test_validation_passes_with_defaults() {
    // No [validated] section → uses Default (port=8080) → validate() passes
    let raw = serde_json::json!({
        "database": { "url": "postgres://localhost" }
    });
    let config = Config::from_value(raw).expect("default should pass validation");
    let v: ValidatedConfig = config.get().unwrap();
    assert_eq!(v.port, 8080);
}

#[test]
fn test_unclaimed_section_does_not_cause_error() {
    // A section with no matching ConfigMeta should produce a warning but not an error
    let raw = serde_json::json!({
        "database": { "url": "postgres://localhost" },
        "validated": { "port": 8080 },
        "typo_section": { "key": "value" }
    });
    let config = Config::from_value(raw);
    assert!(config.is_ok(), "unclaimed section should not cause an error");
}
