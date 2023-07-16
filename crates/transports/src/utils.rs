use serde::Serialize;
use serde_json::{self, value::RawValue};

use crate::error::TransportError;

pub(crate) fn to_json_raw_value<S>(s: &S) -> Result<Box<RawValue>, TransportError>
where
    S: Serialize,
{
    RawValue::from_string(serde_json::to_string(s).map_err(TransportError::ser_err)?)
        .map_err(TransportError::ser_err)
}
