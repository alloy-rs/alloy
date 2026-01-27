//! Beacon block types.
//!
//! See also <https://ethereum.github.io/beacon-APIs/#/Beacon/getBlockV2>

use crate::{header::BeaconBlockHeader, BlsPublicKey, BlsSignature};
use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// The response to a request for a beacon block: `getBlockV2`
///
/// See <https://ethereum.github.io/beacon-APIs/#/Beacon/getBlockV2>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockResponse<T = serde_json::Value> {
    /// The version of the block (e.g., "phase0", "altair", "bellatrix", "capella", "deneb",
    /// "electra").
    pub version: String,
    /// True if the response references an unverified execution payload. Optimistic information may
    /// be invalidated at a later time. If the field is not present, assume the False value.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// True if the response references the finalized history of the chain, as determined by fork
    /// choice. If the field is not present, additional calls are necessary to compare the epoch of
    /// the requested information with the finalized checkpoint.
    #[serde(default)]
    pub finalized: bool,
    /// The signed beacon block.
    pub data: SignedBeaconBlock<T>,
}

/// A signed beacon block.
///
/// The [`SignedBeaconBlock`](https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#signedbeaconblock)
/// object envelope from the CL spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedBeaconBlock<T = serde_json::Value> {
    /// The beacon block message.
    pub message: BeaconBlock<T>,
    /// The BLS signature of the block.
    pub signature: BlsSignature,
}

/// A beacon block.
///
/// The [`BeaconBlock`](https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#beaconblock)
/// object from the CL spec.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlock<T = serde_json::Value> {
    /// The slot to which this block corresponds.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// Index of validator in validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    /// The signing Merkle root of the parent `BeaconBlock`.
    pub parent_root: B256,
    /// The tree hash Merkle root of the `BeaconState` for the `BeaconBlock`.
    pub state_root: B256,
    /// The beacon block body.
    pub body: T,
}

/// The Eth1Data object from the CL spec.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#eth1data>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct Eth1Data {
    /// Root of the deposit tree.
    pub deposit_root: B256,
    /// Total number of deposits.
    #[serde_as(as = "DisplayFromStr")]
    pub deposit_count: u64,
    /// Ethereum 1.x block hash.
    pub block_hash: B256,
}

/// A checkpoint in the beacon chain.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#checkpoint>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct Checkpoint {
    /// The epoch number.
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    /// The root hash.
    pub root: B256,
}

/// Attestation data.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#attestationdata>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct AttestationData {
    /// The slot number.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// The committee index.
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    /// LMD GHOST vote - the beacon block root.
    pub beacon_block_root: B256,
    /// FFG source checkpoint.
    pub source: Checkpoint,
    /// FFG target checkpoint.
    pub target: Checkpoint,
}

/// An attestation.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#attestation>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct Attestation {
    /// Attester aggregation bits.
    pub aggregation_bits: Bytes,
    /// The attestation data.
    pub data: AttestationData,
    /// BLS aggregate signature.
    pub signature: BlsSignature,
}

/// An indexed attestation.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#indexedattestation>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct IndexedAttestation {
    /// Attesting validator indices.
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub attesting_indices: Vec<u64>,
    /// The attestation data.
    pub data: AttestationData,
    /// The BLS signature.
    pub signature: BlsSignature,
}

/// A proposer slashing.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#proposerslashing>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct ProposerSlashing {
    /// First signed block header.
    pub signed_header_1: SignedBeaconBlockHeader,
    /// Second signed block header (conflicting).
    pub signed_header_2: SignedBeaconBlockHeader,
}

/// A signed beacon block header.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#signedbeaconblockheader>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct SignedBeaconBlockHeader {
    /// The beacon block header.
    pub message: BeaconBlockHeader,
    /// The BLS signature.
    pub signature: BlsSignature,
}

/// An attester slashing.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#attesterslashing>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct AttesterSlashing {
    /// First attestation.
    pub attestation_1: IndexedAttestation,
    /// Second attestation (conflicting).
    pub attestation_2: IndexedAttestation,
}

/// Deposit data.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#depositdata>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct DepositData {
    /// The validator's BLS public key.
    pub pubkey: BlsPublicKey,
    /// The withdrawal credentials.
    pub withdrawal_credentials: B256,
    /// Amount in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    /// Container self-signature.
    pub signature: BlsSignature,
}

/// A deposit.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#deposit>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct Deposit {
    /// Branch in the deposit tree (proof).
    pub proof: Vec<B256>,
    /// The deposit data.
    pub data: DepositData,
}

/// A voluntary exit message.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#voluntaryexit>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct VoluntaryExit {
    /// Minimum epoch for processing exit.
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    /// Index of the exiting validator.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
}

/// A signed voluntary exit.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#signedvoluntaryexit>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct SignedVoluntaryExit {
    /// The voluntary exit message.
    pub message: VoluntaryExit,
    /// The BLS signature.
    pub signature: BlsSignature,
}

/// Sync aggregate (Altair+).
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/altair/beacon-chain.md#syncaggregate>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct SyncAggregate {
    /// Aggregation bits of sync committee participation.
    pub sync_committee_bits: Bytes,
    /// BLS signature of the sync committee.
    pub sync_committee_signature: BlsSignature,
}

/// BLS to execution change message (Capella+).
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/capella/beacon-chain.md#blstoexecutionchange>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct BlsToExecutionChange {
    /// Validator index.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// The BLS public key of the validator.
    pub from_bls_pubkey: BlsPublicKey,
    /// The execution address to change to.
    pub to_execution_address: alloy_primitives::Address,
}

/// A signed BLS to execution change (Capella+).
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/capella/beacon-chain.md#signedblstoexecutionchange>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct SignedBlsToExecutionChange {
    /// The BLS to execution change message.
    pub message: BlsToExecutionChange,
    /// The BLS signature.
    pub signature: BlsSignature,
}

/// The beacon block body for Phase0.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct BeaconBlockBodyPhase0 {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings.
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
}

/// The beacon block body for Altair.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/altair/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct BeaconBlockBodyAltair {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings.
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
    /// Sync aggregate (new in Altair).
    pub sync_aggregate: SyncAggregate,
}

/// The beacon block body for Bellatrix.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/bellatrix/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBodyBellatrix<T = serde_json::Value> {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings.
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
    /// Sync aggregate.
    pub sync_aggregate: SyncAggregate,
    /// Execution payload (new in Bellatrix).
    pub execution_payload: T,
}

/// The beacon block body for Capella.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/capella/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBodyCapella<T = serde_json::Value> {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings.
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
    /// Sync aggregate.
    pub sync_aggregate: SyncAggregate,
    /// Execution payload.
    pub execution_payload: T,
    /// BLS to execution changes (new in Capella).
    pub bls_to_execution_changes: Vec<SignedBlsToExecutionChange>,
}

/// The beacon block body for Deneb.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/deneb/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBodyDeneb<T = serde_json::Value> {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings.
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
    /// Sync aggregate.
    pub sync_aggregate: SyncAggregate,
    /// Execution payload.
    pub execution_payload: T,
    /// BLS to execution changes.
    pub bls_to_execution_changes: Vec<SignedBlsToExecutionChange>,
    /// Blob KZG commitments (new in Deneb).
    pub blob_kzg_commitments: Vec<Bytes>,
}

/// The beacon block body for Electra.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/electra/beacon-chain.md#beaconblockbody>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBodyElectra<T = serde_json::Value> {
    /// The RANDAO reveal value provided by the validator.
    pub randao_reveal: BlsSignature,
    /// Eth1 data.
    pub eth1_data: Eth1Data,
    /// Graffiti (32 bytes).
    pub graffiti: B256,
    /// Proposer slashings.
    pub proposer_slashings: Vec<ProposerSlashing>,
    /// Attester slashings (Electra uses a different format).
    pub attester_slashings: Vec<AttesterSlashing>,
    /// Attestations.
    pub attestations: Vec<Attestation>,
    /// Deposits.
    pub deposits: Vec<Deposit>,
    /// Voluntary exits.
    pub voluntary_exits: Vec<SignedVoluntaryExit>,
    /// Sync aggregate.
    pub sync_aggregate: SyncAggregate,
    /// Execution payload.
    pub execution_payload: T,
    /// BLS to execution changes.
    pub bls_to_execution_changes: Vec<SignedBlsToExecutionChange>,
    /// Blob KZG commitments.
    pub blob_kzg_commitments: Vec<Bytes>,
    /// Execution requests (new in Electra).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_requests: Option<serde_json::Value>,
}

/// Type aliases for convenience.
pub type SignedBeaconBlockPhase0 = SignedBeaconBlock<BeaconBlockBodyPhase0>;
/// Type alias for Altair signed beacon block.
pub type SignedBeaconBlockAltair = SignedBeaconBlock<BeaconBlockBodyAltair>;
/// Type alias for Bellatrix signed beacon block.
pub type SignedBeaconBlockBellatrix<T = serde_json::Value> =
    SignedBeaconBlock<BeaconBlockBodyBellatrix<T>>;
/// Type alias for Capella signed beacon block.
pub type SignedBeaconBlockCapella<T = serde_json::Value> =
    SignedBeaconBlock<BeaconBlockBodyCapella<T>>;
/// Type alias for Deneb signed beacon block.
pub type SignedBeaconBlockDeneb<T = serde_json::Value> = SignedBeaconBlock<BeaconBlockBodyDeneb<T>>;
/// Type alias for Electra signed beacon block.
pub type SignedBeaconBlockElectra<T = serde_json::Value> =
    SignedBeaconBlock<BeaconBlockBodyElectra<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_block_response_phase0() {
        let s = r#"{
            "version": "phase0",
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "message": {
                    "slot": "1",
                    "proposer_index": "1",
                    "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "body": {
                        "randao_reveal": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505",
                        "eth1_data": {
                            "deposit_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                            "deposit_count": "1",
                            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                        },
                        "graffiti": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                        "proposer_slashings": [],
                        "attester_slashings": [],
                        "attestations": [],
                        "deposits": [],
                        "voluntary_exits": []
                    }
                },
                "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
            }
        }"#;
        let resp: BlockResponse<BeaconBlockBodyPhase0> = serde_json::from_str(s).unwrap();
        assert_eq!(resp.version, "phase0");
        assert!(resp.finalized);
        assert_eq!(resp.data.message.slot, 1);
    }

    #[test]
    fn serde_block_response_altair() {
        let s = r#"{
            "version": "altair",
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "message": {
                    "slot": "100",
                    "proposer_index": "42",
                    "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "body": {
                        "randao_reveal": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505",
                        "eth1_data": {
                            "deposit_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                            "deposit_count": "100",
                            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                        },
                        "graffiti": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                        "proposer_slashings": [],
                        "attester_slashings": [],
                        "attestations": [],
                        "deposits": [],
                        "voluntary_exits": [],
                        "sync_aggregate": {
                            "sync_committee_bits": "0x01",
                            "sync_committee_signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
                        }
                    }
                },
                "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
            }
        }"#;
        let resp: BlockResponse<BeaconBlockBodyAltair> = serde_json::from_str(s).unwrap();
        assert_eq!(resp.version, "altair");
        assert_eq!(resp.data.message.slot, 100);
        assert_eq!(resp.data.message.proposer_index, 42);
    }

    #[test]
    fn serde_signed_beacon_block_generic() {
        let s = r#"{
            "message": {
                "slot": "12225729",
                "proposer_index": "496520",
                "parent_root": "0x462f4abf9b6881724e6489085b3bb3931312e31ffb43f7cec3d0ee624dc2b58e",
                "state_root": "0x2c6e3ff0b0f7bc33b30a020e75e69c2bba26fb42a7e234e8275e655170925a71",
                "body": {
                    "randao_reveal": "0x825dc181628713b55f40ed3f489be0c60f0513f88eecb25c7aa512ad24b912b3929bdf1930b50af4c18fb8b5f490352218a1c25adc01f7c3aaa50f982d762f589b4f5b6806e1d37e3f70af7afe990d1b1e8e337ac67b53bb7896f2052ecfccc1",
                    "eth1_data": {
                        "deposit_root": "0x2ebc563cabdbbacbc56f0de1d2d1c2d5315a4b071fcd8566aabbf0a45161c64e",
                        "deposit_count": "2045305",
                        "block_hash": "0x0958d83550263ff0d9f9a0bc5ea3cd2a136e0933b6f43cbb17f36e4da8d809b1"
                    },
                    "graffiti": "0x52502d4e502076312e31372e3000000000000000000000000000000000000000",
                    "proposer_slashings": [],
                    "attester_slashings": [],
                    "attestations": [],
                    "deposits": [],
                    "voluntary_exits": [],
                    "sync_aggregate": {
                        "sync_committee_bits": "0x71b7f7596e64ef7f7ef4f938e9f68abfbfe95bff09393315bb93bbec7f7ef27effa4c7f25ba7cbdb87efbbf73fdaebb9efefeb3ef7fff8effafdd7aff5677bfc",
                        "sync_committee_signature": "0xb45afdccf46b3518c295407594d82fcfd7fbff767f1b7bb2e7c9bdc8a0229232d201247b449d4bddf01fc974ce0b57601987fb401bb346062e53981cfb81dd6f9c519d645248a46ceba695c2d9630cfc68b26efc35f6ca14c49af9170581ad90"
                    },
                    "execution_payload": {}
                }
            },
            "signature": "0x8a9cfe747dbb5d6ee1538638b2adfc304c8bcbeb03f489756ca7dc7a12081df892f38b924d19c9f5530c746b86a34beb019070bb7707de5a8efc8bdab8ca5668d7bb0e31c5ffd24913d23c80a6f6f70ba89e280dd46d19d6128ac7f42ffee93e"
        }"#;
        let block: SignedBeaconBlock = serde_json::from_str(s).unwrap();
        assert_eq!(block.message.slot, 12225729);
        assert_eq!(block.message.proposer_index, 496520);
    }

    #[test]
    fn serde_eth1_data() {
        let s = r#"{
            "deposit_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "deposit_count": "1",
            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
        }"#;
        let eth1_data: Eth1Data = serde_json::from_str(s).unwrap();
        assert_eq!(eth1_data.deposit_count, 1);
    }

    #[test]
    fn serde_attestation_data() {
        let s = r#"{
            "slot": "1",
            "index": "1",
            "beacon_block_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "source": {
                "epoch": "1",
                "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
            },
            "target": {
                "epoch": "1",
                "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
            }
        }"#;
        let data: AttestationData = serde_json::from_str(s).unwrap();
        assert_eq!(data.slot, 1);
        assert_eq!(data.index, 1);
    }

    #[test]
    fn serde_voluntary_exit() {
        let s = r#"{
            "message": {
                "epoch": "1",
                "validator_index": "1"
            },
            "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
        }"#;
        let exit: SignedVoluntaryExit = serde_json::from_str(s).unwrap();
        assert_eq!(exit.message.epoch, 1);
        assert_eq!(exit.message.validator_index, 1);
    }

    #[test]
    fn serde_sync_aggregate() {
        let s = r#"{
            "sync_committee_bits": "0x01",
            "sync_committee_signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
        }"#;
        let _aggregate: SyncAggregate = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn serde_bls_to_execution_change() {
        let s = r#"{
            "message": {
                "validator_index": "1",
                "from_bls_pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                "to_execution_address": "0xabcf8e0d4e9587369b2301d0790347320302cc09"
            },
            "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
        }"#;
        let change: SignedBlsToExecutionChange = serde_json::from_str(s).unwrap();
        assert_eq!(change.message.validator_index, 1);
    }

    #[test]
    fn serde_proposer_slashing() {
        let s = r#"{
            "signed_header_1": {
                "message": {
                    "slot": "1",
                    "proposer_index": "1",
                    "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "body_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                },
                "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
            },
            "signed_header_2": {
                "message": {
                    "slot": "1",
                    "proposer_index": "1",
                    "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "body_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                },
                "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
            }
        }"#;
        let slashing: ProposerSlashing = serde_json::from_str(s).unwrap();
        assert_eq!(slashing.signed_header_1.message.slot, 1);
    }

    #[cfg(feature = "ssz")]
    mod ssz_tests {
        use super::*;
        use ssz::{Decode, Encode};

        #[test]
        fn ssz_roundtrip_eth1_data() {
            let eth1_data = Eth1Data {
                deposit_root: B256::repeat_byte(0x11),
                deposit_count: 42,
                block_hash: B256::repeat_byte(0x22),
            };
            let encoded = eth1_data.as_ssz_bytes();
            let decoded = Eth1Data::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(eth1_data, decoded);
        }

        #[test]
        fn ssz_roundtrip_checkpoint() {
            let checkpoint = Checkpoint { epoch: 100, root: B256::repeat_byte(0x33) };
            let encoded = checkpoint.as_ssz_bytes();
            let decoded = Checkpoint::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(checkpoint, decoded);
        }

        #[test]
        fn ssz_roundtrip_attestation_data() {
            let data = AttestationData {
                slot: 1000,
                index: 5,
                beacon_block_root: B256::repeat_byte(0x44),
                source: Checkpoint { epoch: 10, root: B256::repeat_byte(0x55) },
                target: Checkpoint { epoch: 11, root: B256::repeat_byte(0x66) },
            };
            let encoded = data.as_ssz_bytes();
            let decoded = AttestationData::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(data, decoded);
        }

        #[test]
        fn ssz_roundtrip_voluntary_exit() {
            let exit = VoluntaryExit { epoch: 50, validator_index: 123 };
            let encoded = exit.as_ssz_bytes();
            let decoded = VoluntaryExit::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(exit, decoded);
        }

        #[test]
        fn ssz_roundtrip_signed_voluntary_exit() {
            use crate::BlsSignature;
            let exit = SignedVoluntaryExit {
                message: VoluntaryExit { epoch: 50, validator_index: 123 },
                signature: BlsSignature::repeat_byte(0x77),
            };
            let encoded = exit.as_ssz_bytes();
            let decoded = SignedVoluntaryExit::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(exit, decoded);
        }

        #[test]
        fn ssz_roundtrip_deposit_data() {
            use crate::BlsPublicKey;
            let data = DepositData {
                pubkey: BlsPublicKey::repeat_byte(0x88),
                withdrawal_credentials: B256::repeat_byte(0x99),
                amount: 32_000_000_000,
                signature: crate::BlsSignature::repeat_byte(0xaa),
            };
            let encoded = data.as_ssz_bytes();
            let decoded = DepositData::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(data, decoded);
        }

        #[test]
        fn ssz_roundtrip_bls_to_execution_change() {
            use crate::BlsPublicKey;
            let change = BlsToExecutionChange {
                validator_index: 456,
                from_bls_pubkey: BlsPublicKey::repeat_byte(0xbb),
                to_execution_address: alloy_primitives::Address::repeat_byte(0xcc),
            };
            let encoded = change.as_ssz_bytes();
            let decoded = BlsToExecutionChange::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(change, decoded);
        }

        #[test]
        fn ssz_roundtrip_signed_bls_to_execution_change() {
            use crate::{BlsPublicKey, BlsSignature};
            let change = SignedBlsToExecutionChange {
                message: BlsToExecutionChange {
                    validator_index: 789,
                    from_bls_pubkey: BlsPublicKey::repeat_byte(0xdd),
                    to_execution_address: alloy_primitives::Address::repeat_byte(0xee),
                },
                signature: BlsSignature::repeat_byte(0xff),
            };
            let encoded = change.as_ssz_bytes();
            let decoded = SignedBlsToExecutionChange::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(change, decoded);
        }

        #[test]
        fn ssz_roundtrip_sync_aggregate() {
            let aggregate = SyncAggregate {
                sync_committee_bits: Bytes::from_static(&[0x01, 0x02, 0x03]),
                sync_committee_signature: crate::BlsSignature::repeat_byte(0x11),
            };
            let encoded = aggregate.as_ssz_bytes();
            let decoded = SyncAggregate::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(aggregate, decoded);
        }

        #[test]
        fn ssz_roundtrip_beacon_block_body_phase0() {
            let body = BeaconBlockBodyPhase0 {
                randao_reveal: crate::BlsSignature::repeat_byte(0x22),
                eth1_data: Eth1Data {
                    deposit_root: B256::repeat_byte(0x33),
                    deposit_count: 100,
                    block_hash: B256::repeat_byte(0x44),
                },
                graffiti: B256::repeat_byte(0x55),
                proposer_slashings: vec![],
                attester_slashings: vec![],
                attestations: vec![],
                deposits: vec![],
                voluntary_exits: vec![],
            };
            let encoded = body.as_ssz_bytes();
            let decoded = BeaconBlockBodyPhase0::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(body, decoded);
        }

        #[test]
        fn ssz_roundtrip_beacon_block_body_altair() {
            let body = BeaconBlockBodyAltair {
                randao_reveal: crate::BlsSignature::repeat_byte(0x66),
                eth1_data: Eth1Data {
                    deposit_root: B256::repeat_byte(0x77),
                    deposit_count: 200,
                    block_hash: B256::repeat_byte(0x88),
                },
                graffiti: B256::repeat_byte(0x99),
                proposer_slashings: vec![],
                attester_slashings: vec![],
                attestations: vec![],
                deposits: vec![],
                voluntary_exits: vec![],
                sync_aggregate: SyncAggregate {
                    sync_committee_bits: Bytes::from_static(&[0xaa]),
                    sync_committee_signature: crate::BlsSignature::repeat_byte(0xbb),
                },
            };
            let encoded = body.as_ssz_bytes();
            let decoded = BeaconBlockBodyAltair::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(body, decoded);
        }
    }
}
