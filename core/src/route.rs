use axum::routing::MethodRouter;
use std::sync::Arc;

use crate::hub::Hub;

/// A single route registration collected via [`inventory`].
///
/// Each route macro (`#[get]`, `#[post]`, etc.) emits an `inventory::submit!`
/// call that creates one of these. During [`Gearbox::ignite`](crate::Gearbox::ignite),
/// all collected registrations are merged into the axum [`Router`](axum::Router).
pub struct RouteRegistration {
    /// The URL path pattern (e.g. `"/users/{id}"`).
    pub path: &'static str,
    /// The HTTP method (e.g. `"GET"`).
    pub method: &'static str,
    /// Returns a [`MethodRouter`] that captures the actual handler function.
    pub handler: fn() -> MethodRouter<Arc<Hub>>,
}

inventory::collect!(RouteRegistration);
