use crate::managers::InFlight;
use alloy_json_rpc::Id;
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

    /// Keep only the requests for which `keep` returns `true`.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&InFlight) -> bool) {
        self.reqs.retain(|_, req| keep(req));
    }

    /// Insert a new in-flight request.
    pub(crate) fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id().clone(), in_flight);
    }

    /// Remove the request with the given id, e.g. because its response arrived.
    pub(crate) fn remove(&mut self, id: &Id) -> Option<InFlight> {
        self.reqs.remove(id)
    }

    /// Drain all pending requests when the service terminates.
    pub(crate) fn drain(&mut self) -> impl Iterator<Item = InFlight> + '_ {
        self.reqs.drain().map(|(_, request)| request)
    }
}
