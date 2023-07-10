use std::borrow::Cow;

use jsonrpsee_types::{ErrorResponse, Response};

use serde::{Deserialize, Serialize};
use serde_json::{self, value::RawValue};

use crate::{common::JsonRpcResultOwned, TransportError};

pub(crate) fn to_json_raw_value<S>(s: &S) -> Result<Box<RawValue>, TransportError>
where
    S: Serialize,
{
    RawValue::from_string(serde_json::to_string(s).map_err(TransportError::ser_err)?)
        .map_err(TransportError::ser_err)
}

pub(crate) fn from_json<T, S>(s: S) -> Result<T, TransportError>
where
    T: for<'de> Deserialize<'de>,
    S: AsRef<str>,
{
    let s = s.as_ref();
    match serde_json::from_str(s) {
        Ok(val) => Ok(val),
        Err(err) => Err(TransportError::SerdeJson {
            err,
            text: s.to_owned(),
        }),
    }
}

pub(crate) fn deser_rpc_result(resp: &str) -> Result<JsonRpcResultOwned, TransportError> {
    if let Ok(err) = serde_json::from_str::<ErrorResponse<'_>>(resp) {
        return Ok(Err(err.error_object().to_owned().into_owned()));
    }
    let deser = serde_json::from_str::<Response<'_, Cow<'_, RawValue>>>(resp);
    match deser {
        Ok(v) => Ok(Ok(v.result)),
        Err(err) => Err(TransportError::SerdeJson {
            err,
            text: resp.to_owned(),
        }),
    }
}
