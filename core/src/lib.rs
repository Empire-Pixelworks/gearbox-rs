//! Core library for the Gearbox web framework.
//!
//! Provides dependency injection, configuration loading, HTTP routing, and lifecycle
//! management for axum-based applications. Services are called **Cogs** — see the
//! [`Cog`] trait and [`Gearbox`] struct to get started.
//!
//! Most applications use the `gearbox-rs` facade crate and its prelude rather than
//! depending on this crate directly.
//!
//! Enable the `testing` feature for [`TestHubBuilder`], which lets you construct a
//! [`Hub`] with hand-picked services for unit and integration tests.

mod cog;
mod config;
pub mod crud;
mod error;
mod extract;
mod factory;
mod gearbox;
mod hub;
mod registry;
mod route;
#[cfg(feature = "testing")]
mod testing;
#[doc(hidden)]
pub use async_trait::async_trait;
#[doc(hidden)]
pub use inventory;

pub use cog::Cog;
pub use config::{CogConfig, Config, GearboxAppConfig};
#[doc(hidden)]
pub use config::{ConfigMeta, deserialize_config, validate_config};
pub use error::Error;
pub use extract::Inject;
#[doc(hidden)]
pub use factory::{BoxFuture, CogFactory};
pub use gearbox::Gearbox;
pub use hub::Hub;
#[doc(hidden)]
pub use registry::CogRegistry;
#[doc(hidden)]
pub use route::RouteRegistration;

#[cfg(feature = "testing")]
pub use testing::TestHubBuilder;
#[cfg(feature = "testing")]
pub use gearbox::resolve_init_order;

pub use axum::extract::{Json, Path, Query};
pub use axum::response::IntoResponse;
