use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::error::Error;
use crate::factory::CogFactory;
use crate::hub::Hub;
use crate::route::RouteRegistration;

/// Compute the initialization order for a set of factories using topological sort
/// (Kahn's algorithm).
///
/// Builds an in-degree map from each factory's declared dependencies, then
/// iteratively processes factories whose dependencies have all been satisfied.
///
/// Returns the ordered [`TypeId`]s, or:
/// - [`Error::MissingDependency`] if a factory references an unregistered type.
/// - [`Error::CyclicDependency`] if the graph contains a cycle.
pub fn resolve_init_order(
    factories: &HashMap<TypeId, &dyn CogFactory>,
) -> Result<Vec<TypeId>, Error> {
    let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
    let mut dependents: HashMap<TypeId, Vec<TypeId>> = HashMap::new();

    for (type_id, factory) in factories {
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

    let mut init_order: Vec<TypeId> = Vec::new();

    while let Some(type_id) = queue.pop_front() {
        init_order.push(type_id);

        if let Some(deps) = dependents.get(&type_id) {
            for dep_id in deps {
                if let Some(deg) = in_degree.get_mut(dep_id) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(*dep_id);
                    }
                }
            }
        }
    }

    if init_order.len() != factories.len() {
        let stuck: Vec<_> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg > 0)
            .filter_map(|(id, _)| factories.get(id).map(|f| f.type_name()))
            .collect();
        return Err(Error::CyclicDependency(stuck.join(", ")));
    }

    Ok(init_order)
}

/// The main entry point for running a Gearbox application.
///
/// Typical usage:
///
/// ```ignore
/// Gearbox::crank().await?.ignite().await
/// ```
///
/// [`crank`](Gearbox::crank) loads configuration, resolves dependencies,
/// builds all Cogs, and calls their `on_start` hooks.
/// [`router_with`](Gearbox::router_with) optionally adds middleware layers.
/// [`ignite`](Gearbox::ignite) starts the HTTP server and handles graceful shutdown.
pub struct Gearbox {
    hub: Arc<Hub>,
    factories: HashMap<TypeId, &'static dyn CogFactory>,
    init_order: Vec<TypeId>,
    router_hooks: Vec<Box<dyn FnOnce(axum::Router) -> axum::Router + Send>>,
}

impl Gearbox {
    /// Initialize the Gearbox framework.
    ///
    /// This method:
    /// 1. Loads configuration from file and environment
    /// 2. Initializes tracing based on config
    /// 3. Builds all Cogs in dependency order
    /// 4. Calls `on_start` on each Cog in initialization order
    /// 5. Returns a ready-to-run Gearbox instance
    pub async fn crank() -> Result<Self, Error> {
        let config = Config::load()?;

        // Log summary of which configs defaulted
        let defaulted = config.defaulted_configs();
        if defaulted.is_empty() {
            tracing::info!("all registered configs loaded from file/env");
        } else {
            for (key, type_name) in defaulted {
                tracing::warn!(
                    "config '{}' ({}) not found in file/env — using defaults",
                    key, type_name,
                );
            }
        }

        let log_level =
            std::env::var("RUST_LOG").unwrap_or_else(|_| config.app().log_level.clone());
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(&log_level))
            .try_init();

        tracing::debug!("app config: {:?}", config.app());

        let hub = Arc::new(Hub::new(config));

        let factories: HashMap<TypeId, &'static dyn CogFactory> =
            inventory::iter::<&'static dyn CogFactory>()
                .map(|f| (f.type_id(), *f))
                .collect();

        // Cast to &dyn CogFactory for the shared resolve function
        let factory_refs: HashMap<TypeId, &dyn CogFactory> = factories
            .iter()
            .map(|(&id, &f)| (id, f as &dyn CogFactory))
            .collect();
        let init_order = resolve_init_order(&factory_refs)?;

        // Build cogs in resolved order
        for &type_id in &init_order {
            let factory = factories.get(&type_id).ok_or_else(|| {
                Error::CogNotFound(format!("{:?}", type_id))
            })?;
            let cog = factory.build(Arc::clone(&hub)).await?;
            hub.registry.put_any(type_id, cog);
        }

        // Call on_start for each cog in initialization order
        for type_id in &init_order {
            let factory = *factories.get(type_id).ok_or_else(|| {
                Error::CogNotFound(format!("{:?}", type_id))
            })?;
            if let Some(cog) = hub.registry.get_any(type_id) {
                factory.on_start(cog).await?;
            }
        }

        Ok(Self {
            hub,
            factories,
            init_order,
            router_hooks: Vec::new(),
        })
    }

    /// Apply a transformation to the router before the server starts.
    ///
    /// Use this to add middleware layers, fallback handlers, or nest sub-routers.
    /// The closure receives the router after state has been applied, so the type
    /// is `Router<()>` (standard axum router).
    ///
    /// Can be called multiple times — hooks are applied in registration order.
    ///
    /// # Example
    /// ```ignore
    /// use tower_http::cors::CorsLayer;
    ///
    /// Gearbox::crank().await?
    ///     .router_with(|router| router.layer(CorsLayer::permissive()))
    ///     .ignite().await
    /// ```
    pub fn router_with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(axum::Router) -> axum::Router + Send + 'static,
    {
        self.router_hooks.push(Box::new(f));
        self
    }

    /// Start the HTTP server.
    ///
    /// Builds the router from all registered routes, applies any middleware hook,
    /// and serves until a shutdown signal (Ctrl+C or SIGTERM) is received.
    /// On shutdown, calls `on_shutdown` on each Cog in reverse initialization order.
    pub async fn ignite(self) -> Result<(), Error> {
        let port = self.hub.app_config().http_port;

        let mut router = axum::Router::new();
        for route in inventory::iter::<RouteRegistration>() {
            let method_router = (route.handler)();
            router = router.route(route.path, method_router);
        }

        let router = router.with_state(Arc::clone(&self.hub));

        let router = self.router_hooks.into_iter().fold(router, |r, hook| hook(r));

        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| Error::TcpBind(addr.clone(), e))?;

        tracing::info!("Gearbox ignited on http://{}", addr);
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(Error::Serve)?;

        // Call on_shutdown for each cog in reverse initialization order
        tracing::info!("Shutting down...");
        for type_id in self.init_order.iter().rev() {
            if let (Some(factory), Some(cog)) = (self.factories.get(type_id), self.hub.registry.get_any(type_id)) {
                match tokio::time::timeout(
                    Duration::from_secs(30),
                    factory.on_shutdown(cog),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        tracing::error!(
                            "shutdown error for {}: {}",
                            factory.type_name(),
                            e
                        );
                    }
                    Err(_) => {
                        tracing::error!(
                            "shutdown timed out for {}",
                            factory.type_name()
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns a reference to the [`Hub`], useful for testing or inspection.
    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!("failed to install SIGTERM handler: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
