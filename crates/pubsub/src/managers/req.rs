use crate::managers::InFlight;
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::map::HashMap;

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, InFlight>,
    /// Counter for generating IDs that don't collide with in-flight requests.
    next_id: u64,
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

    /// Returns an [`Id`] that is not currently used by any in-flight request.
    pub(crate) fn unused_id(&mut self) -> Id {
        loop {
            let id = Id::Number(self.next_id);
            self.next_id = self.next_id.wrapping_add(1);
            if !self.reqs.contains_key(&id) {
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
