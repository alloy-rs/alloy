//! Generated forwarding of [`Provider`] methods for provider wrappers.
//!
//! Every default method of [`Provider`] talks to the root client. A wrapper around another
//! provider therefore has to forward each method to its inner provider, otherwise it silently
//! bypasses the inner provider's overrides. Writing those forwarders by hand does not scale, so
//! they are generated from a single method table, [`provider_method_table!`].
//!
//! A wrapper opts in with two pieces:
//!
//! 1. An impl of the crate-private [`ForwardingProvider`] trait. It names the inner provider and
//!    holds the wrapper's overrides. Every method it does not override forwards to the inner
//!    provider.
//! 2. An [`impl_provider_forwarding!`] invocation, which generates the [`Provider`] impl. Each
//!    method of that impl calls the matching [`ForwardingProvider`] method.
//!
//! ```ignore
//! impl<P: Provider<N>, N: Network> ForwardingProvider<N> for MyProvider<P, N> {
//!     type Inner = P;
//!
//!     fn inner_provider(&self) -> &P {
//!         &self.inner
//!     }
//!
//!     // Overrides use the same signature as the `Provider` method.
//!     fn get_block_number(&self) -> ProviderCall<NoParams, U64, BlockNumber> {
//!         ..
//!     }
//! }
//!
//! impl_provider_forwarding!(impl[P: Provider<N>, N: Network] Provider<N> for MyProvider<P, N>);
//! ```
//!
//! Async methods are declared as `fn .. -> impl Future + Send` in [`ForwardingProvider`], so an
//! override can be written as a plain `async fn` without `#[async_trait]`.
//!
//! Inside a [`ForwardingProvider`] impl, `self.method()` is ambiguous between the two traits.
//! Call the wrapper's own [`Provider`] method explicitly, e.g.
//! `Provider::send_transaction_internal(self, tx)`, and the inner provider through its field.
//!
//! Forwarding a method skips the wrapper for every method that its trait default calls on
//! `self`. A wrapper that overrides such a building block (e.g. `send_transaction_internal`)
//! must also override the methods built on it (`send_transaction`, ..) to keep the trait default
//! behavior; see `FillProvider`.
//!
//! # Adding or changing a `Provider` method
//!
//! Change the method in `trait.rs`, then mirror the signature in [`provider_method_table!`],
//! adding any new type to [`prelude`]. All wrappers then forward it; a wrapper that needs other
//! behavior overrides it in its [`ForwardingProvider`] impl. The `table_matches_provider_trait`
//! test fails if a method is missing from the table, and the generated impls fail to compile if
//! a signature in the table doesn't match the trait.
//!
//! Methods that can't be forwarded are left out of the table and keep their trait default:
//! `builder` (no receiver), `erased` (takes `self` by value) and `multicall` (borrows `Self`).
//!
//! [`Provider`]: crate::Provider

/// `impl Future<Output = $ret>`, with a `Send` bound on non-wasm targets, matching the
/// `async_trait` configuration of [`Provider`].
#[cfg(not(target_family = "wasm"))]
macro_rules! forward_future {
    ($ret:ty) => { impl ::core::future::Future<Output = $ret> + Send };
}

/// `impl Future<Output = $ret>`, with a `Send` bound on non-wasm targets, matching the
/// `async_trait` configuration of [`Provider`].
#[cfg(target_family = "wasm")]
macro_rules! forward_future {
    ($ret:ty) => { impl ::core::future::Future<Output = $ret> };
}

/// The table of all forwardable [`Provider`] methods.
///
/// This is the single source of truth for the forwarded signatures. Entries are grouped by kind
/// so that consumers can transcribe each group in a single pass, without per-method macro calls
/// (which would expand after `#[async_trait]` has already run). The receiver is implicit, types
/// are spelled as in `trait.rs` and resolved through [`prelude`], and generic methods spell their
/// bounds in a `where` clause.
///
/// Invoked as `provider_method_table!(consumer; args..)`, which expands to
/// `consumer! { args..; sync { .. } async { .. } generic_sync { .. } generic_async { .. } }`.
macro_rules! provider_method_table {
    ($consumer:ident; $($args:tt)*) => {
        $crate::provider::forward::$consumer! {
            $($args)*;
            sync {
                fn root() -> &RootProvider<N>;
                fn client() -> ClientRef<'_>;
                fn weak_client() -> WeakClient;
                fn get_accounts() -> ProviderCall<NoParams, Vec<Address>>;
                fn get_blob_base_fee() -> ProviderCall<NoParams, U128, u128>;
                fn get_block_number() -> ProviderCall<NoParams, U64, BlockNumber>;
                fn call(tx: N::TransactionRequest) -> EthCall<N, Bytes>;
                fn call_many<'req>(
                    bundles: &'req [Bundle],
                ) -> EthCallMany<'req, N, Vec<Vec<EthCallResponse>>>;
                fn simulate<'req>(
                    payload: &'req SimulatePayload,
                ) -> RpcWithBlock<&'req SimulatePayload, Vec<SimulatedBlock<N::BlockResponse>>>;
                fn get_chain_id() -> ProviderCall<NoParams, U64, u64>;
                fn create_access_list<'a>(
                    request: &'a N::TransactionRequest,
                ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult>;
                fn estimate_gas(tx: N::TransactionRequest) -> EthCall<N, U64, u64>;
                fn get_gas_price() -> ProviderCall<NoParams, U128, u128>;
                fn get_account_info(address: Address) -> RpcWithBlock<Address, AccountInfo>;
                fn get_account(address: Address) -> RpcWithBlock<Address, TrieAccount>;
                fn get_balance(address: Address) -> RpcWithBlock<Address, U256, U256>;
                fn get_block(block: BlockId) -> EthGetBlock<N::BlockResponse>;
                fn get_block_by_hash(hash: BlockHash) -> EthGetBlock<N::BlockResponse>;
                fn get_block_by_number(number: BlockNumberOrTag) -> EthGetBlock<N::BlockResponse>;
                fn get_block_receipts(
                    block: BlockId,
                ) -> ProviderCall<(BlockId,), Option<Vec<N::ReceiptResponse>>>;
                fn get_code_at(address: Address) -> RpcWithBlock<Address, Bytes>;
                fn watch_blocks_from(start_block: u64) -> WatchBlocksFrom<N>;
                fn watch_canonical_blocks_from(start_block: u64) -> WatchCanonicalBlocksFrom<N>;
                fn watch_logs_from(start_block: u64, filter: &Filter) -> WatchLogsFrom<N>;
                fn watch_canonical_logs_from(
                    start_block: u64,
                    filter: &Filter,
                ) -> WatchCanonicalLogsFrom<N>;
                fn get_proof(
                    address: Address,
                    keys: Vec<StorageKey>,
                ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse>;
                fn get_storage_at(
                    address: Address,
                    key: U256,
                ) -> RpcWithBlock<(Address, U256), StorageValue>;
                fn get_storage_values(
                    requests: StorageValuesRequest,
                ) -> RpcWithBlock<(StorageValuesRequest,), StorageValuesResponse>;
                fn get_transaction_by_sender_nonce(
                    sender: Address,
                    nonce: u64,
                ) -> ProviderCall<(Address, U64), Option<N::TransactionResponse>>;
                fn get_transaction_by_hash(
                    hash: TxHash,
                ) -> ProviderCall<(TxHash,), Option<N::TransactionResponse>>;
                fn get_transaction_by_block_hash_and_index(
                    block_hash: B256,
                    index: usize,
                ) -> ProviderCall<(B256, Index), Option<N::TransactionResponse>>;
                fn get_raw_transaction_by_block_hash_and_index(
                    block_hash: B256,
                    index: usize,
                ) -> ProviderCall<(B256, Index), Option<Bytes>>;
                fn get_transaction_by_block_number_and_index(
                    block_number: BlockNumberOrTag,
                    index: usize,
                ) -> ProviderCall<(BlockNumberOrTag, Index), Option<N::TransactionResponse>>;
                fn get_raw_transaction_by_block_number_and_index(
                    block_number: BlockNumberOrTag,
                    index: usize,
                ) -> ProviderCall<(BlockNumberOrTag, Index), Option<Bytes>>;
                fn get_raw_transaction_by_hash(hash: TxHash) -> ProviderCall<(TxHash,), Option<Bytes>>;
                fn get_transaction_count(
                    address: Address,
                ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64>;
                fn get_transaction_receipt(
                    hash: TxHash,
                ) -> ProviderCall<(TxHash,), Option<N::ReceiptResponse>>;
                fn get_max_priority_fee_per_gas() -> ProviderCall<NoParams, U128, u128>;
                #[cfg(feature = "pubsub")]
                fn subscribe_blocks() -> GetSubscription<(SubscriptionKind,), N::HeaderResponse>;
                #[cfg(feature = "pubsub")]
                fn subscribe_full_blocks() -> SubFullBlocks<N>;
                #[cfg(feature = "pubsub")]
                fn subscribe_pending_transactions() -> GetSubscription<(SubscriptionKind,), B256>;
                #[cfg(feature = "pubsub")]
                fn subscribe_full_pending_transactions(
                ) -> GetSubscription<(SubscriptionKind, Params), N::TransactionResponse>;
                #[cfg(feature = "pubsub")]
                fn subscribe_logs(
                    filter: &Filter,
                ) -> GetSubscription<(SubscriptionKind, Params), Log>;
                fn syncing() -> ProviderCall<NoParams, SyncStatus>;
                fn get_client_version() -> ProviderCall<NoParams, String>;
                fn get_sha3(data: &[u8]) -> ProviderCall<(String,), B256>;
                fn get_net_version() -> ProviderCall<NoParams, U64, u64>;
                fn transaction_request() -> N::TransactionRequest;
            }
            async {
                fn get_block_number_by_id(block_id: BlockId) -> TransportResult<Option<BlockNumber>>;
                fn estimate_eip1559_fees_with(
                    estimator: Eip1559Estimator,
                ) -> TransportResult<Eip1559Estimation>;
                fn estimate_eip1559_fees() -> TransportResult<Eip1559Estimation>;
                fn get_fee_history(
                    block_count: u64,
                    last_block: BlockNumberOrTag,
                    reward_percentiles: &[f64],
                ) -> TransportResult<FeeHistory>;
                fn get_block_transaction_count_by_hash(
                    hash: BlockHash,
                ) -> TransportResult<Option<u64>>;
                fn get_block_transaction_count_by_number(
                    block_number: BlockNumberOrTag,
                ) -> TransportResult<Option<u64>>;
                fn get_block_access_list(block: BlockId) -> TransportResult<Option<BlockAccessList>>;
                fn get_block_access_list_by_hash(
                    hash: BlockHash,
                ) -> TransportResult<Option<BlockAccessList>>;
                fn get_block_access_list_by_number(
                    number: BlockNumberOrTag,
                ) -> TransportResult<Option<BlockAccessList>>;
                fn get_block_access_list_raw(block: BlockId) -> TransportResult<Option<Bytes>>;
                fn get_header(block: BlockId) -> TransportResult<Option<N::HeaderResponse>>;
                fn get_header_by_hash(hash: BlockHash) -> TransportResult<Option<N::HeaderResponse>>;
                fn get_header_by_number(
                    number: BlockNumberOrTag,
                ) -> TransportResult<Option<N::HeaderResponse>>;
                fn watch_blocks() -> TransportResult<FilterPollerBuilder<B256>>;
                fn watch_full_blocks() -> TransportResult<WatchBlocks<N::BlockResponse>>;
                fn watch_headers() -> TransportResult<WatchHeaders<N::HeaderResponse>>;
                fn watch_pending_transactions() -> TransportResult<FilterPollerBuilder<B256>>;
                fn watch_logs(filter: &Filter) -> TransportResult<FilterPollerBuilder<Log>>;
                fn watch_full_pending_transactions(
                ) -> TransportResult<FilterPollerBuilder<N::TransactionResponse>>;
                fn get_filter_changes_dyn(id: U256) -> TransportResult<FilterChanges>;
                fn get_filter_logs(id: U256) -> TransportResult<Vec<Log>>;
                fn uninstall_filter(id: U256) -> TransportResult<bool>;
                fn watch_pending_transaction(
                    config: PendingTransactionConfig,
                ) -> Result<PendingTransaction, PendingTransactionError>;
                fn get_logs(filter: &Filter) -> TransportResult<Vec<Log>>;
                fn get_uncle(tag: BlockId, idx: u64) -> TransportResult<Option<N::BlockResponse>>;
                fn get_uncle_count(tag: BlockId) -> TransportResult<u64>;
                fn new_block_filter() -> TransportResult<U256>;
                fn new_filter(filter: &Filter) -> TransportResult<U256>;
                fn new_pending_transactions_filter(full: bool) -> TransportResult<U256>;
                fn send_raw_transaction(
                    encoded_tx: &[u8],
                ) -> TransportResult<PendingTransactionBuilder<N>>;
                fn send_raw_transaction_sync(encoded_tx: &[u8]) -> TransportResult<N::ReceiptResponse>;
                fn send_raw_transaction_conditional(
                    encoded_tx: &[u8],
                    conditional: TransactionConditional,
                ) -> TransportResult<PendingTransactionBuilder<N>>;
                fn send_transaction(
                    tx: N::TransactionRequest,
                ) -> TransportResult<PendingTransactionBuilder<N>>;
                fn send_tx_envelope(tx: N::TxEnvelope) -> TransportResult<PendingTransactionBuilder<N>>;
                #[doc(hidden)]
                fn send_transaction_internal(
                    tx: SendableTx<N>,
                ) -> TransportResult<PendingTransactionBuilder<N>>;
                fn send_transaction_sync(
                    tx: N::TransactionRequest,
                ) -> TransportResult<N::ReceiptResponse>;
                #[doc(hidden)]
                fn send_transaction_sync_internal(
                    tx: SendableTx<N>,
                ) -> TransportResult<N::ReceiptResponse>;
                fn sign_transaction(tx: N::TransactionRequest) -> TransportResult<Bytes>;
                fn fill_transaction(
                    tx: N::TransactionRequest,
                ) -> TransportResult<FillTransaction<N::TxEnvelope>>
                where
                    [N::TxEnvelope: RpcRecv];
                fn fill_and_sign_transaction(
                    tx: N::TransactionRequest,
                ) -> TransportResult<N::TxEnvelope>;
                #[cfg(feature = "pubsub")]
                fn unsubscribe(id: B256) -> TransportResult<()>;
                fn raw_request_dyn(
                    method: Cow<'static, str>,
                    params: &RawValue,
                ) -> TransportResult<Box<RawValue>>;
            }
            generic_sync {
                #[cfg(feature = "pubsub")]
                fn subscribe<Req, Resp>(params: Req) -> GetSubscription<Req, Resp>
                where
                    [Req: RpcSend, Resp: RpcRecv];
                #[cfg(feature = "pubsub")]
                fn subscribe_to<Resp>(method: &'static str) -> GetSubscription<NoParams, Resp>
                where
                    [Resp: RpcRecv];
            }
            generic_async {
                fn get_filter_changes<Resp>(id: U256) -> TransportResult<Vec<Resp>>
                where
                    [Resp: RpcRecv];
                fn raw_request<Req, Resp>(
                    method: Cow<'static, str>,
                    params: Req,
                ) -> TransportResult<Resp>
                where
                    [Req: RpcSend, Resp: RpcRecv];
            }
        }
    };
}

/// Defines [`ForwardingProvider`] from [`provider_method_table!`].
macro_rules! define_forwarding_provider {
    (
        ;
        sync { $(
            $(#[$s_attr:meta])*
            fn $s_name:ident $(<$($s_lt:lifetime),+>)?
                ($($s_arg:ident: $s_ty:ty),* $(,)?) -> $s_ret:ty
                $(where [$($s_wc:tt)*])?;
        )* }
        async { $(
            $(#[$a_attr:meta])*
            fn $a_name:ident $(<$($a_lt:lifetime),+>)?
                ($($a_arg:ident: $a_ty:ty),* $(,)?) -> $a_ret:ty
                $(where [$($a_wc:tt)*])?;
        )* }
        generic_sync { $(
            $(#[$gs_attr:meta])*
            fn $gs_name:ident <$($gs_gen:ident),+>
                ($($gs_arg:ident: $gs_ty:ty),* $(,)?) -> $gs_ret:ty
                where [$($gs_wc:tt)*];
        )* }
        generic_async { $(
            $(#[$ga_attr:meta])*
            fn $ga_name:ident <$($ga_gen:ident),+>
                ($($ga_arg:ident: $ga_ty:ty),* $(,)?) -> $ga_ret:ty
                where [$($ga_wc:tt)*];
        )* }
    ) => {
        /// A [`Provider`] that wraps another provider.
        ///
        /// Every method forwards to [`inner_provider`](Self::inner_provider) unless overridden.
        /// Implement this together with [`impl_provider_forwarding!`], which generates the
        /// [`Provider`] impl on top of it. See the [module docs](self) for details.
        ///
        /// Note that forwarding a method also forwards the methods it calls by default. If a
        /// wrapper overrides a method that another default method builds on (e.g.
        /// `send_transaction_internal`), it must override that method as well to keep using its
        /// own implementation.
        pub(crate) trait ForwardingProvider<N: Network>: Send + Sync {
            /// The wrapped provider.
            type Inner: Provider<N> + ?Sized;

            /// Returns the wrapped provider.
            fn inner_provider(&self) -> &Self::Inner;

            $(
                $(#[$s_attr])*
                #[inline]
                fn $s_name $(<$($s_lt),+>)? (&self, $($s_arg: $s_ty),*) -> $s_ret
                $(where $($s_wc)*)?
                {
                    self.inner_provider().$s_name($($s_arg),*)
                }
            )*

            $(
                $(#[$a_attr])*
                #[inline]
                fn $a_name $(<$($a_lt),+>)? (&self, $($a_arg: $a_ty),*) -> forward_future!($a_ret)
                $(where $($a_wc)*)?
                {
                    async move { self.inner_provider().$a_name($($a_arg),*).await }
                }
            )*

            $(
                $(#[$gs_attr])*
                #[inline]
                fn $gs_name <$($gs_gen),+> (&self, $($gs_arg: $gs_ty),*) -> $gs_ret
                where
                    Self::Inner: Sized,
                    $($gs_wc)*
                {
                    self.inner_provider().$gs_name($($gs_arg),*)
                }
            )*

            $(
                $(#[$ga_attr])*
                #[inline]
                fn $ga_name <$($ga_gen),+> (&self, $($ga_arg: $ga_ty),*) -> forward_future!($ga_ret)
                where
                    Self::Inner: Sized,
                    $($ga_wc)*
                {
                    async move { self.inner_provider().$ga_name($($ga_arg),*).await }
                }
            )*
        }
    };
}

/// Generates a [`Provider`] impl that calls the type's [`ForwardingProvider`]
/// impl for every method in [`provider_method_table!`].
///
/// ```ignore
/// impl_provider_forwarding!(impl[P: Provider<N>, N: Network] Provider<N> for Wrapper<P, N>);
///
/// impl_provider_forwarding!(
///     impl[F, P, N] Provider<N> for Wrapper<F, P, N> where [F: Filler<N>, P: Provider<N>, N: Network]
/// );
/// ```
///
/// The network type parameter must be called `N`, matching the table.
///
/// Optional trailing sections:
/// - `; skip_generic_methods` keeps the trait defaults for the generic `Self: Sized` methods
///   (`raw_request`, `get_filter_changes`, `subscribe`, `subscribe_to`). Required when the inner
///   provider is unsized (`dyn Provider<N>`), since those methods can't be called on it.
/// - `; extra { .. }` adds items verbatim to the impl, for methods that are not in the table.
macro_rules! impl_provider_forwarding {
    (
        impl [$($gen:tt)*] Provider<N> for $ty:ty $(where [$($wc:tt)*])?
        ; skip_generic_methods
        $(; extra { $($extra:tt)* })?
        $(;)?
    ) => {
        $crate::provider::forward::provider_method_table! {
            provider_forwarding_impl;
            skip_generic_methods [$($gen)*] [$ty] [$($($wc)*)?] [$($($extra)*)?]
        }
    };
    (
        impl [$($gen:tt)*] Provider<N> for $ty:ty $(where [$($wc:tt)*])?
        $(; extra { $($extra:tt)* })?
        $(;)?
    ) => {
        $crate::provider::forward::provider_method_table! {
            provider_forwarding_impl;
            all [$($gen)*] [$ty] [$($($wc)*)?] [$($($extra)*)?]
        }
    };
}

/// Consumer of [`provider_method_table!`] for [`impl_provider_forwarding!`].
macro_rules! provider_forwarding_impl {
    (
        skip_generic_methods $gen:tt $ty:tt $wc:tt $extra:tt;
        sync $sync:tt
        async $async:tt
        generic_sync $generic_sync:tt
        generic_async $generic_async:tt
    ) => {
        $crate::provider::forward::provider_forwarding_impl! {
            all $gen $ty $wc $extra;
            sync $sync
            async $async
            generic_sync {}
            generic_async {}
        }
    };
    (
        all [$($gen:tt)*] [$ty:ty] [$($wc:tt)*] [$($extra:tt)*];
        sync { $(
            $(#[$s_attr:meta])*
            fn $s_name:ident $(<$($s_lt:lifetime),+>)?
                ($($s_arg:ident: $s_ty:ty),* $(,)?) -> $s_ret:ty
                $(where [$($s_wc:tt)*])?;
        )* }
        async { $(
            $(#[$a_attr:meta])*
            fn $a_name:ident $(<$($a_lt:lifetime),+>)?
                ($($a_arg:ident: $a_ty:ty),* $(,)?) -> $a_ret:ty
                $(where [$($a_wc:tt)*])?;
        )* }
        generic_sync { $(
            $(#[$gs_attr:meta])*
            fn $gs_name:ident <$($gs_gen:ident),+>
                ($($gs_arg:ident: $gs_ty:ty),* $(,)?) -> $gs_ret:ty
                where [$($gs_wc:tt)*];
        )* }
        generic_async { $(
            $(#[$ga_attr:meta])*
            fn $ga_name:ident <$($ga_gen:ident),+>
                ($($ga_arg:ident: $ga_ty:ty),* $(,)?) -> $ga_ret:ty
                where [$($ga_wc:tt)*];
        )* }
    ) => {
        // The anonymous const scopes the prelude import to the generated impl.
        const _: () = {
            #[allow(unused_imports)]
            use $crate::provider::forward::prelude::*;

            #[cfg_attr(target_family = "wasm", ::async_trait::async_trait(?Send))]
            #[cfg_attr(not(target_family = "wasm"), ::async_trait::async_trait)]
            impl<$($gen)*> $crate::Provider<N> for $ty
            where
                $($wc)*
            {
                $($extra)*

                $(
                    $(#[$s_attr])*
                    #[inline]
                    fn $s_name $(<$($s_lt),+>)? (&self, $($s_arg: $s_ty),*) -> $s_ret
                    $(where $($s_wc)*)?
                    {
                        <Self as $crate::provider::forward::ForwardingProvider<N>>::$s_name(
                            self, $($s_arg),*
                        )
                    }
                )*

                $(
                    $(#[$a_attr])*
                    #[inline]
                    async fn $a_name $(<$($a_lt),+>)? (&self, $($a_arg: $a_ty),*) -> $a_ret
                    $(where $($a_wc)*)?
                    {
                        <Self as $crate::provider::forward::ForwardingProvider<N>>::$a_name(
                            self, $($a_arg),*
                        )
                        .await
                    }
                )*

                $(
                    $(#[$gs_attr])*
                    #[inline]
                    fn $gs_name <$($gs_gen),+> (&self, $($gs_arg: $gs_ty),*) -> $gs_ret
                    where
                        Self: Sized,
                        $($gs_wc)*
                    {
                        <Self as $crate::provider::forward::ForwardingProvider<N>>::$gs_name(
                            self, $($gs_arg),*
                        )
                    }
                )*

                $(
                    $(#[$ga_attr])*
                    #[inline]
                    async fn $ga_name <$($ga_gen),+> (&self, $($ga_arg: $ga_ty),*) -> $ga_ret
                    where
                        Self: Sized,
                        $($ga_wc)*
                    {
                        <Self as $crate::provider::forward::ForwardingProvider<N>>::$ga_name(
                            self, $($ga_arg),*
                        )
                        .await
                    }
                )*
            }
        };
    };
}

/// Lists the method names in [`provider_method_table!`].
#[cfg(test)]
macro_rules! provider_method_names {
    (
        ;
        $(
            $group:ident { $(
                $(#[$attr:meta])*
                fn $name:ident $(<$($gen:tt),+>)? ($($args:tt)*) -> $ret:ty $(where $wc:tt)?;
            )* }
        )*
    ) => {
        &[$($(stringify!($name)),*),*]
    };
}

pub(crate) use define_forwarding_provider;
pub(crate) use impl_provider_forwarding;
pub(crate) use provider_forwarding_impl;
#[cfg(test)]
pub(crate) use provider_method_names;
pub(crate) use provider_method_table;

/// The names used by the signatures in [`provider_method_table!`].
///
/// Mirrors the imports of `trait.rs`, so that table entries can be copied from the trait.
pub(crate) mod prelude {
    pub(crate) use crate::{
        heart::PendingTransactionError,
        utils::{Eip1559Estimation, Eip1559Estimator},
        EthCall, EthCallMany, EthGetBlock, FilterPollerBuilder, Network, PendingTransaction,
        PendingTransactionBuilder, PendingTransactionConfig, Provider, ProviderCall, RootProvider,
        RpcWithBlock, SendableTx, WatchBlocks, WatchBlocksFrom, WatchCanonicalBlocksFrom,
        WatchCanonicalLogsFrom, WatchHeaders, WatchLogsFrom,
    };
    #[cfg(feature = "pubsub")]
    pub(crate) use crate::{GetSubscription, SubFullBlocks};
    pub(crate) use alloy_consensus::TrieAccount;
    pub(crate) use alloy_eips::eip7928::BlockAccessList;
    pub(crate) use alloy_json_rpc::{RpcRecv, RpcSend};
    pub(crate) use alloy_primitives::{
        Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U128, U256,
        U64,
    };
    pub(crate) use alloy_rpc_client::{ClientRef, NoParams, WeakClient};
    #[cfg(feature = "pubsub")]
    pub(crate) use alloy_rpc_types_eth::pubsub::{Params, SubscriptionKind};
    pub(crate) use alloy_rpc_types_eth::{
        erc4337::TransactionConditional,
        simulate::{SimulatePayload, SimulatedBlock},
        AccessListResult, AccountInfo, BlockId, BlockNumberOrTag, Bundle,
        EIP1186AccountProofResponse, EthCallResponse, FeeHistory, FillTransaction, Filter,
        FilterChanges, Index, Log, StorageValuesRequest, StorageValuesResponse, SyncStatus,
    };
    pub(crate) use alloy_transport::TransportResult;
    pub(crate) use serde_json::value::RawValue;
    pub(crate) use std::borrow::Cow;
}

#[allow(unused_imports)]
use prelude::*;

provider_method_table!(define_forwarding_provider;);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// [`Provider`] methods that are deliberately not forwarded.
    const NOT_FORWARDED: &[&str] = &[
        // No receiver.
        "builder",
        // Takes `self` by value and wraps it; forwarding would drop the wrapper.
        "erased",
        // Returns a builder that borrows `Self`.
        "multicall",
    ];

    /// Collects the method names declared in the `Provider` trait in `trait.rs`.
    fn provider_trait_methods() -> BTreeSet<&'static str> {
        let src = include_str!("trait.rs");
        let start = src.find("\npub trait Provider<").expect("Provider trait not found");
        let body = &src[start..];
        let end = body.find("\n}\n").expect("end of Provider trait not found");
        body[..end]
            .lines()
            .filter_map(|line| {
                let line = line.strip_prefix("    ")?;
                let line = line.strip_prefix("async ").unwrap_or(line);
                let rest = line.strip_prefix("fn ")?;
                let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
                Some(&rest[..end])
            })
            .collect()
    }

    #[test]
    fn table_matches_provider_trait() {
        let table: &[&str] = provider_method_table!(provider_method_names;);
        let forwarded: BTreeSet<_> = table.iter().copied().collect();
        assert_eq!(forwarded.len(), table.len(), "duplicate entries in provider_method_table!");

        let trait_methods = provider_trait_methods();
        assert!(trait_methods.len() > 50, "failed to parse trait.rs: {trait_methods:?}");

        let expected: BTreeSet<_> =
            trait_methods.iter().copied().filter(|m| !NOT_FORWARDED.contains(m)).collect();

        let missing: Vec<_> = expected.difference(&forwarded).collect();
        let unknown: Vec<_> = forwarded.difference(&expected).collect();
        assert!(
            missing.is_empty() && unknown.is_empty(),
            "provider_method_table! is out of sync with the Provider trait.\n\
             missing from the table: {missing:?}\n\
             not Provider methods: {unknown:?}"
        );
    }
}
