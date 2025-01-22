use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_rpc_types_eth::state::StateOverride;
use serde::ser::SerializeSeq;
use std::borrow::Cow;

/// The parameters for an `"eth_call"` RPC request.
#[derive(Clone, Debug)]
pub enum EthCallParams<'req, N: Network> {
    /// Parameters used for "eth_call" and "eth_estimateGas" RPC requests.
    Call(CallParams<'req, N>),
}

impl<'req, N> EthCallParams<'req, N>
where
    N: Network,
{
    /// Instantiates a new `EthCallParams` with the given data (transaction).
    pub const fn new(data: &'req N::TransactionRequest) -> Self {
        Self::Call(CallParams { data: Cow::Borrowed(data), block: None, overrides: None })
    }

    /// Sets the block to use for this call.
    pub fn with_block(self, block: BlockId) -> Self {
        match self {
            Self::Call(mut params) => {
                params.block = Some(block);
                Self::Call(params)
            }
        }
    }

    /// Sets the state overrides for this call.
    pub fn with_overrides(self, overrides: &'req StateOverride) -> Self {
        match self {
            Self::Call(mut params) => {
                params.overrides = Some(Cow::Borrowed(overrides));
                Self::Call(params)
            }
        }
    }

    /// Returns a reference to the state overrides if set.
    pub fn overrides(&self) -> Option<&StateOverride> {
        match self {
            Self::Call(params) => params.overrides(),
        }
    }

    /// Returns a reference to the transaction data.
    pub fn data(&self) -> &N::TransactionRequest {
        match self {
            Self::Call(params) => params.data(),
        }
    }

    /// Returns the block.
    pub const fn block(&self) -> Option<BlockId> {
        match self {
            Self::Call(params) => params.block(),
        }
    }

    /// Clones the tx data and overrides into owned data.
    pub fn into_owned(self) -> EthCallParams<'static, N> {
        match self {
            Self::Call(params) => EthCallParams::Call(params.into_owned()),
        }
    }
}

impl<N: Network> serde::Serialize for EthCallParams<'_, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.overrides().is_some() { 3 } else { 2 };

        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&self.data())?;

        if let Some(overrides) = self.overrides() {
            seq.serialize_element(&self.block().unwrap_or_default())?;
            seq.serialize_element(overrides)?;
        } else if let Some(block) = self.block() {
            seq.serialize_element(&block)?;
        }

        seq.end()
    }
}

/// The parameters for an `"eth_call"` RPC request.
#[derive(Clone, Debug)]
pub struct CallParams<'req, N: Network> {
    data: Cow<'req, N::TransactionRequest>,
    pub(crate) block: Option<BlockId>,
    pub(crate) overrides: Option<Cow<'req, StateOverride>>,
}

impl<'req, N> CallParams<'req, N>
where
    N: Network,
{
    /// Instantiates a new `EthCallParams` with the given data (transaction).
    pub const fn new(data: &'req N::TransactionRequest) -> Self {
        Self { data: Cow::Borrowed(data), block: None, overrides: None }
    }

    /// Sets the block to use for this call.
    pub const fn with_block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the state overrides for this call.
    pub fn with_overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.overrides = Some(Cow::Borrowed(overrides));
        self
    }

    /// Returns a reference to the state overrides if set.
    pub fn overrides(&self) -> Option<&StateOverride> {
        self.overrides.as_deref()
    }

    /// Returns a reference to the transaction data.
    pub fn data(&self) -> &N::TransactionRequest {
        &self.data
    }

    /// Returns the block.
    pub const fn block(&self) -> Option<BlockId> {
        self.block
    }

    /// Clones the tx data and overrides into owned data.
    pub fn into_owned(self) -> CallParams<'static, N> {
        CallParams {
            data: Cow::Owned(self.data.into_owned()),
            block: self.block,
            overrides: self.overrides.map(|o| Cow::Owned(o.into_owned())),
        }
    }
}
