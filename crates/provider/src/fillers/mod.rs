//! Transaction Fillers
//!
//! Fillers decorate a [`Provider`], filling transaction details before they
//! are sent to the network. Fillers are used to set the nonce, gas price, gas
//! limit, and other transaction details, and are called before any other layer.
//!
//! [`Provider`]: crate::Provider

mod chain_id;
pub use chain_id::ChainIdFiller;

mod signer;
pub use signer::{SignerLayer, SignerProvider};

mod nonce;
pub use nonce::NonceFiller;

mod gas;
pub use gas::GasFiller;

mod join_fill;
pub use join_fill::{FillProvider, FillerControlFlow, JoinFill, TxFiller};
