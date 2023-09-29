use alloy_json_rpc::{Id, Request, Response, ResponsePayload};
use alloy_primitives::U256;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use tokio::sync::oneshot;

use crate::{pubsub::InFlight, TransportError};

use super::in_flight;

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub struct RequestManager {
    reqs: BTreeMap<Id, InFlight>,
}

impl RequestManager {
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
