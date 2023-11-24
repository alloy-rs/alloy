//! These utils are NOT meant for general usage. They are ONLY meant for use
//! within this module. They DO NOT perform basic safety checks and may panic
//! if used incorrectly.

use alloy_primitives::B256;
use alloy_signer::Signature;
use k256::ecdsa::{self, RecoveryId, VerifyingKey};

/// Recover an rsig from a signature under a known key by trial/error.
pub(super) fn sig_from_digest_bytes_trial_recovery(
    sig: ecdsa::Signature,
    digest: &B256,
    vk: &VerifyingKey,
) -> Signature {
    let mut recid = RecoveryId::from_byte(0).unwrap();
    if check_candidate(&sig, recid, digest, vk) {
        return Signature::new(sig, recid);
    }

    recid = RecoveryId::from_byte(1).unwrap();
    if check_candidate(&sig, recid, digest, vk) {
        return Signature::new(sig, recid);
    }

    panic!("bad sig");
}

/// Makes a trial recovery to check whether an RSig corresponds to a known `VerifyingKey`.
fn check_candidate(
    sig: &ecdsa::Signature,
    recid: RecoveryId,
    digest: &B256,
    vk: &VerifyingKey,
) -> bool {
    VerifyingKey::recover_from_prehash(digest.as_slice(), sig, recid)
        .map(|key| key == *vk)
        .unwrap_or(false)
}
