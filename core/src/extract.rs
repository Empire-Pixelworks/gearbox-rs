use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use std::sync::Arc;

use crate::cog::Cog;
use crate::hub::Hub;

/// Extracts a Cog from the Hub registry.
///
/// Use this in handler functions to inject dependencies:
/// ```ignore
/// async fn handler(Inject(repo): Inject<UserRepo>) -> impl IntoResponse {
///     // repo is Arc<UserRepo>
/// }
/// ```
pub struct Inject<T>(pub Arc<T>);

impl<T> std::ops::Deref for Inject<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: Cog + 'static,
    S: Send + Sync,
    Arc<Hub>: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let hub = <Arc<Hub>>::from_ref(state);
        hub.registry
            .get::<T>()
            .map(Inject)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}
