use super::LogOptions;
use alloy_network::Network;
use alloy_rpc_types_eth::Filter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Parameters for `eth_getLogs` with enhanced options.
#[derive(Clone, Debug)]
pub struct EthLogsParams<N: Network> {
    /// The log filter.
    pub filter: Filter,
    /// Enhanced log retrieval options.
    pub options: LogOptions,
    #[allow(dead_code)]
    _phantom: std::marker::PhantomData<fn() -> N>,
}

impl<N: Network> Serialize for EthLogsParams<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.filter.serialize(serializer)
    }
}

impl<'de, N: Network> Deserialize<'de> for EthLogsParams<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let filter = Filter::deserialize(deserializer)?;
        Ok(Self { filter, options: LogOptions::default(), _phantom: std::marker::PhantomData })
    }
}

impl<N: Network> EthLogsParams<N> {
    /// Create new parameters with the given filter.
    pub fn new(filter: Filter) -> Self {
        Self { filter, options: LogOptions::default(), _phantom: std::marker::PhantomData }
    }

    /// Create new parameters with the given filter and options.
    pub fn with_options(filter: Filter, options: LogOptions) -> Self {
        Self { filter, options, _phantom: std::marker::PhantomData }
    }

    /// Get the filter.
    pub const fn filter(&self) -> &Filter {
        &self.filter
    }

    /// Get the options.
    pub const fn options(&self) -> &LogOptions {
        &self.options
    }
}
