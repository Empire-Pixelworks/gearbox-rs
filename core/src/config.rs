use serde::de::DeserializeOwned;

pub trait CogConfig: DeserializeOwned + Default + Send + Sync + 'static {
    const CONFIG_KEY: &'static str;
}

pub struct Config {
    port: u16,
}

impl Config {
    pub fn get<C: CogConfig>(&self) -> C {
        C::default()
    }

    pub fn server_port(&self) -> u16 {
        self.port
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { port: 3000 }
    }
}
