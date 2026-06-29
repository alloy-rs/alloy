use crate::{ErrorPayload, Id, Response, ResponsePayload, SerializedRequest};
use alloy_primitives::map::HashSet;
use http::HeaderMap;
use serde::{
    de::{self, Deserializer, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::value::RawValue;
use std::{borrow::Borrow, fmt, hash::Hash, marker::PhantomData};

/// A [`RequestPacket`] is a [`SerializedRequest`] or a batch of serialized
/// request.
#[derive(Clone, Debug)]
pub enum RequestPacket {
    /// A single request.
    Single(SerializedRequest),
    /// A batch of requests.
    Batch(Vec<SerializedRequest>),
}

impl FromIterator<SerializedRequest> for RequestPacket {
    fn from_iter<T: IntoIterator<Item = SerializedRequest>>(iter: T) -> Self {
        Self::Batch(iter.into_iter().collect())
    }
}

impl From<SerializedRequest> for RequestPacket {
    fn from(req: SerializedRequest) -> Self {
        Self::Single(req)
    }
}

impl Serialize for RequestPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Single(single) => single.serialize(serializer),
            Self::Batch(batch) => batch.serialize(serializer),
        }
    }
}

impl RequestPacket {
    /// Create a new empty packet with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::Batch(Vec::with_capacity(capacity))
    }

    /// Returns the [`SerializedRequest`] if this packet is [`RequestPacket::Single`]
    pub const fn as_single(&self) -> Option<&SerializedRequest> {
        match self {
            Self::Single(req) => Some(req),
            Self::Batch(_) => None,
        }
    }

    /// Returns the batch of [`SerializedRequest`] if this packet is [`RequestPacket::Batch`]
    pub const fn as_batch(&self) -> Option<&[SerializedRequest]> {
        match self {
            Self::Batch(req) => Some(req.as_slice()),
            Self::Single(_) => None,
        }
    }

    /// Serialize the packet as a boxed [`RawValue`].
    pub fn serialize(self) -> serde_json::Result<Box<RawValue>> {
        match self {
            Self::Single(single) => Ok(single.take_request()),
            Self::Batch(batch) => serde_json::value::to_raw_value(&batch),
        }
    }

    /// Get the request IDs of all subscription requests in the packet.
    pub fn subscription_request_ids(&self) -> HashSet<&Id> {
        match self {
            Self::Single(single) => {
                let id = single.is_subscription().then(|| single.id());
                HashSet::from_iter(id)
            }
            Self::Batch(batch) => {
                batch.iter().filter(|req| req.is_subscription()).map(|req| req.id()).collect()
            }
        }
    }

    /// Get the number of requests in the packet.
    pub const fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Batch(batch) => batch.len(),
        }
    }

    /// Check if the packet is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Push a request into the packet.
    pub fn push(&mut self, req: SerializedRequest) {
        match self {
            Self::Batch(batch) => batch.push(req),
            Self::Single(_) => {
                let old = std::mem::replace(self, Self::Batch(Vec::with_capacity(10)));
                if let Self::Single(single) = old {
                    self.push(single);
                }
                self.push(req);
            }
        }
    }

    /// Returns all [`SerializedRequest`].
    pub const fn requests(&self) -> &[SerializedRequest] {
        match self {
            Self::Single(req) => std::slice::from_ref(req),
            Self::Batch(req) => req.as_slice(),
        }
    }

    /// Returns a mutable reference to all [`SerializedRequest`].
    pub const fn requests_mut(&mut self) -> &mut [SerializedRequest] {
        match self {
            Self::Single(req) => std::slice::from_mut(req),
            Self::Batch(req) => req.as_mut_slice(),
        }
    }

    /// Returns an iterator over the requests' method names
    pub fn method_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.requests().iter().map(|req| req.method())
    }

    /// Retrieves the combined headers from all requests in the packet. If
    /// multiple requests contain the same header, the last one wins.
    pub fn headers(&self) -> HeaderMap {
        self.requests().iter().fold(HeaderMap::new(), |mut acc, req| {
            if let Some(http_header_extension) = req.meta().extensions().get::<HeaderMap>() {
                acc.extend(http_header_extension.iter().map(|(k, v)| (k.clone(), v.clone())));
            };
            acc
        })
    }
}

/// A [`ResponsePacket`] is a [`Response`] or a batch of responses.
#[derive(Clone, Debug)]
pub enum ResponsePacket<Payload = Box<RawValue>, ErrData = Box<RawValue>> {
    /// A single response.
    Single(Response<Payload, ErrData>),
    /// A batch of responses.
    Batch(Vec<Response<Payload, ErrData>>),
}

impl<Payload, ErrData> FromIterator<Response<Payload, ErrData>>
    for ResponsePacket<Payload, ErrData>
{
    fn from_iter<T: IntoIterator<Item = Response<Payload, ErrData>>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();
        // return single if iter has exactly one element, else make a batch
        if let Some(first) = iter.next() {
            return if iter.peek().is_none() {
                Self::Single(first)
            } else {
                let mut batch = Vec::new();
                batch.push(first);
                batch.extend(iter);
                Self::Batch(batch)
            };
        }
        Self::Batch(vec![])
    }
}

impl<Payload, ErrData> From<Vec<Response<Payload, ErrData>>> for ResponsePacket<Payload, ErrData> {
    fn from(value: Vec<Response<Payload, ErrData>>) -> Self {
        if value.len() == 1 {
            Self::Single(value.into_iter().next().unwrap())
        } else {
            Self::Batch(value)
        }
    }
}

impl<'de, Payload, ErrData> Deserialize<'de> for ResponsePacket<Payload, ErrData>
where
    Payload: Deserialize<'de>,
    ErrData: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResponsePacketVisitor<Payload, ErrData> {
            marker: PhantomData<fn() -> ResponsePacket<Payload, ErrData>>,
        }

        impl<'de, Payload, ErrData> Visitor<'de> for ResponsePacketVisitor<Payload, ErrData>
        where
            Payload: Deserialize<'de>,
            ErrData: Deserialize<'de>,
        {
            type Value = ResponsePacket<Payload, ErrData>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a single response or a batch of responses")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut responses = Vec::new();

                while let Some(response) = seq.next_element()? {
                    responses.push(response);
                }

                Ok(ResponsePacket::Batch(responses))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let response =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(ResponsePacket::Single(response))
            }
        }

        deserializer.deserialize_any(ResponsePacketVisitor { marker: PhantomData })
    }
}

/// A [`BorrowedResponsePacket`] is a [`ResponsePacket`] that has been partially deserialized,
/// borrowing its contents from the deserializer.
///
/// This is used primarily for intermediate deserialization. Most users will not require it.
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
    /// Returns the [`Response`] if this packet is [`ResponsePacket::Single`].
    pub const fn as_single(&self) -> Option<&Response<Payload, ErrData>> {
        match self {
            Self::Single(resp) => Some(resp),
            Self::Batch(_) => None,
        }
    }

    /// Returns the batch of [`Response`] if this packet is [`ResponsePacket::Batch`].
    pub const fn as_batch(&self) -> Option<&[Response<Payload, ErrData>]> {
        match self {
            Self::Batch(resp) => Some(resp.as_slice()),
            Self::Single(_) => None,
        }
    }

    /// Returns the [`ResponsePayload`] if this packet is [`ResponsePacket::Single`].
    pub fn single_payload(&self) -> Option<&ResponsePayload<Payload, ErrData>> {
        self.as_single().map(|resp| &resp.payload)
    }

    /// Returns `true` if the response payload is a success.
    ///
    /// For batch responses, this returns `true` if __all__ responses are successful.
    pub fn is_success(&self) -> bool {
        match self {
            Self::Single(single) => single.is_success(),
            Self::Batch(batch) => batch.iter().all(|res| res.is_success()),
        }
    }

    /// Returns `true` if the response payload is an error.
    ///
    /// For batch responses, this returns `true` there's at least one error response.
    pub fn is_error(&self) -> bool {
        match self {
            Self::Single(single) => single.is_error(),
            Self::Batch(batch) => batch.iter().any(|res| res.is_error()),
        }
    }

    /// Returns the [ErrorPayload] if the response is an error.
    ///
    /// For batch responses, this returns the first error response.
    pub fn as_error(&self) -> Option<&ErrorPayload<ErrData>> {
        self.iter_errors().next()
    }

    /// Returns an iterator over the [ErrorPayload]s in the response.
    pub fn iter_errors(&self) -> impl Iterator<Item = &ErrorPayload<ErrData>> + '_ {
        match self {
            Self::Single(single) => ResponsePacketErrorsIter::Single(Some(single)),
            Self::Batch(batch) => ResponsePacketErrorsIter::Batch(batch.iter()),
        }
    }

    /// Returns the first error code in this packet if it contains any error responses.
    pub fn first_error_code(&self) -> Option<i64> {
        self.as_error().map(|error| error.code)
    }

    /// Returns the first error message in this packet if it contains any error responses.
    pub fn first_error_message(&self) -> Option<&str> {
        self.as_error().map(|error| error.message.as_ref())
    }

    /// Returns the first error data in this packet if it contains any error responses.
    pub fn first_error_data(&self) -> Option<&ErrData> {
        self.as_error().and_then(|error| error.data.as_ref())
    }

    /// Returns a all [`Response`].
    pub const fn responses(&self) -> &[Response<Payload, ErrData>] {
        match self {
            Self::Single(req) => std::slice::from_ref(req),
            Self::Batch(req) => req.as_slice(),
        }
    }

    /// Returns an iterator over the responses' payloads.
    pub fn payloads(&self) -> impl Iterator<Item = &ResponsePayload<Payload, ErrData>> + '_ {
        self.responses().iter().map(|resp| &resp.payload)
    }

    /// Returns the first [`ResponsePayload`] in this packet.
    pub fn first_payload(&self) -> Option<&ResponsePayload<Payload, ErrData>> {
        self.payloads().next()
    }

    /// Returns an iterator over the responses' identifiers.
    pub fn response_ids(&self) -> impl Iterator<Item = &Id> + '_ {
        self.responses().iter().map(|resp| &resp.id)
    }

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
    pub fn responses_by_ids<K>(&self, ids: &HashSet<K>) -> Vec<&Response<Payload, ErrData>>
    where
        K: Borrow<Id> + Eq + Hash,
    {
        match self {
            Self::Single(single) if ids.contains(&single.id) => vec![single],
            Self::Batch(batch) => batch.iter().filter(|res| ids.contains(&res.id)).collect(),
            _ => Vec::new(),
        }
    }
}

/// An Iterator over the [ErrorPayload]s in a [ResponsePacket].
#[derive(Clone, Debug)]
enum ResponsePacketErrorsIter<'a, Payload, ErrData> {
    Single(Option<&'a Response<Payload, ErrData>>),
    Batch(std::slice::Iter<'a, Response<Payload, ErrData>>),
}

impl<'a, Payload, ErrData> Iterator for ResponsePacketErrorsIter<'a, Payload, ErrData> {
    type Item = &'a ErrorPayload<ErrData>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ResponsePacketErrorsIter::Single(single) => single.take()?.payload.as_error(),
            ResponsePacketErrorsIter::Batch(batch) => loop {
                let res = batch.next()?;
                if let Some(err) = res.payload.as_error() {
                    return Some(err);
                }
            },
        }
    }
}
