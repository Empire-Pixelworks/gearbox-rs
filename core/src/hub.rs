use crate::config::Config;
use crate::registry::CogRegistry;

pub struct Hub {
    pub registry: CogRegistry,
    pub config: Config,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        Self {
            registry: CogRegistry::new(),
            config,
        }
    }
}
