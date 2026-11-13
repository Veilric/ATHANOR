//! Native hosting and application services for Athanor.
pub mod bootstrap;
pub mod config;
pub mod platform;
pub mod services;
#[cfg(feature = "native-ui")]
pub mod ui;
