use crate::managers::InFlight;
use alloy_json_rpc::Id;
use alloy_primitives::map::HashMap;

/// An in-flight request, and the number of times it was replayed on a new backend.
#[derive(Debug)]
struct Pending {
    in_flight: InFlight,
    replays: u32,
}

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, Pending>,
}

impl RequestManager {
    /// Get the number of in-flight requests.
    pub(crate) fn len(&self) -> usize {
        self.reqs.len()
    }

    /// Returns `true` if no request is in flight.
    pub(crate) fn is_empty(&self) -> bool {
        self.reqs.is_empty()
    }

    /// Get an iterator over the in-flight requests.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &InFlight> {
        self.reqs.values().map(|pending| &pending.in_flight)
    }

    /// Keep only the requests for which `keep` returns `true`.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&InFlight) -> bool) {
        self.reqs.retain(|_, pending| keep(&pending.in_flight));
    }

    /// Insert a new in-flight request.
    pub(crate) fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id().clone(), Pending { in_flight, replays: 0 });
    }

    /// Remove the request with the given id, e.g. because its response arrived.
    pub(crate) fn remove(&mut self, id: &Id) -> Option<InFlight> {
        self.reqs.remove(id).map(|pending| pending.in_flight)
    }

    /// Count a replay of every request, then remove and return the requests that were replayed
    /// more than `max_replays` times.
    pub(crate) fn count_replays(&mut self, max_replays: u32) -> Vec<InFlight> {
        let exhausted = self
            .reqs
            .iter_mut()
            .filter_map(|(id, pending)| {
                pending.replays += 1;
                (pending.replays > max_replays).then(|| id.clone())
            })
            .collect::<Vec<_>>();
        exhausted.iter().filter_map(|id| self.remove(id)).collect()
    }

    /// Drain all pending requests when the service terminates.
    pub(crate) fn drain(&mut self) -> impl Iterator<Item = InFlight> + '_ {
        self.reqs.drain().map(|(_, pending)| pending.in_flight)
    }

    /// Get the request with the given id.
    pub(crate) fn get(&self, id: &Id) -> Option<&InFlight> {
        self.reqs.get(id).map(|pending| &pending.in_flight)
    }
}
