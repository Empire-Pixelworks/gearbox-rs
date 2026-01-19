use axum::routing::MethodRouter;
use std::sync::Arc;

use crate::hub::Hub;

/// Registration for a route handler collected via inventory.
pub struct RouteRegistration {
    pub path: &'static str,
    pub method: &'static str,
    pub handler: fn() -> MethodRouter<Arc<Hub>>,
}

inventory::collect!(RouteRegistration);
