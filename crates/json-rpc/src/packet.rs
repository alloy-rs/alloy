use std::collections::HashSet;

use serde::{ser::SerializeSeq, Serialize};
use serde_json::value::RawValue;

use crate::{Id, Response, SerializedRequest};

/// A [`RequestPacket`] is a [`Request`] or a batch of requests.
pub enum RequestPacket {
    Single(SerializedRequest),
    Batch(Vec<SerializedRequest>),
}

impl Serialize for RequestPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RequestPacket::Single(single) => single.request().serialize(serializer),
            RequestPacket::Batch(batch) => {
                let mut seq = serializer.serialize_seq(Some(batch.len()))?;
                for req in batch {
                    seq.serialize_element(req.request())?;
                }
                seq.end()
            }
        }
    }
}

impl RequestPacket {
    /// Serialize the packet as a boxed [`RawValue`].
    pub fn serialize(&self) -> serde_json::Result<Box<RawValue>> {
        serde_json::to_string(self).and_then(RawValue::from_string)
    }

    /// Get the request IDs of all subscription requests in the packet.
    pub fn subscription_request_ids(&self) -> HashSet<&Id> {
        match self {
            RequestPacket::Single(single) => {
                let mut hs = HashSet::with_capacity(1);
                if single.method() == "eth_subscribe" {
                    hs.insert(single.id());
                }
                hs
            }
            RequestPacket::Batch(batch) => batch
                .iter()
                .filter(|req| req.method() == "eth_subscribe")
                .map(|req| req.id())
                .collect(),
        }
    }
}

/// A [`ResponsePacket`] is a [`Response`] or a batch of responses.
pub enum ResponsePacket<Payload = Box<RawValue>, ErrData = Box<RawValue>> {
    Single(Response<Payload, ErrData>),
    Batch(Vec<Response<Payload, ErrData>>),
}

/// A [`BorrowedResponsePacket`] is a [`ResponsePacket`] that has been partially
/// deserialized, borrowing its contents from the deserializer. This is used
/// primarily for intermediate deserialization. Most users will not require it.
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type BorrowedResponsePacket<'a> = ResponsePacket<&'a RawValue, &'a RawValue>;

impl BorrowedResponsePacket<'_> {
    /// Convert this borrowed response packet into an owned packet by copying
    /// the data from the deserializer (if necessary).
    pub fn into_owned(self) -> ResponsePacket {
        match self {
            Self::Single(single) => ResponsePacket::Single(single.into_owned()),
            Self::Batch(batch) => {
                ResponsePacket::Batch(batch.into_iter().map(Response::into_owned).collect())
            }
        }
    }
}

impl<Payload, ErrData> ResponsePacket<Payload, ErrData> {
    /// Find responses by a list of IDs.
    ///
    /// This is intended to be used in conjunction with
    /// [`RequestPacket::subscription_request_ids`] to identify subscription
    /// responses.
    ///
    /// # Note
    ///
    /// - Responses are not guaranteed to be in the same order.
    /// - Responses are not guaranteed to be in the set.
    /// - If the packet contains duplicate IDs, both will be found.
    pub fn responses_by_ids(&self, ids: &HashSet<Id>) -> Vec<&Response<Payload, ErrData>> {
        match self {
            Self::Single(single) => {
                let mut resps = Vec::new();
                if ids.contains(&single.id) {
                    resps.push(single);
                }
                resps
            }
            Self::Batch(batch) => batch.iter().filter(|res| ids.contains(&res.id)).collect(),
        }
    }
}
