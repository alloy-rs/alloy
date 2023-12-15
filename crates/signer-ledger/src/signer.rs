//! Ledger Ethereum app wrapper.

use crate::types::{DerivationType, LedgerError, INS, P1, P1_FIRST, P2};
use alloy_primitives::{hex, Address, B256};
use alloy_signer::{Result, Signature, Signer};
use async_trait::async_trait;
use coins_ledger::{
    common::{APDUCommand, APDUData},
    transports::{Ledger, LedgerAsync},
};
use futures_util::lock::Mutex;

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
    pub(crate) chain_id: u64,
    pub(crate) address: Address,
}

impl std::fmt::Display for LedgerSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LedgerApp. Key at index {} with address {:?} on chain_id {}",
            self.derivation, self.address, self.chain_id
        )
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for LedgerSigner {
    async fn sign_hash(&self, _hash: B256) -> Result<Signature> {
        Err(alloy_signer::Error::UnsupportedOperation(
            alloy_signer::UnsupportedSignerOperation::SignHash,
        ))
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(&(message.len() as u32).to_be_bytes());
        payload.extend_from_slice(message);

        self.sign_payload(INS::SIGN_PERSONAL_MESSAGE, &payload)
            .await
            .map_err(alloy_signer::Error::other)
    }

    #[cfg(TODO)] // TODO: TypedTransaction
    #[inline]
    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature> {
        self.sign_tx(&tx).await.map_err(alloy_signer::Error::other)
    }

    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
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

    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: u64) {
        self.chain_id = chain_id;
    }
}

impl LedgerSigner {
    /// Instantiate the application by acquiring a lock on the ledger device.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_signer_ledger::{HDPath, Ledger};
    ///
    /// let ledger = Ledger::new(HDPath::LedgerLive(0), 1).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(derivation: DerivationType, chain_id: u64) -> Result<Self, LedgerError> {
        let transport = Ledger::init().await?;
        let address = Self::get_address_with_path_transport(&transport, &derivation).await?;

        Ok(Self { transport: Mutex::new(transport), derivation, chain_id, address })
    }

    /// Get the account which corresponds to our derivation path
    pub async fn get_address(&self) -> Result<Address, LedgerError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Gets the account which corresponds to the provided derivation path
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
        let answer = transport.exchange(&command).await?;
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
    pub async fn version(&self) -> Result<semver::Version, LedgerError> {
        let transport = self.transport.lock().await;

        let command = APDUCommand {
            ins: INS::GET_APP_CONFIGURATION as u8,
            p1: P1::NON_CONFIRM as u8,
            p2: P2::NO_CHAINCODE as u8,
            data: APDUData::new(&[]),
            response_len: None,
        };

        debug!("Dispatching get_version");
        let answer = transport.exchange(&command).await?;
        let data = answer.data().ok_or(LedgerError::UnexpectedNullResponse)?;
        let &[_flags, major, minor, patch] = data else {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 4 });
        };
        let version = semver::Version::new(major as u64, minor as u64, patch as u64);
        debug!(%version, "Retrieved version from device");
        Ok(version)
    }

    /// Signs an Ethereum transaction (requires confirmation on the ledger)
    #[cfg(TODO)] // TODO: TypedTransaction
    pub async fn sign_tx(&self, tx: &TypedTransaction) -> Result<Signature, LedgerError> {
        let mut tx_with_chain = tx.clone();
        if tx_with_chain.chain_id().is_none() {
            // in the case we don't have a chain_id, let's use the signer chain id instead
            tx_with_chain.set_chain_id(self.chain_id);
        }
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(tx_with_chain.rlp().as_ref());

        let mut signature = self.sign_payload(INS::SIGN, &payload).await?;

        // modify `v` value of signature to match EIP-155 for chains with large chain ID
        // The logic is derived from Ledger's library
        // https://github.com/LedgerHQ/ledgerjs/blob/e78aac4327e78301b82ba58d63a72476ecb842fc/packages/hw-app-eth/src/Eth.ts#L300
        let eip155_chain_id = self.chain_id * 2 + 35;
        if eip155_chain_id + 1 > 255 {
            let one_byte_chain_id = eip155_chain_id % 256;
            let ecc_parity = if signature.v > one_byte_chain_id {
                signature.v - one_byte_chain_id
            } else {
                one_byte_chain_id - signature.v
            };

            signature.v = match tx {
                TypedTransaction::Eip2930(_) | TypedTransaction::Eip1559(_) => {
                    (ecc_parity % 2 != 1) as u64
                }
                TypedTransaction::Legacy(_) => eip155_chain_id + ecc_parity,
                #[cfg(feature = "optimism")]
                TypedTransaction::DepositTransaction(_) => 0,
            };
        }

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
        let version = self.version().await?;

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
    #[instrument(err, skip_all, fields(command = %command, payload = hex::encode(payload)))]
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
        for chunk in payload.chunks(chunk_size) {
            command.data = APDUData::new(chunk);

            debug!("Dispatching packet to device");

            let ans = transport.exchange(&command).await?;
            let data = ans.data().ok_or(LedgerError::UnexpectedNullResponse)?;
            debug!(response = hex::encode(data), "Received response from device");
            answer = Some(ans);

            // We need more data
            command.p1 = P1::MORE as u8;
        }
        drop(transport);

        let answer = answer.unwrap();
        let data = answer.data().unwrap();
        if data.len() != 65 {
            return Err(LedgerError::ShortResponse { got: data.len(), expected: 65 });
        }

        let sig = Signature::from_bytes_and_parity(&data[1..], data[0] as u64)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const DTYPE: DerivationType = DerivationType::LedgerLive(0);

    fn my_address() -> Address {
        std::env::var("LEDGER_ADDRESS").unwrap().parse().unwrap()
    }

    async fn init_ledger() -> LedgerSigner {
        match LedgerSigner::new(DTYPE, 1).await {
            Ok(ledger) => ledger,
            Err(e) => panic!("{e:?}\n{e}"),
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    #[ignore]
    async fn test_get_address() {
        let ledger = init_ledger().await;
        assert_eq!(ledger.get_address().await.unwrap(), my_address());
        assert_eq!(ledger.get_address_with_path(&DTYPE).await.unwrap(), my_address(),);
    }

    #[tokio::test]
    #[serial_test::serial]
    #[ignore]
    async fn test_version() {
        let ledger = init_ledger().await;
        let version = ledger.version().await.unwrap();
        eprintln!("{version}");
        assert!(version.major >= 1);
    }

    #[tokio::test]
    #[serial_test::serial]
    #[ignore]
    #[cfg(TODO)] // TODO: TypedTransaction
    async fn test_sign_tx() {
        let ledger = init_ledger().await;

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
    #[serial_test::serial]
    #[ignore]
    async fn test_sign_message() {
        let ledger = init_ledger().await;
        let message = "hello world";
        let sig = ledger.sign_message(message.as_bytes()).await.unwrap();
        let addr = ledger.get_address().await.unwrap();
        assert_eq!(addr, my_address());
        assert_eq!(sig.recover_address_from_msg(message.as_bytes()).unwrap(), my_address());
    }
}
