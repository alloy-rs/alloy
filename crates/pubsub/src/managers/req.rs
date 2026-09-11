use crate::managers::InFlight;
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::map::HashMap;

/// Manages in-flight requests.
#[derive(Debug)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, InFlight>,
    next_unsubscribe_request_id: u64,
}

impl Default for RequestManager {
    fn default() -> Self {
        Self { reqs: HashMap::default(), next_unsubscribe_request_id: u64::MAX }
    }
}

impl RequestManager {
    /// Get the number of in-flight requests.
    pub(crate) fn len(&self) -> usize {
        self.reqs.len()
    }

    /// Get an iterator over the in-flight requests.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Id, &InFlight)> {
        self.reqs.iter()
    }

    /// Insert a new in-flight request.
    pub(crate) fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id().clone(), in_flight);
    }

    /// Returns `true` if an in-flight request is using the given ID.
    pub(crate) fn contains_id(&self, id: &Id) -> bool {
        self.reqs.contains_key(id)
    }

    /// Allocate an ID for service-generated `eth_unsubscribe` requests.
    ///
    /// We allocate from the high end of the numeric range to avoid collisions
    /// with client-generated request IDs, which are allocated upward from zero.
    pub(crate) fn next_unsubscribe_request_id(&mut self) -> Id {
        loop {
            let id = Id::Number(self.next_unsubscribe_request_id);
            self.next_unsubscribe_request_id = self.next_unsubscribe_request_id.wrapping_sub(1);
            if !self.contains_id(&id) {
                return id;
            }
        }
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
}
