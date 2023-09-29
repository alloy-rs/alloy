use alloy_json_rpc::{Id, Response};
use alloy_primitives::U256;
use std::collections::BTreeMap;

use crate::{pubsub::InFlight, TransportError};

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub struct RequestManager {
    reqs: BTreeMap<Id, InFlight>,
}

impl RequestManager {
    /// Get the number of in-flight requests.
    pub fn len(&self) -> usize {
        self.reqs.len()
    }

    /// Check if the request manager is empty.
    pub fn is_empty(&self) -> bool {
        self.reqs.is_empty()
    }

    /// Get an iterator over the in-flight requests.
    pub fn iter(&self) -> impl Iterator<Item = (&Id, &InFlight)> {
        self.reqs.iter()
    }

    /// Insert a new in-flight request.
    pub fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id.clone(), in_flight);
    }

    /// Get a reference to an in-flight request.
    pub fn get_req(&self, id: &Id) -> Option<&InFlight> {
        self.reqs.get(id)
    }

    /// Handle a response by sending the payload to the waiter.
    ///
    /// If the request created a new subscription, this function returns the
    /// subscription ID and the in-flight request for conversion to an
    /// `ActiveSubscription`.
    pub fn handle_response(&mut self, resp: Response) -> Option<(U256, InFlight)> {
        if let Some(in_flight) = self.reqs.remove(&resp.id) {
            return in_flight.fulfill(resp.payload);
        }
        None
    }

    /// Send an error to the waiter.
    pub fn handle_error(&mut self, id: &Id, error: TransportError) {
        if let Some(in_flight) = self.reqs.remove(id) {
            let _ = in_flight.tx.send(Err(error));
        }
    }
}
