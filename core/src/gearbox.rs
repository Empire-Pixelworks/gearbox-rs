use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::config::Config;
use crate::error::Error;
use crate::factory::CogFactory;
use crate::hub::Hub;
use crate::route::RouteRegistration;

pub struct Gearbox {
    hub: Arc<Hub>,
}

impl Gearbox {
    /// Initialize the Gearbox framework.
    ///
    /// This method:
    /// 1. Loads configuration from file and environment
    /// 2. Initializes tracing based on config
    /// 3. Builds all Cogs in dependency order
    /// 4. Returns a ready-to-run Gearbox instance
    pub async fn crank() -> Result<Self, Error> {
        let config = Config::load()
            .map_err(|e| Error::ServerError(format!("Config error: {}", e)))?;

        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| config.app().log_level.clone());
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(&log_level))
            .try_init();

        println!("{:?}", config.app());

        let hub = Arc::new(Hub::new(config));

        let factories: HashMap<TypeId, &'static dyn CogFactory> =
            inventory::iter::<&'static dyn CogFactory>()
                .map(|f| (f.type_id(), *f))
                .collect();

        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
        let mut dependents: HashMap<TypeId, Vec<TypeId>> = HashMap::new();

        for (type_id, factory) in &factories {
            in_degree.entry(*type_id).or_insert(0);
            for dep in factory.deps() {
                if !factories.contains_key(&dep) {
                    return Err(Error::MissingDependency(
                        factory.type_name().to_string(),
                        format!("{:?}", dep),
                    ));
                }
                *in_degree.entry(*type_id).or_insert(0) += 1;
                dependents.entry(dep).or_default().push(*type_id);
            }
        }

        let mut queue: VecDeque<TypeId> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut constructed = 0;

        while let Some(type_id) = queue.pop_front() {
            let factory = factories.get(&type_id).unwrap();

            let cog = factory.build(Arc::clone(&hub)).await?;
            hub.registry.put_any(type_id, cog);

            constructed += 1;

            if let Some(deps) = dependents.get(&type_id) {
                for dep_id in deps {
                    let deg = in_degree.get_mut(dep_id).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(*dep_id);
                    }
                }
            }
        }

        if constructed != factories.len() {
            let stuck: Vec<_> = in_degree
                .iter()
                .filter(|&(_, deg)| *deg > 0)
                .filter_map(|(id, _)| factories.get(id).map(|f| f.type_name()))
                .collect();
            return Err(Error::CyclicDependency(stuck.join(", ")));
        }

        Ok(Self { hub })
    }

    /// Start the HTTP server.
    pub async fn ignite(self) -> Result<(), Error> {
        let port = self.hub.app_config().http_port;

        let mut router = axum::Router::new();
        for route in inventory::iter::<RouteRegistration>() {
            let method_router = (route.handler)();
            router = router.route(route.path, method_router);
        }

        let router = router.with_state(self.hub);

        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| Error::ServerError(e.to_string()))?;

        tracing::info!("Gearbox ignited on http://{}", addr);
        axum::serve(listener, router)
            .await
            .map_err(|e| Error::ServerError(e.to_string()))?;

        Ok(())
    }

    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }
}
