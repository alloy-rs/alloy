use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_rpc_types_eth::{state::StateOverride, Bundle, StateContext, TransactionIndex};
use std::borrow::Cow;

/// The parameters for an `"eth_call"` RPC request.
#[derive(Clone, Debug)]
pub enum EthCallParams<'req, N: Network> {
    /// Parameters used for `"eth_call"` and `"eth_estimateGas"` RPC requests.
    Call(CallParams<'req, N>),
    /// Parameters used for `"eth_callMany"` RPC requests.
    CallMany(CallManyParams<'req>),
}

impl<'req, N> EthCallParams<'req, N>
where
    N: Network,
{
    /// Instantiates a new `EthCallParams` with the given data (transaction).
    ///
    /// This is used for `"eth_call"` and `"eth_estimateGas"` requests.
    pub const fn call(data: &'req N::TransactionRequest) -> Self {
        Self::Call(CallParams { data: Cow::Borrowed(data), block: None, overrides: None })
    }

    /// Instantiates a new `EthCallParams` with the given transactions.
    ///
    /// This is used for `"eth_callMany"` requests.
    pub fn call_many(bundle: &'req Vec<Bundle>) -> Self {
        Self::CallMany(CallManyParams::new(bundle))
    }

    /// Sets the block to use for this call.
    ///
    /// In case of `"eth_callMany"` requests, this sets the block in the [`StateContext`].
    pub fn with_block(self, block: BlockId) -> Self {
        match self {
            Self::Call(mut params) => {
                params.block = Some(block);
                Self::Call(params)
            }
            Self::CallMany(mut params) => {
                params = params.with_block(block);
                Self::CallMany(params)
            }
        }
    }

    /// Sets the [`TransactionIndex`] in the [`StateContext`] for the call.
    ///
    /// This is only applicable for `"eth_callMany"` requests, and will be ignored for
    /// `"eth_call"`/`"eth_estimateGas"` requests.
    pub fn with_transaction_index(self, tx_index: TransactionIndex) -> Self {
        match self {
            Self::Call(params) => Self::Call(params),
            Self::CallMany(mut params) => {
                params = params.with_transaction_index(tx_index);
                Self::CallMany(params)
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
            Self::CallMany(mut params) => {
                params.overrides = Some(Cow::Borrowed(overrides));
                Self::CallMany(params)
            }
        }
    }

    /// Sets the state context for the call.
    ///
    /// This is only applicable for `"eth_callMany"` requests, and will be ignored for
    /// `"eth_call"`/`"eth_estimateGas"` requests.
    pub fn with_context(self, context: &'req StateContext) -> Self {
        match self {
            Self::Call(params) => Self::Call(params),
            Self::CallMany(mut params) => {
                params.context = Some(*context);
                Self::CallMany(params)
            }
        }
    }

    /// Returns a reference to the state overrides if set.
    pub fn overrides(&self) -> Option<&StateOverride> {
        match self {
            Self::Call(params) => params.overrides(),
            Self::CallMany(params) => params.overrides(),
        }
    }

    /// Returns a reference to the transaction data if this is a `"eth_call"`/`eth_estimateGas`.
    pub fn data(&self) -> Option<&N::TransactionRequest> {
        self.as_call_params().map(|p| p.data())
    }

    /// Returns a reference to the bundles if this is a `"eth_callMany"` request.
    pub fn bundles(&self) -> Option<&[Bundle]> {
        self.as_call_many_params().map(|p| p.bundles())
    }

    /// Returns the block.
    pub fn block(&self) -> Option<BlockId> {
        match self {
            Self::Call(params) => params.block(),
            Self::CallMany(params) => params.context().and_then(|c| c.block_number),
        }
    }

    /// Returns a reference to the [`StateContext`].
    pub fn context(&self) -> Option<&StateContext> {
        self.as_call_many_params().and_then(|p| p.context())
    }

    /// Clones the tx data and overrides into owned data.
    pub fn into_owned(self) -> EthCallParams<'static, N> {
        match self {
            Self::Call(params) => EthCallParams::Call(params.into_owned()),
            Self::CallMany(params) => EthCallParams::CallMany(params.into_owned()),
        }
    }

    /// Returns a reference to the call parameters if this is a `"eth_call"`/`eth_estimateGas`
    /// request.
    pub const fn as_call_params(&self) -> Option<&CallParams<'req, N>> {
        match self {
            Self::Call(params) => Some(params),
            _ => None,
        }
    }

    /// Returns a reference to the call many parameters if this is a `"eth_callMany"` request.
    pub const fn as_call_many_params(&self) -> Option<&CallManyParams<'req>> {
        match self {
            Self::CallMany(params) => Some(params),
            _ => None,
        }
    }

    /// Returns `true` if this is a `"eth_call"`/`"eth_estimateGas"` request.
    ///
    /// `false` indicates a `"eth_callMany"` request.
    pub const fn is_call(&self) -> bool {
        matches!(self, Self::Call(_))
    }
}

/// The parameters for an `"eth_call"` and `"eth_estimateGas"` RPC request.
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

/// The parameters for an `"eth_callMany"` RPC request.
#[derive(Clone, Debug)]
pub struct CallManyParams<'req> {
    /// The bundle of transactions to execute.
    bundles: Cow<'req, Vec<Bundle>>,
    /// The state context for the call.
    context: Option<StateContext>,
    /// State overrides for the call.
    overrides: Option<Cow<'req, StateOverride>>,
}

impl<'req> CallManyParams<'req> {
    /// Instantiates a new `CallManyParams` with the given bundle.
    pub const fn new(bundles: &'req Vec<Bundle>) -> Self {
        Self { bundles: Cow::Borrowed(bundles), context: None, overrides: None }
    }

    /// Sets the block in [`StateContext`] to use for this call.
    pub fn with_block(mut self, block: BlockId) -> Self {
        let context = self.context.unwrap_or_default();
        self.context = Some(StateContext { block_number: Some(block), ..context });
        self
    }

    /// Sets the [`TransactionIndex`] in the [`StateContext`] for the call.
    pub fn with_transaction_index(mut self, tx_index: TransactionIndex) -> Self {
        let context = self.context.unwrap_or_default();
        self.context = Some(StateContext { transaction_index: Some(tx_index), ..context });
        self
    }

    /// Sets the state context for the call.
    pub const fn with_context(mut self, context: StateContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Sets the state overrides for the call.
    pub fn with_overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.overrides = Some(Cow::Borrowed(overrides));
        self
    }

    /// Returns a reference to the bundle.
    pub fn bundles(&self) -> &[Bundle] {
        &self.bundles
    }

    /// Returns a reference to the state context if set.
    pub const fn context(&self) -> Option<&StateContext> {
        self.context.as_ref()
    }

    /// Returns a reference to the state overrides if set.
    pub fn overrides(&self) -> Option<&StateOverride> {
        self.overrides.as_deref()
    }

    /// Clones the tx data and overrides into owned data.
    pub fn into_owned(self) -> CallManyParams<'static> {
        CallManyParams {
            bundles: Cow::Owned(self.bundles.into_owned()),
            context: self.context,
            overrides: self.overrides.map(|o| Cow::Owned(o.into_owned())),
        }
    }
}
