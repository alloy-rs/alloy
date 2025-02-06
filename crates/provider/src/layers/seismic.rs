//! A provider layer that uses for filling sesimic transactions

use crate::{
    fillers::{
        FillProvider, JoinFill, NonceFiller, RecommendedFillers, SimpleNonceManager, WalletFiller,
    },
    Identity, PendingTransactionBuilder, Provider, ProviderBuilder, ProviderLayer, RootProvider,
    SendableTx,
};
use alloy_consensus::TxSeismic;
use alloy_network::{Ethereum, EthereumWallet, Network, TransactionBuilder};
use alloy_primitives::{Bytes, FixedBytes};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use std::marker::PhantomData;
use tee_service_api::{ecdh_decrypt, ecdh_encrypt, rand, Keypair, PublicKey, Secp256k1};

/// Creates a new provider with seismic and wallet capabilities
pub fn create_seismic_provider(
    wallet: EthereumWallet,
    url: reqwest::Url,
) -> FillProvider<
    JoinFill<Identity, NonceFiller>,
    SeismicProvider<
        FillProvider<
            JoinFill<
                <Ethereum as RecommendedFillers>::RecommendedFillers,
                WalletFiller<EthereumWallet>,
            >,
            RootProvider<alloy_transport_http::Http<alloy_transport_http::Client>, Ethereum>,
            alloy_transport_http::Http<alloy_transport_http::Client>,
            Ethereum,
        >,
        alloy_transport_http::Http<alloy_transport_http::Client>,
        Ethereum,
    >,
    alloy_transport_http::Http<alloy_transport_http::Client>,
    Ethereum,
> {
    // Create wallet layer with recommended fillers
    let wallet_layer =
        JoinFill::new(Ethereum::recommended_fillers(), WalletFiller::new(wallet.clone()));

    // Create nonce management layer
    let nonce_layer: JoinFill<Identity, NonceFiller<SimpleNonceManager>> =
        JoinFill::new(Identity, NonceFiller::default());

    // Build and return the provider
    ProviderBuilder::new()
        .network::<Ethereum>()
        .layer(nonce_layer)
        .layer(SeismicLayer {})
        .layer(wallet_layer)
        .on_http(url)
}

/// Seismic middlware for encrypting transactions and decrypting responses
#[derive(Debug, Clone)]
pub struct SeismicLayer {}

impl<P, T, N> ProviderLayer<P, T, N> for SeismicLayer
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    type Provider = SeismicProvider<P, T, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        SeismicProvider::new(inner)
    }
}

/// Seismic middlware for encrypting transactions and decrypting responses
#[derive(Debug, Clone)]
pub struct SeismicProvider<P, T, N> {
    /// Inner provider.
    inner: P,
    /// Phantom data
    _pd: PhantomData<(T, N)>,
}

impl<P, T, N> SeismicProvider<P, T, N>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    /// Create a new seismic provider
    fn new(inner: P) -> Self {
        Self { inner, _pd: PhantomData }
    }

    /// Get the encryption private key
    pub fn get_encryption_keypair(&self) -> Keypair {
        let secp = Secp256k1::new();
        Keypair::new(&secp, &mut rand::thread_rng())
    }
}

/// Implement the Provider trait for the SeismicProvider
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<P, T, N> Provider<T, N> for SeismicProvider<P, T, N>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    fn root(&self) -> &RootProvider<T, N> {
        self.inner.root()
    }

    async fn seismic_call(&self, mut tx: SendableTx<N>) -> TransportResult<Bytes> {
        if let Some(builder) = tx.as_mut_builder() {
            if builder.output_tx_type().into() == TxSeismic::TX_TYPE
                && builder.input().is_some()
                && builder.nonce().is_some()
            {
                let tee_pubkey =
                    PublicKey::from_slice(self.inner.get_tee_pubkey().await.unwrap().as_slice())
                        .unwrap();
                let encryption_keypair = self.get_encryption_keypair();

                // Generate new public/private keypair for this transaction
                let pubkey_bytes = FixedBytes(encryption_keypair.public_key().serialize());
                builder.set_encryption_pubkey(pubkey_bytes);

                // Encrypt using recipient's public key and generated private key
                let plaintext_input = builder.input().unwrap();
                let encrypted_input = ecdh_encrypt(
                    &tee_pubkey,
                    &encryption_keypair.secret_key(),
                    plaintext_input.to_vec(),
                    builder.nonce().unwrap(),
                )
                .unwrap();
                builder.set_input(Bytes::from(encrypted_input));

                // decrypting output
                return self
                    .inner
                    .seismic_call(SendableTx::Builder(builder.clone()))
                    .await
                    .and_then(|encrypted_output| {
                        // Decrypt the output using the encryption keypair
                        let decrypted_output = ecdh_decrypt(
                            &tee_pubkey,
                            &encryption_keypair.secret_key(),
                            encrypted_output.to_vec(),
                            builder.nonce().unwrap(),
                        )
                        .map_err(|e| {
                            TransportErrorKind::custom_str(&format!(
                                "Error decrypting output: {:?}",
                                e
                            ))
                        })?;
                        Ok(Bytes::from(decrypted_output))
                    });
            }
        }
        let res = self.inner.seismic_call(tx).await;
        res
    }

    async fn send_transaction_internal(
        &self,
        mut tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<T, N>> {
        if let Some(builder) = tx.as_mut_builder() {
            if builder.output_tx_type().into() == TxSeismic::TX_TYPE
                && builder.input().is_some()
                && builder.nonce().is_some()
            {
                let tee_pubkey =
                    PublicKey::from_slice(self.inner.get_tee_pubkey().await.unwrap().as_slice())
                        .unwrap();
                let encryption_keypair = self.get_encryption_keypair();

                // Generate new public/private keypair for this transaction
                let pubkey_bytes = FixedBytes(encryption_keypair.public_key().serialize());
                builder.set_encryption_pubkey(pubkey_bytes);

                // Encrypt using recipient's public key and generated private key
                let plaintext_input = builder.input().unwrap();
                let encrypted_input = ecdh_encrypt(
                    &tee_pubkey,
                    &encryption_keypair.secret_key(),
                    plaintext_input.to_vec(),
                    builder.nonce().unwrap(),
                )
                .unwrap();
                builder.set_input(Bytes::from(encrypted_input));
            }
        }
        let res = self.inner.send_transaction_internal(tx).await;
        res
    }
}

/// Utilities for testing seismic provider
pub mod test_utils {
    use super::*;
    use alloy_primitives::{hex, Address, Bytes, TxKind};
    use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};

    /// Test context for seismic provider
    #[derive(Debug)]
    pub struct ContractTestContext;
    impl ContractTestContext {
        // ==================== first block for encrypted transaction ====================
        // Contract deployed
        //     pragma solidity ^0.8.13;
        // contract SeismicCounter {
        //     suint256 number;
        //     constructor() payable {
        //         number = 0;
        //     }
        //     function setNumber(suint256 newNumber) public {
        //         number = newNumber;
        //     }
        //     function increment() public {
        //         number++;
        //     }
        //     function isOdd() public view returns (bool) {
        //         return number % 2 == 1;
        //     }
        // }
        /// Get the is odd input plaintext
        pub fn get_is_odd_input_plaintext() -> Bytes {
            Bytes::from_static(&hex!("43bd0d70"))
        }

        /// Get the set number input plaintext
        pub fn get_set_number_input_plaintext() -> Bytes {
            Bytes::from_static(&hex!(
                "24a7f0b70000000000000000000000000000000000000000000000000000000000000003"
            ))
        }

        /// Get the increment input plaintext
        pub fn get_increment_input_plaintext() -> Bytes {
            Bytes::from_static(&hex!("d09de08a"))
        }

        /// Get the deploy input plaintext
        pub fn get_deploy_input_plaintext() -> Bytes {
            Bytes::from_static(&hex!("60806040525f5f8190b150610285806100175f395ff3fe608060405234801561000f575f5ffd5b506004361061003f575f3560e01c806324a7f0b71461004357806343bd0d701461005f578063d09de08a1461007d575b5f5ffd5b61005d600480360381019061005891906100f6565b610087565b005b610067610090565b604051610074919061013b565b60405180910390f35b6100856100a7565b005b805f8190b15050565b5f600160025fb06100a19190610181565b14905090565b5f5f81b0809291906100b8906101de565b919050b150565b5f5ffd5b5f819050919050565b6100d5816100c3565b81146100df575f5ffd5b50565b5f813590506100f0816100cc565b92915050565b5f6020828403121561010b5761010a6100bf565b5b5f610118848285016100e2565b91505092915050565b5f8115159050919050565b61013581610121565b82525050565b5f60208201905061014e5f83018461012c565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601260045260245ffd5b5f61018b826100c3565b9150610196836100c3565b9250826101a6576101a5610154565b5b828206905092915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f6101e8826100c3565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff820361021a576102196101b1565b5b60018201905091905056fea2646970667358221220ea421d58b6748a9089335034d76eb2f01bceafe3dfac2e57d9d2e766852904df64736f6c63782c302e382e32382d646576656c6f702e323032342e31322e392b636f6d6d69742e39383863313261662e6d6f64005d"))
        }

        /// Results from solc compilation
        pub fn get_code() -> Bytes {
            Bytes::from_static(&hex!("608060405234801561000f575f5ffd5b506004361061003f575f3560e01c806324a7f0b71461004357806343bd0d701461005f578063d09de08a1461007d575b5f5ffd5b61005d600480360381019061005891906100f6565b610087565b005b610067610090565b604051610074919061013b565b60405180910390f35b6100856100a7565b005b805f8190b15050565b5f600160025fb06100a19190610181565b14905090565b5f5f81b0809291906100b8906101de565b919050b150565b5f5ffd5b5f819050919050565b6100d5816100c3565b81146100df575f5ffd5b50565b5f813590506100f0816100cc565b92915050565b5f6020828403121561010b5761010a6100bf565b5b5f610118848285016100e2565b91505092915050565b5f8115159050919050565b61013581610121565b82525050565b5f60208201905061014e5f83018461012c565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601260045260245ffd5b5f61018b826100c3565b9150610196836100c3565b9250826101a6576101a5610154565b5b828206905092915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f6101e8826100c3565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff820361021a576102196101b1565b5b60018201905091905056fea2646970667358221220ea421d58b6748a9089335034d76eb2f01bceafe3dfac2e57d9d2e766852904df64736f6c63782c302e382e32382d646576656c6f702e323032342e31322e392b636f6d6d69742e39383863313261662e6d6f64005d"))
        }
    }

    /// Get a seismic transaction builder
    pub fn get_seismic_tx_builder(
        plaintext: Bytes,
        to: TxKind,
        from: Address,
    ) -> TransactionRequest {
        TransactionRequest {
            from: Some(from),
            to: Some(to),
            input: TransactionInput { input: Some(plaintext), data: None },
            transaction_type: Some(TxSeismic::TX_TYPE),
            gas_price: Some(20e9 as u128), /* make seismic tx treated as legacy tx when estimate
                                            * for gas */
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[ignore]
mod tests {
    use alloy_network::{Ethereum, EthereumWallet};
    use alloy_node_bindings::{Anvil, AnvilInstance};
    use alloy_primitives::TxKind;
    use alloy_signer_local::PrivateKeySigner;

    use crate::test_utils::*;

    use super::*;

    #[tokio::test]
    async fn test_seismic_signed_call() {
        let plaintext = ContractTestContext::get_deploy_input_plaintext();
        let anvil = Anvil::new().spawn();
        let wallet = get_wallet(&anvil);
        let provider = create_seismic_provider(
            wallet.clone(),
            reqwest::Url::parse("http://localhost:8545").unwrap(),
        );

        let from = wallet.default_signer().address();
        let tx = get_seismic_tx_builder(plaintext, TxKind::Create, from);

        let res = provider.seismic_call(SendableTx::Builder(tx)).await.unwrap();
        println!("test_seismic_call: res: {:?}", res);
    }

    #[tokio::test]
    async fn test_seismic_unsigned_call() {
        let plaintext = ContractTestContext::get_deploy_input_plaintext();
        let anvil = Anvil::new().spawn();
        let wallet = get_wallet(&anvil);

        // Create nonce management layer
        let nonce_layer: JoinFill<Identity, NonceFiller<SimpleNonceManager>> =
            JoinFill::new(Identity, NonceFiller::default());

        // Build and return the provider
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .layer(nonce_layer)
            .layer(SeismicLayer {})
            .on_http(reqwest::Url::parse("http://localhost:8545").unwrap());

        let from = wallet.default_signer().address();
        let tx = get_seismic_tx_builder(plaintext, TxKind::Create, from);

        let res = provider.seismic_call(SendableTx::Builder(tx)).await.unwrap();
        println!("test_seismic_call: res: {:?}", res);
    }

    #[tokio::test]
    async fn test_send_transaction() {
        let plaintext = ContractTestContext::get_deploy_input_plaintext();
        // let anvil = Anvil::at("/Users/phe/repos/seismic-foundry/target/debug/sanvil").spawn();
        let anvil = Anvil::new().spawn();
        let wallet = get_wallet(&anvil);
        let provider = create_seismic_provider(
            wallet.clone(),
            reqwest::Url::parse("http://localhost:8545").unwrap(),
        );

        let from = wallet.default_signer().address();
        let tx = get_seismic_tx_builder(plaintext, TxKind::Create, from);

        println!("test_send_transaction_internal: tx: {:?}", tx);
        let res = provider.send_transaction(tx).await.unwrap();
        println!("test_send_transaction_internal: res: {:?}", res);

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    fn get_wallet(anvil: &AnvilInstance) -> EthereumWallet {
        let bob: PrivateKeySigner = anvil.keys()[1].clone().into();
        let wallet = EthereumWallet::from(bob.clone());
        wallet
    }

    #[tokio::test]
    async fn test_get_tee_pubkey() {
        let anvil = Anvil::at("/Users/phe/repos/seismic-foundry/target/debug/sanvil").spawn();
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .layer(SeismicLayer {})
            .on_http(anvil.endpoint_url());
        let tee_pubkey = provider.get_tee_pubkey().await.unwrap();
        println!("test_get_tee_pubkey: tee_pubkey: {:?}", tee_pubkey);
    }
}
