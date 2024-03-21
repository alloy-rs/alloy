//! Constants related to EIP-7547.
//!
//! Based on the following part of the spec:
//!
//! ## Constants
//!
//! | Name | Value |
//! | - | - |
//! | `INCLUSION_LIST_MAX_GAS` |  `uint64(4194304) = 2**22` |

/// The maximum gas allowed for the inclusion list.
pub const INCLUSION_LIST_MAX_GAS: u64 = 4194304;
