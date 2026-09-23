use super::types::{DerivationType, TrezorError};
use alloy_consensus::SignableTransaction;
use alloy_primitives::{
    hex, normalize_v, Address, ChainId, Signature, SignatureError, TxKind, B256, U256,
};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
use std::fmt;
use trezor_client::{
    client::{InteractionType, Trezor},
    protos,
    transport::ProtoMessage,
    TrezorMessage,
};

#[cfg(feature = "eip712")]
use crate::eip712::TrezorTypedData;
#[cfg(feature = "eip712")]
use alloy_dyn_abi::TypedData;
#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

// We require firmware that supports EIP-1559 transaction signing.
const FIRMWARE_1_MIN_VERSION: &str = ">=1.11.1";
const FIRMWARE_2_MIN_VERSION: &str = ">=2.5.1";

/// A Trezor Ethereum signer.
///
/// This is a simple wrapper around the [Trezor transport](Trezor).
///
/// Device operations are available through the asynchronous [`Signer`] and
/// [`alloy_network::TxSigner`] traits; [`alloy_signer::SignerSync`] is not implemented. The
/// underlying USB operations are blocking. Raw digest signing is unsupported, while
/// [`Signer::sign_message`] performs EIP-191 personal-message signing and requires confirmation on
/// the device.
///
/// With the `eip712` feature, [`Signer::sign_typed_data`] and [`Signer::sign_dynamic_typed_data`]
/// sign EIP-712 typed data. Firmware major version 2 (Trezor Model T and Safe devices) reviews the
/// typed data on the device, while firmware major version 1 (Trezor Model One) cannot display
/// typed data and only signs the precomputed domain separator and message hashes.
///
/// Devices with passphrase protection prompt for the passphrase on the device unless the signer
/// was created with [`TrezorSigner::new_with_passphrase`], which answers those prompts from the
/// host to open a hidden wallet.
pub struct TrezorSigner {
    derivation: DerivationType,
    session_id: Vec<u8>,
    firmware_version: semver::Version,
    passphrase: Option<String>,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

impl fmt::Debug for TrezorSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrezorSigner")
            .field("derivation", &self.derivation)
            .field("session_id", &hex::encode(&self.session_id))
            .field("firmware_version", &self.firmware_version)
            .field("passphrase", &self.passphrase.as_ref().map(|_| "<redacted>"))
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

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature> {
        self.sign_typed_data_inner(payload, domain).await.map_err(alloy_signer::Error::other)
    }

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_dynamic_typed_data(&self, payload: &TypedData) -> Result<Signature> {
        self.sign_dynamic_typed_data_inner(payload).await.map_err(alloy_signer::Error::other)
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
    /// Connects to the unique available Trezor device and reads the derived address.
    ///
    /// Exactly one device or emulator must be discoverable. Firmware major version 1 requires
    /// version 1.11.1 or newer, and major version 2 requires 2.5.1 or newer. `chain_id` applies
    /// only to transaction signing: it fills a missing transaction chain ID and rejects a
    /// different one.
    ///
    /// If passphrase protection is enabled, the device asks for the passphrase on its own screen.
    /// Use [`Self::new_with_passphrase`] to provide it from the host instead.
    pub async fn new(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
    ) -> Result<Self, TrezorError> {
        Self::connect(derivation, chain_id, None).await
    }

    /// Connects like [`Self::new`], but answers the device's passphrase prompts with `passphrase`
    /// to open the corresponding hidden wallet.
    ///
    /// An empty passphrase selects the standard wallet without a device prompt. The passphrase is
    /// ignored by devices configured to always enter it on the device.
    pub async fn new_with_passphrase(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
        passphrase: impl Into<String>,
    ) -> Result<Self, TrezorError> {
        Self::connect(derivation, chain_id, Some(passphrase.into())).await
    }

    #[instrument(skip(passphrase), ret)]
    async fn connect(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
        passphrase: Option<String>,
    ) -> Result<Self, TrezorError> {
        let mut signer = Self {
            derivation: derivation.clone(),
            session_id: vec![],
            firmware_version: semver::Version::new(0, 0, 0),
            passphrase,
            chain_id,
            address: Address::ZERO,
        };
        signer.initiate_session()?;
        signer.address = signer.get_address_with_path(&derivation).await?;
        Ok(signer)
    }

    fn check_version(version: &semver::Version) -> Result<(), TrezorError> {
        let min_version = match version.major {
            1 => FIRMWARE_1_MIN_VERSION,
            2 => FIRMWARE_2_MIN_VERSION,
            // unknown major version, possibly newer models that we don't know about yet
            // It's probably safe to assume they support EIP-1559.
            _ => return Ok(()),
        };

        let req = semver::VersionReq::parse(min_version)?;
        // Enforce firmware version is greater than "min_version"
        if !req.matches(version) {
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
        Self::check_version(&version)?;

        self.session_id = features.session_id().to_vec();
        self.firmware_version = version;

        Ok(())
    }

    fn get_client(&self) -> Result<Trezor, TrezorError> {
        let mut client = trezor_client::unique(false)?;
        client.init_device(Some(self.session_id.clone()))?;
        Ok(client)
    }

    /// Drives a device call to completion without logging the outgoing passphrase acknowledgement.
    /// `Trezor::call` traces every request with `Debug`, including plaintext `PassphraseAck`s.
    fn call_with_interaction<S: TrezorMessage, R: TrezorMessage>(
        &self,
        client: &mut Trezor,
        request: S,
    ) -> Result<R, TrezorError> {
        use protos::MessageType;

        let mut response = client.call_raw(request)?;
        loop {
            response = match response.message_type() {
                ty if ty == R::MESSAGE_TYPE => return decode_message(response),
                MessageType::MessageType_ButtonRequest => {
                    let _: protos::ButtonRequest = decode_message(response)?;
                    client.call_raw(protos::ButtonAck::new())?
                }
                MessageType::MessageType_PassphraseRequest => {
                    let request: protos::PassphraseRequest = decode_message(response)?;
                    client.call_raw(self.passphrase_ack(&request))?
                }
                MessageType::MessageType_Failure => {
                    let failure: protos::Failure = decode_message(response)?;
                    return Err(trezor_client::Error::FailureResponse(failure).into());
                }
                MessageType::MessageType_PinMatrixRequest => {
                    return Err(trezor_client::Error::UnexpectedInteractionRequest(
                        InteractionType::PinMatrix,
                    )
                    .into());
                }
                ty => return Err(trezor_client::Error::UnexpectedMessageType(ty).into()),
            }
        }
    }

    /// Answers a passphrase prompt with the configured passphrase, or asks the device to collect
    /// it on its own screen like [`trezor_client::client::handle_interaction`] does.
    fn passphrase_ack(&self, request: &protos::PassphraseRequest) -> protos::PassphraseAck {
        let mut ack = protos::PassphraseAck::new();
        match &self.passphrase {
            Some(passphrase) => ack.set_passphrase(passphrase.clone()),
            None if !request._on_device() => ack.set_on_device(true),
            None => {}
        }
        ack
    }

    /// Re-queries the address for this signer's derivation path.
    ///
    /// This does not request on-device display confirmation. [`Signer::address`] returns the
    /// address cached when this signer was constructed.
    pub async fn get_address(&self) -> Result<Address, TrezorError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Queries the address for `derivation` without requesting on-device display confirmation.
    #[instrument(ret)]
    pub async fn get_address_with_path(
        &self,
        derivation: &DerivationType,
    ) -> Result<Address, TrezorError> {
        let mut client = self.get_client()?;
        let mut req = protos::EthereumGetAddress::new();
        req.address_n = Self::convert_path(derivation);
        let address: protos::EthereumAddress = self.call_with_interaction(&mut client, req)?;
        Ok(address.address().parse()?)
    }

    /// Signs a legacy or EIP-1559 transaction and requires confirmation on the Trezor.
    #[doc(alias = "sign_transaction_inner")]
    async fn sign_tx_inner(
        &self,
        tx: &dyn SignableTransaction<Signature>,
    ) -> Result<Signature, TrezorError> {
        let mut client = self.get_client()?;
        let path = Self::convert_path(&self.derivation);

        match build_sign_request(tx)? {
            TrezorSignRequest::Legacy(req) => {
                let mut msg = protos::EthereumSignTx::new();
                msg.address_n = path;
                msg.set_nonce(req.nonce);
                msg.set_gas_price(req.gas_price);
                msg.set_gas_limit(req.gas_limit);
                msg.set_to(req.to);
                msg.set_value(req.value);
                if let Some(chain_id) = req.chain_id {
                    msg.set_chain_id(chain_id);
                }
                let mut data = req.data;
                msg.set_data_length(data.len() as u32);
                msg.set_data_initial_chunk(initial_chunk(&mut data));
                self.sign_tx_messages(&mut client, msg, data)
            }
            TrezorSignRequest::Eip1559(req) => {
                let mut msg = protos::EthereumSignTxEIP1559::new();
                msg.address_n = path;
                msg.set_nonce(req.nonce);
                msg.set_max_gas_fee(req.max_gas_fee);
                msg.set_max_priority_fee(req.max_priority_fee);
                msg.set_gas_limit(req.gas_limit);
                msg.set_to(req.to);
                msg.set_value(req.value);
                if let Some(chain_id) = req.chain_id {
                    msg.set_chain_id(chain_id);
                }
                msg.access_list = req.access_list;
                let mut data = req.data;
                msg.set_data_length(data.len() as u32);
                msg.set_data_initial_chunk(initial_chunk(&mut data));
                self.sign_tx_messages(&mut client, msg, data)
            }
        }
    }

    /// Sends a transaction signing request and the remaining calldata chunks the device asks for,
    /// and returns the signature from the final `EthereumTxRequest`.
    fn sign_tx_messages<S: TrezorMessage>(
        &self,
        client: &mut Trezor,
        request: S,
        mut data: Vec<u8>,
    ) -> Result<Signature, TrezorError> {
        let mut response: protos::EthereumTxRequest =
            self.call_with_interaction(client, request)?;
        while response.data_length() > 0 {
            let chunk_len = (response.data_length() as usize).min(data.len());
            let mut ack = protos::EthereumTxAck::new();
            ack.set_data_chunk(data.drain(..chunk_len).collect());
            response = self.call_with_interaction(client, ack)?;
        }
        signature_from_tx_request(&response)
    }

    #[instrument(skip(message), fields(message=hex::encode(message)), ret)]
    async fn sign_message_inner(&self, message: &[u8]) -> Result<Signature, TrezorError> {
        let mut client = self.get_client()?;
        let mut req = protos::EthereumSignMessage::new();
        req.address_n = Self::convert_path(&self.derivation);
        req.set_message(message.to_vec());
        let signature: protos::EthereumMessageSignature =
            self.call_with_interaction(&mut client, req)?;
        Ok(Signature::from_raw(signature.signature())?)
    }

    /// Firmware major version 1 (Trezor Model One) cannot display typed data and only signs the
    /// EIP-712 domain separator and message hashes.
    #[cfg(feature = "eip712")]
    const fn signs_typed_hash_only(&self) -> bool {
        self.firmware_version.major == 1
    }

    /// Signs a [`SolStruct`] as EIP-712 typed data and requires confirmation on the Trezor.
    #[cfg(feature = "eip712")]
    async fn sign_typed_data_inner<T: SolStruct>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature, TrezorError> {
        if self.signs_typed_hash_only() {
            return self
                .sign_typed_hash(domain.separator(), Some(payload.eip712_hash_struct()))
                .await;
        }
        self.sign_typed_data_flow(&TrezorTypedData::from_struct(payload, domain)?).await
    }

    /// Signs dynamic [`TypedData`] and requires confirmation on the Trezor.
    #[cfg(feature = "eip712")]
    async fn sign_dynamic_typed_data_inner(
        &self,
        payload: &TypedData,
    ) -> Result<Signature, TrezorError> {
        if self.signs_typed_hash_only() {
            // Matches `TypedData::eip712_signing_hash`, which omits the message hash when the
            // domain itself is the primary type.
            let message_hash = (payload.primary_type != Eip712Domain::NAME)
                .then(|| payload.hash_struct())
                .transpose()?;
            return self.sign_typed_hash(payload.domain.separator(), message_hash).await;
        }
        self.sign_typed_data_flow(&TrezorTypedData::from_typed_data(payload)?).await
    }

    /// Signs precomputed EIP-712 hashes with `EthereumSignTypedHash`, the only typed-data
    /// operation firmware major version 1 supports.
    #[cfg(feature = "eip712")]
    #[instrument(ret)]
    async fn sign_typed_hash(
        &self,
        domain_separator: B256,
        message_hash: Option<B256>,
    ) -> Result<Signature, TrezorError> {
        let mut client = self.get_client()?;
        let mut req = protos::EthereumSignTypedHash::new();
        req.address_n = Self::convert_path(&self.derivation);
        req.set_domain_separator_hash(domain_separator.to_vec());
        if let Some(message_hash) = message_hash {
            req.set_message_hash(message_hash.to_vec());
        }
        let signature: protos::EthereumTypedDataSignature =
            self.call_with_interaction(&mut client, req)?;
        Ok(Signature::from_raw(signature.signature())?)
    }

    /// Runs the interactive `EthereumSignTypedData` flow, answering the device's struct and value
    /// requests until it returns the signature.
    ///
    /// `trezor_client` has no wrapper for this multi-message workflow, so the raw messages are
    /// exchanged here, including the button and passphrase prompts that
    /// [`Self::call_with_interaction`] covers for single-message calls.
    #[cfg(feature = "eip712")]
    #[instrument(skip_all, fields(primary_type = data.primary_type()), ret)]
    async fn sign_typed_data_flow(&self, data: &TrezorTypedData) -> Result<Signature, TrezorError> {
        use protos::MessageType;

        let mut client = self.get_client()?;
        let mut req = protos::EthereumSignTypedData::new();
        req.address_n = Self::convert_path(&self.derivation);
        req.set_primary_type(data.primary_type().to_string());
        req.set_metamask_v4_compat(true);

        let mut response = client.call_raw(req)?;
        loop {
            response = match response.message_type() {
                MessageType::MessageType_EthereumTypedDataStructRequest => {
                    let request: protos::EthereumTypedDataStructRequest = decode_message(response)?;
                    let mut ack = protos::EthereumTypedDataStructAck::new();
                    ack.members =
                        cancel_on_error(&mut client, data.struct_members(request.name()))?;
                    client.call_raw(ack)?
                }
                MessageType::MessageType_EthereumTypedDataValueRequest => {
                    let request: protos::EthereumTypedDataValueRequest = decode_message(response)?;
                    let mut ack = protos::EthereumTypedDataValueAck::new();
                    ack.set_value(cancel_on_error(
                        &mut client,
                        data.encode_value(&request.member_path),
                    )?);
                    client.call_raw(ack)?
                }
                MessageType::MessageType_EthereumTypedDataSignature => {
                    let signature: protos::EthereumTypedDataSignature = decode_message(response)?;
                    return Ok(Signature::from_raw(signature.signature())?);
                }
                MessageType::MessageType_ButtonRequest => {
                    client.call_raw(protos::ButtonAck::new())?
                }
                MessageType::MessageType_PassphraseRequest => {
                    let request: protos::PassphraseRequest = decode_message(response)?;
                    client.call_raw(self.passphrase_ack(&request))?
                }
                MessageType::MessageType_Failure => {
                    let failure: protos::Failure = decode_message(response)?;
                    return Err(trezor_client::Error::FailureResponse(failure).into());
                }
                MessageType::MessageType_PinMatrixRequest => {
                    return Err(trezor_client::Error::UnexpectedInteractionRequest(
                        trezor_client::client::InteractionType::PinMatrix,
                    )
                    .into());
                }
                ty => return Err(trezor_client::Error::UnexpectedMessageType(ty).into()),
            };
        }
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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Eip1559Request {
    nonce: Vec<u8>,
    gas_limit: Vec<u8>,
    to: String,
    value: Vec<u8>,
    data: Vec<u8>,
    chain_id: Option<u64>,
    max_gas_fee: Vec<u8>,
    max_priority_fee: Vec<u8>,
    access_list: Vec<protos::ethereum_sign_tx_eip1559::EthereumAccessList>,
}

/// The dispatch payload for a Trezor signing call.
#[derive(Debug, Clone, PartialEq)]
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
                    .map(|item| protos::ethereum_sign_tx_eip1559::EthereumAccessList {
                        address: Some(address_to_trezor(&item.address)),
                        storage_keys: item.storage_keys.iter().map(|key| key.to_vec()).collect(),
                        ..Default::default()
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

/// Splits off the calldata that is sent with the initial signing request; the device asks for the
/// rest in chunks.
fn initial_chunk(data: &mut Vec<u8>) -> Vec<u8> {
    data.drain(..data.len().min(1024)).collect()
}

fn signature_from_trezor(x: trezor_client::client::Signature) -> Result<Signature, TrezorError> {
    let r = U256::from_be_bytes(x.r);
    let s = U256::from_be_bytes(x.s);
    let v =
        normalize_v(x.v).ok_or(TrezorError::SignatureError(SignatureError::InvalidParity(x.v)))?;
    Ok(Signature::new(r, s, v))
}

/// Parses the signature the device returns with its final `EthereumTxRequest`. For legacy
/// transactions the device returns the EIP-155 recovery ID, or a bare parity for chain IDs that
/// would overflow it, both of which [`normalize_v`] accepts.
fn signature_from_tx_request(
    response: &protos::EthereumTxRequest,
) -> Result<Signature, TrezorError> {
    let malformed = || trezor_client::Error::MalformedSignature;
    let r = response.signature_r().try_into().map_err(|_| malformed())?;
    let s = response.signature_s().try_into().map_err(|_| malformed())?;
    signature_from_trezor(trezor_client::client::Signature {
        r,
        s,
        v: response.signature_v() as u64,
    })
}

/// Parses a raw device response into the expected protobuf message.
fn decode_message<M: TrezorMessage>(response: ProtoMessage) -> Result<M, TrezorError> {
    response.into_message().map_err(|err| trezor_client::Error::Protobuf(err).into())
}

/// Aborts the device workflow before surfacing a host-side encoding error, so the device does not
/// stay blocked waiting for an acknowledgement.
#[cfg(feature = "eip712")]
fn cancel_on_error<T>(
    client: &mut Trezor,
    result: Result<T, TrezorError>,
) -> Result<T, TrezorError> {
    if result.is_err() {
        let _ = client.call_raw(protos::Cancel::new());
    }
    result
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
    async fn test_new_with_passphrase() {
        let standard = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();
        let hidden =
            TrezorSigner::new_with_passphrase(DerivationType::TrezorLive(0), Some(1), "hidden")
                .await
                .unwrap();
        assert_ne!(standard.address(), hidden.address());
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
    #[cfg(feature = "eip712")]
    async fn test_sign_typed_data() {
        use alloy_sol_types::{eip712_domain, sol};

        sol! {
            #[derive(serde::Serialize)]
            struct Message {
                string text;
                uint256 nonce;
            }
        }

        let trezor = TrezorSigner::new(DerivationType::TrezorLive(0), Some(1)).await.unwrap();
        let domain = eip712_domain! { name: "Alloy", version: "1", chain_id: 1, };
        let message = Message { text: "hello".into(), nonce: U256::from(1) };

        let sig = trezor.sign_typed_data(&message, &domain).await.unwrap();
        let hash = message.eip712_signing_hash(&domain);
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), trezor.address());

        let typed_data = TypedData::from_struct(&message, Some(domain));
        let sig = trezor.sign_dynamic_typed_data(&typed_data).await.unwrap();
        let hash = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), trezor.address());
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
