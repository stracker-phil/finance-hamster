//! Helpers shared by unit tests, integration tests (`test-util`) and the debug build (ADR 0021).

#[cfg(any(test, feature = "test-util"))]
mod fixtures;

#[cfg(any(test, feature = "test-util"))]
pub use fixtures::*;
