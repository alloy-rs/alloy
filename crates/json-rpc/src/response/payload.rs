use serde_json::value::RawValue;

use crate::ErrorPayload;

/// A Response payload is either a successful response or an error response.
pub type ResponsePayload<Payload = Box<RawValue>, ErrData = Box<RawValue>> =
    Result<Payload, ErrorPayload<ErrData>>;

/// A [`ResponsePayload`] whose values are borrowed from the deserialzer.
pub type BorrowedResponsePayload<'a> = ResponsePayload<&'a RawValue, &'a RawValue>;
