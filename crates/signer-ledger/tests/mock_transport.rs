//! Drives [`LedgerSigner`] through a scripted in-memory transport.
//!
//! Anything implementing [`LedgerTransport`] can back the signer, which is the seam an emulator
//! transport such as Speculos plugs into. These tests script device responses and assert on the
//! APDUs the signer sends, so the signer's wire logic runs without hardware.

use alloy_primitives::{address, hex, Address, U256};
use alloy_signer::Signer;
use alloy_signer_ledger::{
    coins_ledger::{
        transports::{DeviceIdentity, LedgerTransport},
        ApduAnswer, ApduBuffer, ApduCommand, Ledger, LedgerError, StatusWord,
    },
    HDPath, LedgerSigner,
};
use futures_util::lock::Mutex;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

/// Raw bytes of every APDU the signer sent, shared with the test.
type Sent = Arc<StdMutex<Vec<Vec<u8>>>>;

/// A transport that records commands and answers them from a script of raw responses.
#[derive(Debug)]
struct MockTransport {
    responses: VecDeque<Vec<u8>>,
    sent: Sent,
}

impl LedgerTransport for MockTransport {
    fn identity(&self) -> Option<DeviceIdentity<'_>> {
        None
    }

    async fn exchange(
        &mut self,
        packet: ApduCommand,
        _timeout: Option<Duration>,
    ) -> Result<ApduAnswer, LedgerError> {
        self.sent.lock().unwrap().push(packet.as_bytes().to_vec());
        let response = self.responses.pop_front().expect("signer sent more APDUs than scripted");
        let mut buf = ApduBuffer::new();
        buf.extend_from_slice(&response).unwrap();
        ApduAnswer::from_answer(buf)?.into_result()
    }
}

fn scripted(
    responses: impl IntoIterator<Item = Vec<u8>>,
) -> (Arc<Mutex<Ledger<MockTransport>>>, Sent) {
    let sent = Sent::default();
    let transport =
        MockTransport { responses: responses.into_iter().collect(), sent: sent.clone() };
    (Arc::new(Mutex::new(Ledger::from_transport(transport))), sent)
}

const ADDRESS: Address = address!("0xc4ccf517bf8c4ce01463a2518c4a9b22ce1e03b3");

/// A `GET_PUBLIC_KEY` answer: public key length and bytes, then the address as ASCII hex.
fn get_address_response() -> Vec<u8> {
    let mut resp = vec![65];
    resp.extend_from_slice(&[0; 65]);
    resp.push(40);
    resp.extend_from_slice(hex::encode(ADDRESS).as_bytes());
    resp.extend_from_slice(&[0x90, 0x00]);
    resp
}

#[tokio::test]
async fn custom_transport_reads_address_for_ledger_live_path() {
    let (transport, sent) = scripted([get_address_response()]);
    let signer =
        LedgerSigner::new_with_transport(HDPath::LedgerLive(0), Some(1), transport).await.unwrap();

    assert_eq!(signer.address(), ADDRESS);
    // CLA, INS GET_PUBLIC_KEY, P1 no confirmation, P2 no chain code, Lc, then the depth-prefixed
    // path m/44'/60'/0'/0/0.
    let expected = hex!("e00200001505" "8000002c" "8000003c" "80000000" "00000000" "00000000");
    assert_eq!(sent.lock().unwrap().as_slice(), [expected.to_vec()]);
}

/// A `GET_APP_CONFIGURATION` answer for app version `major.minor.patch`.
fn version_response(major: u8, minor: u8, patch: u8) -> Vec<u8> {
    vec![0x00, major, minor, patch, 0x90, 0x00]
}

/// A signing answer carrying `v` and the given `r` and `s` bytes.
fn signature_response(v: u8, r: u8, s: u8) -> Vec<u8> {
    let mut resp = vec![v];
    resp.extend_from_slice(&[r; 32]);
    resp.extend_from_slice(&[s; 32]);
    resp.extend_from_slice(&[0x90, 0x00]);
    resp
}

#[tokio::test]
async fn version_parses_app_configuration() {
    let (transport, sent) = scripted([get_address_response(), version_response(1, 16, 2)]);
    let signer =
        LedgerSigner::new_with_transport(HDPath::LedgerLive(0), None, transport).await.unwrap();

    assert_eq!(signer.version().await.unwrap(), semver::Version::new(1, 16, 2));
    // GET_APP_CONFIGURATION carries no data, so there is no Lc byte.
    assert_eq!(sent.lock().unwrap()[1], hex!("e0060000"));
}

#[tokio::test]
async fn sign_message_chunks_payload_and_marks_continuations() {
    let message = [0xab; 300];
    let (transport, sent) = scripted([
        get_address_response(),
        // The first chunk is acknowledged without data; the last one carries the signature.
        vec![0x90, 0x00],
        signature_response(1, 0x11, 0x22),
    ]);
    let signer =
        LedgerSigner::new_with_transport(HDPath::LedgerLive(0), None, transport).await.unwrap();

    let sig = signer.sign_message(&message).await.unwrap();
    assert_eq!(sig.r(), U256::from_be_bytes([0x11; 32]));
    assert_eq!(sig.s(), U256::from_be_bytes([0x22; 32]));
    assert!(sig.v());

    // Payload: 21-byte path, 4-byte big-endian length, then the message: 325 bytes, so one full
    // 255-byte chunk and a 70-byte remainder.
    let mut payload = hex!("058000002c8000003c800000000000000000000000").to_vec();
    payload.extend_from_slice(&300u32.to_be_bytes());
    payload.extend_from_slice(&message);

    let sent = sent.lock().unwrap();
    let mut first = hex!("e00800" "00" "ff").to_vec();
    first.extend_from_slice(&payload[..255]);
    let mut second = hex!("e00880" "00" "46").to_vec();
    second.extend_from_slice(&payload[255..]);
    assert_eq!(sent[1], first, "first chunk uses P1_FIRST");
    assert_eq!(sent[2], second, "second chunk uses P1_MORE");
}

#[tokio::test]
async fn rejected_status_word_is_reported_with_its_code() {
    // The device refuses the address request with "conditions not satisfied".
    let (transport, _) = scripted([vec![0x69, 0x85]]);
    let err =
        LedgerSigner::new_with_transport(HDPath::LedgerLive(0), None, transport).await.unwrap_err();

    assert!(
        matches!(
            err,
            alloy_signer_ledger::LedgerError::LedgerError(LedgerError::BadRetcode(sw))
                if sw == StatusWord::CONDITIONS_NOT_SATISFIED
        ),
        "{err:?}"
    );
}

#[cfg(feature = "eip7702")]
#[tokio::test]
async fn sign_auth_rejects_apps_without_eip7702() {
    let (transport, sent) = scripted([get_address_response(), version_response(1, 16, 2)]);
    let signer =
        LedgerSigner::new_with_transport(HDPath::LedgerLive(0), None, transport).await.unwrap();

    let auth =
        alloy_eip7702::Authorization { chain_id: U256::from(1), address: Address::ZERO, nonce: 0 };
    let err = signer.sign_auth(&auth).await.unwrap_err();

    assert!(
        matches!(err, alloy_signer_ledger::LedgerError::UnsupportedAppVersion(">=1.17.0")),
        "{err:?}"
    );
    // Only the address and version queries went to the device; no signing APDU was sent.
    assert_eq!(sent.lock().unwrap().len(), 2);
}
