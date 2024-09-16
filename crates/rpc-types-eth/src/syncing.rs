use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use alloy_primitives::{B512, U256};

/// Syncing info
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SyncInfo {
    /// Starting block
    pub starting_block: U256,
    /// Current block
    pub current_block: U256,
    /// Highest block seen so far
    pub highest_block: U256,
    /// Warp sync snapshot chunks total.
    pub warp_chunks_amount: Option<U256>,
    /// Warp sync snapshot chunks processed.
    pub warp_chunks_processed: Option<U256>,
    /// The details of the sync stages as an hashmap
    /// where the key is the name of the stage and the value is the block number.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub stages: Option<Vec<Stage>>,
}

/// The detail of the sync stages.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Stage {
    /// The name of the sync stage.
    #[cfg_attr(feature = "serde", serde(alias = "stage_name"))]
    pub name: String,
    /// Indicates the progress of the sync stage.
    #[cfg_attr(feature = "serde", serde(alias = "block_number", with = "alloy_serde::quantity"))]
    pub block: u64,
}

/// Peers info
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Peers {
    /// Number of active peers
    pub active: usize,
    /// Number of connected peers
    pub connected: usize,
    /// Max number of peers
    pub max: u32,
    /// Detailed information on peers
    pub peers: Vec<PeerInfo>,
}

/// Peer connection information
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PeerInfo {
    /// Public node id
    pub id: Option<String>,
    /// Node client ID
    pub name: String,
    /// Capabilities
    pub caps: Vec<String>,
    /// Network information
    pub network: PeerNetworkInfo,
    /// Protocols information
    pub protocols: PeerProtocolsInfo,
}

/// Peer network information
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PeerNetworkInfo {
    /// Remote endpoint address
    pub remote_address: String,
    /// Local endpoint address
    pub local_address: String,
}

/// Peer protocols information
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PeerProtocolsInfo {
    /// Ethereum protocol information
    pub eth: Option<PeerEthProtocolInfo>,
    /// PIP protocol information.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub pip: Option<PipProtocolInfo>,
}

/// Peer Ethereum protocol information
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PeerEthProtocolInfo {
    /// Negotiated ethereum protocol version
    pub version: u32,
    /// Peer total difficulty if known
    pub difficulty: Option<U256>,
    /// SHA3 of peer best block hash
    pub head: String,
}

/// Peer PIP protocol information
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PipProtocolInfo {
    /// Negotiated PIP protocol version
    pub version: u32,
    /// Peer total difficulty
    pub difficulty: U256,
    /// SHA3 of peer best block hash
    pub head: String,
}

/// Sync status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    /// Info when syncing
    Info(Box<SyncInfo>),
    /// Not syncing
    None,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SyncStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Syncing {
            /// When client is synced to the highest block, eth_syncing with return "false"
            None(bool),
            IsSyncing(Box<SyncInfo>),
        }

        match Syncing::deserialize(deserializer)? {
            Syncing::None(false) => Ok(Self::None),
            Syncing::None(true) => Err(serde::de::Error::custom(
                "eth_syncing returned `true` that is undefined value.",
            )),
            Syncing::IsSyncing(sync) => Ok(Self::Info(sync)),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SyncStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Info(info) => info.serialize(serializer),
            Self::None => serializer.serialize_bool(false),
        }
    }
}

/// Propagation statistics for pending transaction.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "TxStats")]
pub struct TransactionStats {
    /// Block no this transaction was first seen.
    pub first_seen: u64,
    /// Peers this transaction was propagated to with count.
    pub propagated_to: BTreeMap<B512, usize>,
}

/// Chain status.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChainStatus {
    /// Describes the gap in the blockchain, if there is one: (first, last)
    pub block_gap: Option<(U256, U256)>,
}
