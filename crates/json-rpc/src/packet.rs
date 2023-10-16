use std::collections::HashSet;

use serde_json::value::RawValue;

use crate::{Id, Request, Response, RpcParam};

/// A [`RequestPacket`] is a [`Request`] or a batch of requests.
pub enum RequestPacket<Params> {
    Single(Request<Params>),
    Batch(Vec<Request<Params>>),
}

impl<Params> RequestPacket<Params>
where
    Params: RpcParam,
{
    /// Serialize request paramaters as a boxed [`RawValue`].
    ///
    /// # Panics
    ///
    /// If serialization of the params fails.
    pub fn box_params(self) -> RequestPacket<Box<RawValue>> {
        match self {
            Self::Single(req) => RequestPacket::Single(req.box_params()),
            Self::Batch(batch) => {
                RequestPacket::Batch(batch.into_iter().map(Request::box_params).collect())
            }
        }
    }

    /// Serialize the packet as a boxed [`RawValue`].
    pub fn serialize(&self) -> serde_json::Result<Box<RawValue>> {
        match self {
            Self::Single(req) => serde_json::to_string(req).and_then(RawValue::from_string),
            Self::Batch(batch) => serde_json::to_string(batch).and_then(RawValue::from_string),
        }
    }

    /// Get the request IDs of all subscription requests in the packet.
    pub fn subscription_request_ids(&self) -> HashSet<Id> {
        match self {
            RequestPacket::Single(single) => {
                let mut hs = HashSet::with_capacity(1);
                if single.method == "eth_subscribe" {
                    hs.insert(single.id.clone());
                }
                hs
            }
            RequestPacket::Batch(batch) => batch
                .iter()
                .filter(|req| req.method == "eth_subscribe")
                .map(|req| req.id.clone())
                .collect(),
        }
    }
}

/// A [`ResponsePacket`] is a [`Response`] or a batch of responses.
pub enum ResponsePacket<Payload, ErrData> {
    Single(Response<Payload, ErrData>),
    Batch(Vec<Response<Payload, ErrData>>),
}

/// A [`BorrowedResponsePacket`] is a [`ResponsePacket`] that has been partially
/// deserialized, borrowing its contents from the deserializer. This is used
/// primarily for intermediate deserialization. Most users will not require it.
pub type BorrowedResponsePacket<'a> = ResponsePacket<&'a RawValue, &'a RawValue>;

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
    pub fn responses_by_id(&self, ids: &HashSet<Id>) -> Vec<&Response<Payload, ErrData>> {
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
