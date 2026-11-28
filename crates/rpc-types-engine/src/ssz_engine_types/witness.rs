use super::*;

/// Stateless execution witness returned by `POST /payloads/witness` and
/// `GET /payloads/{payloadId}/witness`.
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
/// Producers must include a complete witness for every `VALID` payload and omit it for other
/// statuses. The wire container has two variable fields and an eight-byte fixed section.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode)]
pub struct PayloadStatusWithWitness {
    /// Result of processing the submitted payload.
    pub payload_status: PayloadStatus,
    /// Execution witness for a `VALID` payload.
    pub witness: Optional<ExecutionWitness>,
}

impl PayloadStatusWithWitness {
    /// Creates a response, dropping the witness unless the status is `VALID`.
    pub fn new(payload_status: PayloadStatus, witness: Option<ExecutionWitness>) -> Self {
        let witness = if payload_status.status == PayloadStatusKind::Valid {
            witness.into()
        } else {
            Optional::none()
        };
        Self { payload_status, witness }
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
        }
        let Fields { payload_status, witness } = Fields::from_ssz_bytes(bytes)?;
        if payload_status.status != PayloadStatusKind::Valid && witness.is_some() {
            return Err(ssz::DecodeError::BytesInvalid(
                "execution witness is only valid for VALID payload status".into(),
            ));
        }
        if payload_status.status == PayloadStatusKind::Valid && witness.is_none() {
            return Err(ssz::DecodeError::BytesInvalid(
                "VALID payload status requires an execution witness".into(),
            ));
        }
        Ok(Self { payload_status, witness })
    }
}

/// Experimental response of `GET /payloads/{payloadId}/witness` for Amsterdam.
///
/// Both fields describe the same selected build snapshot. Every successful response includes a
/// complete witness; a producer that cannot generate it must return an HTTP error instead.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadWithWitness {
    /// The Amsterdam payload returned by ordinary payload retrieval.
    pub built_payload: BuiltPayloadAmsterdam,
    /// Complete execution witness of the selected payload.
    pub witness: ExecutionWitness,
}
