use crate::managers::InFlight;
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::{map::HashMap, B256};

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, InFlight>,
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

    /// Remove pending subscription requests for the provided local ID.
    pub(crate) fn remove_subscription_requests(&mut self, local_id: &B256) -> usize {
        let req_ids = self
            .reqs
            .iter()
            .filter_map(|(id, in_flight)| {
                (in_flight.is_subscription() && in_flight.request.params_hash() == *local_id)
                    .then(|| id.clone())
            })
            .collect::<Vec<_>>();

        let removed = req_ids.len();
        for req_id in req_ids {
            self.reqs.remove(&req_id);
        }

        removed
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
