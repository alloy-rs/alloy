//! Types for the `rpc` API.

use alloy_primitives::map::HashMap;
use serde::{Deserialize, Serialize};

/// The `rpc_modules` response, mapping available RPC module names to their versions.
///
/// Serializes as a JSON object, for example `{"eth":"1.0","net":"1.0"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct RpcModules {
    module_map: HashMap<String, String>,
}

impl RpcModules {
    /// Creates a response from a map of RPC module names to their versions.
    pub const fn new(module_map: HashMap<String, String>) -> Self {
        Self { module_map }
    }

    /// Consumes the response and returns the map of RPC module names to their versions.
    pub fn into_modules(self) -> HashMap<String, String> {
        self.module_map
    }

    /// Returns the map of RPC module names to their versions without consuming the response.
    pub const fn modules(&self) -> &HashMap<String, String> {
        &self.module_map
    }
}
