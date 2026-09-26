use crate::managers::InFlight;
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::map::HashMap;

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

    /// Returns an unused request ID for internally-generated requests.
    pub(crate) fn next_unused_id(&self) -> Id {
        let mut nonce = 0u64;
        loop {
            let candidate = Id::String(format!("alloy-internal-unsubscribe-{nonce}"));
            if !self.reqs.contains_key(&candidate) {
                return candidate;
            }
            nonce = nonce.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::Request;

    fn in_flight_with_id(id: Id) -> InFlight {
        let req = Request::new("eth_blockNumber", id, ()).serialize().unwrap();
        InFlight::new(req, 16).0
    }

    #[test]
    fn next_unused_id_uses_internal_namespace() {
        let manager = RequestManager::default();
        assert_eq!(manager.next_unused_id(), Id::String("alloy-internal-unsubscribe-0".to_owned()));
    }

    #[test]
    fn next_unused_id_skips_existing_internal_ids() {
        let mut manager = RequestManager::default();
        manager.insert(in_flight_with_id(Id::String("alloy-internal-unsubscribe-0".to_owned())));
        manager.insert(in_flight_with_id(Id::String("alloy-internal-unsubscribe-2".to_owned())));

        assert_eq!(manager.next_unused_id(), Id::String("alloy-internal-unsubscribe-1".to_owned()));
    }
}
