use config::{Config as ConfigLoader, Environment, File};
use dashmap::DashMap;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::{Any, TypeId, type_name};
use std::sync::Arc;

use crate::error::Error;

/// Trait for configuration structs.
pub trait CogConfig: DeserializeOwned + Default + Send + Sync + 'static {
    const CONFIG_KEY: &'static str;

    /// Optional validation hook called after deserialization during startup.
    ///
    /// Return `Err(message)` to abort startup with a clear diagnostic.
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Metadata registered via inventory for each config type.
pub struct ConfigMeta {
    pub key: &'static str,
    pub type_id_fn: fn() -> TypeId,
    pub type_name: &'static str,
    pub deserialize_fn: fn(&Value) -> Result<Box<dyn Any + Send + Sync>, serde_json::Error>,
    pub validate_fn: fn(&dyn Any) -> Result<(), String>,
}

inventory::collect!(ConfigMeta);

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GearboxAppConfig {
    pub http_port: u16,
    pub log_level: String,
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

pub struct Config {
    raw: Value,
    configs: DashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    app: GearboxAppConfig,
    defaulted_keys: Vec<(&'static str, &'static str)>,
}

impl Config {
    pub fn load() -> Result<Self, Error> {
        let config_path =
            std::env::var("CONFIG_LOCATION").unwrap_or_else(|_| "./config.toml".to_string());

        let mut builder = ConfigLoader::builder();

        if std::path::Path::new(&config_path).exists() {
            builder = builder.add_source(File::with_name(&config_path));
        }

        builder = builder.add_source(
            Environment::with_prefix("GEARBOX")
                .separator("__")
                .try_parsing(true),
        );

        let raw: Value = builder.build()?.try_deserialize()?;

        // Debug: print all GEARBOX env vars
        for (key, value) in std::env::vars() {
            if key.starts_with("GEARBOX") {
                tracing::debug!("env override: {}={}", key, value);
            }
        }
        tracing::debug!("raw config: {:?}", raw);

        Self::from_value(raw)
    }

    /// Build a Config from a pre-built `serde_json::Value`, running inventory
    /// deserialization and validation for all registered `ConfigMeta` types.
    pub fn from_value(raw: Value) -> Result<Self, Error> {
        let app: GearboxAppConfig = raw
            .get("gearbox_app")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| Error::ConfigDeserialize("GearboxAppConfig".into(), Box::new(e)))?
            .unwrap_or_else(|| {
                tracing::debug!("config section 'gearbox_app' not found, using defaults");
                GearboxAppConfig::default()
            });

        let configs = DashMap::new();
        let mut defaulted_keys = Vec::new();

        for meta in inventory::iter::<ConfigMeta> {
            let section = match raw.get(meta.key).cloned() {
                Some(v) => v,
                None => {
                    defaulted_keys.push((meta.key, meta.type_name));
                    tracing::debug!(
                        "config section '{}' ({}) not found, using defaults",
                        meta.key,
                        meta.type_name,
                    );
                    Value::Object(Default::default())
                }
            };
            tracing::debug!("loading config section {:?}", section);
            let config = (meta.deserialize_fn)(&section)
                .map_err(|e| Error::ConfigDeserialize(meta.type_name.to_string(), Box::new(e)))?;
            (meta.validate_fn)(config.as_ref())
                .map_err(|msg| Error::ConfigValidation(meta.type_name.to_string(), msg))?;
            configs.insert((meta.type_id_fn)(), Arc::from(config));
        }

        // Warn about TOML sections that no registered config claims (possible typos)
        if let Some(obj) = raw.as_object() {
            let registered_keys: std::collections::HashSet<&str> =
                inventory::iter::<ConfigMeta>.into_iter().map(|m| m.key).collect();
            for key in obj.keys() {
                if key != "gearbox_app" && !registered_keys.contains(key.as_str()) {
                    tracing::warn!(
                        "config section '{}' found in file but no #[cog_config(\"{}\")] is registered — possible typo?",
                        key, key,
                    );
                }
            }
        }

        Ok(Self {
            raw,
            configs,
            app,
            defaulted_keys,
        })
    }

    /// Insert a typed config directly, bypassing TOML/env loading.
    pub fn insert<C: CogConfig + Clone + 'static>(&self, config: C) {
        self.configs.insert(TypeId::of::<C>(), Arc::new(config));
    }

    /// Override the app config.
    pub(crate) fn set_app(&mut self, app: GearboxAppConfig) {
        self.app = app;
    }

    pub fn get<C: CogConfig + Clone>(&self) -> Result<C, Error> {
        self.configs
            .get(&TypeId::of::<C>())
            .and_then(|v| v.value().downcast_ref::<C>().cloned())
            .ok_or_else(|| Error::ConfigNotFound(type_name::<C>().to_string()))
    }

    pub fn app(&self) -> &GearboxAppConfig {
        &self.app
    }

    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// Returns config sections that fell back to defaults during loading.
    ///
    /// Each entry is `(config_key, type_name)`.
    pub fn defaulted_configs(&self) -> &[(&str, &str)] {
        &self.defaulted_keys
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            raw: Value::Object(Default::default()),
            configs: DashMap::new(),
            app: GearboxAppConfig::default(),
            defaulted_keys: Vec::new(),
        }
    }
}

#[doc(hidden)]
pub fn deserialize_config<T: DeserializeOwned + Send + Sync + 'static>(
    value: &Value,
) -> Result<Box<dyn Any + Send + Sync>, serde_json::Error> {
    Ok(Box::new(serde_json::from_value::<T>(value.clone())?))
}

#[doc(hidden)]
pub fn validate_config<T: CogConfig + 'static>(value: &dyn Any) -> Result<(), String> {
    value
        .downcast_ref::<T>()
        .ok_or_else(|| {
            format!(
                "internal error: config type mismatch for {}",
                type_name::<T>()
            )
        })
        .and_then(|c| c.validate())
}
