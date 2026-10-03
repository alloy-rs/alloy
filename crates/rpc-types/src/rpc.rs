//! Types for the `rpc` API.

use alloy_primitives::map::HashMap;
use serde::{Deserialize, Serialize};

/// Represents the `rpc_modules` response, which returns the
/// list of all available modules on that transport and their version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct RpcModules {
    module_map: HashMap<String, String>,
}

impl RpcModules {
    /// Create a new instance of `RPCModules`
    pub const fn new(module_map: HashMap<String, String>) -> Self {
        Self { module_map }
    }

    /// Consumes self and returns the inner hashmap mapping module names to their versions
    pub fn into_modules(self) -> HashMap<String, String> {
        self.module_map
    }
}
