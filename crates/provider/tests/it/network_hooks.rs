use alloy_consensus::{
    BlobTransactionSidecar, SignableTransaction, TxEnvelope, TxType, TypedTransaction,
};
use alloy_network::{
    BuildResult, Ethereum, EthereumWallet, Network, NetworkTransactionBuilder, NetworkWallet,
    TransactionBuilder, TransactionBuilderError,
};
use alloy_primitives::{Address, Signature, U256};
use alloy_provider::{
    fillers::{BlobGasFiller, FillerControlFlow, GasFiller, TxFiller, WalletFiller},
    SendableTx,
};
use alloy_rpc_types_eth::TransactionRequest;

/// Ethereum, except that legacy transactions with a zero gas price authorize themselves: they take
/// no outer signature and the gas fillers leave them alone.
#[derive(Clone, Copy, Debug)]
struct SelfAuthorizing;

impl Network for SelfAuthorizing {
    type TxType = TxType;

    type TxEnvelope = TxEnvelope;

    type UnsignedTx = TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = TransactionRequest;

    type TransactionResponse = alloy_rpc_types_eth::Transaction;

    type ReceiptResponse = alloy_rpc_types_eth::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::Header;

    type BlockResponse = alloy_rpc_types_eth::Block;

    fn try_into_presigned(tx: TypedTransaction) -> Result<TxEnvelope, TypedTransaction> {
        match tx {
            // The envelope has no unsigned variant, an empty signature stands in for one.
            TypedTransaction::Legacy(tx) if tx.gas_price == 0 => {
                Ok(tx.into_signed(Signature::new(U256::ZERO, U256::ZERO, false)).into())
            }
            tx => Err(tx),
        }
    }
}

impl NetworkTransactionBuilder<SelfAuthorizing> for TransactionRequest {
    fn can_submit(&self) -> bool {
        NetworkTransactionBuilder::<Ethereum>::can_submit(self)
    }

    fn can_build(&self) -> bool {
        NetworkTransactionBuilder::<Ethereum>::can_build(self)
    }

    fn should_fill_gas(&self) -> bool {
        self.gas_price != Some(0)
    }

    fn complete_type(&self, ty: TxType) -> Result<(), Vec<&'static str>> {
        NetworkTransactionBuilder::<Ethereum>::complete_type(self, ty)
    }

    fn output_tx_type(&self) -> TxType {
        NetworkTransactionBuilder::<Ethereum>::output_tx_type(self)
    }

    fn output_tx_type_checked(&self) -> Option<TxType> {
        NetworkTransactionBuilder::<Ethereum>::output_tx_type_checked(self)
    }

    fn prep_for_submission(&mut self) {
        NetworkTransactionBuilder::<Ethereum>::prep_for_submission(self)
    }

    fn build_unsigned(self) -> BuildResult<TypedTransaction, SelfAuthorizing> {
        if let Err((tx_type, missing)) = self.missing_keys() {
            return Err(TransactionBuilderError::InvalidTransactionRequest(tx_type, missing)
                .into_unbuilt(self));
        }
        Ok(self.build_typed_tx().expect("checked by missing_keys"))
    }

    async fn build<W: NetworkWallet<SelfAuthorizing>>(
        self,
        wallet: &W,
    ) -> Result<TxEnvelope, TransactionBuilderError<SelfAuthorizing>> {
        Ok(wallet.sign_request(self).await?)
    }
}

#[test]
fn gas_fillers_skip_request_that_opts_out() {
    let tx = TransactionRequest {
        gas_price: Some(0),
        sidecar: Some(BlobTransactionSidecar::default().into()),
        ..Default::default()
    };

    let gas = GasFiller::default();
    let blob_gas = BlobGasFiller::default();

    assert_eq!(TxFiller::<Ethereum>::status(&gas, &tx), FillerControlFlow::Ready);
    assert_eq!(TxFiller::<Ethereum>::status(&blob_gas, &tx), FillerControlFlow::Ready);

    assert_eq!(TxFiller::<SelfAuthorizing>::status(&gas, &tx), FillerControlFlow::Finished);
    assert_eq!(TxFiller::<SelfAuthorizing>::status(&blob_gas, &tx), FillerControlFlow::Finished);
}

#[tokio::test]
async fn wallet_returns_presigned_envelope_without_signer() {
    // No signing credential is registered for the sender.
    let filler = WalletFiller::new(EthereumWallet::default());
    let tx = TransactionRequest {
        from: Some(Address::with_last_byte(1)),
        to: Some(Address::with_last_byte(2).into()),
        nonce: Some(0),
        gas: Some(21_000),
        gas_price: Some(0),
        chain_id: Some(1),
        ..Default::default()
    };

    let presigned =
        TxFiller::<SelfAuthorizing>::fill(&filler, (), SendableTx::Builder(tx.clone())).await;
    assert!(presigned.unwrap().is_envelope());

    let unsigned =
        TxFiller::<SelfAuthorizing>::fill(&filler, (), SendableTx::Builder(tx.with_gas_price(1)))
            .await;
    assert!(unsigned.unwrap_err().is_local_usage_error());
}
