//! Support for the Flashbots [priority update registry].
//!
//! The priority update registry is an on-chain registry that allows authorized updaters
//! (e.g. PropAMM market makers) to publish per-block priority updates that are read by their
//! target contracts during execution. Block builders that support priority updates guarantee
//! that updates for a contract land in the block before any transaction that interacts with
//! that contract, and that updates for contracts not touched in the block are excluded.
//!
//! The registry is deployed at [`PRIORITY_UPDATE_REGISTRY_ADDRESS`] via the deterministic
//! CREATE2 factory, see the [priority update registry] repository for deployment details.
//!
//! Priority updates are either submitted directly by an authorized updater via
//! `updateState`, or relayed as EIP-712 signed updates via `batchUpdateStateWithSignature`.
//! Use [`sign_priority_update`] to create such a [`SignedUpdate`] from an [`UpdateState`]
//! message.
//!
//! Builders currently ingest priority update transactions from makers over builder-specific
//! authenticated WebSocket endpoints, see e.g.:
//! - Titan: <https://docs.titanbuilder.xyz/propamms/makers>
//! - Bombora: <https://bombora.build/docs/propamm-makers>
//! - BuilderNet: <https://buildernet.org/docs/api>
//!
//! [priority update registry]: https://github.com/flashbots/priority-update-registry

use alloy_primitives::{address, Address, ChainId};
use alloy_signer::Signer;
use alloy_sol_types::{eip712_domain, sol, Eip712Domain, SolStruct};

pub use PrioUpdateRegistry::SignedUpdate;

/// The address of the Flashbots priority update registry:
/// [`0xda7afeed01fe625cf15d187a19f94b45f00b8c5f`](https://etherscan.io/address/0xda7afeed01fe625cf15d187a19f94b45f00b8c5f)
///
/// The registry is deployed via the deterministic CREATE2 factory, see the
/// [deployment details](https://github.com/flashbots/priority-update-registry#deployments).
pub const PRIORITY_UPDATE_REGISTRY_ADDRESS: Address =
    address!("0xda7afeed01fe625cf15d187a19f94b45f00b8c5f");

sol! {
    /// The EIP-712 message signed by an updater for
    /// `PrioUpdateRegistry::batchUpdateStateWithSignature`.
    ///
    /// The encoded type is
    /// `UpdateState(address target,uint256 laneIndex,uint32 updateTimestamp,uint256[] slots)`,
    /// matching the registry's `UPDATE_TYPEHASH`. Note that the relayed [`SignedUpdate`]
    /// additionally carries the `signer`, which is not part of the signed payload.
    #[derive(Debug, PartialEq, Eq)]
    struct UpdateState {
        /// The address whose state is being updated.
        address target;
        /// The lane to write, scoped to `target`.
        uint256 laneIndex;
        /// The timestamp associated with this update.
        uint32 updateTimestamp;
        /// The slot values to write. Length must be in `[1, 255]` and `slots[0]` must fit in
        /// 27 bytes (its top 5 bytes are reserved for `updateTimestamp` and the slot count).
        uint256[] slots;
    }

    /// The Flashbots [priority update registry](https://github.com/flashbots/priority-update-registry)
    /// contract.
    #[derive(Debug, PartialEq, Eq)]
    interface PrioUpdateRegistry {
        /// A signed state update relayed via `batchUpdateStateWithSignature`.
        ///
        /// `signer` is not part of the EIP-712 hash: it is supplied alongside the signature so
        /// the contract knows which address's authorization to check. If `signer == target`,
        /// the signature is verified against `target` via ERC-1271; otherwise it must
        /// ECDSA-recover to `signer`, and `signer` must be an authorized updater for `target`.
        struct SignedUpdate {
            /// The address whose state is being updated.
            address target;
            /// The address whose signature authorizes this update.
            address signer;
            /// The lane to write, scoped to `target`.
            uint256 laneIndex;
            /// The timestamp associated with this update.
            uint32 updateTimestamp;
            /// The slot values to write.
            uint256[] slots;
            /// The EIP-712 signature over `(target, laneIndex, updateTimestamp, slots)`.
            bytes signature;
        }

        /// Thrown when the caller or signer is not authorized to update state for `target`.
        error NotAuthorized();
        /// Thrown when `slots` has length zero.
        error EmptySlots();
        /// Thrown when `slots[0]` does not fit in 27 bytes.
        error Slot0Exceeds27Bytes();
        /// Thrown when `slots` has more than 255 entries.
        error TooManySlots();
        /// Thrown when `updateTimestamp` lies outside the accepted window around
        /// `block.timestamp`.
        error InvalidUpdateTimestamp();
        /// Thrown on writes when `updateTimestamp` is older than the stored timestamp, or on
        /// reads when the stored timestamp lies outside the requested window.
        error StaleUpdate();

        /// Emitted when `updater` is authorized to write state on behalf of `target`.
        event UpdaterAdded(address indexed target, address indexed updater);
        /// Emitted when the authorization of `updater` for `target` is revoked.
        event UpdaterRemoved(address indexed target, address indexed updater);

        /// Maximum age (in seconds) by which `updateTimestamp` may lag `block.timestamp` on
        /// writes.
        function MAX_UPDATE_AGE() external view returns (uint256);
        /// Maximum lead time (in seconds) by which `updateTimestamp` may exceed
        /// `block.timestamp` on writes.
        function MAX_UPDATE_LEAD_TIME() external view returns (uint256);
        /// The EIP-712 type hash for an [`UpdateState`] message.
        function UPDATE_TYPEHASH() external view returns (bytes32);
        /// The EIP-712 domain separator used for signed updates.
        function DOMAIN_SEPARATOR() external view returns (bytes32);
        /// Returns whether `updater` is authorized to write state on behalf of `target`.
        function isUpdater(address target, address updater) external view returns (bool);
        /// Authorizes `updater` to write state on behalf of the caller.
        function addUpdater(address updater) external;
        /// Revokes the authorization of `updater` to write state on behalf of the caller.
        function removeUpdater(address updater) external;
        /// Returns the stored state for the caller at the given `laneIndex`, enforcing that
        /// the stored `updateTimestamp` lies within `[minTimestamp, maxTimestamp]`.
        function getState(uint256 laneIndex, uint32 minTimestamp, uint32 maxTimestamp)
            external
            view
            returns (uint32 updateTimestamp, uint256[] memory slots);
        /// Writes a state update for `target` at `laneIndex`, with the caller acting as the
        /// updater.
        function updateState(
            address target,
            uint256 laneIndex,
            uint32 updateTimestamp,
            uint256[] calldata slots
        ) external;
        /// Applies a batch of signed state updates. Anyone may relay the batch.
        function batchUpdateStateWithSignature(SignedUpdate[] calldata updates) external;
    }
}

/// Returns the [`Eip712Domain`] of the priority update registry for the given chain.
///
/// The domain is `("PrioUpdateRegistry", "1", chain_id, `[`PRIORITY_UPDATE_REGISTRY_ADDRESS`]`)`.
pub fn priority_update_registry_domain(chain_id: ChainId) -> Eip712Domain {
    eip712_domain! {
        name: "PrioUpdateRegistry",
        version: "1",
        chain_id: chain_id,
        verifying_contract: PRIORITY_UPDATE_REGISTRY_ADDRESS,
    }
}

/// Signs the given [`UpdateState`] message for the priority update registry on the given
/// chain, returning a [`SignedUpdate`] that can be relayed by anyone via
/// `PrioUpdateRegistry::batchUpdateStateWithSignature`.
///
/// Note: signatures are not single-use, a [`SignedUpdate`] remains relayable as long as the
/// lane's stored timestamp does not exceed `updateTimestamp` and `updateTimestamp` is within
/// the registry's accepted window. Replays always produce the same on-chain state.
///
/// # Example
///
/// ```
/// use alloy_primitives::U256;
/// use alloy_provider::ext::{sign_priority_update, UpdateState};
/// use alloy_signer_local::PrivateKeySigner;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let signer = PrivateKeySigner::random();
/// let update = UpdateState {
///     target: "0x5979458912f80b96d30d4220af8e2e4925a33320".parse()?,
///     laneIndex: U256::ZERO,
///     updateTimestamp: 1753977388,
///     slots: vec![U256::from(1)],
/// };
/// let signed = sign_priority_update(&signer, 1, update).await?;
/// assert_eq!(signed.signer, signer.address());
/// # Ok(())
/// # }
/// ```
pub async fn sign_priority_update<S>(
    signer: &S,
    chain_id: ChainId,
    update: UpdateState,
) -> Result<SignedUpdate, alloy_signer::Error>
where
    S: Signer + Send + Sync,
{
    let hash = update.eip712_signing_hash(&priority_update_registry_domain(chain_id));
    let signature = signer.sign_hash(&hash).await?;
    Ok(SignedUpdate {
        target: update.target,
        signer: signer.address(),
        laneIndex: update.laneIndex,
        updateTimestamp: update.updateTimestamp,
        slots: update.slots,
        signature: signature.as_bytes().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{keccak256, Signature, U256};
    use alloy_signer_local::PrivateKeySigner;

    #[test]
    fn update_state_type_hash_matches_registry() {
        // Must match the registry's `UPDATE_TYPEHASH` constant.
        let expected = keccak256(
            "UpdateState(address target,uint256 laneIndex,uint32 updateTimestamp,uint256[] slots)",
        );
        let update = UpdateState {
            target: Address::ZERO,
            laneIndex: U256::ZERO,
            updateTimestamp: 0,
            slots: vec![],
        };
        assert_eq!(update.eip712_type_hash(), expected);
    }

    #[tokio::test]
    async fn can_sign_priority_update() {
        let signer = PrivateKeySigner::random();
        let update = UpdateState {
            target: address!("0x5979458912f80b96d30d4220af8e2e4925a33320"),
            laneIndex: U256::ZERO,
            updateTimestamp: 1753977388,
            slots: vec![U256::from(1), U256::from(2)],
        };

        let signed = sign_priority_update(&signer, 1, update.clone()).await.unwrap();
        assert_eq!(signed.signer, signer.address());
        assert_eq!(signed.target, update.target);
        assert_eq!(signed.slots, update.slots);

        // the signature must recover to the signer and carry a 27/28 recovery byte as
        // expected by the registry's on-chain ECDSA recovery
        assert_eq!(signed.signature.len(), 65);
        assert!(matches!(signed.signature[64], 27 | 28));
        let digest = update.eip712_signing_hash(&priority_update_registry_domain(1));
        let signature = Signature::from_raw(&signed.signature).unwrap();
        assert_eq!(signature.recover_address_from_prehash(&digest).unwrap(), signer.address());
    }
}
