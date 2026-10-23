//! Runs [`LedgerSigner`] against the Ledger Ethereum app in the [Speculos] emulator.
//!
//! The tests skip unless `SPECULOS_APDU_ADDR` is set. With it set, `SPECULOS_API_URL` and
//! `SPECULOS_MODEL` must be set too, and Speculos must run the Ethereum app with `--seed` set to
//! [`SEED`], so that the Ledger Live path 0 derives [`ADDRESS`]. The crate README shows how to
//! start it. Only the button-driven Nano models are navigated; Flex and Stax need touch input.
//!
//! [Speculos]: https://github.com/LedgerHQ/speculos
#![cfg(feature = "speculos")]

use alloy_consensus::{SignableTransaction, TxEip1559, TxEip2930, TxLegacy};
use alloy_network::TxSigner;
use alloy_primitives::{address, bytes, hex, Address, Signature, U256};
use alloy_rlp::Decodable;
use alloy_signer::Signer;
use alloy_signer_ledger::{
    coins_ledger::{
        transports::speculos::{Action, Button, ScreenEvent, SpeculosClient, SpeculosTransport},
        Ledger, LedgerError as CoinsLedgerError, Model, StatusWord,
    },
    HDPath, LedgerError, LedgerSigner,
};
use futures_util::lock::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::time::{sleep, timeout, Instant};

/// The mnemonic Speculos must be started with (`--seed`).
const SEED: &str = "test test test test test test test test test test test junk";
/// `m/44'/60'/0'/0/0` under [`SEED`].
const ADDRESS: Address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
const PATH: HDPath = HDPath::LedgerLive(0);

const STEP: Duration = Duration::from_secs(10);
const EXCHANGE: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(100);
const MAX_PAGES: usize = 20;
const TAP: Action = Action::PressAndRelease { delay: None };

/// Serializes the tests, which share one emulator.
static DEVICE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A running emulator and its control API.
struct Speculos {
    addr: String,
    client: SpeculosClient,
    model: Model,
    _guard: tokio::sync::MutexGuard<'static, ()>,
}

impl Speculos {
    /// Connects to the emulator named by the environment, or returns `None` to skip the test.
    async fn from_env() -> Option<Self> {
        let Ok(addr) = std::env::var("SPECULOS_APDU_ADDR") else {
            eprintln!("SPECULOS_APDU_ADDR is unset; skipping Speculos test");
            return None;
        };
        let url = std::env::var("SPECULOS_API_URL")
            .expect("SPECULOS_API_URL must be set when SPECULOS_APDU_ADDR is");
        let model = std::env::var("SPECULOS_MODEL")
            .expect("SPECULOS_MODEL must be set when SPECULOS_APDU_ADDR is");
        let model = match model.as_str() {
            "nanosp" => Model::NanoSPlus,
            "nanox" => Model::NanoX,
            other => panic!(
                "SPECULOS_MODEL {other:?} is not a button-driven model these tests can navigate"
            ),
        };
        let _ = tracing_subscriber::fmt::try_init();
        let client = SpeculosClient::new(url).expect("SPECULOS_API_URL is a URL");
        let guard = DEVICE.lock().await;
        let this = Self { addr, client, model, _guard: guard };
        this.wait_home().await;
        Some(this)
    }

    /// A signer over a fresh APDU connection.
    async fn signer(&self, chain_id: Option<u64>) -> LedgerSigner<SpeculosTransport> {
        let mut ledger = timeout(STEP, Ledger::connect_speculos(self.addr.as_str(), self.model))
            .await
            .expect("connecting to Speculos timed out")
            .expect("Speculos APDU port is reachable");
        ledger.set_timeout(Some(EXCHANGE));
        LedgerSigner::new_with_transport(PATH, chain_id, Arc::new(Mutex::new(ledger)))
            .await
            .expect("address query")
    }

    async fn press(&self, button: Button) {
        self.client.button(button, TAP).await.expect("button press");
    }

    /// The current screen's text once two consecutive reads agree.
    async fn screen(&self) -> String {
        let deadline = Instant::now() + STEP;
        let mut last = self.client.current_screen().await.expect("current screen");
        loop {
            sleep(POLL).await;
            let now = self.client.current_screen().await.expect("current screen");
            if now == last && !now.is_empty() {
                return text(&now);
            }
            assert!(Instant::now() < deadline, "screen did not settle; last saw {now:?}");
            last = now;
        }
    }

    /// Waits for the app's home screen or menu.
    async fn wait_home(&self) {
        let deadline = Instant::now() + STEP;
        loop {
            let screen = self.screen().await;
            if is_home(&screen) {
                return;
            }
            assert!(Instant::now() < deadline, "app did not return home; screen is {screen:?}");
            sleep(POLL).await;
        }
    }

    /// Waits for the app to leave its home screen for a review.
    async fn wait_review(&self) {
        let deadline = Instant::now() + STEP;
        loop {
            let screen = self.screen().await;
            if !is_home(&screen) {
                return;
            }
            assert!(Instant::now() < deadline, "no review started; screen is {screen:?}");
            sleep(POLL).await;
        }
    }

    /// Pages through a review and approves it, accepting any blind-signing warning on the way.
    async fn approve(&self) {
        self.wait_review().await;
        for _ in 0..MAX_PAGES {
            let screen = self.screen().await;
            if screen.contains("press both buttons") {
                self.press(Button::Both).await;
            } else if screen.starts_with("Sign ") || screen.starts_with("Accept risk and") {
                self.press(Button::Both).await;
                return;
            } else {
                assert!(
                    !screen.starts_with("Reject"),
                    "walked past the approval page; screen is {screen:?}"
                );
                self.press(Button::Right).await;
            }
        }
        panic!("no approval page within {MAX_PAGES} pages");
    }

    /// Pages through a review and rejects it.
    async fn reject(&self) {
        self.wait_review().await;
        for _ in 0..MAX_PAGES {
            let screen = self.screen().await;
            if screen.starts_with("Reject") {
                self.press(Button::Both).await;
                return;
            }
            self.press(Button::Right).await;
        }
        panic!("no rejection page within {MAX_PAGES} pages");
    }

    /// Pages to the menu entry starting with `entry` and selects it.
    async fn open_menu(&self, entry: &str) {
        let mut direction = Button::Right;
        let mut previous = String::new();
        for _ in 0..MAX_PAGES {
            let screen = self.screen().await;
            if screen.starts_with(entry) {
                self.press(Button::Both).await;
                return;
            }
            // The menu does not wrap; turn around at either end.
            if screen == previous {
                direction = Button::Left;
            }
            previous = screen;
            self.press(direction).await;
        }
        panic!("menu entry {entry:?} not found within {MAX_PAGES} pages");
    }

    /// Turns `setting` on in the app settings if it is off.
    async fn enable(&self, setting: &str) {
        self.wait_home().await;
        self.open_menu("App settings").await;
        for _ in 0..MAX_PAGES {
            let screen = self.screen().await;
            if screen.starts_with(setting) {
                if !screen.ends_with("Enabled") {
                    self.press(Button::Both).await;
                    let screen = self.screen().await;
                    assert!(
                        screen.ends_with("Enabled"),
                        "{setting:?} did not turn on; screen is {screen:?}"
                    );
                }
                self.open_menu("Back").await;
                return;
            }
            assert!(!screen.starts_with("Back"), "{setting:?} is not in the app settings");
            self.press(Button::Right).await;
        }
        panic!("{setting:?} not found within {MAX_PAGES} pages");
    }
}

fn text(screen: &[ScreenEvent]) -> String {
    screen.iter().map(|e| e.text.trim()).collect::<Vec<_>>().join(" ")
}

fn is_home(screen: &str) -> bool {
    ["Ethereum", "App settings", "App info", "Quit app"].iter().any(|s| screen.starts_with(s))
}

/// Signs `tx` with on-screen approval and checks the signature recovers to [`ADDRESS`].
async fn sign_tx(spec: &Speculos, tx: &mut dyn SignableTransaction<Signature>) {
    let signer = spec.signer(None).await;
    let sighash = tx.signature_hash();
    let (sig, ()) = tokio::join!(TxSigner::sign_transaction(&signer, tx), spec.approve());
    let sig = sig.unwrap_or_else(|e| panic!("signing failed: {e}"));
    assert_eq!(sig.recover_address_from_prehash(&sighash).unwrap(), ADDRESS);
}

#[tokio::test]
async fn address_matches_seed() {
    let Some(spec) = Speculos::from_env().await else { return };
    let signer = spec.signer(Some(1)).await;
    assert_eq!(Signer::address(&signer), ADDRESS, "start Speculos with --seed {SEED:?}");
    assert_eq!(signer.get_address_with_path(&PATH).await.unwrap(), ADDRESS);
}

#[tokio::test]
async fn app_version_supports_eip7702() {
    let Some(spec) = Speculos::from_env().await else { return };
    let signer = spec.signer(None).await;
    let version = signer.version().await.unwrap();
    assert!(version >= semver::Version::new(1, 17, 0), "app {version} predates EIP-7702");
}

#[tokio::test]
async fn sign_message() {
    let Some(spec) = Speculos::from_env().await else { return };
    let signer = spec.signer(None).await;
    let message = b"hello world";
    let (sig, ()) = tokio::join!(signer.sign_message(message), spec.approve());
    assert_eq!(sig.unwrap().recover_address_from_msg(message).unwrap(), ADDRESS);
}

#[tokio::test]
async fn rejected_message_reports_status_word() {
    let Some(spec) = Speculos::from_env().await else { return };
    let signer = spec.signer(None).await;
    let (res, ()) = tokio::join!(signer.sign_message(b"hello world"), spec.reject());
    let alloy_signer::Error::Other(err) = res.unwrap_err() else { panic!("not a signer error") };
    let err = err.downcast::<LedgerError>().expect("a Ledger error");
    assert!(
        matches!(
            *err,
            LedgerError::LedgerError(CoinsLedgerError::BadRetcode(sw))
                if sw == StatusWord::CONDITIONS_NOT_SATISFIED
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn sign_tx_eip1559() {
    let Some(spec) = Speculos::from_env().await else { return };
    // From the Ledger Ethereum app example: https://github.com/LedgerHQ/app-ethereum/blob/2264f677568cbc1e3177f9eccb3c14a229ab3255/examples/signTx.py#L100-L102
    let tx_rlp = hex!("02ef0306843b9aca008504a817c80082520894b2bb2b958afa2e96dab3f3ce7162b87daea39017872386f26fc1000080c0");
    let mut tx = TxEip1559::decode(&mut &tx_rlp[1..]).unwrap();
    assert_eq!(tx.encoded_for_signing(), tx_rlp);
    sign_tx(&spec, &mut tx).await;
}

#[tokio::test]
async fn sign_tx_eip2930() {
    let Some(spec) = Speculos::from_env().await else { return };
    // The calldata is not clear-signable, so the app requires blind signing.
    spec.enable("Blind signing").await;
    // From the Ledger Ethereum app example, without its signature: https://github.com/LedgerHQ/app-ethereum/blob/2264f677568cbc1e3177f9eccb3c14a229ab3255/examples/signTx.py#L104-L106
    let tx_rlp = hex!("01f8a30380018402625a0094cccccccccccccccccccccccccccccccccccccccc830186a0a4693c61390000000000000000000000000000000000000000000000000000000000000002f85bf859940000000000000000000000000000000000000102f842a00000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000060a7");
    let mut tx = TxEip2930::decode(&mut &tx_rlp[1..]).unwrap();
    assert_eq!(tx.encoded_for_signing(), tx_rlp);
    sign_tx(&spec, &mut tx).await;
}

#[tokio::test]
async fn sign_tx_legacy() {
    let Some(spec) = Speculos::from_env().await else { return };
    spec.enable("Blind signing").await;
    // https://github.com/gakonst/ethers-rs/blob/90b87bd85be98caa8bb592b67f3f9acbc8a409cf/ethers-signers/src/ledger/app.rs#L321
    let mut tx = TxLegacy {
        nonce: 5,
        gas_price: 400e9 as u128,
        gas_limit: 1000000,
        to: address!("2ed7afa17473e17ac59908f088b4371d28585476").into(),
        input: bytes!("01020304"),
        value: U256::from(100e18 as u128),
        chain_id: Some(69420),
    };
    sign_tx(&spec, &mut tx).await;
}

#[cfg(feature = "eip7702")]
#[tokio::test]
async fn sign_auth() {
    let Some(spec) = Speculos::from_env().await else { return };
    spec.enable("Smart accounts").await;
    let signer = spec.signer(None).await;
    let auth = alloy_eip7702::Authorization {
        chain_id: U256::from(11155111),
        address: address!("0x4Cd241E8d1510e30b2076397afc7508Ae59C66c9"),
        nonce: 72,
    };
    let (sig, ()) = tokio::join!(signer.sign_auth(&auth), spec.approve());
    let sig = sig.expect("sign_auth should succeed");
    assert_eq!(sig.recover_address_from_prehash(&auth.signature_hash()).unwrap(), ADDRESS);
}
