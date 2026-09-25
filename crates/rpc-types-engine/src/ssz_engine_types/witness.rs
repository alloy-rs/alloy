use super::*;

/// Stateless execution witness returned by `POST /payloads/witness`.
///
/// Items are opaque byte lists following the execution-specs stateless witness. For Amsterdam,
/// `headers` holds between 1 and 256 parent-linked headers in oldest-to-newest order, ending at the
/// payload's parent.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionWitness {
    /// RLP-encoded account and storage trie nodes needed during execution and state-root
    /// recomputation.
    #[ssz(with = "witness_state")]
    pub state: Vec<Bytes>,
    /// Contract bytecode fetched from the pre-state during execution.
    #[ssz(with = "witness_codes")]
    pub codes: Vec<Bytes>,
    /// RLP-encoded ancestor headers that establish the pre-state and verify `BLOCKHASH` results.
    #[ssz(with = "witness_headers")]
    pub headers: Vec<Bytes>,
}

/// Response of `POST /payloads/witness`.
///
/// `witness` and `public_keys` are populated only for `VALID` payloads, and decoding rejects
/// either one for any other status. Producers must include a complete witness and one sender
/// public key per transaction in every `VALID` response.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode)]
pub struct PayloadStatusWithWitness {
    /// Result of processing the submitted payload.
    pub payload_status: PayloadStatus,
    /// Execution witness for a `VALID` payload.
    pub witness: Optional<ExecutionWitness>,
    /// Uncompressed SEC1 secp256k1 sender public keys (`0x04 || x || y`), one per transaction in
    /// payload order.
    ///
    /// These are untrusted inputs: stateless validators must verify each key against the
    /// corresponding transaction's signature and recovery ID.
    #[ssz(with = "public_keys")]
    pub public_keys: Vec<FixedBytes<65>>,
}

impl PayloadStatusWithWitness {
    /// Creates a response, dropping the witness and public keys unless the status is `VALID`.
    pub fn new(
        payload_status: PayloadStatus,
        witness: Option<ExecutionWitness>,
        public_keys: Vec<FixedBytes<65>>,
    ) -> Self {
        if payload_status.status != PayloadStatusKind::Valid {
            return Self { payload_status, witness: Optional::none(), public_keys: Vec::new() };
        }
        Self { payload_status, witness: witness.into(), public_keys }
    }
}

impl ssz::Decode for PayloadStatusWithWitness {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        #[derive(ssz_derive::Decode)]
        struct Fields {
            payload_status: PayloadStatus,
            witness: Optional<ExecutionWitness>,
            #[ssz(with = "public_keys")]
            public_keys: Vec<FixedBytes<65>>,
        }
        let Fields { payload_status, witness, public_keys } = Fields::from_ssz_bytes(bytes)?;
        if payload_status.status != PayloadStatusKind::Valid
            && (witness.is_some() || !public_keys.is_empty())
        {
            return Err(ssz::DecodeError::BytesInvalid(
                "witness and public keys are only valid for VALID payload status".into(),
            ));
        }
        Ok(Self { payload_status, witness, public_keys })
    }
}
