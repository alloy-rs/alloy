//! Ledger Ethereum app wrapper.

use std::{fmt::Debug, sync::Arc};

use crate::types::{DerivationType, LedgerError, INS, P1, P1_FIRST_0, P1_FIRST_1, P2};
use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, normalize_v, Address, ChainId, Signature, SignatureError, B256};
use alloy_signer::{Result, Signer};
use async_trait::async_trait;
use coins_ledger::{
    transports::{HidTransport, LedgerTransport},
    ApduCommand, Ledger,
};
use futures_util::lock::Mutex;

#[cfg(feature = "eip712")]
use alloy_dyn_abi::TypedData;
#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

/// Minimum Ledger Ethereum app version for EIP-712 typed data signing.
///
/// See <https://github.com/LedgerHQ/app-ethereum/issues/105#issuecomment-765316999>.
#[cfg(feature = "eip712")]
const EIP712_MIN_VERSION: &str = ">=1.6.0";

/// Minimum Ledger Ethereum app version for EIP-7702 authorization signing.
///
/// The Ethereum app added the instruction in 1.17.0; older builds reject it with status `0x6d00`.
#[cfg(feature = "eip7702")]
const EIP7702_MIN_VERSION: &str = ">=1.17.0";

/// A Ledger Ethereum signer.
///
/// This is a simple wrapper around a [`Ledger`] handle. It is generic over the
/// [`LedgerTransport`] that carries APDUs to the device and defaults to the USB HID transport;
/// [`Self::new_with_transport`] accepts any other transport, such as an emulator.
///
/// Device operations are available through the asynchronous [`Signer`] and
/// [`alloy_network::TxSigner`] traits; [`alloy_signer::SignerSync`] is not implemented. Raw digest
/// signing is unsupported, while [`Signer::sign_message`] performs EIP-191 personal-message
/// signing and requires confirmation on the device.
#[derive(Debug)]
pub struct LedgerSigner<T = HidTransport> {
    transport: Arc<Mutex<Ledger<T>>>,
    derivation: DerivationType,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

#[async_trait]
impl<T: LedgerTransport + Debug + Send> alloy_network::TxSigner<Signature> for LedgerSigner<T> {
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    #[doc(alias = "sign_tx")]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        let encoded = signing_payload(self.chain_id, tx)?;

        match encoded.as_slice() {
            // Ledger requires passing EIP712 data to a separate instruction
            #[cfg(feature = "eip712")]
            [0x19, 0x1, data @ ..] => {
                let domain_sep = data
                    .get(..32)
                    .ok_or_else(|| {
                        alloy_signer::Error::other(
                            "eip712 encoded data did not have a domain separator",
                        )
                    })
                    .map(B256::from_slice)?;

                let hash = data[32..]
                    .get(..32)
                    .ok_or_else(|| {
                        alloy_signer::Error::other("eip712 encoded data did not have hash struct")
                    })
                    .map(B256::from_slice)?;

                self.sign_typed_data_with_separator(&hash, &domain_sep)
                    .await
                    .map_err(alloy_signer::Error::other)
            }
            // Usual flow
            encoded => self.sign_tx_rlp(encoded).await.map_err(alloy_signer::Error::other),
        }
    }
}

#[async_trait]
impl<T: LedgerTransport + Debug + Send> Signer for LedgerSigner<T> {
    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        Err(alloy_signer::Error::UnsupportedOperation(
            alloy_signer::UnsupportedSignerOperation::SignHash,
        ))
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        let mut payload = path_to_bytes(&self.derivation).map_err(alloy_signer::Error::other)?;
        // Ensure message length fits into u32 as required by Ledger APDU format.
        let msg_len_u32 = u32::try_from(message.len())
            .map_err(|_| alloy_signer::Error::other("message too long (>4GiB)"))?;
        payload.extend_from_slice(&msg_len_u32.to_be_bytes());
        payload.extend_from_slice(message);

        self.sign_payload(INS::SIGN_PERSONAL_MESSAGE, &payload, VEncoding::Normalized)
            .await
            .map_err(alloy_signer::Error::other)
    }

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data<S: SolStruct + Send + Sync>(
        &self,
        payload: &S,
        domain: &Eip712Domain,
    ) -> Result<Signature> {
        self.sign_typed_data_(&payload.eip712_hash_struct(), domain)
            .await
            .map_err(alloy_signer::Error::other)
    }

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_dynamic_typed_data(&self, payload: &TypedData) -> Result<Signature> {
        self.sign_typed_data_(&payload.hash_struct()?, &payload.domain)
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

alloy_network::impl_into_wallet!(@[T: LedgerTransport + Debug + Send + 'static] LedgerSigner<T>);

impl LedgerSigner {
    /// Connects to the first compatible Ledger device and reads the derived address.
    ///
    /// The device must be unlocked with its Ethereum app open. `chain_id` applies only to
    /// transaction signing: it fills a missing transaction chain ID and rejects a different one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_signer_ledger::{HDPath, LedgerSigner};
    ///
    /// let ledger = LedgerSigner::new(HDPath::LedgerLive(0), Some(1)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
    ) -> Result<Self, LedgerError> {
        let mut transport = Ledger::open_first_hid()?;
        let address = Self::get_address_with_path_transport(&mut transport, &derivation).await?;
        debug!(%address, "Connected to Ledger");

        Ok(Self { transport: Mutex::new(transport).into(), derivation, chain_id, address })
    }
}

impl<T: LedgerTransport> LedgerSigner<T> {
    /// Creates a signer using an existing shared transport.
    ///
    /// The transport mutex serializes device operations and can be shared by signers using
    /// different derivation paths. `chain_id` has the same transaction-only validation behavior
    /// as in [`Self::new`].
    pub async fn new_with_transport(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
        transport: Arc<Mutex<Ledger<T>>>,
    ) -> Result<Self, LedgerError> {
        let address = {
            let mut transport_guard = transport.lock().await;
            Self::get_address_with_path_transport(&mut transport_guard, &derivation).await?
        };
        debug!(%address, "Connected to Ledger");

        Ok(Self { transport, derivation, chain_id, address })
    }

    /// Re-queries the address for this signer's derivation path.
    ///
    /// This does not request on-device display confirmation. [`Signer::address`] returns the
    /// address cached when this signer was constructed.
    pub async fn get_address(&self) -> Result<Address, LedgerError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Queries the address for `derivation` without requesting on-device display confirmation.
    pub async fn get_address_with_path(
        &self,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let mut transport = self.transport.lock().await;
        Self::get_address_with_path_transport(&mut transport, derivation).await
    }

    #[instrument(skip(transport))]
    async fn get_address_with_path_transport(
        transport: &mut Ledger<T>,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let path = path_to_bytes(derivation)?;
        let command = ApduCommand::builder()
            .cla(0xe0)
            .ins(INS::GET_PUBLIC_KEY as u8)
            .p1(P1::NON_CONFIRM as u8)
            .p2(P2::NO_CHAINCODE as u8)
            .data(&path)
            .build()?;

        debug!("Dispatching get_address request to ethereum app");
        let answer = transport.exchange(command).await?;
        let result = answer.data();
        if result.is_empty() {
            return Err(LedgerError::UnexpectedNullResponse);
        }

        let address = {
            // extract the address from the response
            let offset = 1 + result[0] as usize;
            let address_str = &result[offset + 1..offset + 1 + result[offset] as usize];
            let mut address = [0; 20];
            address.copy_from_slice(&hex::decode(address_str)?);
            address.into()
        };
        debug!(?address, "Received address from device");
        Ok(address)
    }

    /// Returns the semantic version of the Ledger Ethereum app.
    pub async fn version(&self) -> Result<semver::Version, LedgerError> {
        let mut transport = self.transport.lock().await;

        let command = ApduCommand::builder()
            .cla(0xe0)
            .ins(INS::GET_APP_CONFIGURATION as u8)
            .p1(P1::NON_CONFIRM as u8)
            .p2(P2::NO_CHAINCODE as u8)
            .build()?;

        debug!("Dispatching get_version");
        let answer = transport.exchange(command).await?;
        let data = answer.data();
        let &[_flags, major, minor, patch] = data else {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 4 });
        };
        let version = semver::Version::new(major as u64, minor as u64, patch as u64);
        debug!(%version, "Retrieved version from device");
        Ok(version)
    }

    /// Returns [`LedgerError::UnsupportedAppVersion`] if the Ledger Ethereum app does not satisfy
    /// `min_version`.
    #[cfg(any(feature = "eip712", feature = "eip7702"))]
    async fn require_app_version(&self, min_version: &'static str) -> Result<(), LedgerError> {
        let req = semver::VersionReq::parse(min_version)?;
        let version = self.version().await?;
        if !req.matches(&version) {
            return Err(LedgerError::UnsupportedAppVersion(min_version));
        }
        Ok(())
    }

    /// Signs an encoded transaction signing payload and requires confirmation on the Ledger.
    ///
    /// The payload must be legacy RLP or an EIP-2718 transaction type byte followed by its encoded
    /// payload. This method does not encode a transaction, inject or check a chain ID, or accept a
    /// transaction hash. Prefer [`alloy_network::TxSigner::sign_transaction`] when a
    /// [`SignableTransaction`] is available.
    #[doc(alias = "sign_transaction_rlp")]
    pub async fn sign_tx_rlp(&self, tx_rlp: &[u8]) -> Result<Signature, LedgerError> {
        let mut payload = path_to_bytes(&self.derivation)?;
        payload.extend_from_slice(tx_rlp);
        self.sign_payload(INS::SIGN, &payload, VEncoding::for_tx(tx_rlp)).await
    }

    #[cfg(feature = "eip712")]
    async fn sign_typed_data_with_separator(
        &self,
        hash_struct: &B256,
        separator: &B256,
    ) -> Result<Signature, LedgerError> {
        self.require_app_version(EIP712_MIN_VERSION).await?;

        let mut data = path_to_bytes(&self.derivation)?;
        data.extend_from_slice(separator.as_slice());
        data.extend_from_slice(hash_struct.as_slice());

        self.sign_payload(INS::SIGN_ETH_EIP_712, &data, VEncoding::Normalized).await
    }

    #[cfg(feature = "eip712")]
    async fn sign_typed_data_(
        &self,
        hash_struct: &B256,
        domain: &Eip712Domain,
    ) -> Result<Signature, LedgerError> {
        self.sign_typed_data_with_separator(hash_struct, &domain.separator()).await
    }

    /// Sign “auth data” per EIP-7702:
    /// msg = keccak256(0x05 ‖ rlp([chain_id, address, nonce]))
    ///
    /// Requires Ledger Ethereum app 1.17.0 or newer; older apps return
    /// [`LedgerError::UnsupportedAppVersion`].
    #[cfg(feature = "eip7702")]
    pub async fn sign_auth(
        &self,
        auth: &alloy_eip7702::Authorization,
    ) -> Result<Signature, LedgerError> {
        self.require_app_version(EIP7702_MIN_VERSION).await?;

        let path_bytes = path_to_bytes(&self.derivation)?;
        let tlv_payload = crate::utils::make_eip7702_tlv(auth.chain_id, &auth.address, auth.nonce);

        let tlv_length = (tlv_payload.len() as u16).to_be_bytes();

        let mut payload = Vec::with_capacity(path_bytes.len() + 2 + tlv_payload.len());
        payload.extend_from_slice(&path_bytes);
        payload.extend_from_slice(&tlv_length);
        payload.extend_from_slice(&tlv_payload);

        self.sign_payload(INS::SIGN_EIP7702_AUTHORIZATION, &payload, VEncoding::Normalized).await
    }

    /// Helper function for signing either transaction data, personal messages or EIP712 derived
    /// structs.
    #[instrument(err, skip_all, fields(command = ?command, payload = hex::encode(payload)))]
    async fn sign_payload(
        &self,
        command: INS,
        payload: &[u8],
        v_encoding: VEncoding,
    ) -> Result<Signature, LedgerError> {
        // @note Because tlv encoding is done on 7702 auth types sig, it checks if chunks are the
        // header or continuations. @note We need to mention the starter chunk first.
        let mut p1 =
            if command == INS::SIGN_EIP7702_AUTHORIZATION { P1_FIRST_1 } else { P1_FIRST_0 };
        let mut transport = self.transport.lock().await;

        let mut answer = None;
        // Workaround for https://github.com/LedgerHQ/app-ethereum/issues/409: apps before 1.13.0
        // treated the RLP as complete when a chunk boundary fell right before the EIP-155 chain
        // ID, and signed with chain ID 0. Keep the chain ID out of a trailing 3-byte chunk until
        // those apps are gone. `tests/speculos.rs` covers both the raw and the worked-around path.
        let chunk_size =
            (0..=255).rev().find(|i| payload.len() % i != 3).expect("true for any length");

        // Iterate in 255 byte chunks
        for chunk in payload.chunks(chunk_size) {
            let apdu = ApduCommand::builder()
                .cla(0xe0)
                .ins(command as u8)
                .p1(p1)
                .p2(P2::NO_CHAINCODE as u8)
                .data(chunk)
                .build()?;

            debug!(chunk = hex::encode(chunk), "Dispatching packet to device");
            let res = transport.exchange(apdu).await;
            let ans = match res {
                Ok(ans) => ans,
                Err(err) => {
                    error!(%err, "Failed to receive response from device");
                    return Err(err.into());
                }
            };
            debug!(response = hex::encode(ans.data()), "Received response from device");
            answer = Some(ans);

            // We need more data
            p1 = P1::MORE as u8;
        }
        drop(transport);

        let answer = answer.unwrap();
        let data = answer.data();
        if data.len() != 65 {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 65 });
        }

        let parity = v_encoding.parity(data[0])?;
        let sig = Signature::from_bytes_and_parity(&data[1..], parity);
        debug!(?sig, "Received signature from device");
        Ok(sig)
    }
}

/// Applies the signer's `chain_id` to `tx` and returns the payload to send to the device.
fn signing_payload(
    chain_id: Option<ChainId>,
    tx: &mut dyn SignableTransaction<Signature>,
) -> Result<Vec<u8>> {
    if let Some(chain_id) = chain_id {
        if !tx.set_chain_id_checked(chain_id) {
            return Err(alloy_signer::Error::TransactionChainIdMismatch {
                signer: chain_id,
                // we can only end up here if the tx has a chain id
                tx: tx.chain_id().unwrap(),
            });
        }
    }
    Ok(tx.encoded_for_signing())
}

/// Encodes a derivation path as the device expects it: the depth, then each big-endian index.
fn path_to_bytes(derivation: &DerivationType) -> Result<Vec<u8>, LedgerError> {
    let derivation = derivation.to_string();
    let elements = derivation.split('/').skip(1).collect::<Vec<_>>();
    let depth = elements.len();

    let mut bytes = vec![depth as u8];
    for derivation_index in elements {
        let hardened = derivation_index.contains('\'');
        let mut index = derivation_index.replace('\'', "").parse::<u32>()?;
        if hardened {
            index |= 0x80000000;
        }

        bytes.extend(index.to_be_bytes());
    }

    Ok(bytes)
}

/// How the device encodes the signature parity in the first byte of its response.
///
/// See [app-ethereum](https://github.com/LedgerHQ/app-ethereum/blob/master/src/features/sign_tx/ui_common_sign_tx.c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VEncoding {
    /// `0`/`1` or `27`/`28`: typed transactions, personal messages, EIP-712 and EIP-7702
    /// authorizations.
    Normalized,
    /// Legacy transactions: `27 + parity`, or `2 * chain_id + 35 + parity` truncated to a single
    /// byte if the transaction has a chain ID.
    Legacy,
}

impl VEncoding {
    /// Returns the encoding the device uses when signing `tx_rlp`.
    fn for_tx(tx_rlp: &[u8]) -> Self {
        // Legacy transactions are an RLP list, typed transactions start with their type byte.
        if tx_rlp.first().is_some_and(|&b| b >= 0xc0) {
            Self::Legacy
        } else {
            Self::Normalized
        }
    }

    /// Decodes the y-parity from the `v` byte returned by the device.
    fn parity(self, v: u8) -> Result<bool, LedgerError> {
        match self {
            Self::Normalized => normalize_v(v as u64)
                .ok_or(LedgerError::SignatureError(SignatureError::InvalidParity(v as u64))),
            // `27` and `2 * chain_id + 35` are odd, and truncation to a byte or the `+ 2` the
            // device adds for `xGTn` keep the low bit, so an even byte means an odd y-parity.
            Self::Legacy => Ok(v.is_multiple_of(2)),
        }
    }
}

#[cfg(test)]
mod v_encoding_tests {
    use super::*;
    use alloy_consensus::{TxEip1559, TxLegacy};

    /// The `v` byte the device returns for an EIP-155 legacy transaction.
    const fn eip155_v(chain_id: u64, parity: bool) -> u8 {
        (2 * chain_id + 35 + parity as u64) as u8
    }

    #[test]
    fn legacy_eip155_parity() {
        for chain_id in [1, 110, 111, 250, 369, 69420] {
            for parity in [false, true] {
                let v = eip155_v(chain_id, parity);
                assert_eq!(VEncoding::Legacy.parity(v).unwrap(), parity, "{chain_id} {v}");
                // xGTn
                let v = v.wrapping_add(2);
                assert_eq!(VEncoding::Legacy.parity(v).unwrap(), parity, "{chain_id} {v}");
            }
        }
    }

    #[test]
    fn legacy_pre_eip155_parity() {
        assert!(!VEncoding::Legacy.parity(27).unwrap());
        assert!(VEncoding::Legacy.parity(28).unwrap());
    }

    #[test]
    fn normalized_parity() {
        assert!(!VEncoding::Normalized.parity(0).unwrap());
        assert!(VEncoding::Normalized.parity(1).unwrap());
        assert!(!VEncoding::Normalized.parity(27).unwrap());
        assert!(VEncoding::Normalized.parity(28).unwrap());
        assert!(matches!(
            VEncoding::Normalized.parity(2),
            Err(LedgerError::SignatureError(SignatureError::InvalidParity(2)))
        ));
    }

    #[test]
    fn tx_encoding() {
        let legacy = TxLegacy { chain_id: Some(1), ..Default::default() };
        assert_eq!(VEncoding::for_tx(&legacy.encoded_for_signing()), VEncoding::Legacy);
        let pre_eip155 = TxLegacy { chain_id: None, ..Default::default() };
        assert_eq!(VEncoding::for_tx(&pre_eip155.encoded_for_signing()), VEncoding::Legacy);
        let typed = TxEip1559 { chain_id: 1, ..Default::default() };
        assert_eq!(VEncoding::for_tx(&typed.encoded_for_signing()), VEncoding::Normalized);
    }
}

#[cfg(all(test, any(feature = "eip712", feature = "eip7702")))]
mod app_version_tests {
    use super::*;

    fn matches(req: &str, version: &str) -> bool {
        semver::VersionReq::parse(req).unwrap().matches(&semver::Version::parse(version).unwrap())
    }

    #[cfg(feature = "eip712")]
    #[test]
    fn eip712_min_version() {
        assert!(!matches(EIP712_MIN_VERSION, "1.5.9"));
        assert!(matches(EIP712_MIN_VERSION, "1.6.0"));
    }

    #[cfg(feature = "eip7702")]
    #[test]
    fn eip7702_min_version() {
        assert!(!matches(EIP7702_MIN_VERSION, "1.16.2"));
        assert!(matches(EIP7702_MIN_VERSION, "1.17.0"));
        assert!(matches(EIP7702_MIN_VERSION, "1.23.0"));
    }
}

#[cfg(test)]
mod signing_payload_tests {
    use super::*;
    use alloy_consensus::TxLegacy;

    #[test]
    fn signer_chain_id_is_applied_before_encoding() {
        let mut tx = TxLegacy::default();
        let payload = signing_payload(Some(1), &mut tx).unwrap();
        assert_eq!(tx.chain_id, Some(1));
        assert_eq!(payload, tx.encoded_for_signing());
    }

    #[test]
    fn conflicting_chain_id_is_rejected() {
        let mut tx = TxLegacy { chain_id: Some(2), ..Default::default() };
        let err = signing_payload(Some(1), &mut tx).unwrap_err();
        assert!(matches!(
            err,
            alloy_signer::Error::TransactionChainIdMismatch { signer: 1, tx: 2 }
        ));
        assert_eq!(tx.chain_id, Some(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_derivation_path_returns_error() {
        let err = path_to_bytes(&DerivationType::Other("m/44'/invalid/0".into())).unwrap_err();
        assert!(matches!(err, LedgerError::InvalidDerivationPath(_)));
    }
}
