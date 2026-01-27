use config::{Config as ConfigLoader, Environment, File};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::any::{type_name, Any, TypeId};
use std::sync::Arc;

/// Trait for configuration structs.
pub trait CogConfig: DeserializeOwned + Default + Send + Sync + 'static {
    const CONFIG_KEY: &'static str;
}

/// Metadata registered via inventory for each config type.
pub struct ConfigMeta {
    pub key: &'static str,
    pub type_id_fn: fn() -> TypeId,
    pub type_name: &'static str,
    pub deserialize_fn: fn(&Value) -> Box<dyn Any + Send + Sync>,
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
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = std::env::var("CONFIG_LOCATION")
            .unwrap_or_else(|_| "./config.toml".to_string());

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
                eprintln!("DEBUG env: {}={}", key, value);
            }
        }
        eprintln!("DEBUG raw config: {:?}", raw);

        let app: GearboxAppConfig = raw
            .get("gearbox_app")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let configs = DashMap::new();
        for meta in inventory::iter::<ConfigMeta> {
            let section = raw.get(meta.key).cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            println!("Section {:?}", section);
            let config = (meta.deserialize_fn)(&section);
            println!("{:?}", config);
            configs.insert((meta.type_id_fn)(), Arc::from(config));
        }

        Ok(Self { raw, configs, app })
    }

    pub fn get<C: CogConfig + Clone>(&self) -> C {
        self.configs
            .get(&TypeId::of::<C>())
            .and_then(|v| v.value().downcast_ref::<C>().cloned())
            .expect(&format!("Config for {} not found!", type_name::<C>()))
    }

    pub fn app(&self) -> &GearboxAppConfig {
        &self.app
    }

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

pub fn deserialize_config<T: DeserializeOwned + Default + Send + Sync + 'static>(
    value: &Value,
) -> Box<dyn Any + Send + Sync> {
    Box::new(serde_json::from_value::<T>(value.clone()).unwrap_or_default())
}
