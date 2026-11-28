//! [EIP-7594] sidecars that keep only some of each blob's cells, as held by eth/72 ([EIP-8070])
//! samplers.
//!
//! [EIP-7594]: https://eips.ethereum.org/EIPS/eip-7594
//! [EIP-8070]: https://eips.ethereum.org/EIPS/eip-8070

use crate::{
    eip4844::{kzg_to_versioned_hash, BlobCellsAndProofsV1, Bytes48},
    eip7594::{BlobCellMask, BlobTransactionSidecarEip7594, Cell, CELLS_PER_EXT_BLOB},
};
use alloc::vec::Vec;
use alloy_primitives::B256;

#[cfg(feature = "kzg")]
use crate::{
    eip4844::{c_kzg, AsCkzg, BlobTransactionValidationError},
    eip7594::BlobCellRecoveryError,
};

/// An EIP-7594 sidecar that keeps the same subset of cells for every blob.
///
/// All commitments and all [`CELLS_PER_EXT_BLOB`] proofs per blob are kept, so the transaction
/// body can be served without its blobs. Cells are ordered by blob, then by ascending cell index.
/// [`Self::new`] ensures the cell and proof counts match the commitments and the mask.
///
/// # Examples
///
/// ```
/// use alloy_eips::eip7594::{
///     BlobCellMask, BlobTransactionCellSidecar, BlobTransactionSidecarEip7594,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let settings = alloy_eips::eip4844::env_settings::EnvKzgSettings::Default.get();
/// let full = BlobTransactionSidecarEip7594::try_from_blobs(vec![Default::default()])?;
/// let all = BlobTransactionCellSidecar::try_from_sidecar(&full, settings)?;
///
/// // Keep the first half of the columns: enough to rebuild the blob.
/// let half = BlobCellMask::ALL.first(64);
/// let cells = all.matching_cells(half).expect("all cells are kept");
/// let kept = BlobTransactionCellSidecar::new(
///     all.commitments().to_vec(),
///     all.proofs().to_vec(),
///     cells,
///     half,
/// )
/// .expect("counts match");
///
/// kept.validate(&full.versioned_hashes().collect::<Vec<_>>(), settings)?;
/// assert_eq!(kept.recover(settings)?, full);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BlobTransactionCellSidecar {
    // One commitment per blob.
    commitments: Vec<Bytes48>,
    // All `CELLS_PER_EXT_BLOB` cell proofs of every blob, ordered by blob.
    proofs: Vec<Bytes48>,
    // The kept cells, ordered by blob, then by ascending cell index.
    cells: Vec<Cell>,
    // Indices of the cells kept for every blob.
    mask: BlobCellMask,
}

impl core::fmt::Debug for BlobTransactionCellSidecar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobTransactionCellSidecar")
            .field("commitments", &self.commitments)
            .field("proofs", &self.proofs)
            .field("cells", &self.cells.len())
            .field("mask", &self.mask)
            .finish()
    }
}

impl BlobTransactionCellSidecar {
    /// Creates a sidecar from its parts.
    ///
    /// Returns `None` unless there is at least one commitment and one cell index, `cells` holds
    /// the masked cells of every blob, and `proofs` holds all [`CELLS_PER_EXT_BLOB`] proofs of
    /// every blob.
    pub fn new(
        commitments: Vec<Bytes48>,
        proofs: Vec<Bytes48>,
        cells: Vec<Cell>,
        mask: BlobCellMask,
    ) -> Option<Self> {
        let blobs = commitments.len();
        (blobs > 0
            && mask.count() > 0
            && blobs.checked_mul(mask.count()) == Some(cells.len())
            && blobs.checked_mul(CELLS_PER_EXT_BLOB) == Some(proofs.len()))
        .then_some(Self { commitments, proofs, cells, mask })
    }

    /// Computes every cell of a full sidecar.
    ///
    /// This does not check the sidecar's proofs. Call
    /// [`BlobTransactionSidecarEip7594::validate`] first for untrusted input.
    ///
    /// # Errors
    ///
    /// Returns an error if a blob is not valid, or if the blob, commitment and proof counts do
    /// not match.
    #[cfg(feature = "kzg")]
    pub fn try_from_sidecar(
        sidecar: &BlobTransactionSidecarEip7594,
        settings: &c_kzg::KzgSettings,
    ) -> Result<Self, c_kzg::Error> {
        let cells = sidecar.compute_cells_with_settings(settings)?;
        Self::new(
            sidecar.commitments.clone(),
            sidecar.cell_proofs.clone(),
            cells,
            BlobCellMask::ALL,
        )
        .ok_or_else(|| {
            c_kzg::Error::MismatchLength(alloc::format!(
                "{} blobs, {} commitments and {} cell proofs",
                sidecar.blobs.len(),
                sidecar.commitments.len(),
                sidecar.cell_proofs.len()
            ))
        })
    }

    /// Returns one commitment per blob.
    pub fn commitments(&self) -> &[Bytes48] {
        &self.commitments
    }

    /// Returns all [`CELLS_PER_EXT_BLOB`] cell proofs of every blob, ordered by blob.
    pub fn proofs(&self) -> &[Bytes48] {
        &self.proofs
    }

    /// Returns the kept cells, ordered by blob, then by ascending cell index.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Returns the indices of the cells kept for every blob.
    pub const fn mask(&self) -> BlobCellMask {
        self.mask
    }

    /// Returns true if enough cells are kept to rebuild every blob.
    pub const fn is_recoverable(&self) -> bool {
        self.mask.is_recoverable()
    }

    /// Returns the versioned hash of every commitment, in blob order.
    pub fn versioned_hashes(&self) -> impl Iterator<Item = B256> + '_ {
        self.commitments.iter().map(|commitment| kzg_to_versioned_hash(commitment.as_slice()))
    }

    /// Checks the commitments against `versioned_hashes` and every kept cell against its proof.
    ///
    /// # Errors
    ///
    /// Returns [`BlobTransactionValidationError::WrongVersionedHash`] for a commitment that does
    /// not match its versioned hash, and [`BlobTransactionValidationError::InvalidProof`] if a
    /// cell does not match its proof.
    #[cfg(feature = "kzg")]
    pub fn validate(
        &self,
        versioned_hashes: &[B256],
        settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        if versioned_hashes.len() != self.commitments.len() {
            return Err(c_kzg::Error::MismatchLength(alloc::format!(
                "{} versioned hashes and {} commitments",
                versioned_hashes.len(),
                self.commitments.len()
            ))
            .into());
        }
        for (&have, expected) in versioned_hashes.iter().zip(self.versioned_hashes()) {
            if have != expected {
                return Err(BlobTransactionValidationError::WrongVersionedHash { have, expected });
            }
        }
        let mut commitments = Vec::with_capacity(self.cells.len());
        let mut proofs = Vec::with_capacity(self.cells.len());
        let mut indices = Vec::with_capacity(self.cells.len());
        for (blob_proofs, commitment) in
            self.proofs.as_chunks::<CELLS_PER_EXT_BLOB>().0.iter().zip(&self.commitments)
        {
            for index in self.mask.selected_indices() {
                commitments.push(*commitment);
                proofs.push(blob_proofs[index]);
                indices.push(index as u64);
            }
        }
        if !settings.verify_cell_kzg_proof_batch(
            Bytes48::slice_as_ckzg(&commitments),
            &indices,
            Cell::slice_as_ckzg(&self.cells),
            Bytes48::slice_as_ckzg(&proofs),
        )? {
            return Err(BlobTransactionValidationError::InvalidProof);
        }
        Ok(())
    }

    /// Rebuilds the full sidecar, checking each rebuilt blob against its commitment.
    ///
    /// Only the first half of the kept columns is used: it determines the blob, so once its
    /// commitment matches, the recomputed proofs are valid too, even if other kept cells are
    /// corrupt.
    ///
    /// # Errors
    ///
    /// Returns [`BlobCellRecoveryError::InsufficientCells`] if fewer than half of the columns
    /// are kept, and [`BlobCellRecoveryError::CommitmentMismatch`] if a rebuilt blob does not
    /// match its commitment.
    #[cfg(feature = "kzg")]
    pub fn recover(
        &self,
        settings: &c_kzg::KzgSettings,
    ) -> Result<BlobTransactionSidecarEip7594, BlobCellRecoveryError> {
        let needed = CELLS_PER_EXT_BLOB / 2;
        let kept = self.mask.count();
        if kept <= needed {
            return BlobTransactionSidecarEip7594::try_recover_from_cells_with_settings(
                self.commitments.clone(),
                self.mask,
                &self.cells,
                settings,
            );
        }
        let cells: Vec<_> =
            self.cells.chunks_exact(kept).flat_map(|blob| &blob[..needed]).copied().collect();
        BlobTransactionSidecarEip7594::try_recover_from_cells_with_settings(
            self.commitments.clone(),
            self.mask.first(needed),
            &cells,
            settings,
        )
    }

    /// Returns the sidecar without blobs, as sent in eth/72 `PooledTransactions` responses.
    pub fn without_blobs(&self) -> BlobTransactionSidecarEip7594 {
        BlobTransactionSidecarEip7594::new(
            Vec::new(),
            self.commitments.clone(),
            self.proofs.clone(),
        )
    }

    /// Returns the cells at `mask` for every blob, ordered by blob, or `None` if any of them is
    /// not kept.
    pub fn matching_cells(&self, mask: BlobCellMask) -> Option<Vec<Cell>> {
        if !mask.is_subset_of(self.mask) {
            return None;
        }
        let positions: Vec<_> = mask.selected_indices().map(|index| self.position(index)).collect();
        let mut cells = Vec::with_capacity(self.commitments.len() * positions.len());
        for blob in self.cells.chunks_exact(self.mask.count()) {
            cells.extend(positions.iter().map(|&position| blob[position]));
        }
        Some(cells)
    }

    /// Returns `(index, cells and proofs at mask)` for every entry of `versioned_hashes` that is
    /// one of this sidecar's blobs, as `engine_getBlobsV4` returns them. Cells that are not kept
    /// are `None`, with their proof.
    pub fn match_versioned_hashes_cells<'a>(
        &'a self,
        versioned_hashes: &'a [B256],
        mask: BlobCellMask,
    ) -> impl Iterator<Item = (usize, BlobCellsAndProofsV1)> + 'a {
        let hashes: Vec<_> = self.versioned_hashes().collect();
        // Each requested cell index with its position among the kept cells, if kept.
        let requested: Vec<_> = mask
            .selected_indices()
            .map(|index| (index, self.mask.contains(index).then(|| self.position(index))))
            .collect();
        let kept = self.mask.count();
        versioned_hashes.iter().enumerate().filter_map(move |(index, wanted)| {
            let blob = hashes.iter().position(|hash| hash == wanted)?;
            let cells = &self.cells[blob * kept..][..kept];
            let proofs = &self.proofs[blob * CELLS_PER_EXT_BLOB..][..CELLS_PER_EXT_BLOB];
            let (blob_cells, proofs) = requested
                .iter()
                .map(|&(cell, position)| {
                    position.map(|position| (cells[position], proofs[cell])).unzip()
                })
                .unzip();
            Some((index, BlobCellsAndProofsV1 { blob_cells, proofs }))
        })
    }

    /// Returns the heap size of the cells, commitments and proofs, in bytes, including spare
    /// capacity.
    pub const fn size(&self) -> usize {
        self.cells.capacity() * size_of::<Cell>()
            + (self.commitments.capacity() + self.proofs.capacity()) * size_of::<Bytes48>()
    }

    // Returns the position of the kept cell `index` within each blob's kept cells.
    const fn position(&self, index: usize) -> usize {
        // `index` comes from a mask, so it is below `CELLS_PER_EXT_BLOB` and the shift can't
        // overflow.
        (self.mask.bits() & ((1u128 << index) - 1)).count_ones() as usize
    }
}

#[cfg(all(test, feature = "kzg"))]
mod tests {
    use super::*;
    use crate::eip4844::{env_settings::EnvKzgSettings, Blob};
    use alloc::vec;

    fn settings() -> &'static c_kzg::KzgSettings {
        EnvKzgSettings::Default.get()
    }

    // A full sidecar with two different blobs, and all of its cells.
    fn fixture() -> (BlobTransactionSidecarEip7594, BlobTransactionCellSidecar) {
        let mut second = Blob::default();
        second[31] = 1;
        let full =
            BlobTransactionSidecarEip7594::try_from_blobs(vec![Blob::default(), second]).unwrap();
        let cells = BlobTransactionCellSidecar::try_from_sidecar(&full, settings()).unwrap();
        (full, cells)
    }

    // Keeps only the cells of `all` at `mask`.
    fn subset(all: &BlobTransactionCellSidecar, mask: BlobCellMask) -> BlobTransactionCellSidecar {
        BlobTransactionCellSidecar::new(
            all.commitments().to_vec(),
            all.proofs().to_vec(),
            all.matching_cells(mask).unwrap(),
            mask,
        )
        .unwrap()
    }

    #[test]
    fn new_rejects_counts_that_do_not_match() {
        let (_, all) = fixture();
        let mask = BlobCellMask::from_bits(0b11);
        let cells = all.matching_cells(mask).unwrap();
        let new = |commitments: &[Bytes48], proofs: &[Bytes48], cells: &[Cell], mask| {
            BlobTransactionCellSidecar::new(
                commitments.to_vec(),
                proofs.to_vec(),
                cells.to_vec(),
                mask,
            )
        };
        assert!(new(all.commitments(), all.proofs(), &cells, mask).is_some());
        assert!(new(&[], &[], &[], mask).is_none());
        assert!(new(all.commitments(), all.proofs(), &[], BlobCellMask::default()).is_none());
        assert!(new(all.commitments(), all.proofs(), &cells[1..], mask).is_none());
        assert!(new(all.commitments(), &all.proofs()[1..], &cells, mask).is_none());
        assert!(new(&all.commitments()[1..], all.proofs(), &cells, mask).is_none());
    }

    #[test]
    fn recovers_blobs_from_half_of_the_columns() {
        let (full, all) = fixture();
        assert!(all.is_recoverable());
        assert_eq!(all.recover(settings()).unwrap(), full);

        // Every other column, so the kept columns are not contiguous.
        let half = subset(&all, BlobCellMask::from_bits(u128::MAX / 3));
        assert_eq!(half.mask().count(), CELLS_PER_EXT_BLOB / 2);
        assert!(half.is_recoverable());
        assert_eq!(half.recover(settings()).unwrap(), full);

        // Cells past the first half of the kept columns are not used, so a corrupt one still
        // recovers valid proofs.
        let mut corrupt = all.clone();
        corrupt.cells[100][31] ^= 1;
        let recovered = corrupt.recover(settings()).unwrap();
        recovered.validate(&all.versioned_hashes().collect::<Vec<_>>(), settings()).unwrap();
        assert_eq!(recovered, full);

        let too_few = subset(&all, BlobCellMask::ALL.first(CELLS_PER_EXT_BLOB / 2 - 1));
        assert!(!too_few.is_recoverable());
        assert!(matches!(
            too_few.recover(settings()),
            Err(BlobCellRecoveryError::InsufficientCells { provided: 63, required: 64 })
        ));
    }

    #[test]
    fn validate_rejects_bad_cells_proofs_and_hashes() {
        let (full, all) = fixture();
        let valid = subset(&all, BlobCellMask::from_bits((1 << 127) | 1));
        let hashes: Vec<_> = valid.versioned_hashes().collect();
        valid.validate(&hashes, settings()).unwrap();

        let mut bad_cell = valid.clone();
        bad_cell.cells[0][31] ^= 1;
        assert!(matches!(
            bad_cell.validate(&hashes, settings()),
            Err(BlobTransactionValidationError::InvalidProof)
        ));

        // The second blob's proof for the same column is a valid point but the wrong proof.
        let mut bad_proof = valid.clone();
        bad_proof.proofs.swap(0, CELLS_PER_EXT_BLOB);
        assert!(matches!(
            bad_proof.validate(&hashes, settings()),
            Err(BlobTransactionValidationError::InvalidProof)
        ));

        // `have` is the transaction's hash and `expected` the commitment's, as for full sidecars.
        let mut wrong_hashes = hashes.clone();
        wrong_hashes[1] = B256::ZERO;
        for result in
            [valid.validate(&wrong_hashes, settings()), full.validate(&wrong_hashes, settings())]
        {
            assert!(matches!(
                result,
                Err(BlobTransactionValidationError::WrongVersionedHash { have, expected })
                    if have == B256::ZERO && expected == hashes[1]
            ));
        }
        assert!(matches!(
            valid.validate(&hashes[..1], settings()),
            Err(BlobTransactionValidationError::KZGError(c_kzg::Error::MismatchLength(_)))
        ));
    }

    #[test]
    fn matching_cells_returns_kept_columns_only() {
        let (_, all) = fixture();
        let kept = subset(&all, BlobCellMask::from_bits(0b1011));
        let cells = kept.matching_cells(BlobCellMask::from_bits(0b1001)).unwrap();
        let expected = vec![all.cells[0], all.cells[3], all.cells[128], all.cells[131]];
        assert_eq!(cells.capacity(), expected.len());
        assert_eq!(cells, expected);
        assert_eq!(kept.matching_cells(BlobCellMask::from_bits(0b0100)), None);
    }

    #[test]
    fn without_blobs_keeps_commitments_and_proofs() {
        let (full, all) = fixture();
        let mut expected = full;
        expected.blobs.clear();
        assert_eq!(subset(&all, BlobCellMask::from_bits(1)).without_blobs(), expected);
        assert_eq!(
            all.size(),
            2 * CELLS_PER_EXT_BLOB * size_of::<Cell>()
                + (2 + 2 * CELLS_PER_EXT_BLOB) * size_of::<Bytes48>()
        );

        // Spare capacity is still allocated, so it is counted.
        let mask = BlobCellMask::from_bits(1);
        let mut cells = Vec::with_capacity(all.cells().len());
        cells.extend(all.matching_cells(mask).unwrap());
        let capacity = cells.capacity();
        let sparse = BlobTransactionCellSidecar::new(
            all.commitments().to_vec(),
            all.proofs().to_vec(),
            cells,
            mask,
        )
        .unwrap();
        assert_eq!(
            sparse.size(),
            capacity * size_of::<Cell>() + (2 + 2 * CELLS_PER_EXT_BLOB) * size_of::<Bytes48>()
        );
    }

    #[test]
    fn match_versioned_hashes_cells_marks_missing_cells() {
        let (_, all) = fixture();
        let kept = subset(&all, BlobCellMask::from_bits(0b1011));
        let hashes: Vec<_> = kept.versioned_hashes().collect();
        let request = [hashes[1], B256::ZERO, hashes[1]];
        let cell = |blob: usize, index: usize| Some(all.cells[blob * CELLS_PER_EXT_BLOB + index]);
        let proof = |blob: usize, index: usize| Some(all.proofs[blob * CELLS_PER_EXT_BLOB + index]);
        let expected = BlobCellsAndProofsV1 {
            blob_cells: vec![cell(1, 0), None, cell(1, 3)],
            proofs: vec![proof(1, 0), None, proof(1, 3)],
        };
        assert_eq!(
            kept.match_versioned_hashes_cells(&request, BlobCellMask::from_bits(0b1101))
                .collect::<Vec<_>>(),
            vec![(0, expected.clone()), (2, expected)]
        );
    }

    #[test]
    fn try_from_sidecar_rejects_counts_that_do_not_match() {
        let (mut full, _) = fixture();
        full.cell_proofs.pop();
        assert!(matches!(
            BlobTransactionCellSidecar::try_from_sidecar(&full, settings()),
            Err(c_kzg::Error::MismatchLength(message))
                if message == "2 blobs, 2 commitments and 255 cell proofs"
        ));
    }
}
