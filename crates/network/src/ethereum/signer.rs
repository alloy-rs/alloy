use super::Ethereum;
use crate::{NetworkSigner, TxSigner};
use alloy_consensus::{SignableTransaction, TxEnvelope, TypedTransaction};
use alloy_signer::Signature;
use async_trait::async_trait;

/// A signer capable of signing any transaction for the Ethereum network.
pub struct EthereumSigner(Box<dyn TxSigner<Signature> + Sync>);

impl std::fmt::Debug for EthereumSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EthereumSigner").finish()
    }
}

impl<S> From<S> for EthereumSigner
where
    S: TxSigner<Signature> + Sync + 'static,
{
    fn from(signer: S) -> Self {
        Self(Box::new(signer))
    }
}

impl EthereumSigner {
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        self.0.sign_transaction(tx).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NetworkSigner<Ethereum> for EthereumSigner {
    async fn sign(&self, tx: TypedTransaction) -> alloy_signer::Result<TxEnvelope> {
        match tx {
            TypedTransaction::Legacy(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip2930(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip1559(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip4844(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use alloy_consensus::{SignableTransaction, TxLegacy};
    use alloy_primitives::{address, ChainId, Signature, U256};
    use alloy_signer::{k256, Result, Signer, TxSigner, TxSignerSync};

    #[tokio::test]
    async fn signs_tx() {
        async fn sign_tx_test(tx: &mut TxLegacy, chain_id: Option<ChainId>) -> Result<Signature> {
            let mut before = tx.clone();
            let sig = sign_dyn_tx_test(tx, chain_id).await?;
            if let Some(chain_id) = chain_id {
                assert_eq!(tx.chain_id, Some(chain_id), "chain ID was not set");
                before.chain_id = Some(chain_id);
            }
            assert_eq!(*tx, before);
            Ok(sig)
        }

        async fn sign_dyn_tx_test(
            tx: &mut dyn SignableTransaction<Signature>,
            chain_id: Option<ChainId>,
        ) -> Result<Signature> {
            let mut wallet: alloy_signer::Wallet<k256::ecdsa::SigningKey> =
                "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318".parse().unwrap();
            wallet.set_chain_id(chain_id);

            let sig = wallet.sign_transaction_sync(tx)?;
            let sighash = tx.signature_hash();
            assert_eq!(sig.recover_address_from_prehash(&sighash).unwrap(), wallet.address());

            let sig_async = wallet.sign_transaction(tx).await.unwrap();
            assert_eq!(sig_async, sig);

            Ok(sig)
        }

        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction
        let mut tx = TxLegacy {
            to: alloy_primitives::TxKind::Call(address!(
                "F0109fC8DF283027b6285cc889F5aA624EaC1F55"
            )),
            value: U256::from(1_000_000_000),
            gas_limit: 2_000_000,
            nonce: 0,
            gas_price: 21_000_000_000,
            input: Default::default(),
            chain_id: None,
        };
        let sig_none = sign_tx_test(&mut tx, None).await.unwrap();

        tx.chain_id = Some(1);
        let sig_1 = sign_tx_test(&mut tx, None).await.unwrap();
        let expected = "c9cf86333bcb065d140032ecaab5d9281bde80f21b9687b3e94161de42d51895727a108a0b8d101465414033c3f705a9c7b826e596766046ee1183dbc8aeaa6825".parse().unwrap();
        assert_eq!(sig_1, expected);
        assert_ne!(sig_1, sig_none);

        tx.chain_id = Some(2);
        let sig_2 = sign_tx_test(&mut tx, None).await.unwrap();
        assert_ne!(sig_2, sig_1);
        assert_ne!(sig_2, sig_none);

        // Sets chain ID.
        tx.chain_id = None;
        let sig_none_none = sign_tx_test(&mut tx, None).await.unwrap();
        assert_eq!(sig_none_none, sig_none);

        tx.chain_id = None;
        let sig_none_1 = sign_tx_test(&mut tx, Some(1)).await.unwrap();
        assert_eq!(sig_none_1, sig_1);

        tx.chain_id = None;
        let sig_none_2 = sign_tx_test(&mut tx, Some(2)).await.unwrap();
        assert_eq!(sig_none_2, sig_2);

        // Errors on mismatch.
        tx.chain_id = Some(2);
        let error = sign_tx_test(&mut tx, Some(1)).await.unwrap_err();
        let expected_error = alloy_signer::Error::TransactionChainIdMismatch { signer: 1, tx: 2 };
        assert_eq!(error.to_string(), expected_error.to_string());
    }
}
