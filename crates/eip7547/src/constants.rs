//! Constants related to EIP-7547.

/// The maximum gas allowed for the inclusion list.
///
/// Based on the following part of the spec:
///
/// ## Constants
///
/// | Name | Value |
/// | - | - |
/// | `INCLUSION_LIST_MAX_GAS` |  `uint64(4194304) = 2**22` |
pub const INCLUSION_LIST_MAX_GAS: u64 = 4194304;

/// The capabilities for inclusion list engine API endpoints.
pub const CAPABILITIES: [&str; 2] = ["engine_newInclusionListV1", "engine_getInclusionListV1"];
