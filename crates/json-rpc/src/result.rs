use crate::{Response, ResponsePayload, RpcError, RpcRecv};
use serde_json::value::RawValue;
use std::{any::TypeId, borrow::Borrow};

/// The result of a JSON-RPC request.
///
/// Either a success response, an error response, or a non-response error. The
/// non-response error is intended to be used for errors returned by a
/// transport, or serde errors.
///
/// The common cases are:
/// - `Ok(T)` - The server returned a successful response.
/// - `Err(RpcError::ErrorResponse(ErrResp))` - The server returned an error response.
/// - `Err(RpcError::SerError(E))` - A serialization error occurred.
/// - `Err(RpcError::DeserError { err: E, text: String })` - A deserialization error occurred.
/// - `Err(RpcError::TransportError(E))` - Some client-side or communication error occurred.
pub type RpcResult<T, E, ErrResp = Box<RawValue>> = Result<T, RpcError<E, ErrResp>>;

/// A partially deserialized [`RpcResult`], borrowing from the deserializer.
pub type BorrowedRpcResult<'a, E> = RpcResult<&'a RawValue, E, &'a RawValue>;

/// Transform a transport response into an [`RpcResult`], discarding the [`Id`].
///
/// [`Id`]: crate::Id
pub fn transform_response<T, E, ErrResp>(response: Response<T, ErrResp>) -> RpcResult<T, E, ErrResp>
where
    ErrResp: RpcRecv,
{
    transform_response_payload(response.payload)
}

/// Transform a response payload into an [`RpcResult`].
pub fn transform_response_payload<T, E, ErrResp>(
    payload: ResponsePayload<T, ErrResp>,
) -> RpcResult<T, E, ErrResp>
where
    ErrResp: RpcRecv,
{
    match payload {
        ResponsePayload::Failure(err_resp) => Err(RpcError::err_resp(err_resp)),
        ResponsePayload::Success(result) => Ok(result),
    }
}

/// Transform a transport outcome into an [`RpcResult`], discarding the [`Id`].
///
/// [`Id`]: crate::Id
pub fn transform_result<T, E, ErrResp>(
    response: Result<Response<T, ErrResp>, E>,
) -> Result<T, RpcError<E, ErrResp>>
where
    ErrResp: RpcRecv,
{
    match response {
        Ok(resp) => transform_response(resp),
        Err(e) => Err(RpcError::Transport(e)),
    }
}

/// Attempt to deserialize the `Ok(_)` variant of an [`RpcResult`].
pub fn try_deserialize_ok<J, T, E, ErrResp>(
    result: RpcResult<J, E, ErrResp>,
) -> RpcResult<T, E, ErrResp>
where
    J: Borrow<RawValue> + 'static,
    T: RpcRecv,
    ErrResp: RpcRecv,
{
    let json = result?;

    // Fast path: the caller wants the already-owned `Box<RawValue>` back unchanged. Hand it
    // over directly, skipping the byte copy and full JSON validation scan that
    // `from_str::<Box<RawValue>>` would repeat over an already-validated value.
    if TypeId::of::<J>() == TypeId::of::<Box<RawValue>>()
        && TypeId::of::<T>() == TypeId::of::<Box<RawValue>>()
    {
        // SAFETY: `J` and `T` are both `Box<RawValue>`, so this is a no-op reinterpretation.
        // `transmute_copy` stands in for the unstable `transmute_unchecked` (sizes are equal).
        let json = std::mem::ManuallyDrop::new(json);
        return Ok(unsafe { std::mem::transmute_copy::<J, T>(&json) });
    }

    let _guard = debug_span!("deserialize_response", ty=%std::any::type_name::<T>()).entered();
    let json = json.borrow().get();
    trace!(%json, "deserializing");
    serde_json::from_str(json)
        .inspect(|response| trace!(?response, "deserialized"))
        .inspect_err(|err| trace!(?err, "failed to deserialize"))
        .map_err(|err| RpcError::deser_err(err, json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_raw_value;

    #[test]
    fn raw_value_success_returns_payload_without_reencoding() {
        // Deliberately non-canonical spacing: a byte-identical result proves the payload
        // was not re-parsed and re-encoded.
        let src = "{  \"a\" :1,\"b\": [ 2 ,3 ] }";
        let raw = RawValue::from_string(src.to_owned()).unwrap();
        let ptr = raw.get().as_ptr();
        let input: RpcResult<Box<RawValue>, (), Box<RawValue>> = Ok(raw);

        let out = try_deserialize_ok::<_, Box<RawValue>, (), Box<RawValue>>(input).unwrap();

        assert_eq!(out.get(), src);
        // Same allocation: the owned payload was handed back, not rebuilt.
        assert_eq!(out.get().as_ptr(), ptr);
    }

    #[test]
    fn generic_deserialize_path_unchanged() {
        let raw = to_raw_value(&42u64).unwrap();
        let input: RpcResult<Box<RawValue>, (), Box<RawValue>> = Ok(raw);
        let out = try_deserialize_ok::<_, u64, (), Box<RawValue>>(input).unwrap();
        assert_eq!(out, 42);
    }
}
