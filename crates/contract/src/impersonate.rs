use alloy_network::Network;
use alloy_primitives::U256;
use alloy_provider::{ext::AnvilApi, Provider};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_transport::TransportError;
use futures::try_join;

/// A utility for impersonating an Ethereum account to send transactions using Anvil.
///
/// This helper simplifies the process of:
/// 1. Impersonating an account
/// 2. Optionally funding it with ETH
/// 3. Sending a transaction from it without a signer
/// 4. Stopping the impersonation
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = ProviderBuilder::new().connect_anvil();
///
/// let from = address!("0x000000000000000000000000000000000000dEaD");
/// let to = address!("0x000000000000000000000000000000000000beef");
/// let tx =
///     TransactionRequest::default().with_from(from).with_to(to).with_value(U256::from(1_000_000));
///
/// let call = ImpersonatedCall::new(provider, tx, Some(U256::from(1e18 as u64)));
/// let receipt = call.send_impersonated_tx().await?;
/// println!("Impersonated tx succeeded in block: {:?}", receipt.block_number);
/// # Ok(())
/// # }
/// ```

#[derive(Debug, Clone)]
pub struct ImpersonatedCall<P, N>
where
    P: Provider<N> + AnvilApi<N>,
    N: Network,
{
    provider: P,
    tx_request: TransactionRequest,
    fund_amount: Option<U256>,
    _phantom: std::marker::PhantomData<N>,
}

impl<P, N> ImpersonatedCall<P, N>
where
    P: Provider<N> + AnvilApi<N>,
    N: Network,
    N: Network<TransactionRequest = TransactionRequest>,
{
    /// Creates a new [`ImpersonatedCall`] with the given provider, transaction, and optional
    /// funding amount.
    ///
    /// # Parameters
    /// * `provider`: An Anvil-compatible provider instance.
    /// * `tx_request`: The transaction to send. Must include a `from` field.
    /// * `fund_amount`: Optional amount of ETH to fund the impersonated account with.

    pub fn new(provider: P, tx_request: TransactionRequest, fund_amount: Option<U256>) -> Self {
        Self { provider, tx_request, fund_amount, _phantom: std::marker::PhantomData }
    }

    /// Executes the impersonation flow:
    /// 1. Impersonates the `from` address in the transaction.
    /// 2. Optionally funds the address with ETH.
    /// 3. Sends the transaction via Anvil's `anvil_send_impersonated_transaction`.
    /// 4. Waits for the transaction receipt.
    /// 5. Stops impersonation of the account.
    ///
    /// # Errors
    /// Returns a [`TransportError`] if any step in the process fails.
    ///
    /// # Returns
    /// * On success: The transaction receipt returned by [`Provider::get_transaction_receipt`].
    /// * On failure: Any error during impersonation, funding, sending, or receipt retrieval.
    pub async fn send_impersonated_tx(self) -> Result<N::ReceiptResponse, TransportError> {
        let Self { provider, tx_request, fund_amount, _phantom } = self;

        let from = tx_request.from.unwrap();

        // Create impersonation future
        let impersonate_future = provider.anvil_impersonate_account(from);

        // Create fund future if needed
        if let Some(amount) = fund_amount {
            let fund_future = provider.anvil_set_balance(from, amount);
            try_join!(fund_future, impersonate_future)?;
        } else {
            impersonate_future.await?;
        }

        // Send transaction
        let tx_hash = provider.anvil_send_impersonated_transaction(tx_request.clone()).await?;

        // Wait for receipt and stop impersonation
        let receipt_future = provider.get_transaction_receipt(tx_hash);
        let stop_impersonate_future = provider.anvil_stop_impersonating_account(from);

        let (reciept, _) = try_join!(receipt_future, stop_impersonate_future)?;
        Ok(reciept.unwrap())
    }
}

#[cfg(test)]
mod tests {

    use crate::ImpersonatedCall;
    use alloy_network::TransactionBuilder;
    use alloy_primitives::{address, U256};
    use alloy_provider::{
        fillers::{ChainIdFiller, GasFiller},
        Provider, ProviderBuilder,
    };
    use alloy_rpc_types_eth::TransactionRequest;

    #[tokio::test]
    async fn test_impersonated_call_executes_successfully() {
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .with_simple_nonce_management()
            .filler(GasFiller)
            .filler(ChainIdFiller::default())
            .connect_anvil();

        let impersonate = address!("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let to = address!("0xfaca325c86bf9c2d5b413cd7b90b209be92229c2");
        let val = U256::from(1337);
        let funding = U256::from(1e18 as u64);

        // Build tx
        let tx = TransactionRequest::default().with_from(impersonate).with_to(to).with_value(val);

        // Use the ImpersonatedCall helper
        let call = ImpersonatedCall::new(provider.clone(), tx.clone(), Some(funding));
        let receipt = call.send_impersonated_tx().await.unwrap();

        assert_eq!(receipt.from, impersonate);
        assert_eq!(provider.get_balance(to).await.unwrap(), val);
    }
}
