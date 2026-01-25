use config::{Config as ConfigLoader, Environment, File};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// Trait for configuration structs.
///
/// Implement this trait (usually via `#[cog_config("key")]` macro) to make
/// a struct loadable from configuration files and environment variables.
pub trait CogConfig: DeserializeOwned + Default + Send + Sync + 'static {
    /// The configuration section key (e.g., "database" loads from `[database]` section)
    const CONFIG_KEY: &'static str;
}

/// Metadata registered via inventory for each config type.
///
/// This enables auto-discovery of all config types at startup.
pub struct ConfigMeta {
    pub key: &'static str,
    pub type_id_fn: fn() -> TypeId,
    pub type_name: &'static str,
    pub deserialize_fn: fn(&Value) -> Box<dyn Any + Send + Sync>,
}

inventory::collect!(ConfigMeta);

/// Framework-level configuration for Gearbox itself.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GearboxAppConfig {
    /// HTTP server port (default: 8080)
    #[serde(alias = "httpport")]
    pub http_port: u16,
    /// Log level (default: "info")
    #[serde(alias = "loglevel")]
    pub log_level: String,
    /// Application name (default: "gearbox-app")
    #[serde(alias = "appname")]
    pub app_name: String,
}

impl Default for GearboxAppConfig {
    fn default() -> Self {
        Self {
            http_port: 8080,
            log_level: "info".to_string(),
            app_name: "gearbox-app".to_string(),
        }
    }
}

/// Main configuration container.
///
/// Loads configuration from file and environment variables at startup,
/// then provides typed access to configuration sections.
pub struct Config {
    raw: Value,
    configs: DashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    app: GearboxAppConfig,
}

impl Config {
    /// Load configuration from file and environment.
    ///
    /// # Sources (in order of precedence, later overrides earlier):
    /// 1. Config file (default: `config.toml`, override with `CONFIG_LOCATION` env var)
    /// 2. Environment variables with `GEARBOX_` prefix
    ///
    /// # Environment Variable Format
    /// - Prefix: `GEARBOX_`
    /// - Section and key separated by single underscore
    /// - Keys normalized (no dashes/underscores, case-insensitive)
    ///
    /// Examples:
    /// - `GEARBOX_DATABASE_URL` -> `database.url`
    /// - `GEARBOX_DATABASE_MAXCONNECTIONS` -> `database.max_connections`
    /// - `GEARBOX_GEARBOXAPP_HTTPPORT` -> `gearbox-app.http_port`
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = std::env::var("CONFIG_LOCATION")
            .unwrap_or_else(|_| "config.toml".to_string());

        let mut builder = ConfigLoader::builder();

        // Add file source if exists
        if std::path::Path::new(&config_path).exists() {
            builder = builder.add_source(File::with_name(&config_path));
        }

        // Add environment source with GEARBOX_ prefix
        builder = builder.add_source(
            Environment::with_prefix("GEARBOX")
                .separator("_")
                .try_parsing(true),
        );

        let settings = builder.build()?;

        // Convert to JSON Value for manipulation
        let raw: Value = settings.try_deserialize()?;

        // Load GearboxAppConfig with relaxed binding
        let app = load_section_relaxed::<GearboxAppConfig>(&raw, "gearbox-app");

        // Load all registered configs via inventory
        let configs = DashMap::new();
        for meta in inventory::iter::<ConfigMeta> {
            let section = extract_section(&raw, meta.key);
            let normalized = normalize_object(&section);
            let config = (meta.deserialize_fn)(&normalized);
            let type_id = (meta.type_id_fn)();
            configs.insert(type_id, Arc::from(config));
        }

        Ok(Self { raw, configs, app })
    }

    /// Get a configuration by type.
    ///
    /// Returns the loaded configuration, or `Default::default()` if the
    /// config type wasn't registered or the section was missing.
    pub fn get<C: CogConfig + Clone>(&self) -> C {
        self.configs
            .get(&TypeId::of::<C>())
            .and_then(|v| v.value().downcast_ref::<C>().cloned())
            .unwrap_or_default()
    }

    /// Get the framework app config.
    pub fn app(&self) -> &GearboxAppConfig {
        &self.app
    }

    /// Get the raw configuration value (for debugging/introspection).
    pub fn raw(&self) -> &Value {
        &self.raw
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            raw: Value::Object(Default::default()),
            configs: DashMap::new(),
            app: GearboxAppConfig::default(),
        }
    }
}

/// Normalize a key for relaxed binding: remove `-` and `_`, lowercase.
///
/// This allows all these forms to match `max_connections`:
/// - `max-connections` (TOML kebab-case)
/// - `max_connections` (TOML/Rust snake_case)
/// - `MAXCONNECTIONS` (env var normalized)
fn normalize_key(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Normalize all keys in a JSON object recursively.
fn normalize_object(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let normalized: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (normalize_key(k), normalize_object(v)))
                .collect();
            Value::Object(normalized)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_object).collect()),
        other => other.clone(),
    }
}

/// Extract a section from the raw config, trying both original and normalized keys.
fn extract_section(raw: &Value, section_key: &str) -> Value {
    // Try original key first
    if let Some(section) = raw.get(section_key) {
        return section.clone();
    }

    // Try normalized key
    let normalized_key = normalize_key(section_key);
    if let Value::Object(map) = raw {
        for (k, v) in map {
            if normalize_key(k) == normalized_key {
                return v.clone();
            }
        }
    }

    Value::Object(Default::default())
}

/// Load a section with relaxed binding.
fn load_section_relaxed<T: DeserializeOwned + Default>(raw: &Value, section_key: &str) -> T {
    let section = extract_section(raw, section_key);
    let normalized = normalize_object(&section);
    serde_json::from_value(normalized).unwrap_or_default()
}

/// Deserialize a config type from a normalized Value.
///
/// This is used by the generated ConfigMeta to deserialize each config type.
pub fn deserialize_config<T: DeserializeOwned + Default + Send + Sync + 'static>(
    value: &Value,
) -> Box<dyn Any + Send + Sync> {
    let config: T = serde_json::from_value(value.clone()).unwrap_or_default();
    Box::new(config)
}
