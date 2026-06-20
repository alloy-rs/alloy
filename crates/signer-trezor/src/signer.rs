use super::types::{DerivationType, TrezorError};
use alloy_consensus::SignableTransaction;
use alloy_dyn_abi::TypedData;
use alloy_primitives::{
    hex, normalize_v, Address, ChainId, Signature, SignatureError, TxKind, B256, U256,
};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
use std::fmt;
use trezor_client::client::Trezor;

// we need firmware that supports EIP-1559 and EIP-712
const FIRMWARE_1_MIN_VERSION: &str = ">=1.11.1";
const FIRMWARE_2_MIN_VERSION: &str = ">=2.5.1";

/// A Trezor Ethereum signer.
///
/// This is a simple wrapper around the [Trezor transport](Trezor).
///
/// Note that this wallet only supports asynchronous operations. Calling a non-asynchronous method
/// will always return an error.
pub struct TrezorSigner {
    derivation: DerivationType,
    session_id: Vec<u8>,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

impl fmt::Debug for TrezorSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrezorSigner")
            .field("derivation", &self.derivation)
            .field("session_id", &hex::encode(&self.session_id))
            .field("address", &self.address)
            .finish()
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl Signer for TrezorSigner {
    #[inline]
    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        Err(alloy_signer::Error::UnsupportedOperation(
            alloy_signer::UnsupportedSignerOperation::SignHash,
        ))
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        self.sign_message_inner(message).await.map_err(alloy_signer::Error::other)
    }

    #[inline]
    async fn sign_dynamic_typed_data(
        &self,
        payload: &TypedData,
    ) -> Result<Signature> {
        self.sign_typed_data_inner(payload)
            .await
            .map_err(alloy_signer::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl alloy_network::TxSigner<Signature> for TrezorSigner {
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    #[doc(alias = "sign_tx")]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_tx_inner(tx).await)
    }
}

alloy_network::impl_into_wallet!(TrezorSigner);

impl TrezorSigner {
    /// Instantiates a new Trezor signer.
    #[instrument(ret)]
    pub async fn new(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
    ) -> Result<Self, TrezorError> {
        let mut signer = Self {
            derivation: derivation.clone(),
            chain_id,
            address: Address::ZERO,
            session_id: vec![],
        };
        signer.initiate_session()?;
        signer.address = signer.get_address_with_path(&derivation).await?;
        Ok(signer)
    }

    fn check_version(version: semver::Version) -> Result<(), TrezorError> {
        let min_version = match version.major {
            1 => FIRMWARE_1_MIN_VERSION,
            2 => FIRMWARE_2_MIN_VERSION,
            // unknown major version, possibly newer models that we don't know about yet
            // it's probably safe to assume they support EIP-1559 and EIP-712
            _ => return Ok(()),
        };

        let req = semver::VersionReq::parse(min_version)?;
        // Enforce firmware version is greater than "min_version"
        if !req.matches(&version) {
            return Err(TrezorError::UnsupportedFirmwareVersion(min_version.to_string()));
        }

        Ok(())
    }

    fn initiate_session(&mut self) -> Result<(), TrezorError> {
        let mut client = trezor_client::unique(false)?;
        client.init_device(None)?;

        let features = client.features().ok_or(TrezorError::Features)?;
        let version = semver::Version::new(
            features.major_version() as u64,
            features.minor_version() as u64,
            features.patch_version() as u64,
        );
        Self::check_version(version)?;

        self.session_id = features.session_id().to_vec();

        Ok(())
    }

    fn get_client(&self) -> Result<Trezor, TrezorError> {
        let mut client = trezor_client::unique(false)?;
        client.init_device(Some(self.session_id.clone()))?;
        Ok(client)
    }

    /// Get the account which corresponds to our derivation path
    pub async fn get_address(&self) -> Result<Address, TrezorError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Gets the account which corresponds to the provided derivation path
    #[instrument(ret)]
    pub async fn get_address_with_path(
        &self,
        derivation: &DerivationType,
    ) -> Result<Address, TrezorError> {
        let mut client = self.get_client()?;
        let address_str = client.ethereum_get_address(Self::convert_path(derivation))?;
        Ok(address_str.parse()?)
    }

    /// Signs an Ethereum transaction (requires confirmation on the Trezor).
    ///
    /// Does not apply EIP-155.
    #[doc(alias = "sign_transaction_inner")]
    async fn sign_tx_inner(
        &self,
        tx: &dyn SignableTransaction<Signature>,
    ) -> Result<Signature, TrezorError> {
        let mut client = self.get_client()?;
        let path = Self::convert_path(&self.derivation);
        let request = build_sign_request(tx)?;

        let signature = match request {
            TrezorSignRequest::Legacy(req) => client.ethereum_sign_tx(
                path,
                req.nonce,
                req.gas_price,
                req.gas_limit,
                req.to,
                req.value,
                req.data,
                req.chain_id,
            ),
            TrezorSignRequest::Eip1559(req) => client.ethereum_sign_eip1559_tx(
                path,
                req.nonce,
                req.gas_limit,
                req.to,
                req.value,
                req.data,
                req.chain_id,
                req.max_gas_fee,
                req.max_priority_fee,
                req.access_list,
            ),
        }?;
        signature_from_trezor(signature)
    }

    #[instrument(skip(message), fields(message=hex::encode(message)), ret)]
    async fn sign_message_inner(&self, message: &[u8]) -> Result<Signature, TrezorError> {
        let mut client = self.get_client()?;
        let apath = Self::convert_path(&self.derivation);
        let signature = client.ethereum_sign_message(message.into(), apath)?;
        signature_from_trezor(signature)
    }

    async fn sign_typed_data_inner(
        &self,
        data: &TypedData,
    ) -> Result<Signature, TrezorError> {
        let mut types_json: serde_json::Map<String, serde_json::Value> =
            serde_json::to_value(&data.resolver)
                .map_err(TrezorError::Eip712)?
                .as_object()
                .cloned()
                .unwrap_or_default();

        let domain_json =
            serde_json::to_value(&data.domain).map_err(TrezorError::Eip712)?;

        if !types_json.contains_key("EIP712Domain") {
            let mut domain_fields = Vec::new();
            let domain = &data.domain;
            if domain.name.is_some() {
                domain_fields.push(serde_json::json!({"name": "name", "type": "string"}));
            }
            if domain.version.is_some() {
                domain_fields.push(serde_json::json!({"name": "version", "type": "string"}));
            }
            if domain.chain_id.is_some() {
                domain_fields.push(serde_json::json!({"name": "chainId", "type": "uint256"}));
            }
            if domain.verifying_contract.is_some() {
                domain_fields.push(serde_json::json!({"name": "verifyingContract", "type": "address"}));
            }
            if domain.salt.is_some() {
                domain_fields.push(serde_json::json!({"name": "salt", "type": "bytes32"}));
            }
            types_json.insert(
                "EIP712Domain".to_string(),
                serde_json::Value::Array(domain_fields),
            );
        }

        let mut client = self.get_client()?;
        let path = Self::convert_path(&self.derivation);

        let sig = client.ethereum_sign_typed_data(
            path,
            &data.primary_type,
            &types_json,
            &domain_json,
            &data.message,
        )?;
        signature_from_trezor(sig)
    }

    // helper which converts a derivation path to [u32]
    fn convert_path(derivation: &DerivationType) -> Vec<u32> {
        let derivation = derivation.to_string();
        let elements = derivation.split('/').skip(1).collect::<Vec<_>>();

        let mut path = vec![];
        for derivation_index in elements {
            let hardened = derivation_index.contains('\'');
            let mut index = derivation_index.replace('\'', "").parse::<u32>().unwrap();
            if hardened {
                index |= 0x80000000;
            }
            path.push(index);
        }

        path
    }
}

/// Parameters for a Trezor legacy transaction signing request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyRequest {
    nonce: Vec<u8>,
    gas_price: Vec<u8>,
    gas_limit: Vec<u8>,
    to: String,
    value: Vec<u8>,
    data: Vec<u8>,
    chain_id: Option<u64>,
}

/// Parameters for a Trezor EIP-1559 transaction signing request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Eip1559Request {
    nonce: Vec<u8>,
    gas_limit: Vec<u8>,
    to: String,
    value: Vec<u8>,
    data: Vec<u8>,
    chain_id: Option<u64>,
    max_gas_fee: Vec<u8>,
    max_priority_fee: Vec<u8>,
    access_list: Vec<trezor_client::client::AccessListItem>,
}

/// The dispatch payload for a Trezor signing call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrezorSignRequest {
    Legacy(LegacyRequest),
    Eip1559(Eip1559Request),
}

/// Selects the correct Trezor signing API based on the transaction's EIP-2718 type, and gathers
/// the parameters by calling the [`SignableTransaction`] / [`alloy_consensus::Transaction`] trait
/// methods rather than downcasting to a concrete transaction type.
///
/// This must dispatch on `tx.is_eip1559()` (i.e. the EIP-2718 type byte) so that wrapper types
/// such as `TypedTransaction::Eip1559` or network-specific signable wrappers around
/// [`alloy_consensus::TxEip1559`] are still routed to the EIP-1559 Trezor API. Routing them to
/// the legacy API instead causes Trezor to sign the EIP-155 legacy preimage and the resulting
/// signature recovers to a different address than the one displayed on the device.
pub(crate) fn build_sign_request(
    tx: &dyn SignableTransaction<Signature>,
) -> Result<TrezorSignRequest, TrezorError> {
    let nonce = u64_to_trezor(tx.nonce());
    let gas_limit = u64_to_trezor(tx.gas_limit());
    let to = match tx.kind() {
        TxKind::Call(to) => address_to_trezor(&to),
        TxKind::Create => String::new(),
    };
    let value = u256_to_trezor(tx.value());
    let data = tx.input().to_vec();
    let chain_id = tx.chain_id();

    if tx.is_eip1559() {
        let max_gas_fee = u128_to_trezor(tx.max_fee_per_gas());
        let max_priority_fee = u128_to_trezor(tx.max_priority_fee_per_gas().unwrap_or_default());
        let access_list = tx
            .access_list()
            .map(|al| {
                al.0.iter()
                    .map(|item| trezor_client::client::AccessListItem {
                        address: address_to_trezor(&item.address),
                        storage_keys: item.storage_keys.iter().map(|key| key.to_vec()).collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(TrezorSignRequest::Eip1559(Eip1559Request {
            nonce,
            gas_limit,
            to,
            value,
            data,
            chain_id,
            max_gas_fee,
            max_priority_fee,
            access_list,
        }))
    } else if tx.is_legacy() {
        let gas_price = u128_to_trezor(tx.max_fee_per_gas());
        Ok(TrezorSignRequest::Legacy(LegacyRequest {
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data,
            chain_id,
        }))
    } else {
        Err(TrezorError::UnsupportedTransactionType(tx.ty()))
    }
}

fn u64_to_trezor(x: u64) -> Vec<u8> {
    let bytes = x.to_be_bytes();
    bytes[x.leading_zeros() as usize / 8..].to_vec()
}

fn u128_to_trezor(x: u128) -> Vec<u8> {
    let bytes = x.to_be_bytes();
    bytes[x.leading_zeros() as usize / 8..].to_vec()
}

fn u256_to_trezor(x: U256) -> Vec<u8> {
    let bytes = x.to_be_bytes::<32>();
    bytes[x.leading_zeros() / 8..].to_vec()
}

fn address_to_trezor(x: &Address) -> String {
    format!("{x:?}")
}

fn signature_from_trezor(x: trezor_client::client::Signature) -> Result<Signature, TrezorError> {
    let r = U256::from_be_bytes(x.r);
    let s = U256::from_be_bytes(x.s);
    let v =
        normalize_v(x.v).ok_or(TrezorError::SignatureError(SignatureError::InvalidParity(x.v)))?;
    Ok(Signature::new(r, s, v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::{Transaction, TxEip1559, TxEip2930, TxLegacy, Typed2718};
    use alloy_network::{EthereumWallet, NetworkTransactionBuilder, TransactionBuilder};
    use alloy_primitives::{address, b256, Bytes};
    use alloy_rpc_types_eth::{AccessList, AccessListItem, TransactionRequest};

    #[tokio::test]
    #[ignore]
    // Replace this with your ETH addresses.
    async fn test_get_address() {
        // Instantiate it with the default trezor derivation path
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(1), Some(1)).await.unwrap();
        assert_eq!(
            trezor.get_address().await.unwrap(),
            address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
        );
        assert_eq!(
            trezor.get_address_with_path(&DerivationType::TrezorLive(0)).await.unwrap(),
            address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_message() {
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();
        let message = "hello world";
        let sig = trezor.sign_message(message.as_bytes()).await.unwrap();
        let addr = trezor.get_address().await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), addr);
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_tx() {
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();

        // approve uni v2 router 0xff
        let data = hex::decode("095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap();
        let _tx = TransactionRequest::default()
            .to(address!("2ed7afa17473e17ac59908f088b4371d28585476"))
            .with_gas_limit(1000000)
            .with_gas_price(400e9 as u128)
            .with_nonce(5)
            .with_input(data)
            .with_value(U256::from(100e18 as u128))
            .build(&EthereumWallet::new(trezor))
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_big_data_tx() {
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();

        // invalid data
        let big_data = hex::decode("095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string()+ &"ff".repeat(1032*2) + "aa").unwrap();
        let _tx = TransactionRequest::default()
            .to(address!("2ed7afa17473e17ac59908f088b4371d28585476"))
            .with_gas_limit(1000000)
            .with_gas_price(400e9 as u128)
            .with_nonce(5)
            .with_input(big_data)
            .with_value(U256::from(100e18 as u128))
            .build(&EthereumWallet::new(trezor))
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_empty_txes() {
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();
        TransactionRequest::default()
            .to(address!("2ed7afa17473e17ac59908f088b4371d28585476"))
            .with_gas_price(1)
            .build(&EthereumWallet::new(trezor))
            .await
            .unwrap();

        let data = hex::decode("095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap();

        // Contract creation (empty `to`, with data) should show on the trezor device as:
        //  ` "0 Wei ETH
        //  ` new contract?"
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();
        {
            let _tx = TransactionRequest::default()
                .into_create()
                .with_input(data)
                .with_gas_price(1)
                .build(&EthereumWallet::new(trezor))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_eip1559_tx() {
        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();

        // approve uni v2 router 0xff
        let data = hex::decode("095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap();

        let lst = AccessList(vec![
            AccessListItem {
                address: address!("8ba1f109551bd432803012645ac136ddd64dba72"),
                storage_keys: vec![
                    b256!("0000000000000000000000000000000000000000000000000000000000000000"),
                    b256!("0000000000000000000000000000000000000000000000000000000000000042"),
                ],
            },
            AccessListItem {
                address: address!("2ed7afa17473e17ac59908f088b4371d28585476"),
                storage_keys: vec![
                    b256!("0000000000000000000000000000000000000000000000000000000000000000"),
                    b256!("0000000000000000000000000000000000000000000000000000000000000042"),
                ],
            },
        ]);

        let _tx = TransactionRequest::default()
            .to(address!("2ed7afa17473e17ac59908f088b4371d28585476"))
            .with_gas_limit(1000000)
            .max_fee_per_gas(400e9 as u128)
            .max_priority_fee_per_gas(400e9 as u128)
            .with_nonce(5)
            .with_input(data)
            .with_access_list(lst)
            .with_value(U256::from(100e18 as u128))
            .build(&EthereumWallet::new(trezor))
            .await
            .unwrap();
    }

    /// Helpers for the dispatch tests below: construct sample transactions matching the
    /// Foundry repro from the bug report.
    fn sample_eip1559_tx() -> TxEip1559 {
        TxEip1559 {
            chain_id: 42431,
            nonce: 1,
            gas_limit: 356_613,
            max_fee_per_gas: 40_000_000_001,
            max_priority_fee_per_gas: 1,
            to: TxKind::Call(Address::ZERO),
            value: U256::ZERO,
            access_list: Default::default(),
            input: Bytes::new(),
        }
    }

    fn sample_legacy_tx() -> TxLegacy {
        TxLegacy {
            chain_id: Some(42431),
            nonce: 1,
            gas_price: 40_000_000_001,
            gas_limit: 356_613,
            to: TxKind::Call(Address::ZERO),
            value: U256::ZERO,
            input: Bytes::new(),
        }
    }

    fn sample_eip2930_tx() -> TxEip2930 {
        TxEip2930 {
            chain_id: 42431,
            nonce: 1,
            gas_price: 40_000_000_001,
            gas_limit: 356_613,
            to: TxKind::Call(Address::ZERO),
            value: U256::ZERO,
            access_list: Default::default(),
            input: Bytes::new(),
        }
    }

    /// A transparent wrapper around any [`SignableTransaction`]. It is a *different* concrete
    /// type from the wrapped value, so the previous `(tx as &dyn Any).downcast_ref::<TxEip1559>()`
    /// dispatch in the Trezor signer fails for it. Used to reproduce the Foundry bug where a
    /// wrapper around a `TxEip1559` was incorrectly signed as legacy.
    #[derive(Debug)]
    struct SignableWrapper<T>(T);

    impl<T: Typed2718> Typed2718 for SignableWrapper<T> {
        fn ty(&self) -> u8 {
            self.0.ty()
        }
    }

    impl<T: Transaction> Transaction for SignableWrapper<T> {
        fn chain_id(&self) -> Option<ChainId> {
            self.0.chain_id()
        }
        fn nonce(&self) -> u64 {
            self.0.nonce()
        }
        fn gas_limit(&self) -> u64 {
            self.0.gas_limit()
        }
        fn gas_price(&self) -> Option<u128> {
            self.0.gas_price()
        }
        fn max_fee_per_gas(&self) -> u128 {
            self.0.max_fee_per_gas()
        }
        fn max_priority_fee_per_gas(&self) -> Option<u128> {
            self.0.max_priority_fee_per_gas()
        }
        fn max_fee_per_blob_gas(&self) -> Option<u128> {
            self.0.max_fee_per_blob_gas()
        }
        fn priority_fee_or_price(&self) -> u128 {
            self.0.priority_fee_or_price()
        }
        fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
            self.0.effective_gas_price(base_fee)
        }
        fn is_dynamic_fee(&self) -> bool {
            self.0.is_dynamic_fee()
        }
        fn kind(&self) -> TxKind {
            self.0.kind()
        }
        fn is_create(&self) -> bool {
            self.0.is_create()
        }
        fn value(&self) -> U256 {
            self.0.value()
        }
        fn input(&self) -> &Bytes {
            self.0.input()
        }
        fn access_list(
            &self,
        ) -> Option<&alloy_consensus::private::alloy_eips::eip2930::AccessList> {
            self.0.access_list()
        }
        fn blob_versioned_hashes(&self) -> Option<&[B256]> {
            self.0.blob_versioned_hashes()
        }
        fn authorization_list(
            &self,
        ) -> Option<&[alloy_consensus::private::alloy_eips::eip7702::SignedAuthorization]> {
            self.0.authorization_list()
        }
    }

    impl<T: SignableTransaction<Signature>> SignableTransaction<Signature> for SignableWrapper<T> {
        fn set_chain_id(&mut self, chain_id: ChainId) {
            self.0.set_chain_id(chain_id);
        }
        fn encode_for_signing(&self, out: &mut dyn alloy_consensus::private::alloy_rlp::BufMut) {
            self.0.encode_for_signing(out);
        }
        fn payload_len_for_signature(&self) -> usize {
            self.0.payload_len_for_signature()
        }
    }

    /// Concrete `TxEip1559` must dispatch to the EIP-1559 Trezor API.
    #[test]
    fn build_sign_request_dispatches_concrete_eip1559_to_eip1559_api() {
        let tx = sample_eip1559_tx();
        let request = build_sign_request(&tx as &dyn SignableTransaction<Signature>).unwrap();
        assert!(
            matches!(request, TrezorSignRequest::Eip1559(_)),
            "concrete TxEip1559 must dispatch to the EIP-1559 path, got {request:?}",
        );
    }

    /// Regression test for the Foundry bug: a wrapper around `TxEip1559` that implements
    /// `SignableTransaction<Signature>` (so its EIP-2718 type is 0x02 and `encoded_for_signing`
    /// is a valid type-2 preimage) must still dispatch to the EIP-1559 Trezor API. The
    /// previous `(tx as &dyn Any).downcast_ref::<TxEip1559>()` check failed for any wrapper
    /// type and wrongly fell through to the legacy signing path, which caused Trezor to sign
    /// the EIP-155 legacy preimage and the resulting envelope recovered to a different signer
    /// than the address shown on the device.
    #[test]
    fn build_sign_request_dispatches_wrapped_eip1559_to_eip1559_api() {
        let inner = sample_eip1559_tx();
        let wrapped = SignableWrapper(inner.clone());
        // Sanity check: the wrapper is a different concrete type than `TxEip1559`, so a
        // downcast based dispatch would have rejected it.
        assert!(
            (&wrapped as &dyn std::any::Any).downcast_ref::<TxEip1559>().is_none(),
            "wrapper must not be downcastable to TxEip1559",
        );
        // It is, however, still an EIP-1559 transaction by EIP-2718 type.
        assert!(wrapped.is_eip1559());
        assert_eq!(wrapped.ty(), 0x02);

        let wrapped_request =
            build_sign_request(&wrapped as &dyn SignableTransaction<Signature>).unwrap();
        assert!(
            matches!(wrapped_request, TrezorSignRequest::Eip1559(_)),
            "wrapper around TxEip1559 must dispatch to the EIP-1559 path, got {wrapped_request:?}",
        );

        // The wrapper request must also produce the same parameters as the concrete one,
        // so that signing the wrapper is byte-for-byte equivalent to signing the inner tx.
        let concrete_request =
            build_sign_request(&inner as &dyn SignableTransaction<Signature>).unwrap();
        assert_eq!(wrapped_request, concrete_request);
    }

    /// Legacy transactions must still dispatch to the legacy Trezor API.
    #[test]
    fn build_sign_request_dispatches_legacy_to_legacy_api() {
        let tx = sample_legacy_tx();
        let request = build_sign_request(&tx as &dyn SignableTransaction<Signature>).unwrap();
        assert!(
            matches!(request, TrezorSignRequest::Legacy(_)),
            "TxLegacy must dispatch to the legacy path, got {request:?}",
        );
    }

    /// A wrapper around a legacy transaction must also dispatch to the legacy API.
    #[test]
    fn build_sign_request_dispatches_wrapped_legacy_to_legacy_api() {
        let wrapped = SignableWrapper(sample_legacy_tx());
        let request = build_sign_request(&wrapped as &dyn SignableTransaction<Signature>).unwrap();
        assert!(
            matches!(request, TrezorSignRequest::Legacy(_)),
            "wrapper around TxLegacy must dispatch to the legacy path, got {request:?}",
        );
    }

    #[test]
    fn build_sign_request_rejects_unsupported_typed_transactions() {
        let tx = sample_eip2930_tx();
        let err = build_sign_request(&tx as &dyn SignableTransaction<Signature>).unwrap_err();
        assert!(
            matches!(err, TrezorError::UnsupportedTransactionType(0x01)),
            "EIP-2930 must be rejected instead of routed to the legacy path, got {err:?}",
        );
    }
}
