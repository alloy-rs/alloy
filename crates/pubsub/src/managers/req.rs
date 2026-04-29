use crate::managers::InFlight;
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::map::HashMap;
use alloy_transport::TransportErrorKind;
use serde_json::value::RawValue;

/// Maximum number of times a single in-flight request may be re-issued across
/// reconnects before being dropped with a `backend_gone` error.
const MAX_REQUEST_REISSUES: u32 = 3;

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, InFlight>,
}

impl RequestManager {
    /// Insert a new in-flight request.
    pub(crate) fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id().clone(), in_flight);
    }

    /// Handle a response by sending the payload to the waiter.
    ///
    /// If the request created a new subscription, this function returns the
    /// subscription ID and the in-flight request for conversion to an
    /// `ActiveSubscription`.
    pub(crate) fn handle_response(&mut self, resp: Response) -> Option<(SubId, InFlight)> {
        if let Some(in_flight) = self.reqs.remove(&resp.id) {
            return in_flight.fulfill(resp);
        }
        None
    }

    /// Bump each request's reconnect count. Requests that have been re-issued
    /// [`MAX_REQUEST_REISSUES`] times are drained with a `backend_gone` error.
    /// Returns the serialized messages for the surviving requests so the caller
    /// can dispatch them to the new backend.
    pub(crate) fn reissue_or_drain(&mut self) -> Vec<Box<RawValue>> {
        let reqs = std::mem::take(&mut self.reqs);
        let mut to_reissue = Vec::new();

        for (id, mut in_flight) in reqs {
            if in_flight.reconnect_count >= MAX_REQUEST_REISSUES {
                warn!(
                    id = %id,
                    reconnect_count = in_flight.reconnect_count,
                    "Dropping in-flight request after too many reconnects"
                );
                let _ = in_flight.tx.send(Err(TransportErrorKind::backend_gone()));
            } else {
                in_flight.reconnect_count += 1;
                to_reissue.push(in_flight.request.serialized().to_owned());
                self.reqs.insert(id, in_flight);
            }
        }

        to_reissue
    }
}
