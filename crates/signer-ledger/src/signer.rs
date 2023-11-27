//! Ledger Ethereum app wrapper.

use crate::types::{DerivationType, LedgerError, INS, P1, P1_FIRST, P2};
use alloy_primitives::{hex, Address};
use alloy_signer::{Result, Signature, Signer};
use async_trait::async_trait;
use coins_ledger::{
    common::{APDUCommand, APDUData},
    transports::{Ledger, LedgerAsync},
};
use futures_util::lock::Mutex;
use tracing::field;

// TODO: Ledger futures aren't Send.
use futures_executor::block_on;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

/// A Ledger Ethereum signer.
///
/// This is a simple wrapper around the [Ledger transport](Ledger).
///
/// Note that this signer only supports asynchronous operations. Calling a non-asynchronous method
/// will always return an error.
#[derive(Debug)]
pub struct LedgerSigner {
    transport: Mutex<Ledger>,
    derivation: DerivationType,
    pub(crate) address: Address,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for LedgerSigner {
    #[inline]
    async fn sign_message_async(&self, message: &[u8]) -> Result<Signature> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(&(message.len() as u32).to_be_bytes());
        payload.extend_from_slice(message);

        self.sign_payload(INS::SIGN_PERSONAL_MESSAGE, &payload)
            .await
            .map_err(alloy_signer::Error::other)
    }

    #[cfg(TODO)]
    #[inline]
    async fn sign_transaction_async(&self, tx: &TypedTransaction) -> Result<Signature> {
        self.sign_tx(&tx).await.map_err(alloy_signer::Error::other)
    }

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data_async<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature> {
        self.sign_typed_data_(payload, domain).await.map_err(alloy_signer::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }
}

impl LedgerSigner {
    /// Instantiate the application by acquiring a lock on the ledger device.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_signer_ledger::{HDPath, Ledger};
    ///
    /// let ledger = Ledger::new(HDPath::LedgerLive(0)).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(err)]
    pub async fn new(derivation: DerivationType) -> Result<Self, LedgerError> {
        let transport = Ledger::init().await?;
        let address = Self::get_address_with_path_transport(&transport, &derivation).await?;
        Ok(Self { transport: Mutex::new(transport), derivation, address })
    }

    /// Returns the account that corresponds to the current device.
    #[inline]
    pub async fn get_address(&self) -> Result<Address, LedgerError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Returns the account that corresponds to the provided derivation path.
    #[inline]
    pub async fn get_address_with_path(
        &self,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let transport = self.transport.lock().await;
        Self::get_address_with_path_transport(&transport, derivation).await
    }

    #[instrument(skip(transport))]
    async fn get_address_with_path_transport(
        transport: &Ledger,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let data = APDUData::new(&Self::path_to_bytes(derivation));

        let command = APDUCommand {
            ins: INS::GET_PUBLIC_KEY as u8,
            p1: P1::NON_CONFIRM as u8,
            p2: P2::NO_CHAINCODE as u8,
            data,
            response_len: None,
        };

        debug!("Dispatching get_address request to ethereum app");
        let answer = block_on(transport.exchange(&command))?;
        let result = answer.data().ok_or(LedgerError::UnexpectedNullResponse)?;

        let address = {
            // extract the address from the response
            let offset = 1 + result[0] as usize;
            let address_str = &result[offset + 1..offset + 1 + result[offset] as usize];
            let mut address = [0; 20];
            address.copy_from_slice(&hex::decode(address_str)?);
            Address::from(address)
        };
        debug!(?address, "Received address from device");
        Ok(address)
    }

    /// Returns the semver of the Ethereum ledger app
    pub async fn version(&self) -> Result<String, LedgerError> {
        let transport = self.transport.lock().await;

        let command = APDUCommand {
            ins: INS::GET_APP_CONFIGURATION as u8,
            p1: P1::NON_CONFIRM as u8,
            p2: P2::NO_CHAINCODE as u8,
            data: APDUData::new(&[]),
            response_len: None,
        };

        debug!("Dispatching get_version");
        let answer = block_on(transport.exchange(&command))?;
        let data = answer.data().ok_or(LedgerError::UnexpectedNullResponse)?;
        if data.len() != 4 {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 4 });
        }
        let version = format!("{}.{}.{}", data[1], data[2], data[3]);
        debug!(version, "Retrieved version from device");
        Ok(version)
    }

    /// Signs an Ethereum transaction (requires confirmation on the ledger)
    #[cfg(TODO)]
    pub async fn sign_tx(&self, tx: &TypedTransaction) -> Result<Signature, LedgerError> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(tx.rlp().as_ref());
        let mut signature = self.sign_payload(INS::SIGN, &payload).await?;
        Ok(signature)
    }

    #[cfg(feature = "eip712")]
    async fn sign_typed_data_<T: SolStruct>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature, LedgerError> {
        // See comment for v1.6.0 requirement
        // https://github.com/LedgerHQ/app-ethereum/issues/105#issuecomment-765316999
        const EIP712_MIN_VERSION: &str = ">=1.6.0";
        let req = semver::VersionReq::parse(EIP712_MIN_VERSION).unwrap();
        let version = semver::Version::parse(&self.version().await?)?;

        // Enforce app version is greater than EIP712_MIN_VERSION
        if !req.matches(&version) {
            return Err(LedgerError::UnsupportedAppVersion(EIP712_MIN_VERSION));
        }

        let mut data = Self::path_to_bytes(&self.derivation);
        data.extend_from_slice(domain.separator().as_slice());
        data.extend_from_slice(payload.eip712_hash_struct().as_slice());

        self.sign_payload(INS::SIGN_ETH_EIP_712, &data).await
    }

    /// Helper function for signing either transaction data, personal messages or EIP712 derived
    /// structs.
    #[instrument(skip_all, fields(command = %command, payload = hex::encode(payload)), ret)]
    async fn sign_payload(&self, command: INS, payload: &[u8]) -> Result<Signature, LedgerError> {
        let transport = self.transport.lock().await;
        let mut command = APDUCommand {
            ins: command as u8,
            p1: P1_FIRST,
            p2: P2::NO_CHAINCODE as u8,
            data: APDUData::new(&[]),
            response_len: None,
        };

        let mut answer = None;
        // workaround for https://github.com/LedgerHQ/app-ethereum/issues/409
        // TODO: remove in future version
        let chunk_size =
            (0..=255).rev().find(|i| payload.len() % i != 3).expect("true for any length");

        // Iterate in 255 byte chunks
        let span = debug_span!("send_loop", index = field::Empty, chunk = field::Empty).entered();
        for (index, chunk) in payload.chunks(chunk_size).enumerate() {
            if !span.is_disabled() {
                span.record("index", index);
                span.record("chunk", hex::encode(chunk));
            }
            command.data = APDUData::new(chunk);

            debug!("Dispatching packet to device");

            let ans = block_on(transport.exchange(&command))?;
            let data = ans.data().ok_or(LedgerError::UnexpectedNullResponse)?;
            debug!(response = hex::encode(data), "Received response from device");
            answer = Some(ans);

            // We need more data
            command.p1 = P1::MORE as u8;
        }
        drop(span);
        drop(transport);

        let answer = answer.unwrap();
        let data = answer.data().unwrap();
        if data.len() != 65 {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 65 });
        }

        // TODO: don't unwrap
        let sig = Signature::from_bytes(&data[1..], data[0] as u64).unwrap();
        debug!(?sig, "Received signature from device");
        Ok(sig)
    }

    // helper which converts a derivation path to bytes
    fn path_to_bytes(derivation: &DerivationType) -> Vec<u8> {
        let derivation = derivation.to_string();
        let elements = derivation.split('/').skip(1).collect::<Vec<_>>();
        let depth = elements.len();

        let mut bytes = vec![depth as u8];
        for derivation_index in elements {
            let hardened = derivation_index.contains('\'');
            let mut index = derivation_index.replace('\'', "").parse::<u32>().unwrap();
            if hardened {
                index |= 0x80000000;
            }

            bytes.extend(index.to_be_bytes());
        }

        bytes
    }
}

#[cfg(all(test, feature = "ledger"))]
mod tests {
    use super::*;
    use crate::Signer;
    use alloy_primitives::{hex, Address, I256, U256};
    use std::str::FromStr;

    #[tokio::test]
    #[ignore]
    // Replace this with your ETH addresses.
    async fn test_get_address() {
        // Instantiate it with the default ledger derivation path
        let ledger = LedgerSigner::new(DerivationType::LedgerLive(0), 1).await.unwrap();
        assert_eq!(
            ledger.get_address().await.unwrap(),
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".parse().unwrap()
        );
        assert_eq!(
            ledger.get_address_with_path(&DerivationType::Legacy(0)).await.unwrap(),
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".parse().unwrap()
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_tx() {
        let ledger = LedgerSigner::new(DerivationType::LedgerLive(0), 1).await.unwrap();

        // approve uni v2 router 0xff
        let data = hex::decode("095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap();

        let tx_req = TransactionRequest::new()
            .to("2ed7afa17473e17ac59908f088b4371d28585476".parse::<Address>().unwrap())
            .gas(1000000)
            .gas_price(400e9 as u64)
            .nonce(5)
            .data(data)
            .value(alloy_primitives::utils::parse_ether(100).unwrap())
            .into();
        let tx = ledger.sign_transaction(&tx_req).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_version() {
        let ledger = LedgerSigner::new(DerivationType::LedgerLive(0), 1).await.unwrap();

        let version = ledger.version().await.unwrap();
        assert_eq!(version, "1.3.7");
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_message() {
        let ledger = LedgerSigner::new(DerivationType::Legacy(0), 1).await.unwrap();
        let message = "hello world";
        let sig = ledger.sign_message(message).await.unwrap();
        let addr = ledger.get_address().await.unwrap();
        sig.verify(message, addr).unwrap();
    }
}
