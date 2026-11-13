//! Helpers shared by unit tests, integration tests (`test-util`) and the debug build (ADR 0021).

#[cfg(any(test, debug_assertions, feature = "test-util"))]
pub mod demo;
#[cfg(any(test, feature = "test-util"))]
mod fixtures;
#[cfg(test)]
pub mod rules;

#[cfg(any(test, feature = "test-util"))]
pub use fixtures::*;
